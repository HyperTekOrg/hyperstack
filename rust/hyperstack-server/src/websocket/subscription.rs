use serde::{Deserialize, Serialize};

/// Client subscription to a specific view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub view: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition: Option<String>,
}

impl Subscription {
    pub fn matches_view(&self, view_id: &str) -> bool {
        self.view == view_id
    }

    pub fn matches_key(&self, key: &str) -> bool {
        self.key.as_ref().map_or(true, |k| k == key)
    }

    pub fn matches(&self, view_id: &str, key: &str) -> bool {
        self.matches_view(view_id) && self.matches_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_subscription_parse() {
        let json = json!({
            "view": "SettlementGame/kv",
            "key": "835"
        });

        let sub: Subscription = serde_json::from_value(json).unwrap();
        assert_eq!(sub.view, "SettlementGame/kv");
        assert_eq!(sub.key, Some("835".to_string()));
    }

    #[test]
    fn test_subscription_no_key() {
        let json = json!({
            "view": "SettlementGame/list"
        });

        let sub: Subscription = serde_json::from_value(json).unwrap();
        assert_eq!(sub.view, "SettlementGame/list");
        assert!(sub.key.is_none());
    }

    #[test]
    fn test_subscription_matches() {
        let sub = Subscription {
            view: "SettlementGame/kv".to_string(),
            key: Some("835".to_string()),
            partition: None,
        };

        assert!(sub.matches("SettlementGame/kv", "835"));
        assert!(!sub.matches("SettlementGame/kv", "836"));
        assert!(!sub.matches("SettlementGame/list", "835"));
    }

    #[test]
    fn test_subscription_matches_all_keys() {
        let sub = Subscription {
            view: "SettlementGame/kv".to_string(),
            key: None,
            partition: None,
        };

        assert!(sub.matches("SettlementGame/kv", "835"));
        assert!(sub.matches("SettlementGame/kv", "836"));
        assert!(!sub.matches("SettlementGame/list", "835"));
    }
}
