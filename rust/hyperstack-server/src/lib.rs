//! # hyperstack-server
//!
//! WebSocket server and projection handlers for HyperStack streaming pipelines.
//!
//! This crate provides a builder API for creating HyperStack servers that:
//!
//! - Process Solana blockchain data via Yellowstone gRPC
//! - Transform data using the HyperStack VM
//! - Stream entity updates over WebSockets to connected clients
//! - Support multiple streaming modes (State, List, Append)
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use hyperstack_server::{Server, Spec};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     Server::builder()
//!         .spec(my_spec())
//!         .websocket()
//!         .bind("[::]:8877".parse()?)
//!         .health_monitoring()
//!         .start()
//!         .await
//! }
//! ```
//!
//! ## Feature Flags
//!
//! - `otel` - OpenTelemetry integration for metrics and distributed tracing

pub mod bus;
pub mod cache;
pub mod config;
pub mod health;
pub mod http_health;
#[cfg(feature = "otel")]
pub mod metrics;
pub mod projector;
pub mod runtime;
pub mod view;
pub mod websocket;

pub use bus::{BusManager, BusMessage};
pub use cache::{EntityCache, EntityCacheConfig};
pub use config::{
    HealthConfig, HttpHealthConfig, ServerConfig, WebSocketConfig, YellowstoneConfig,
};
pub use health::{HealthMonitor, StreamStatus};
pub use http_health::HttpHealthServer;
#[cfg(feature = "otel")]
pub use metrics::Metrics;
pub use projector::Projector;
pub use runtime::Runtime;
pub use view::{Delivery, Filters, Projection, ViewIndex, ViewSpec};
pub use websocket::{ClientInfo, ClientManager, Frame, Mode, Subscription, WebSocketServer};

use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;

/// Type alias for a parser setup function
/// This function receives a mutations sender and optional health monitor, then sets up the Vixen runtime
pub type ParserSetupFn = Arc<
    dyn Fn(
            tokio::sync::mpsc::Sender<smallvec::SmallVec<[hyperstack_interpreter::Mutation; 6]>>,
            Option<HealthMonitor>,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<()>> + Send>>
        + Send
        + Sync,
>;

/// Specification for a HyperStack server
/// Contains bytecode, parsers, and program information
pub struct Spec {
    pub bytecode: hyperstack_interpreter::compiler::MultiEntityBytecode,
    pub program_id: String,
    pub parser_setup: Option<ParserSetupFn>,
}

impl Spec {
    pub fn new(
        bytecode: hyperstack_interpreter::compiler::MultiEntityBytecode,
        program_id: impl Into<String>,
    ) -> Self {
        Self {
            bytecode,
            program_id: program_id.into(),
            parser_setup: None,
        }
    }

    /// Add a parser setup function that will configure Vixen parsers
    pub fn with_parser_setup(mut self, setup_fn: ParserSetupFn) -> Self {
        self.parser_setup = Some(setup_fn);
        self
    }
}

/// Main server interface with fluent builder API
pub struct Server;

impl Server {
    /// Create a new server builder
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }
}

/// Builder for configuring and creating a HyperStack server
pub struct ServerBuilder {
    spec: Option<Spec>,
    views: Option<ViewIndex>,
    config: ServerConfig,
    #[cfg(feature = "otel")]
    metrics: Option<Arc<Metrics>>,
}

impl ServerBuilder {
    fn new() -> Self {
        Self {
            spec: None,
            views: None,
            config: ServerConfig::new(),
            #[cfg(feature = "otel")]
            metrics: None,
        }
    }

    /// Set the specification (bytecode, parsers, program_id)
    pub fn spec(mut self, spec: Spec) -> Self {
        self.spec = Some(spec);
        self
    }

    /// Set custom view index
    pub fn views(mut self, views: ViewIndex) -> Self {
        self.views = Some(views);
        self
    }

    /// Enable metrics collection (requires 'otel' feature)
    #[cfg(feature = "otel")]
    pub fn metrics(mut self, metrics: Metrics) -> Self {
        self.metrics = Some(Arc::new(metrics));
        self
    }

    /// Enable WebSocket server with default configuration
    pub fn websocket(mut self) -> Self {
        self.config.websocket = Some(WebSocketConfig::default());
        self
    }

    /// Configure WebSocket server
    pub fn websocket_config(mut self, config: WebSocketConfig) -> Self {
        self.config.websocket = Some(config);
        self
    }

    /// Set the bind address for WebSocket server
    pub fn bind(mut self, addr: impl Into<SocketAddr>) -> Self {
        if let Some(ws_config) = &mut self.config.websocket {
            ws_config.bind_address = addr.into();
        } else {
            self.config.websocket = Some(WebSocketConfig::new(addr.into()));
        }
        self
    }

