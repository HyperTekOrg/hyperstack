use hyperstack_sdk::Entity;
use super::types::PumpfunToken;

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
