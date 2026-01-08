use crate::mutation::{Frame, Mode};
use crate::state::EntityStore;
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub view: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition: Option<String>,
}

pub struct HyperStackClient<T> {
    url: String,
    view: String,
    key: Option<String>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> HyperStackClient<T>
where
    T: Serialize + DeserializeOwned + Clone + Send + Sync + 'static,
{
    pub fn new(url: impl Into<String>, view: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            view: view.into(),
            key: None,
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub async fn connect(self) -> Result<EntityStore<T>> {
        let (ws, _) = connect_async(&self.url).await?;
        let (mut tx, mut rx) = ws.split();

        let subscription = Subscription {
            view: self.view.clone(),
            key: self.key,
            partition: None,
        };

        tx.send(Message::Text(serde_json::to_string(&subscription)?)).await?;

        let mode = infer_mode_from_view(&self.view);
        let store = EntityStore::new(mode);
        let store_ref = store.clone();

        let mut tx_clone = tx;
        tokio::spawn(async move {
            while let Some(Ok(msg)) = rx.next().await {
                match msg {
                    Message::Binary(bytes) => {
                        let text = String::from_utf8_lossy(&bytes);
                        if let Ok(frame) = serde_json::from_str::<Frame>(&text) {
                            match frame.op.as_str() {
                                "patch" => {
                                    store_ref.apply_patch(frame.key, frame.data).await;
                                }
                                "upsert" => {
                                    store_ref.apply_upsert(frame.key, frame.data).await;
                                }
                                "delete" => {
                                    store_ref.apply_delete(frame.key).await;
                                }
                                _ => {
                                    store_ref.apply_upsert(frame.key, frame.data).await;
                                }
                            }
                        }
                    }
                    Message::Ping(payload) => {
                        let _ = tx_clone.send(Message::Pong(payload)).await;
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
        });

        Ok(store)
    }
}

fn infer_mode_from_view(view: &str) -> Mode {
    if view.ends_with("/state") {
        Mode::State
    } else if view.ends_with("/list") {
        Mode::List
    } else if view.ends_with("/append") {
        Mode::Append
    } else {
        Mode::Kv
    }
}
