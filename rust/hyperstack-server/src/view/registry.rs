use crate::sorted_cache::{SortOrder, SortedViewCache};
use crate::view::ViewSpec;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct ViewIndex {
    by_export: HashMap<String, Vec<ViewSpec>>,
    by_id: HashMap<String, ViewSpec>,
    sorted_caches: Arc<RwLock<HashMap<String, SortedViewCache>>>,
}

impl ViewIndex {
    pub fn new() -> Self {
        Self {
            by_export: HashMap::new(),
            by_id: HashMap::new(),
            sorted_caches: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn add_spec(&mut self, spec: ViewSpec) {
        self.by_export
            .entry(spec.export.clone())
            .or_default()
            .push(spec.clone());
        self.by_id.insert(spec.id.clone(), spec);
    }

    pub fn by_export(&self, entity: &str) -> &[ViewSpec] {
        self.by_export
            .get(entity)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    pub fn get_view(&self, id: &str) -> Option<&ViewSpec> {
        self.by_id.get(id)
    }

    pub fn get_derived_views(&self) -> Vec<&ViewSpec> {
        self.by_id.values().filter(|s| s.is_derived()).collect()
    }

    pub fn sorted_caches(&self) -> Arc<RwLock<HashMap<String, SortedViewCache>>> {
        self.sorted_caches.clone()
    }

    pub async fn init_sorted_cache(
        &self,
        view_id: &str,
        sort_field: Vec<String>,
        order: SortOrder,
    ) {
        let mut caches = self.sorted_caches.write().await;
        if !caches.contains_key(view_id) {
            caches.insert(
                view_id.to_string(),
                SortedViewCache::new(view_id.to_string(), sort_field, order),
            );
        }
    }
}

impl Default for ViewIndex {
    fn default() -> Self {
        Self::new()
    }
}
