use crate::frame::Operation;
use crate::store::StoreUpdate;
use futures_util::Stream;
use serde::de::DeserializeOwned;
use std::collections::HashSet;
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

#[derive(Debug, Clone)]
pub enum RichUpdate<T> {
    Created {
        key: String,
        data: T,
    },
    Updated {
        key: String,
        before: T,
        after: T,
        patch: Option<serde_json::Value>,
    },
    Deleted {
        key: String,
        last_known: Option<T>,
    },
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

    pub fn into_data(self) -> Option<T> {
        match self {
            Update::Upsert { data, .. } => Some(data),
            Update::Patch { data, .. } => Some(data),
            Update::Delete { .. } => None,
        }
    }

    pub fn has_data(&self) -> bool {
        matches!(self, Update::Upsert { .. } | Update::Patch { .. })
    }

    pub fn into_key(self) -> String {
        match self {
            Update::Upsert { key, .. } => key,
            Update::Patch { key, .. } => key,
            Update::Delete { key } => key,
        }
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> Update<U> {
        match self {
            Update::Upsert { key, data } => Update::Upsert { key, data: f(data) },
            Update::Patch { key, data } => Update::Patch { key, data: f(data) },
            Update::Delete { key } => Update::Delete { key },
        }
    }
}

impl<T> RichUpdate<T> {
    pub fn key(&self) -> &str {
        match self {
            RichUpdate::Created { key, .. } => key,
            RichUpdate::Updated { key, .. } => key,
            RichUpdate::Deleted { key, .. } => key,
        }
    }

    pub fn data(&self) -> Option<&T> {
        match self {
            RichUpdate::Created { data, .. } => Some(data),
            RichUpdate::Updated { after, .. } => Some(after),
            RichUpdate::Deleted { last_known, .. } => last_known.as_ref(),
        }
    }

    pub fn before(&self) -> Option<&T> {
        match self {
            RichUpdate::Created { .. } => None,
            RichUpdate::Updated { before, .. } => Some(before),
            RichUpdate::Deleted { last_known, .. } => last_known.as_ref(),
        }
    }

    pub fn into_data(self) -> Option<T> {
        match self {
            RichUpdate::Created { data, .. } => Some(data),
            RichUpdate::Updated { after, .. } => Some(after),
            RichUpdate::Deleted { last_known, .. } => last_known,
        }
    }

    pub fn is_created(&self) -> bool {
        matches!(self, RichUpdate::Created { .. })
    }

    pub fn is_updated(&self) -> bool {
        matches!(self, RichUpdate::Updated { .. })
    }

    pub fn is_deleted(&self) -> bool {
        matches!(self, RichUpdate::Deleted { .. })
    }

    pub fn patch(&self) -> Option<&serde_json::Value> {
        match self {
            RichUpdate::Updated { patch, .. } => patch.as_ref(),
            _ => None,
        }
    }

    pub fn has_patch_field(&self, field: &str) -> bool {
        self.patch()
            .and_then(|p| p.as_object())
            .map(|obj| obj.contains_key(field))
            .unwrap_or(false)
    }
}

#[derive(Clone)]
enum KeyFilter {
    None,
    Single(String),
    Multiple(HashSet<String>),
}

impl KeyFilter {
    fn matches(&self, key: &str) -> bool {
        match self {
            KeyFilter::None => true,
            KeyFilter::Single(k) => k == key,
            KeyFilter::Multiple(keys) => keys.contains(key),
        }
    }
}

pub struct EntityStream<T> {
    inner: BroadcastStream<StoreUpdate>,
    view: String,
    key_filter: KeyFilter,
    _marker: PhantomData<T>,
}

impl<T: DeserializeOwned + Clone + Send + 'static> EntityStream<T> {
    pub fn new(rx: broadcast::Receiver<StoreUpdate>, view: String) -> Self {
        Self {
            inner: BroadcastStream::new(rx),
            view,
            key_filter: KeyFilter::None,
            _marker: PhantomData,
        }
    }

    pub fn new_filtered(rx: broadcast::Receiver<StoreUpdate>, view: String, key: String) -> Self {
        Self {
            inner: BroadcastStream::new(rx),
            view,
            key_filter: KeyFilter::Single(key),
            _marker: PhantomData,
        }
    }

    pub fn new_multi_filtered(
        rx: broadcast::Receiver<StoreUpdate>,
        view: String,
        keys: HashSet<String>,
    ) -> Self {
        Self {
            inner: BroadcastStream::new(rx),
            view,
            key_filter: KeyFilter::Multiple(keys),
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

                    if !this.key_filter.matches(&update.key) {
                        continue;
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
                                    Err(e) => {
                                        tracing::warn!(
                                            key = %update.key,
                                            error = %e,
                                            "Patch failed to deserialize to full type, skipping"
                                        );
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                }
                Poll::Ready(Some(Err(_lagged))) => {
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

pub struct RichEntityStream<T> {
    inner: BroadcastStream<StoreUpdate>,
    view: String,
    key_filter: KeyFilter,
    _marker: PhantomData<T>,
}

impl<T: DeserializeOwned + Clone + Send + 'static> RichEntityStream<T> {
    pub fn new(rx: broadcast::Receiver<StoreUpdate>, view: String) -> Self {
        Self {
            inner: BroadcastStream::new(rx),
            view,
            key_filter: KeyFilter::None,
            _marker: PhantomData,
        }
    }

    pub fn new_filtered(rx: broadcast::Receiver<StoreUpdate>, view: String, key: String) -> Self {
        Self {
            inner: BroadcastStream::new(rx),
            view,
            key_filter: KeyFilter::Single(key),
            _marker: PhantomData,
        }
    }
}

impl<T: DeserializeOwned + Clone + Send + Unpin + 'static> Stream for RichEntityStream<T> {
    type Item = RichUpdate<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            match Pin::new(&mut this.inner).poll_next(cx) {
                Poll::Ready(Some(Ok(update))) => {
                    if update.view != this.view {
                        continue;
                    }

                    if !this.key_filter.matches(&update.key) {
                        continue;
                    }

                    let previous: Option<T> =
                        update.previous.and_then(|v| serde_json::from_value(v).ok());

                    match update.operation {
                        Operation::Delete => {
                            return Poll::Ready(Some(RichUpdate::Deleted {
                                key: update.key,
                                last_known: previous,
                            }));
                        }
                        Operation::Create => {
                            if let Some(data) = update.data {
                                if let Ok(typed) = serde_json::from_value::<T>(data) {
                                    return Poll::Ready(Some(RichUpdate::Created {
                                        key: update.key,
                                        data: typed,
                                    }));
                                }
                            }
                        }
                        Operation::Upsert | Operation::Patch => {
                            if let Some(data) = update.data {
                                match serde_json::from_value::<T>(data.clone()) {
                                    Ok(after) => {
                                        if let Some(before) = previous {
                                            return Poll::Ready(Some(RichUpdate::Updated {
                                                key: update.key,
                                                before,
                                                after,
                                                patch: update.patch,
                                            }));
                                        } else {
                                            return Poll::Ready(Some(RichUpdate::Created {
                                                key: update.key,
                                                data: after,
                                            }));
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            key = %update.key,
                                            error = %e,
                                            "Update failed to deserialize, skipping"
                                        );
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                }
                Poll::Ready(Some(Err(_lagged))) => {
                    tracing::warn!("RichEntityStream lagged behind, some messages were dropped");
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
