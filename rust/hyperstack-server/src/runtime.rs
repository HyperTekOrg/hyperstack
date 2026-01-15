use crate::bus::BusManager;
use crate::cache::EntityCache;
use crate::config::ServerConfig;
use crate::health::HealthMonitor;
use crate::http_health::HttpHealthServer;
use crate::projector::Projector;
use crate::view::ViewIndex;
use crate::websocket::WebSocketServer;
use crate::Spec;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

#[cfg(feature = "otel")]
use crate::metrics::Metrics;

/// Wait for shutdown signal (SIGINT on all platforms, SIGTERM on Unix)
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received SIGINT (Ctrl+C), initiating shutdown");
        }
        _ = terminate => {
            info!("Received SIGTERM, initiating graceful shutdown");
        }
    }
}

/// Runtime orchestrator that manages all server components
pub struct Runtime {
    config: ServerConfig,
    view_index: Arc<ViewIndex>,
    spec: Option<Spec>,
    #[cfg(feature = "otel")]
    metrics: Option<Arc<Metrics>>,
}

impl Runtime {
    #[cfg(feature = "otel")]
    pub fn new(config: ServerConfig, view_index: ViewIndex, metrics: Option<Arc<Metrics>>) -> Self {
        Self {
            config,
            view_index: Arc::new(view_index),
            spec: None,
            metrics,
        }
    }

    #[cfg(not(feature = "otel"))]
    pub fn new(config: ServerConfig, view_index: ViewIndex) -> Self {
        Self {
            config,
            view_index: Arc::new(view_index),
            spec: None,
        }
    }

    pub fn with_spec(mut self, spec: Spec) -> Self {
        self.spec = Some(spec);
        self
    }

    pub async fn run(self) -> Result<()> {
        info!("Starting HyperStack runtime");

        // Create bounded mutations channel for Transform Library -> Projector communication
        let (mutations_tx, mutations_rx) =
            mpsc::channel::<smallvec::SmallVec<[hyperstack_interpreter::Mutation; 6]>>(1024);

        let bus_manager = BusManager::new();
        let entity_cache = EntityCache::new();

        let health_monitor = if let Some(health_config) = &self.config.health {
            let monitor = HealthMonitor::new(health_config.clone());
            let _health_task = monitor.start().await;
            info!("Health monitoring enabled");
            Some(monitor)
        } else {
            None
        };

        #[cfg(feature = "otel")]
        let projector = Projector::new(
            self.view_index.clone(),
            bus_manager.clone(),
            entity_cache.clone(),
            mutations_rx,
            self.metrics.clone(),
        );
        #[cfg(not(feature = "otel"))]
        let projector = Projector::new(
            self.view_index.clone(),
            bus_manager.clone(),
            entity_cache.clone(),
            mutations_rx,
        );

        let projector_handle = tokio::spawn(async move {
            projector.run().await;
        });

        let ws_handle = if let Some(ws_config) = &self.config.websocket {
            #[cfg(feature = "otel")]
            let ws_server = WebSocketServer::new(
                ws_config.bind_address,
                bus_manager.clone(),
                entity_cache.clone(),
                self.view_index.clone(),
                self.metrics.clone(),
            );
            #[cfg(not(feature = "otel"))]
            let ws_server = WebSocketServer::new(
                ws_config.bind_address,
                bus_manager.clone(),
                entity_cache.clone(),
                self.view_index.clone(),
            );

            Some(tokio::spawn(async move {
                if let Err(e) = ws_server.start().await {
                    error!("WebSocket server error: {}", e);
                }
            }))
        } else {
            None
        };

        // Start Vixen parser/stream consumer (if spec with parser setup is provided)
        let parser_handle = if let Some(spec) = self.spec {
            if let Some(parser_setup) = spec.parser_setup {
                info!(
                    "Starting Vixen parser runtime for program: {}",
                    spec.program_id
                );
                let tx = mutations_tx.clone();
                let health = health_monitor.clone();
                Some(tokio::spawn(async move {
                    if let Err(e) = parser_setup(tx, health).await {
                        error!("Vixen parser runtime error: {}", e);
                    }
                }))
            } else {
                info!("Spec provided but no parser_setup configured - skipping Vixen runtime");
                None
            }
        } else {
            info!("No spec provided - running in websocket-only mode");
            None
        };

        // Start HTTP health server (if configured)
        let http_health_handle = if let Some(http_health_config) = &self.config.http_health {
            let mut http_server = HttpHealthServer::new(http_health_config.bind_address);
            if let Some(monitor) = health_monitor.clone() {
                http_server = http_server.with_health_monitor(monitor);
            }

            Some(tokio::spawn(async move {
                if let Err(e) = http_server.start().await {
                    error!("HTTP health server error: {}", e);
                }
            }))
        } else {
            None
        };

        info!("HyperStack runtime is running. Press Ctrl+C to stop.");

        // Wait for any task to complete (or handle shutdown signals)
        tokio::select! {
            _ = async {
                if let Some(handle) = ws_handle {
                    handle.await
                } else {
                    std::future::pending().await
                }
            } => {
                info!("WebSocket server task completed");
            }
            _ = projector_handle => {
                info!("Projector task completed");
            }
            _ = async {
                if let Some(handle) = parser_handle {
                    handle.await
                } else {
                    std::future::pending().await
                }
            } => {
                info!("Parser runtime task completed");
            }
            _ = async {
                if let Some(handle) = http_health_handle {
                    handle.await
                } else {
                    std::future::pending().await
                }
            } => {
                info!("HTTP health server task completed");
            }
            _ = shutdown_signal() => {}
        }

        info!("Shutting down HyperStack runtime");
        Ok(())
    }
}
