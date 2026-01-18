use crate::bus::{BusManager, BusMessage};
use crate::cache::EntityCache;
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
    entity_cache: EntityCache,
    mutations_rx: mpsc::Receiver<SmallVec<[hyperstack_interpreter::Mutation; 6]>>,
    #[cfg(feature = "otel")]
    metrics: Option<Arc<Metrics>>,
}

impl Projector {
    #[cfg(feature = "otel")]
    pub fn new(
        view_index: Arc<ViewIndex>,
        bus_manager: BusManager,
        entity_cache: EntityCache,
        mutations_rx: mpsc::Receiver<SmallVec<[hyperstack_interpreter::Mutation; 6]>>,
        metrics: Option<Arc<Metrics>>,
    ) -> Self {
        Self {
            view_index,
            bus_manager,
            entity_cache,
            mutations_rx,
            metrics,
        }
    }

    #[cfg(not(feature = "otel"))]
    pub fn new(
        view_index: Arc<ViewIndex>,
        bus_manager: BusManager,
        entity_cache: EntityCache,
        mutations_rx: mpsc::Receiver<SmallVec<[hyperstack_interpreter::Mutation; 6]>>,
    ) -> Self {
        Self {
            view_index,
            bus_manager,
            entity_cache,
            mutations_rx,
        }
    }

    pub async fn run(mut self) {
        debug!("Projector started");

        let mut json_buffer = Vec::with_capacity(4096);

        while let Some(mutations) = self.mutations_rx.recv().await {
            // Consume mutations by value to avoid cloning patch data
            for mutation in mutations.into_iter() {
                #[cfg(feature = "otel")]
                let start = Instant::now();

                // Capture export before consuming mutation
                #[cfg(feature = "otel")]
                let export = mutation.export.clone();

                if let Err(e) = self.process_mutation(mutation, &mut json_buffer).await {
                    error!("Failed to process mutation: {}", e);
                }

                #[cfg(feature = "otel")]
                if let Some(ref metrics) = self.metrics {
                    let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
                    metrics.record_projector_latency(latency_ms);
                    metrics.record_mutation_processed(&export);
                }
            }
        }

        debug!("Projector stopped");
    }

    async fn process_mutation(
        &self,
        mutation: hyperstack_interpreter::Mutation,
        json_buffer: &mut Vec<u8>,
    ) -> anyhow::Result<()> {
        let specs = self.view_index.by_export(&mutation.export);

        if specs.is_empty() {
            return Ok(());
        }

        let key = Self::extract_key(&mutation.key);
        let hyperstack_interpreter::Mutation {
            mut patch, append, ..
        } = mutation;

        let matching_specs: SmallVec<[&ViewSpec; 4]> = specs
            .iter()
            .filter(|spec| spec.filters.matches(&key))
            .collect();

        let match_count = matching_specs.len();
        if match_count == 0 {
            return Ok(());
        }

        for (i, spec) in matching_specs.into_iter().enumerate() {
            let is_last = i == match_count - 1;
            let patch_data = if is_last {
                std::mem::take(&mut patch)
            } else {
                patch.clone()
            };

            let projected = spec.projection.apply(patch_data);

            let frame = Frame {
                mode: spec.mode,
                export: spec.id.clone(),
                op: "patch",
                key: key.clone(),
                data: projected,
                append: append.clone(),
            };

            json_buffer.clear();
            serde_json::to_writer(&mut *json_buffer, &frame)?;
            let payload = Arc::new(Bytes::copy_from_slice(json_buffer));

            self.entity_cache
                .upsert_with_append(&spec.id, &key, frame.data.clone(), &frame.append)
                .await;

            let message = Arc::new(BusMessage {
                key: key.clone(),
                entity: spec.id.clone(),
                payload,
            });

            self.publish_frame(spec, message).await;

            #[cfg(feature = "otel")]
            if let Some(ref metrics) = self.metrics {
                let mode_str = match spec.mode {
                    Mode::List => "list",
                    Mode::State => "state",
                    Mode::Append => "append",
                };
                metrics.record_frame_published(mode_str, &spec.export);
            }
        }

        Ok(())
    }

    fn extract_key(key: &serde_json::Value) -> String {
        key.as_str()
            .map(|s| s.to_string())
            .or_else(|| key.as_u64().map(|n| n.to_string()))
            .or_else(|| key.as_i64().map(|n| n.to_string()))
            .or_else(|| {
                key.as_array().and_then(|arr| {
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
            .unwrap_or_else(|| key.to_string())
    }

    async fn publish_frame(&self, spec: &ViewSpec, message: Arc<BusMessage>) {
        match spec.mode {
            Mode::State => {
                self.bus_manager
                    .publish_state(&spec.id, &message.key, message.payload.clone())
                    .await;
            }
            Mode::List | Mode::Append => {
                self.bus_manager.publish_list(&spec.id, message).await;
            }
        }
    }
}
