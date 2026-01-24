use hyperstack_sdk::{Entity, StateView, ViewBuilder, ViewHandle, Views};
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


pub struct OreRoundViews {
    builder: ViewBuilder,
}

impl Views for OreRoundViews {
    type Entity = OreRoundEntity;

    fn from_builder(builder: ViewBuilder) -> Self {
        Self { builder }
    }
}

impl OreRoundViews {
    pub fn state(&self) -> StateView<OreRound> {
        StateView::new(
            self.builder.connection().clone(),
            self.builder.store().clone(),
            "OreRound/state".to_string(),
            self.builder.initial_data_timeout(),
        )
    }

    pub fn list(&self) -> ViewHandle<OreRound> {
        self.builder.view("OreRound/list")
    }

    pub fn latest(&self) -> ViewHandle<OreRound> {
        self.builder.view("OreRound/latest")
    }
}
