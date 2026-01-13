use crate::bus::BusManager;
use crate::view::ViewIndex;
use crate::websocket::client_manager::ClientManager;
use crate::websocket::frame::Mode;
use crate::websocket::subscription::Subscription;
use anyhow::Result;
use futures_util::StreamExt;
use std::net::SocketAddr;
use std::sync::Arc;
#[cfg(feature = "otel")]
use std::time::Instant;

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

#[cfg(feature = "otel")]
use crate::metrics::Metrics;

pub struct WebSocketServer {
    bind_addr: SocketAddr,
    client_manager: ClientManager,
    bus_manager: BusManager,
    view_index: Arc<ViewIndex>,
    max_clients: usize,
    #[cfg(feature = "otel")]
    metrics: Option<Arc<Metrics>>,
}

impl WebSocketServer {
    #[cfg(feature = "otel")]
    pub fn new(
        bind_addr: SocketAddr,
        bus_manager: BusManager,
        view_index: Arc<ViewIndex>,
        metrics: Option<Arc<Metrics>>,
    ) -> Self {
        Self {
            bind_addr,
            client_manager: ClientManager::new(),
            bus_manager,
            view_index,
            max_clients: 10000,
            metrics,
        }
    }

    #[cfg(not(feature = "otel"))]
    pub fn new(bind_addr: SocketAddr, bus_manager: BusManager, view_index: Arc<ViewIndex>) -> Self {
        Self {
            bind_addr,
            client_manager: ClientManager::new(),
            bus_manager,
            view_index,
            max_clients: 10000,
        }
    }

    pub fn with_max_clients(mut self, max_clients: usize) -> Self {
        self.max_clients = max_clients;
        self
    }

