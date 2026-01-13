use crate::bus::{BusManager, BusMessage};
use crate::view::{ViewIndex, ViewSpec};
use crate::websocket::frame::{Frame, Mode};
use bytes::Bytes;
use smallvec::SmallVec;
use std::sync::Arc;
#[cfg(feature = "otel")]
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::{debug, error};

#[cfg(feature = "otel")]
use crate::metrics::Metrics;

pub struct Projector {
    view_index: Arc<ViewIndex>,
    bus_manager: BusManager,
    mutations_rx: mpsc::Receiver<SmallVec<[hyperstack_interpreter::Mutation; 6]>>,
    #[cfg(feature = "otel")]
    metrics: Option<Arc<Metrics>>,
}

impl Projector {
    #[cfg(feature = "otel")]
    pub fn new(
        view_index: Arc<ViewIndex>,
        bus_manager: BusManager,
        mutations_rx: mpsc::Receiver<SmallVec<[hyperstack_interpreter::Mutation; 6]>>,
        metrics: Option<Arc<Metrics>>,
    ) -> Self {
        Self {
            view_index,
            bus_manager,
            mutations_rx,
            metrics,
        }
    }

    #[cfg(not(feature = "otel"))]
    pub fn new(
        view_index: Arc<ViewIndex>,
        bus_manager: BusManager,
        mutations_rx: mpsc::Receiver<SmallVec<[hyperstack_interpreter::Mutation; 6]>>,
    ) -> Self {
        Self {
            view_index,
            bus_manager,
            mutations_rx,
        }
    }

    pub async fn run(mut self) {
        debug!("Projector started");

        while let Some(mutations) = self.mutations_rx.recv().await {
            for mutation in mutations.iter() {
                #[cfg(feature = "otel")]
                let start = Instant::now();

                if let Err(e) = self.process_mutation(mutation).await {
                    error!("Failed to process mutation: {}", e);
                }

                #[cfg(feature = "otel")]
                if let Some(ref metrics) = self.metrics {
                    let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
                    metrics.record_projector_latency(latency_ms);
                    metrics.record_mutation_processed(&mutation.export);
                }
            }
        }

        debug!("Projector stopped");
    }

    async fn process_mutation(
        &self,
        mutation: &hyperstack_interpreter::Mutation,
    ) -> anyhow::Result<()> {
        let specs = self.view_index.by_export(&mutation.export);

        for spec in specs {
            if !self.should_process(spec, mutation) {
                continue;
            }

            let frame = self.create_frame(spec, mutation).await?;
            let key = mutation
                .key
                .as_str()
                .map(|s| s.to_string())
                .or_else(|| mutation.key.as_u64().map(|n| n.to_string()))
                .or_else(|| mutation.key.as_i64().map(|n| n.to_string()))
                .or_else(|| {
                    // Handle byte arrays by converting to hex string
                    mutation.key.as_array().and_then(|arr| {
                        let bytes: Vec<u8> = arr
                            .iter()
                            .filter_map(|v| v.as_u64().map(|n| n as u8))
                            .collect();
                        if bytes.len() == arr.len() {
                            Some(hex::encode(&bytes))
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or_else(|| mutation.key.to_string());

            let message = Arc::new(BusMessage {
                key: key.clone(),
                entity: spec.id.clone(),
                payload: Arc::new(Bytes::from(serde_json::to_vec(&frame)?)),
            });

            self.publish_frame(spec, mutation, message).await;

            #[cfg(feature = "otel")]
            if let Some(ref metrics) = self.metrics {
                let mode_str = match spec.mode {
                    Mode::Kv => "kv",
                    Mode::List => "list",
                    Mode::State => "state",
                    Mode::Append => "append",
                };
                metrics.record_frame_published(mode_str, &spec.export);
            }
        }

        Ok(())
    }

    fn should_process(&self, spec: &ViewSpec, mutation: &hyperstack_interpreter::Mutation) -> bool {
        let key = mutation
            .key
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| mutation.key.as_u64().map(|n| n.to_string()))
            .or_else(|| mutation.key.as_i64().map(|n| n.to_string()))
            .unwrap_or_else(|| mutation.key.to_string());

        spec.filters.matches(&key)
    }

    async fn create_frame(
        &self,
        spec: &ViewSpec,
        mutation: &hyperstack_interpreter::Mutation,
    ) -> anyhow::Result<Frame> {
        match spec.mode {
            Mode::Kv | Mode::State => self.create_kv_frame(spec, mutation),
            Mode::List => self.create_list_frame(spec, mutation),
            Mode::Append => self.create_append_frame(spec, mutation),
        }
    }

    fn create_kv_frame(
        &self,
        spec: &ViewSpec,
        mutation: &hyperstack_interpreter::Mutation,
    ) -> anyhow::Result<Frame> {
        let key = mutation
            .key
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| mutation.key.as_u64().map(|n| n.to_string()))
            .or_else(|| mutation.key.as_i64().map(|n| n.to_string()))
            .unwrap_or_else(|| mutation.key.to_string());

        let projected = spec.projection.apply(mutation.patch.clone());

        Ok(Frame {
            mode: spec.mode,
            export: spec.id.clone(),
            op: "patch",
            key,
            data: projected,
        })
    }

    fn create_list_frame(
        &self,
        spec: &ViewSpec,
        mutation: &hyperstack_interpreter::Mutation,
    ) -> anyhow::Result<Frame> {
        let key = mutation
            .key
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| mutation.key.as_u64().map(|n| n.to_string()))
            .or_else(|| mutation.key.as_i64().map(|n| n.to_string()))
            .unwrap_or_else(|| mutation.key.to_string());

        let projected = spec.projection.apply(mutation.patch.clone());

        let list_item = serde_json::json!({
            "id": key,
            "order": 0,
            "item": projected,
        });

        Ok(Frame {
            mode: spec.mode,
            export: spec.id.clone(),
            op: "patch",
            key: key.clone(),
            data: list_item,
        })
    }

    fn create_append_frame(
        &self,
        spec: &ViewSpec,
        mutation: &hyperstack_interpreter::Mutation,
    ) -> anyhow::Result<Frame> {
        self.create_kv_frame(spec, mutation)
    }

    async fn publish_frame(
        &self,
        spec: &ViewSpec,
        _mutation: &hyperstack_interpreter::Mutation,
        message: Arc<BusMessage>,
    ) {
        match spec.mode {
            Mode::State => {
                self.bus_manager
                    .publish_state(&spec.id, &message.key, message.payload.clone())
                    .await;
            }
            Mode::Kv | Mode::Append => {
                self.bus_manager.publish_kv(&spec.id, message).await;
            }
            Mode::List => {
                self.bus_manager.publish_list(&spec.id, message).await;
            }
        }
    }
}
