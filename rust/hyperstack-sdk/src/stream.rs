use crate::frame::Operation;
use crate::store::StoreUpdate;
use futures_util::Stream;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

#[derive(Debug, Clone)]
pub enum Update<T> {
    Upsert { key: String, data: T },
    Patch { key: String, data: T },
    Delete { key: String },
}

impl<T> Update<T> {
    pub fn key(&self) -> &str {
        match self {
            Update::Upsert { key, .. } => key,
            Update::Patch { key, .. } => key,
            Update::Delete { key } => key,
        }
    }

    pub fn data(&self) -> Option<&T> {
        match self {
            Update::Upsert { data, .. } => Some(data),
            Update::Patch { data, .. } => Some(data),
            Update::Delete { .. } => None,
        }
    }

    pub fn is_delete(&self) -> bool {
        matches!(self, Update::Delete { .. })
    }
}

pub struct EntityStream<T> {
    inner: BroadcastStream<StoreUpdate>,
    view: String,
    key_filter: Option<String>,
    _marker: PhantomData<T>,
}

impl<T: DeserializeOwned + Clone + Send + 'static> EntityStream<T> {
    pub fn new(rx: broadcast::Receiver<StoreUpdate>, view: String) -> Self {
        Self {
            inner: BroadcastStream::new(rx),
            view,
            key_filter: None,
            _marker: PhantomData,
        }
    }

    pub fn new_filtered(rx: broadcast::Receiver<StoreUpdate>, view: String, key: String) -> Self {
        Self {
            inner: BroadcastStream::new(rx),
            view,
            key_filter: Some(key),
            _marker: PhantomData,
        }
    }
}

impl<T: DeserializeOwned + Clone + Send + Unpin + 'static> Stream for EntityStream<T> {
    type Item = Update<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            match Pin::new(&mut this.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(update))) => {
                    if update.view != this.view {
                        continue;
                    }

                    if let Some(ref key_filter) = this.key_filter {
                        if &update.key != key_filter {
                            continue;
                        }
                    }

                    match update.operation {
                        Operation::Delete => {
                            return Poll::Ready(Some(Update::Delete { key: update.key }));
                        }
                        Operation::Upsert | Operation::Create => {
                            if let Some(data) = update.data {
                                if let Ok(typed) = serde_json::from_value::<T>(data) {
                                    return Poll::Ready(Some(Update::Upsert {
                                        key: update.key,
                                        data: typed,
                                    }));
                                }
                            }
                        }
                        Operation::Patch => {
                            if let Some(data) = update.data {
                                match serde_json::from_value::<T>(data) {
                                    Ok(typed) => {
                                        return Poll::Ready(Some(Update::Patch {
                                            key: update.key,
                                            data: typed,
                                        }));
                                    }
                                    Err(_) => {
                                        // Partial patches can't deserialize to full type - skip
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                }
                Poll::Ready(Some(Err(_lagged))) => {
                    // Receiver lagged behind - messages were dropped. Continue to next.
                    tracing::warn!("EntityStream lagged behind, some messages were dropped");
                    continue;
                }
                Poll::Ready(None) => {
                    return Poll::Ready(None);
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }
    }
}