    pub async fn start(self) -> Result<()> {
        info!(
            "Starting WebSocket server on {} (max_clients: {})",
            self.bind_addr, self.max_clients
        );

        let listener = TcpListener::bind(&self.bind_addr).await?;
        info!("WebSocket server listening on {}", self.bind_addr);

        // Start cleanup task
        self.client_manager.start_cleanup_task().await;

        // Accept incoming connections
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    // Check if we've reached the maximum number of clients
                    let client_count = self.client_manager.client_count().await;
                    if client_count >= self.max_clients {
                        warn!(
                            "Rejecting connection from {} - max clients ({}) reached",
                            addr, self.max_clients
                        );
                        // Accept the connection but immediately close it
                        if let Ok(mut ws_stream) = accept_async(stream).await {
                            let _ = ws_stream.close(None).await;
                        }
                        continue;
                    }

                    // Record connection metric
                    #[cfg(feature = "otel")]
                    if let Some(ref metrics) = self.metrics {
                        metrics.record_ws_connection();
                    }

                    info!(
                        "New WebSocket connection from {} ({}/{} clients)",
                        addr,
                        client_count + 1,
                        self.max_clients
                    );
                    let client_manager = self.client_manager.clone();
                    let bus_manager = self.bus_manager.clone();
                    let view_index = self.view_index.clone();
                    #[cfg(feature = "otel")]
                    let metrics = self.metrics.clone();

                    tokio::spawn(async move {
                        #[cfg(feature = "otel")]
                        let result = handle_connection(
                            stream,
                            client_manager,
                            bus_manager,
                            view_index,
                            metrics,
                        )
                        .await;
                        #[cfg(not(feature = "otel"))]
                        let result =
                            handle_connection(stream, client_manager, bus_manager, view_index)
                                .await;

                        if let Err(e) = result {
                            error!("WebSocket connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}

#[cfg(feature = "otel")]
async fn handle_connection(
    stream: TcpStream,
    client_manager: ClientManager,
    bus_manager: BusManager,
    view_index: Arc<ViewIndex>,
    metrics: Option<Arc<Metrics>>,
) -> Result<()> {
    let ws_stream = accept_async(stream).await?;
    let client_id = Uuid::new_v4();
    let connection_start = Instant::now();

    info!("WebSocket connection established for client {}", client_id);

    let (ws_sender, mut ws_receiver) = ws_stream.split();

    // Register client
    client_manager.add_client(client_id, ws_sender).await?;

    // Track active subscriptions for this client (for cleanup)
    let mut active_subscriptions: Vec<String> = Vec::new();

    // Handle incoming messages from client
    loop {
        tokio::select! {
            ws_msg = ws_receiver.next() => {
                match ws_msg {
                    Some(Ok(msg)) => {
                        if msg.is_close() {
                            info!("Client {} requested close", client_id);
                            break;
                        }

                        client_manager.update_client_last_seen(client_id).await;

                        if msg.is_text() {
                            // Record message received metric
                            if let Some(ref m) = metrics {
                                m.record_ws_message_received();
                            }

                            if let Ok(text) = msg.to_text() {
                                debug!("Received text message from client {}: {}", client_id, text);

                                // Try to parse as subscription
                                if let Ok(subscription) = serde_json::from_str::<Subscription>(text) {
                                    let view_id = subscription.view.clone();
                                    client_manager.update_subscription(client_id, subscription.clone()).await;

                                    // Record subscription metric
                                    if let Some(ref m) = metrics {
                                        m.record_subscription_created(&view_id);
                                    }
                                    active_subscriptions.push(view_id);

                                    // Attach client to appropriate bus
                                    attach_client_to_bus(
                                        client_id,
                                        subscription,
                                        &client_manager,
                                        &bus_manager,
                                        &view_index,
                                        metrics.clone(),
                                    ).await;
                                } else {
                                    debug!("Received non-subscription message from client {}: {}", client_id, text);
                                }
                            }
                        }
                    }
                    Some(Err(e)) => {
                        warn!("WebSocket error for client {}: {}", client_id, e);
                        break;
                    }
                    None => {
                        debug!("WebSocket stream ended for client {}", client_id);
                        break;
                    }
                }
            }
        }
    }

    // Clean up client
    client_manager.remove_client(client_id).await;

    // Record disconnection metrics
    if let Some(ref m) = metrics {
        let duration_secs = connection_start.elapsed().as_secs_f64();
        m.record_ws_disconnection(duration_secs);

        // Clean up subscription metrics
        for view_id in active_subscriptions {
            m.record_subscription_removed(&view_id);
        }
    }

    info!("Client {} disconnected", client_id);

    Ok(())
}

#[cfg(not(feature = "otel"))]
async fn handle_connection(
    stream: TcpStream,
    client_manager: ClientManager,
    bus_manager: BusManager,
    view_index: Arc<ViewIndex>,
) -> Result<()> {
    let ws_stream = accept_async(stream).await?;
    let client_id = Uuid::new_v4();

    info!("WebSocket connection established for client {}", client_id);

    let (ws_sender, mut ws_receiver) = ws_stream.split();

    // Register client
    client_manager.add_client(client_id, ws_sender).await?;

    // Handle incoming messages from client
    loop {
        tokio::select! {
            ws_msg = ws_receiver.next() => {
                match ws_msg {
                    Some(Ok(msg)) => {
                        if msg.is_close() {
                            info!("Client {} requested close", client_id);
                            break;
                        }

                        client_manager.update_client_last_seen(client_id).await;

                        if msg.is_text() {
                            if let Ok(text) = msg.to_text() {
                                debug!("Received text message from client {}: {}", client_id, text);

                                // Try to parse as subscription
                                if let Ok(subscription) = serde_json::from_str::<Subscription>(text) {
                                    client_manager.update_subscription(client_id, subscription.clone()).await;

                                    // Attach client to appropriate bus
                                    attach_client_to_bus(client_id, subscription, &client_manager, &bus_manager, &view_index).await;
                                } else {
                                    debug!("Received non-subscription message from client {}: {}", client_id, text);
                                }
                            }
                        }
                    }
                    Some(Err(e)) => {
                        warn!("WebSocket error for client {}: {}", client_id, e);
                        break;
                    }
                    None => {
                        debug!("WebSocket stream ended for client {}", client_id);
                        break;
                    }
                }
            }
        }
    }

    // Clean up client
    client_manager.remove_client(client_id).await;
    info!("Client {} disconnected", client_id);

    Ok(())
}

#[cfg(feature = "otel")]
async fn attach_client_to_bus(
    client_id: Uuid,
    subscription: Subscription,
    client_manager: &ClientManager,
    bus_manager: &BusManager,
    view_index: &ViewIndex,
    metrics: Option<Arc<Metrics>>,
) {
    let view_id = &subscription.view;

    // Get the view spec to determine the mode
    let view_spec = match view_index.get_view(view_id) {
        Some(spec) => spec,
        None => {
            warn!("Unknown view ID: {}", view_id);
            return;
        }
    };

    match view_spec.mode {
        Mode::State => {
            let key = subscription.key.as_deref().unwrap_or("");
            let mut rx = bus_manager.get_or_create_state_bus(view_id, key).await;

            // Send current value immediately (latest-only semantics)
            if !rx.borrow().is_empty() {
                let data = rx.borrow().clone();
                let _ = client_manager.send_to_client(client_id, data).await;
                if let Some(ref m) = metrics {
                    m.record_ws_message_sent();
                }
            }

            // Spawn task to listen for updates
            let client_mgr = client_manager.clone();
            let metrics_clone = metrics.clone();
            tokio::spawn(async move {
                while rx.changed().await.is_ok() {
                    let data = rx.borrow().clone();
                    if client_mgr.send_to_client(client_id, data).await.is_err() {
                        break; // Client disconnected
                    }
                    if let Some(ref m) = metrics_clone {
                        m.record_ws_message_sent();
                    }
                }
            });
        }
        Mode::Kv | Mode::Append => {
            let mut rx = bus_manager.get_or_create_kv_bus(view_id).await;

            let client_mgr = client_manager.clone();
            let sub = subscription.clone();
            let metrics_clone = metrics.clone();
            tokio::spawn(async move {
                while let Ok(envelope) = rx.recv().await {
                    // Filter messages based on subscription
                    if sub.matches(&envelope.entity, &envelope.key) {
                        if client_mgr
                            .send_to_client(client_id, envelope.payload.clone())
                            .await
                            .is_err()
                        {
                            break; // Client disconnected
                        }
                        if let Some(ref m) = metrics_clone {
                            m.record_ws_message_sent();
                        }
                    }
                }
            });
        }
        Mode::List => {
            let mut rx = bus_manager.get_or_create_list_bus(view_id).await;

            let client_mgr = client_manager.clone();
            let sub = subscription.clone();
            let metrics_clone = metrics.clone();
            tokio::spawn(async move {
                while let Ok(envelope) = rx.recv().await {
                    // Filter messages based on subscription
                    if sub.matches(&envelope.entity, &envelope.key) {
                        if client_mgr
                            .send_to_client(client_id, envelope.payload.clone())
                            .await
                            .is_err()
                        {
                            break; // Client disconnected
                        }
                        if let Some(ref m) = metrics_clone {
                            m.record_ws_message_sent();
                        }
                    }
                }
            });
        }
    }

    info!(
        "Client {} subscribed to {} (mode: {:?})",
        client_id, view_id, view_spec.mode
    );
}

#[cfg(not(feature = "otel"))]
async fn attach_client_to_bus(
    client_id: Uuid,
    subscription: Subscription,
    client_manager: &ClientManager,
    bus_manager: &BusManager,
    view_index: &ViewIndex,
) {
    let view_id = &subscription.view;

    // Get the view spec to determine the mode
    let view_spec = match view_index.get_view(view_id) {
        Some(spec) => spec,
        None => {
            warn!("Unknown view ID: {}", view_id);
            return;
        }
    };

    match view_spec.mode {
        Mode::State => {
            let key = subscription.key.as_deref().unwrap_or("");
            let mut rx = bus_manager.get_or_create_state_bus(view_id, key).await;

            // Send current value immediately (latest-only semantics)
            if !rx.borrow().is_empty() {
                let data = rx.borrow().clone();
                let _ = client_manager.send_to_client(client_id, data).await;
            }

            // Spawn task to listen for updates
            let client_mgr = client_manager.clone();
            tokio::spawn(async move {
                while rx.changed().await.is_ok() {
                    let data = rx.borrow().clone();
                    if client_mgr.send_to_client(client_id, data).await.is_err() {
                        break; // Client disconnected
                    }
                }
            });
        }
        Mode::Kv | Mode::Append => {
            let mut rx = bus_manager.get_or_create_kv_bus(view_id).await;

            let client_mgr = client_manager.clone();
            let sub = subscription.clone();
            tokio::spawn(async move {
                while let Ok(envelope) = rx.recv().await {
                    // Filter messages based on subscription
                    if sub.matches(&envelope.entity, &envelope.key)
                        && client_mgr
                            .send_to_client(client_id, envelope.payload.clone())
                            .await
                            .is_err()
                    {
                        break; // Client disconnected
                    }
                }
            });
        }
        Mode::List => {
            let mut rx = bus_manager.get_or_create_list_bus(view_id).await;

            let client_mgr = client_manager.clone();
            let sub = subscription.clone();
            tokio::spawn(async move {
                while let Ok(envelope) = rx.recv().await {
                    // Filter messages based on subscription
                    if sub.matches(&envelope.entity, &envelope.key)
                        && client_mgr
                            .send_to_client(client_id, envelope.payload.clone())
                            .await
                            .is_err()
                    {
                        break; // Client disconnected
                    }
                }
            });
        }
    }

    info!(
        "Client {} subscribed to {} (mode: {:?})",
        client_id, view_id, view_spec.mode
    );
}
