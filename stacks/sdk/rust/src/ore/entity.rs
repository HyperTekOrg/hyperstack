use super::types::OreRound;
use hyperstack_sdk::{Stack, StateView, ViewBuilder, ViewHandle, Views};

pub struct OreStack;

impl Stack for OreStack {
    type Views = OreStackViews;

    fn name() -> &'static str {
        "ore-round"
    }

    fn url() -> &'static str {
        "wss://ore.stack.usehyperstack.com"
    }
}

pub struct OreStackViews {
    pub ore_round: OreRoundEntityViews,
}

impl Views for OreStackViews {
    fn from_builder(builder: ViewBuilder) -> Self {
        Self {
            ore_round: OreRoundEntityViews { builder },
        }
    }
}

pub struct OreRoundEntityViews {
    builder: ViewBuilder,
}

impl OreRoundEntityViews {
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