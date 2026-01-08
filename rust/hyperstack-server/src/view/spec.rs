use crate::websocket::frame::Mode;

// # View System Architecture
// 
// The view system uses hierarchical View IDs instead of simple entity names,
// enabling sophisticated filtering and organization:
//
// ## View ID Structure
// - Basic views: `EntityName/mode` (e.g., `SettlementGame/kv`, `SettlementGame/list`)
// - Filtered views: `EntityName/mode/filter1/filter2/...` (e.g., `SettlementGame/list/active/large`)
//
// ## Subscription Model
// Clients subscribe using the full view ID:
// ```json
// {
//   "view": "SettlementGame/list/active/large"
// }
// ```
//
// ## Future Filter Examples
// - `SettlementGame/list/active/large` - Active games with large bets
// - `SettlementGame/list/user/123` - Games for specific user
// - `SettlementGame/kv/recent` - Recently created games only

#[derive(Clone, Debug)]
pub struct ViewSpec {
    pub id: String,
    pub export: String,
    pub mode: Mode,
    pub projection: Projection,
    pub filters: Filters,
    pub delivery: Delivery,
}

#[derive(Clone, Debug, Default)]
pub struct Projection {
    pub fields: Option<Vec<String>>,
}

impl Projection {
    pub fn all() -> Self {
        Self { fields: None }
    }

    pub fn apply(&self, mut data: serde_json::Value) -> serde_json::Value {
        if let Some(ref field_list) = self.fields {
            if let Some(obj) = data.as_object_mut() {
                obj.retain(|k, _| field_list.contains(&k.to_string()));
            }
        }
        data
    }
}

#[derive(Clone, Debug, Default)]
pub struct Filters {
    pub keys: Option<Vec<String>>,
}

impl Filters {
    pub fn all() -> Self {
        Self { keys: None }
    }

    pub fn matches(&self, key: &str) -> bool {
        match &self.keys {
            None => true,
            Some(keys) => keys.iter().any(|k| k == key),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Delivery {
    pub coalesce_ms: Option<u64>,
}