    /// Configure Yellowstone gRPC connection
    pub fn yellowstone(mut self, config: YellowstoneConfig) -> Self {
        self.config.yellowstone = Some(config);
        self
    }

    /// Enable health monitoring with default configuration
    pub fn health_monitoring(mut self) -> Self {
        self.config.health = Some(HealthConfig::default());
        self
    }

    /// Configure health monitoring
    pub fn health_config(mut self, config: HealthConfig) -> Self {
        self.config.health = Some(config);
        self
    }

    /// Enable HTTP health server with default configuration (port 8081)
    pub fn http_health(mut self) -> Self {
        self.config.http_health = Some(HttpHealthConfig::default());
        self
    }

    /// Configure HTTP health server
    pub fn http_health_config(mut self, config: HttpHealthConfig) -> Self {
        self.config.http_health = Some(config);
        self
    }

    /// Set the bind address for HTTP health server
    pub fn health_bind(mut self, addr: impl Into<SocketAddr>) -> Self {
        if let Some(http_config) = &mut self.config.http_health {
            http_config.bind_address = addr.into();
        } else {
            self.config.http_health = Some(HttpHealthConfig::new(addr.into()));
        }
        self
    }

    /// Start the server (consumes the builder)
    pub async fn start(self) -> Result<()> {
        // Use provided views or create default views from spec
        let view_index = self.views.unwrap_or_else(|| {
            let mut index = ViewIndex::new();

            if let Some(ref spec) = self.spec {
                for entity_name in spec.bytecode.entities.keys() {
                    index.add_spec(ViewSpec {
                        id: format!("{}/list", entity_name),
                        export: entity_name.clone(),
                        mode: Mode::List,
                        projection: Projection::all(),
                        filters: Filters::all(),
                        delivery: Delivery::default(),
                    });

                    index.add_spec(ViewSpec {
                        id: format!("{}/state", entity_name),
                        export: entity_name.clone(),
                        mode: Mode::State,
                        projection: Projection::all(),
                        filters: Filters::all(),
                        delivery: Delivery::default(),
                    });

                    index.add_spec(ViewSpec {
                        id: format!("{}/append", entity_name),
                        export: entity_name.clone(),
                        mode: Mode::Append,
                        projection: Projection::all(),
                        filters: Filters::all(),
                        delivery: Delivery::default(),
                    });

                    tracing::info!("Registered views for entity: {}", entity_name);
                }
            }

            index
        });

        // Create runtime
        #[cfg(feature = "otel")]
        let mut runtime = Runtime::new(self.config, view_index, self.metrics);
        #[cfg(not(feature = "otel"))]
        let mut runtime = Runtime::new(self.config, view_index);

        // Add spec if provided
        if let Some(spec) = self.spec {
            runtime = runtime.with_spec(spec);
        }

        // Run the server
        runtime.run().await
    }

    /// Build the runtime without starting it
    pub fn build(self) -> Result<Runtime> {
        // Use provided views or create default views from spec
        let view_index = self.views.unwrap_or_else(|| {
            let mut index = ViewIndex::new();

            if let Some(ref spec) = self.spec {
                for entity_name in spec.bytecode.entities.keys() {
                    index.add_spec(ViewSpec {
                        id: format!("{}/list", entity_name),
                        export: entity_name.clone(),
                        mode: Mode::List,
                        projection: Projection::all(),
                        filters: Filters::all(),
                        delivery: Delivery::default(),
                    });

                    index.add_spec(ViewSpec {
                        id: format!("{}/state", entity_name),
                        export: entity_name.clone(),
                        mode: Mode::State,
                        projection: Projection::all(),
                        filters: Filters::all(),
                        delivery: Delivery::default(),
                    });

                    index.add_spec(ViewSpec {
                        id: format!("{}/append", entity_name),
                        export: entity_name.clone(),
                        mode: Mode::Append,
                        projection: Projection::all(),
                        filters: Filters::all(),
                        delivery: Delivery::default(),
                    });
                }
            }

            index
        });

        #[cfg(feature = "otel")]
        let mut runtime = Runtime::new(self.config, view_index, self.metrics);
        #[cfg(not(feature = "otel"))]
        let mut runtime = Runtime::new(self.config, view_index);

        if let Some(spec) = self.spec {
            runtime = runtime.with_spec(spec);
        }
        Ok(runtime)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_pattern() {
        let _builder = Server::builder()
            .websocket()
            .bind("[::]:8877".parse::<SocketAddr>().unwrap());
    }

    #[test]
    fn test_spec_creation() {
        let bytecode = hyperstack_interpreter::compiler::MultiEntityBytecode::new().build();
        let spec = Spec::new(bytecode, "test_program");
        assert_eq!(spec.program_id, "test_program");
    }
}
