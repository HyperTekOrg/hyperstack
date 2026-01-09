//! OpenTelemetry metrics for HyperStack Server
//!
//! This module provides centralized metrics collection for monitoring
//! server health, performance, and business metrics.
//!
//! # Metric Categories
//!
//! ## Engineering Metrics
//! - WebSocket connection metrics (active, total, duration)
//! - Projector throughput (mutations processed, frames published)
//! - Stream health (status, events received, errors)
//!
//! ## Business Metrics
//! - Active subscriptions by view
//! - Events processed by program/type
//!
//! # Usage
//!
//! Initialize metrics once at server startup:
//! ```ignore
//! let metrics = Metrics::new("my_service_name");
//! ```
//!
//! Then pass the metrics instance to components that need it.

use opentelemetry::{
    global,
    metrics::{Counter, Histogram, Meter, UpDownCounter},
    KeyValue,
};

use std::time::Instant;

/// Central metrics container for HyperStack server components
#[derive(Clone)]
pub struct Metrics {
    #[allow(dead_code)]
    meter: Meter,

    // WebSocket metrics
    pub ws_connections_total: Counter<u64>,
    pub ws_connections_active: UpDownCounter<i64>,
    pub ws_messages_received: Counter<u64>,
    pub ws_messages_sent: Counter<u64>,
    pub ws_connection_duration: Histogram<f64>,
    pub ws_subscriptions_active: UpDownCounter<i64>,

    // Projector metrics
    pub projector_mutations_processed: Counter<u64>,
    pub projector_frames_published: Counter<u64>,
    pub projector_processing_latency: Histogram<f64>,

    // Stream/Parser metrics
    pub stream_events_received: Counter<u64>,
    pub stream_errors_total: Counter<u64>,

    // VM metrics (for interpreter)
    pub vm_instructions_executed: Counter<u64>,
    pub vm_events_processed: Counter<u64>,
    pub vm_event_processing_duration: Histogram<f64>,
    pub vm_mutations_emitted: Counter<u64>,
    pub vm_pda_cache_hits: Counter<u64>,
    pub vm_pda_cache_misses: Counter<u64>,
    pub vm_pending_queue_size: UpDownCounter<i64>,

    // Entity metrics (business)
    pub entities_active: UpDownCounter<i64>,
}

impl Metrics {
    /// Create a new Metrics instance with the given service name
    ///
    /// The service name is used as the meter name for all metrics.
    pub fn new(service_name: impl Into<std::borrow::Cow<'static, str>>) -> Self {
        let meter = global::meter(service_name);

        // WebSocket metrics
        let ws_connections_total = meter
            .u64_counter("hyperstack.ws.connections.total")
            .with_description("Total number of WebSocket connections")
            .init();

        let ws_connections_active = meter
            .i64_up_down_counter("hyperstack.ws.connections.active")
            .with_description("Number of active WebSocket connections")
            .init();

        let ws_messages_received = meter
            .u64_counter("hyperstack.ws.messages.received")
            .with_description("Total WebSocket messages received from clients")
            .init();

        let ws_messages_sent = meter
            .u64_counter("hyperstack.ws.messages.sent")
            .with_description("Total WebSocket messages sent to clients")
            .init();

        let ws_connection_duration = meter
            .f64_histogram("hyperstack.ws.connection.duration")
            .with_description("Duration of WebSocket connections in seconds")
            .init();

        let ws_subscriptions_active = meter
            .i64_up_down_counter("hyperstack.ws.subscriptions.active")
            .with_description("Number of active subscriptions by view")
            .init();

        // Projector metrics
        let projector_mutations_processed = meter
            .u64_counter("hyperstack.projector.mutations.processed")
            .with_description("Total mutations processed by the projector")
            .init();

        let projector_frames_published = meter
            .u64_counter("hyperstack.projector.frames.published")
            .with_description("Total frames published by mode")
            .init();

        let projector_processing_latency = meter
            .f64_histogram("hyperstack.projector.latency")
            .with_description("Latency of mutation processing in milliseconds")
            .init();

        // Stream metrics
        let stream_events_received = meter
            .u64_counter("hyperstack.stream.events.received")
            .with_description("Total events received from the stream")
            .init();

        let stream_errors_total = meter
            .u64_counter("hyperstack.stream.errors.total")
            .with_description("Total stream errors")
            .init();

        // VM metrics
        let vm_instructions_executed = meter
            .u64_counter("hyperstack.vm.instructions.executed")
            .with_description("Total VM instructions executed")
            .init();

        let vm_events_processed = meter
            .u64_counter("hyperstack.vm.events.processed")
            .with_description("Total events processed by the VM")
            .init();

        let vm_event_processing_duration = meter
            .f64_histogram("hyperstack.vm.event.duration")
            .with_description("Duration of event processing in the VM in milliseconds")
            .init();

        let vm_mutations_emitted = meter
            .u64_counter("hyperstack.vm.mutations.emitted")
            .with_description("Total mutations emitted by the VM")
            .init();

        let vm_pda_cache_hits = meter
            .u64_counter("hyperstack.vm.pda_cache.hits")
            .with_description("PDA reverse lookup cache hits")
            .init();

        let vm_pda_cache_misses = meter
            .u64_counter("hyperstack.vm.pda_cache.misses")
            .with_description("PDA reverse lookup cache misses")
            .init();

        let vm_pending_queue_size = meter
            .i64_up_down_counter("hyperstack.vm.pending_queue.size")
            .with_description("Size of the pending account updates queue")
            .init();

        // Business metrics
        let entities_active = meter
            .i64_up_down_counter("hyperstack.entities.active")
            .with_description("Number of active entities being tracked")
            .init();

        Self {
            meter,
            ws_connections_total,
            ws_connections_active,
            ws_messages_received,
            ws_messages_sent,
            ws_connection_duration,
            ws_subscriptions_active,
            projector_mutations_processed,
            projector_frames_published,
            projector_processing_latency,
            stream_events_received,
            stream_errors_total,
            vm_instructions_executed,
            vm_events_processed,
            vm_event_processing_duration,
            vm_mutations_emitted,
            vm_pda_cache_hits,
            vm_pda_cache_misses,
            vm_pending_queue_size,
            entities_active,
        }
    }

