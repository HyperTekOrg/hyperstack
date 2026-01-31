use super::types::PumpfunToken;
use hyperstack_sdk::{Stack, StateView, ViewBuilder, ViewHandle, Views};

pub struct PumpfunStack;

impl Stack for PumpfunStack {
    type Views = PumpfunStackViews;

    fn name() -> &'static str {
        "pumpfun-token"
    }

    fn url() -> &'static str {
        "wss://pumpfun.stack.usehyperstack.com"
    }
}

pub struct PumpfunStackViews {
    pub pumpfun_token: PumpfunTokenEntityViews,
}

impl Views for PumpfunStackViews {
    fn from_builder(builder: ViewBuilder) -> Self {
        Self {
            pumpfun_token: PumpfunTokenEntityViews { builder },
        }
    }
}

pub struct PumpfunTokenEntityViews {
    builder: ViewBuilder,
}

impl PumpfunTokenEntityViews {
    pub fn state(&self) -> StateView<PumpfunToken> {
        StateView::new(
            self.builder.connection().clone(),
            self.builder.store().clone(),
            "PumpfunToken/state".to_string(),
            self.builder.initial_data_timeout(),
        )
    }

    pub fn list(&self) -> ViewHandle<PumpfunToken> {
        self.builder.view("PumpfunToken/list")
    }
}