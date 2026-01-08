use super::subscription::Subscription;
use anyhow::Result;
use bytes::Bytes;
use futures_util::stream::SplitSink;
use futures_util::SinkExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use tracing::{debug, info, warn};
use uuid::Uuid;

pub type WebSocketSender = SplitSink<WebSocketStream<TcpStream>, Message>;

/// Information about a connected client
#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub id: Uuid,
    pub subscription: Option<Subscription>,
    pub last_seen: SystemTime,
    pub sender: mpsc::Sender<Message>,
}

impl ClientInfo {
    pub fn new(id: Uuid, sender: mpsc::Sender<Message>) -> Self {
        Self {
            id,
            subscription: None,
            last_seen: SystemTime::now(),
            sender,
        }
    }

    pub fn update_last_seen(&mut self) {
        self.last_seen = SystemTime::now();
    }

    pub fn is_stale(&self, timeout: Duration) -> bool {
        self.last_seen.elapsed().unwrap_or(Duration::MAX) > timeout
    }
}

/// Manages all connected WebSocket clients
#[derive(Clone)]
pub struct ClientManager {
    clients: Arc<RwLock<HashMap<Uuid, ClientInfo>>>,
    client_timeout: Duration,
    message_queue_size: usize,
}

impl ClientManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            client_timeout: Duration::from_secs(300),
            message_queue_size: 1000,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.client_timeout = timeout;
        self
    }

    pub fn with_message_queue_size(mut self, queue_size: usize) -> Self {
        self.message_queue_size = queue_size;
        self
    }

    /// Add a new client connection
    pub async fn add_client(&self, client_id: Uuid, mut ws_sender: WebSocketSender) -> Result<()> {
        let (client_tx, mut client_rx) = mpsc::channel::<Message>(self.message_queue_size);

        let client_info = ClientInfo::new(client_id, client_tx);

        // Spawn task to handle WebSocket sending for this client
        let clients_ref = self.clients.clone();
        tokio::spawn(async move {
            // The client info struct is given client_tx. The below listens to this channel
            // This gives us clean decoupling and handles backpressure without blocking
            // We also get natural cleanup of failed clients without putting that complexity into
            // the ClientInfo struct
            while let Some(message) = client_rx.recv().await {
                if let Err(e) = ws_sender.send(message).await {
                    warn!("Failed to send message to client {}: {}", client_id, e);

                    // Remove failed client
                    clients_ref.write().await.remove(&client_id);
                    break;
                }
            }

            debug!("WebSocket sender for client {} stopped", client_id);
        });

        // Register client
        self.clients.write().await.insert(client_id, client_info);
        info!("Client {} registered", client_id);

        Ok(())
    }

    pub async fn remove_client(&self, client_id: Uuid) {
        if self.clients.write().await.remove(&client_id).is_some() {
            info!("Client {} removed", client_id);
        }
    }

    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }

    pub async fn send_to_client(&self, client_id: Uuid, data: Arc<Bytes>) -> Result<()> {
        let clients = self.clients.read().await;
        if let Some(client) = clients.get(&client_id) {
            let msg = Message::Binary((*data).clone());
            client.sender.send(msg).await?;
        }
        Ok(())
    }

    pub async fn update_subscription(&self, client_id: Uuid, subscription: Subscription) -> bool {
        if let Some(client_info) = self.clients.write().await.get_mut(&client_id) {
            client_info.subscription = Some(subscription);
            client_info.update_last_seen();
            debug!("Updated subscription for client {}", client_id);
            true
        } else {
            warn!(
                "Failed to update subscription for unknown client {}",
                client_id
            );
            false
        }
    }

    pub async fn update_client_last_seen(&self, client_id: Uuid) {
        if let Some(client_info) = self.clients.write().await.get_mut(&client_id) {
            client_info.update_last_seen();
        }
    }

    pub async fn get_subscription(&self, client_id: Uuid) -> Option<Subscription> {
        let clients = self.clients.read().await;
        clients.get(&client_id).and_then(|c| c.subscription.clone())
    }

    pub async fn cleanup_stale_clients(&self) -> usize {
        let mut clients = self.clients.write().await;
        let mut stale_clients = Vec::new();

        for (client_id, client_info) in clients.iter() {
            if client_info.is_stale(self.client_timeout) {
                stale_clients.push(*client_id);
            }
        }

        let removed_count = stale_clients.len();
        for client_id in stale_clients {
            clients.remove(&client_id);
            info!("Removed stale client {}", client_id);
        }

        removed_count
    }

    pub async fn start_cleanup_task(&self) {
        let client_manager = self.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;
                let removed = client_manager.cleanup_stale_clients().await;
                if removed > 0 {
                    info!("Cleaned up {} stale clients", removed);
                }
            }
        });
    }
}

impl Default for ClientManager {
    fn default() -> Self {
        Self::new()
    }
}
