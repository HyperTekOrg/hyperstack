use crate::types::PumpfunToken;
use hyperstack_sdk::Entity;

pub struct PumpfunTokenEntity;

impl Entity for PumpfunTokenEntity {
    type Data = PumpfunToken;

    const NAME: &'static str = "PumpfunToken";

    fn state_view() -> &'static str {
        "PumpfunToken/state"
    }

    fn list_view() -> &'static str {
        "PumpfunToken/list"
    }
}
