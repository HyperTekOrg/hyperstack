use crate::view::ViewSpec;
use std::collections::HashMap;

#[derive(Clone)]
pub struct ViewIndex {
    by_export: HashMap<String, Vec<ViewSpec>>,
    by_id: HashMap<String, ViewSpec>,
}

impl ViewIndex {
    pub fn new() -> Self {
        Self {
            by_export: HashMap::new(),
            by_id: HashMap::new(),
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
}

impl Default for ViewIndex {
    fn default() -> Self {
        Self::new()
    }
}
