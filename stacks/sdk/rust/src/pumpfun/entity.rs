use hyperstack_sdk::{Entity, StateView, ViewBuilder, ViewHandle, Views};
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


pub struct PumpfunTokenViews {
    builder: ViewBuilder,
}

impl Views for PumpfunTokenViews {
    type Entity = PumpfunTokenEntity;

    fn from_builder(builder: ViewBuilder) -> Self {
        Self { builder }
    }
}

impl PumpfunTokenViews {
    pub fn state(&self) -> StateView<PumpfunToken> {
        StateView::new(
            self.builder.connection().clone(),
            self.builder.store().clone(),
            "PumpfunToken/state".to_string(),
            self.builder.initial_data_timeout(),
        )
    }

    pub fn list(&self) -> ViewHandle<PumpfunToken, false> {
        self.builder.collection("PumpfunToken/list")
    }
}