    // ==================== WebSocket Helpers ====================

    /// Record a new WebSocket connection
    pub fn record_ws_connection(&self) {
        self.ws_connections_total.add(1, &[]);
        self.ws_connections_active.add(1, &[]);
    }

    /// Record a WebSocket disconnection with duration
    pub fn record_ws_disconnection(&self, duration_secs: f64) {
        self.ws_connections_active.add(-1, &[]);
        self.ws_connection_duration.record(duration_secs, &[]);
    }

    /// Record a WebSocket message received
    pub fn record_ws_message_received(&self) {
        self.ws_messages_received.add(1, &[]);
    }

    /// Record a WebSocket message sent
    pub fn record_ws_message_sent(&self) {
        self.ws_messages_sent.add(1, &[]);
    }

    /// Record a subscription created for a view
    pub fn record_subscription_created(&self, view_id: &str) {
        self.ws_subscriptions_active
            .add(1, &[KeyValue::new("view_id", view_id.to_string())]);
    }

    /// Record a subscription removed for a view
    pub fn record_subscription_removed(&self, view_id: &str) {
        self.ws_subscriptions_active
            .add(-1, &[KeyValue::new("view_id", view_id.to_string())]);
    }

    // ==================== Projector Helpers ====================

    /// Record a mutation processed
    pub fn record_mutation_processed(&self, entity: &str) {
        self.projector_mutations_processed
            .add(1, &[KeyValue::new("entity", entity.to_string())]);
    }

    /// Record a frame published
    pub fn record_frame_published(&self, mode: &str, entity: &str) {
        self.projector_frames_published.add(
            1,
            &[
                KeyValue::new("mode", mode.to_string()),
                KeyValue::new("entity", entity.to_string()),
            ],
        );
    }

    /// Record projector processing latency in milliseconds
    pub fn record_projector_latency(&self, latency_ms: f64) {
        self.projector_processing_latency.record(latency_ms, &[]);
    }

    // ==================== Stream Helpers ====================

    /// Record an event received from the stream
    pub fn record_stream_event(&self, event_type: &str) {
        self.stream_events_received
            .add(1, &[KeyValue::new("event_type", event_type.to_string())]);
    }

    /// Record a stream error
    pub fn record_stream_error(&self, error_type: &str) {
        self.stream_errors_total
            .add(1, &[KeyValue::new("error_type", error_type.to_string())]);
    }

    // ==================== VM Helpers ====================

    /// Record VM instructions executed
    pub fn record_vm_instructions(&self, count: u64) {
        self.vm_instructions_executed.add(count, &[]);
    }

    /// Record a VM event processed
    pub fn record_vm_event(&self, event_type: &str, program_id: &str) {
        self.vm_events_processed.add(
            1,
            &[
                KeyValue::new("event_type", event_type.to_string()),
                KeyValue::new("program_id", program_id.to_string()),
            ],
        );
    }

    /// Record VM event processing duration in milliseconds
    pub fn record_vm_event_duration(&self, duration_ms: f64, event_type: &str) {
        self.vm_event_processing_duration.record(
            duration_ms,
            &[KeyValue::new("event_type", event_type.to_string())],
        );
    }

    /// Record mutations emitted by the VM
    pub fn record_vm_mutations(&self, count: u64, entity: &str) {
        self.vm_mutations_emitted
            .add(count, &[KeyValue::new("entity", entity.to_string())]);
    }

    /// Record a PDA cache hit
    pub fn record_pda_cache_hit(&self) {
        self.vm_pda_cache_hits.add(1, &[]);
    }

    /// Record a PDA cache miss
    pub fn record_pda_cache_miss(&self) {
        self.vm_pda_cache_misses.add(1, &[]);
    }

    /// Update the pending queue size
    pub fn update_pending_queue_size(&self, delta: i64) {
        self.vm_pending_queue_size.add(delta, &[]);
    }

    // ==================== Business Helpers ====================

    /// Record an entity being tracked
    pub fn record_entity_active(&self, entity_name: &str) {
        self.entities_active
            .add(1, &[KeyValue::new("entity", entity_name.to_string())]);
    }

    /// Record an entity no longer being tracked
    pub fn record_entity_inactive(&self, entity_name: &str) {
        self.entities_active
            .add(-1, &[KeyValue::new("entity", entity_name.to_string())]);
    }
}

/// Timer guard for automatic duration recording
pub struct MetricsTimer {
    start: Instant,
    histogram: Histogram<f64>,
    attributes: Vec<KeyValue>,
}

impl MetricsTimer {
    /// Create a new timer for recording duration
    pub fn new(histogram: Histogram<f64>, attributes: Vec<KeyValue>) -> Self {
        Self {
            start: Instant::now(),
            histogram,
            attributes,
        }
    }

    /// Stop the timer and record the duration in milliseconds
    pub fn stop(self) -> f64 {
        let duration_ms = self.start.elapsed().as_secs_f64() * 1000.0;
        self.histogram.record(duration_ms, &self.attributes);
        duration_ms
    }
}

impl Drop for MetricsTimer {
    fn drop(&mut self) {
        // Timer is recorded on explicit stop() call, not on drop
        // This allows for optional recording
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        // Just verify that metrics can be created without panic
        let _metrics = Metrics::new("test_service");
    }
}
