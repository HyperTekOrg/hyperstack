use hyperstack_sdk::Entity;
use super::types::OreRound;

pub struct OreRoundEntity;

impl Entity for OreRoundEntity {
    type Data = OreRound;
    
    const NAME: &'static str = "OreRound";
    
    fn state_view() -> &'static str {
        "OreRound/state"
    }
    
    fn list_view() -> &'static str {
        "OreRound/list"
    }
}

/// Derived view identifiers
impl OreRoundEntity {
    /// Derived view: OreRound/latest (output: single)
    pub const LATEST_VIEW: &'static str = "OreRound/latest";
}
