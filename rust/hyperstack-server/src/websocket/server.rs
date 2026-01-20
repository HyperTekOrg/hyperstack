use crate::bus::BusManager;
use crate::cache::EntityCache;
use crate::compression::maybe_compress;
use crate::view::ViewIndex;
use crate::websocket::client_manager::ClientManager;
use crate::websocket::frame::{Mode, SnapshotEntity, SnapshotFrame};
use crate::websocket::subscription::{ClientMessage, Subscription};
use anyhow::Result;
use futures_util::StreamExt;
use std::net::SocketAddr;
use std::sync::Arc;
#[cfg(feature = "otel")]
use std::time::Instant;

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::accept_async;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, info_span, warn, Instrument};
use uuid::Uuid;

#[cfg(feature = "otel")]
use crate::metrics::Metrics;

pub struct WebSocketServer {
    bind_addr: SocketAddr,
    client_manager: ClientManager,
    bus_manager: BusManager,
    entity_cache: EntityCache,
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
        entity_cache: EntityCache,
        view_index: Arc<ViewIndex>,
        metrics: Option<Arc<Metrics>>,
    ) -> Self {
        Self {
            bind_addr,
            client_manager: ClientManager::new(),
            bus_manager,
            entity_cache,
            view_index,
            max_clients: 10000,
            metrics,
        }
    }

    #[cfg(not(feature = "otel"))]
    pub fn new(
        bind_addr: SocketAddr,
        bus_manager: BusManager,
        entity_cache: EntityCache,
        view_index: Arc<ViewIndex>,
    ) -> Self {
        Self {
            bind_addr,
            client_manager: ClientManager::new(),
            bus_manager,
            entity_cache,
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

        self.client_manager.start_cleanup_task();

        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let client_count = self.client_manager.client_count();
                    if client_count >= self.max_clients {
                        warn!(
                            "Rejecting connection from {} - max clients ({}) reached",
                            addr, self.max_clients
                        );
                        drop(stream);
                        continue;
                    }

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
                    let entity_cache = self.entity_cache.clone();
                    let view_index = self.view_index.clone();
                    #[cfg(feature = "otel")]
                    let metrics = self.metrics.clone();

                    tokio::spawn(
                        async move {
                            #[cfg(feature = "otel")]
                            let result = handle_connection(
                                stream,
                                client_manager,
                                bus_manager,
                                entity_cache,
                                view_index,
                                metrics,
                            )
                            .await;
                            #[cfg(not(feature = "otel"))]
                            let result = handle_connection(
                                stream,
                                client_manager,
                                bus_manager,
                                entity_cache,
                                view_index,
                            )
                            .await;

                            if let Err(e) = result {
                                error!("WebSocket connection error: {}", e);
                            }
                        }
                        .instrument(info_span!("ws.connection", %addr)),
                    );
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
    entity_cache: EntityCache,
    view_index: Arc<ViewIndex>,
    metrics: Option<Arc<Metrics>>,
) -> Result<()> {
    let ws_stream = accept_async(stream).await?;
    let client_id = Uuid::new_v4();
    let connection_start = Instant::now();

    info!("WebSocket connection established for client {}", client_id);

    let (ws_sender, mut ws_receiver) = ws_stream.split();

    client_manager.add_client(client_id, ws_sender);

    let mut active_subscriptions: Vec<String> = Vec::new();

    loop {
        tokio::select! {
            ws_msg = ws_receiver.next() => {
                match ws_msg {
                    Some(Ok(msg)) => {
                        if msg.is_close() {
                            info!("Client {} requested close", client_id);
                            break;
                        }

                        client_manager.update_client_last_seen(client_id);

                        if msg.is_text() {
                            if let Some(ref m) = metrics {
                                m.record_ws_message_received();
                            }

                            if let Ok(text) = msg.to_text() {
                                debug!("Received text message from client {}: {}", client_id, text);

                                if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(text) {
                                    match client_msg {
                                        ClientMessage::Subscribe(subscription) => {
                                            let view_id = subscription.view.clone();
                                            let sub_key = subscription.sub_key();
                                            client_manager.update_subscription(client_id, subscription.clone());

                                            let cancel_token = CancellationToken::new();
                                            let is_new = client_manager.add_client_subscription(
                                                client_id,
                                                sub_key.clone(),
                                                cancel_token.clone(),
                                            ).await;

                                            if !is_new {
                                                debug!("Client {} already subscribed to {}, ignoring duplicate", client_id, sub_key);
                                                continue;
                                            }

                                            if let Some(ref m) = metrics {
                                                m.record_subscription_created(&view_id);
                                            }
                                            active_subscriptions.push(view_id);

                                            attach_client_to_bus(
                                                client_id,
                                                subscription,
                                                &client_manager,
                                                &bus_manager,
                                                &entity_cache,
                                                &view_index,
                                                cancel_token,
                                                metrics.clone(),
                                            ).await;
                                        }
                                        ClientMessage::Unsubscribe(unsub) => {
                                            let sub_key = unsub.sub_key();
                                            let removed = client_manager
                                                .remove_client_subscription(client_id, &sub_key)
                                                .await;

                                            if removed {
                                                info!("Client {} unsubscribed from {}", client_id, sub_key);
                                                if let Some(ref m) = metrics {
                                                    m.record_subscription_removed(&unsub.view);
                                                }
                                            }
                                        }
                                        ClientMessage::Ping => {
                                            debug!("Received ping from client {}", client_id);
                                        }
                                    }
                                } else if let Ok(subscription) = serde_json::from_str::<Subscription>(text) {
                                    let view_id = subscription.view.clone();
                                    let sub_key = subscription.sub_key();
                                    client_manager.update_subscription(client_id, subscription.clone());

                                    let cancel_token = CancellationToken::new();
                                    let is_new = client_manager.add_client_subscription(
                                        client_id,
                                        sub_key.clone(),
                                        cancel_token.clone(),
                                    ).await;

                                    if !is_new {
                                        debug!("Client {} already subscribed to {}, ignoring duplicate", client_id, sub_key);
                                        continue;
                                    }

                                    if let Some(ref m) = metrics {
                                        m.record_subscription_created(&view_id);
                                    }
                                    active_subscriptions.push(view_id);

                                    attach_client_to_bus(
                                        client_id,
                                        subscription,
                                        &client_manager,
                                        &bus_manager,
                                        &entity_cache,
                                        &view_index,
                                        cancel_token,
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

    client_manager
        .cancel_all_client_subscriptions(client_id)
        .await;
    client_manager.remove_client(client_id);

    if let Some(ref m) = metrics {
        let duration_secs = connection_start.elapsed().as_secs_f64();
        m.record_ws_disconnection(duration_secs);

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
    entity_cache: EntityCache,
    view_index: Arc<ViewIndex>,
) -> Result<()> {
    let ws_stream = accept_async(stream).await?;
    let client_id = Uuid::new_v4();

    info!("WebSocket connection established for client {}", client_id);

    let (ws_sender, mut ws_receiver) = ws_stream.split();

    client_manager.add_client(client_id, ws_sender);

    loop {
        tokio::select! {
            ws_msg = ws_receiver.next() => {
                match ws_msg {
                    Some(Ok(msg)) => {
                        if msg.is_close() {
                            info!("Client {} requested close", client_id);
                            break;
                        }

                        client_manager.update_client_last_seen(client_id);

                        if msg.is_text() {
                            if let Ok(text) = msg.to_text() {
                                debug!("Received text message from client {}: {}", client_id, text);

                                if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(text) {
                                    match client_msg {
                                        ClientMessage::Subscribe(subscription) => {
                                            let sub_key = subscription.sub_key();
                                            client_manager.update_subscription(client_id, subscription.clone());

                                            let cancel_token = CancellationToken::new();
                                            let is_new = client_manager.add_client_subscription(
                                                client_id,
                                                sub_key.clone(),
                                                cancel_token.clone(),
                                            ).await;

                                            if !is_new {
                                                debug!("Client {} already subscribed to {}, ignoring duplicate", client_id, sub_key);
                                                continue;
                                            }

                                            attach_client_to_bus(
                                                client_id,
                                                subscription,
                                                &client_manager,
                                                &bus_manager,
                                                &entity_cache,
                                                &view_index,
                                                cancel_token,
                                            ).await;
                                        }
                                        ClientMessage::Unsubscribe(unsub) => {
                                            let sub_key = unsub.sub_key();
                                            let removed = client_manager
                                                .remove_client_subscription(client_id, &sub_key)
                                                .await;

                                            if removed {
                                                info!("Client {} unsubscribed from {}", client_id, sub_key);
                                            }
                                        }
                                        ClientMessage::Ping => {
                                            debug!("Received ping from client {}", client_id);
                                        }
                                    }
                                } else if let Ok(subscription) = serde_json::from_str::<Subscription>(text) {
                                    let sub_key = subscription.sub_key();
                                    client_manager.update_subscription(client_id, subscription.clone());

                                    let cancel_token = CancellationToken::new();
                                    let is_new = client_manager.add_client_subscription(
                                        client_id,
                                        sub_key.clone(),
                                        cancel_token.clone(),
                                    ).await;

                                    if !is_new {
                                        debug!("Client {} already subscribed to {}, ignoring duplicate", client_id, sub_key);
                                        continue;
                                    }

                                    attach_client_to_bus(
                                        client_id,
                                        subscription,
                                        &client_manager,
                                        &bus_manager,
                                        &entity_cache,
                                        &view_index,
                                        cancel_token,
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

    client_manager
        .cancel_all_client_subscriptions(client_id)
        .await;
    client_manager.remove_client(client_id);
    info!("Client {} disconnected", client_id);

    Ok(())
}

#[cfg(feature = "otel")]
async fn attach_client_to_bus(
    client_id: Uuid,
    subscription: Subscription,
    client_manager: &ClientManager,
    bus_manager: &BusManager,
    entity_cache: &EntityCache,
    view_index: &ViewIndex,
    cancel_token: CancellationToken,
    metrics: Option<Arc<Metrics>>,
) {
    let view_id = &subscription.view;

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

            if !rx.borrow().is_empty() {
                let data = rx.borrow().clone();
                let _ = client_manager.send_to_client(client_id, data);
                if let Some(ref m) = metrics {
                    m.record_ws_message_sent();
                }
            }

            let client_mgr = client_manager.clone();
            let metrics_clone = metrics.clone();
            let view_id_clone = view_id.clone();
            let key_clone = key.to_string();
            tokio::spawn(
                async move {
                    loop {
                        tokio::select! {
                            _ = cancel_token.cancelled() => {
                                debug!("State subscription cancelled for client {}", client_id);
                                break;
                            }
                            result = rx.changed() => {
                                if result.is_err() {
                                    break;
                                }
                                let data = rx.borrow().clone();
                                if client_mgr.send_to_client(client_id, data).is_err() {
                                    break;
                                }
                                if let Some(ref m) = metrics_clone {
                                    m.record_ws_message_sent();
                                }
                            }
                        }
                    }
                }
                .instrument(info_span!("ws.subscribe.state", %client_id, view = %view_id_clone, key = %key_clone)),
            );
        }
        Mode::List | Mode::Append => {
            let mut rx = bus_manager.get_or_create_list_bus(view_id).await;

            let snapshots = entity_cache.get_all(view_id).await;
            let snapshot_entities: Vec<SnapshotEntity> = snapshots
                .into_iter()
                .filter(|(key, _)| subscription.matches_key(key))
                .map(|(key, data)| SnapshotEntity { key, data })
                .collect();

            if !snapshot_entities.is_empty() {
                let snapshot_frame = SnapshotFrame {
                    mode: view_spec.mode,
                    export: view_id.clone(),
                    op: "snapshot",
                    data: snapshot_entities,
                };
                if let Ok(json_payload) = serde_json::to_vec(&snapshot_frame) {
                    let payload = maybe_compress(&json_payload);
                    if client_manager
                        .send_compressed_async(client_id, payload)
                        .await
                        .is_err()
                    {
                        return;
                    }
                    if let Some(ref m) = metrics {
                        m.record_ws_message_sent();
                    }
                }
            }

            let client_mgr = client_manager.clone();
            let sub = subscription.clone();
            let metrics_clone = metrics.clone();
            let view_id_clone = view_id.clone();
            let mode = view_spec.mode;
            tokio::spawn(
                async move {
                    loop {
                        tokio::select! {
                            _ = cancel_token.cancelled() => {
                                debug!("List subscription cancelled for client {}", client_id);
                                break;
                            }
                            result = rx.recv() => {
                                match result {
                                    Ok(envelope) => {
                                        if sub.matches(&envelope.entity, &envelope.key) {
                                            if client_mgr
                                                .send_to_client(client_id, envelope.payload.clone())
                                                .is_err()
                                            {
                                                break;
                                            }
                                            if let Some(ref m) = metrics_clone {
                                                m.record_ws_message_sent();
                                            }
                                        }
                                    }
                                    Err(_) => break,
                                }
                            }
                        }
                    }
                }
                .instrument(info_span!("ws.subscribe.list", %client_id, view = %view_id_clone, mode = ?mode)),
            );
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
    entity_cache: &EntityCache,
    view_index: &ViewIndex,
    cancel_token: CancellationToken,
) {
    let view_id = &subscription.view;

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

            if !rx.borrow().is_empty() {
                let data = rx.borrow().clone();
                let _ = client_manager.send_to_client(client_id, data);
            }

            let client_mgr = client_manager.clone();
            let view_id_clone = view_id.clone();
            let key_clone = key.to_string();
            tokio::spawn(
                async move {
                    loop {
                        tokio::select! {
                            _ = cancel_token.cancelled() => {
                                debug!("State subscription cancelled for client {}", client_id);
                                break;
                            }
                            result = rx.changed() => {
                                if result.is_err() {
                                    break;
                                }
                                let data = rx.borrow().clone();
                                if client_mgr.send_to_client(client_id, data).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                }
                .instrument(info_span!("ws.subscribe.state", %client_id, view = %view_id_clone, key = %key_clone)),
            );
        }
        Mode::List | Mode::Append => {
            let mut rx = bus_manager.get_or_create_list_bus(view_id).await;

            let snapshots = entity_cache.get_all(view_id).await;
            let snapshot_entities: Vec<SnapshotEntity> = snapshots
                .into_iter()
                .filter(|(key, _)| subscription.matches_key(key))
                .map(|(key, data)| SnapshotEntity { key, data })
                .collect();

            if !snapshot_entities.is_empty() {
                let snapshot_frame = SnapshotFrame {
                    mode: view_spec.mode,
                    export: view_id.clone(),
                    op: "snapshot",
                    data: snapshot_entities,
                };
                if let Ok(json_payload) = serde_json::to_vec(&snapshot_frame) {
                    let payload = maybe_compress(&json_payload);
                    if client_manager
                        .send_compressed_async(client_id, payload)
                        .await
                        .is_err()
                    {
                        return;
                    }
                }
            }

            let client_mgr = client_manager.clone();
            let sub = subscription.clone();
            let view_id_clone = view_id.clone();
            let mode = view_spec.mode;
            tokio::spawn(
                async move {
                    loop {
                        tokio::select! {
                            _ = cancel_token.cancelled() => {
                                debug!("List subscription cancelled for client {}", client_id);
                                break;
                            }
                            result = rx.recv() => {
                                match result {
                                    Ok(envelope) => {
                                        if sub.matches(&envelope.entity, &envelope.key)
                                            && client_mgr
                                                .send_to_client(client_id, envelope.payload.clone())
                                                .is_err()
                                        {
                                            break;
                                        }
                                    }
                                    Err(_) => break,
                                }
                            }
                        }
                    }
                }
                .instrument(info_span!("ws.subscribe.list", %client_id, view = %view_id_clone, mode = ?mode)),
            );
        }
    }

    info!(
        "Client {} subscribed to {} (mode: {:?})",
        client_id, view_id, view_spec.mode
    );
}
