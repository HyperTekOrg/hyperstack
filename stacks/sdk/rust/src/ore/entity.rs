use super::types::OreRound;
use hyperstack_sdk::Entity;

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
