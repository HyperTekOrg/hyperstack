//! spec() function generation for hyperstack-server integration.
//!
//! Generates the spec() function and associated runtime setup code.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generate the spec() function for hyperstack-server integration.
pub fn generate_spec_function(
    state_enum_name: &str,
    instruction_enum_name: &str,
    program_name: &str,
) -> TokenStream {
    let _state_enum = format_ident!("{}", state_enum_name);
    let _instruction_enum = format_ident!("{}", instruction_enum_name);

    quote! {
        pub fn spec() -> hyperstack::runtime::hyperstack_server::Spec {
            let bytecode = create_multi_entity_bytecode();
            let program_id = parsers::PROGRAM_ID_STR.to_string();
            let views = get_view_definitions();

            hyperstack::runtime::hyperstack_server::Spec::new(bytecode, program_id)
                .with_parser_setup(create_parser_setup())
                .with_views(views)
        }

        fn create_parser_setup() -> hyperstack::runtime::hyperstack_server::ParserSetupFn {
            use std::sync::Arc;

            Arc::new(|mutations_tx, health_monitor, reconnection_config| {
                Box::pin(async move {
                    run_vixen_runtime_with_channel(mutations_tx, health_monitor, reconnection_config).await
                })
            })
        }

        async fn run_vixen_runtime_with_channel(
            mutations_tx: hyperstack::runtime::tokio::sync::mpsc::Sender<hyperstack::runtime::smallvec::SmallVec<[hyperstack::runtime::hyperstack_interpreter::Mutation; 6]>>,
            health_monitor: Option<hyperstack::runtime::hyperstack_server::HealthMonitor>,
            reconnection_config: hyperstack::runtime::hyperstack_server::ReconnectionConfig,
        ) -> hyperstack::runtime::anyhow::Result<()> {
            use hyperstack::runtime::yellowstone_vixen::config::{BufferConfig, VixenConfig};
            use hyperstack::runtime::yellowstone_vixen_yellowstone_grpc_source::YellowstoneGrpcConfig;
            use hyperstack::runtime::yellowstone_vixen_yellowstone_grpc_source::YellowstoneGrpcSource;
            use hyperstack::runtime::yellowstone_vixen::Pipeline;

            let _ = hyperstack::runtime::dotenvy::from_filename(".env.local")
                .or_else(|_| hyperstack::runtime::dotenvy::from_filename(".env"))
                .or_else(|_| hyperstack::runtime::dotenvy::dotenv());

            let endpoint = std::env::var("YELLOWSTONE_ENDPOINT")
                .map_err(|_| hyperstack::runtime::anyhow::anyhow!(
                    "YELLOWSTONE_ENDPOINT environment variable must be set"
                ))?;
            let x_token = std::env::var("YELLOWSTONE_X_TOKEN").ok();

            let slot_tracker = hyperstack::runtime::hyperstack_server::SlotTracker::new();
            let mut attempt = 0u32;
            let mut backoff = reconnection_config.initial_delay;

            let bytecode = std::sync::Arc::new(create_multi_entity_bytecode());
            let vm = std::sync::Arc::new(std::sync::Mutex::new(hyperstack::runtime::hyperstack_interpreter::vm::VmContext::new()));

            loop {
                let from_slot = {
                    let last = slot_tracker.get();
                    if last > 0 { Some(last) } else { None }
                };

                if from_slot.is_some() {
                    hyperstack::runtime::tracing::info!("Resuming from slot {}", from_slot.unwrap());
                }

                let vixen_config = VixenConfig {
                    source: YellowstoneGrpcConfig {
                        endpoint: endpoint.clone(),
                        x_token: x_token.clone(),
                        timeout: 60,
                        commitment_level: None,
                        from_slot,
                        accept_compression: None,
                        max_decoding_message_size: None,
                    },
                    buffer: BufferConfig::default(),
                };

                let handler = VmHandler::new(
                    vm.clone(),
                    bytecode.clone(),
                    mutations_tx.clone(),
                    health_monitor.clone(),
                    slot_tracker.clone(),
                );

                let account_parser = parsers::AccountParser;
                let instruction_parser = parsers::InstructionParser;

                if attempt == 0 {
                    hyperstack::runtime::tracing::info!("Starting yellowstone-vixen runtime for {} program", #program_name);
                    hyperstack::runtime::tracing::info!("Program ID: {}", parsers::PROGRAM_ID_STR);
                }

                if let Some(ref health) = health_monitor {
                    health.record_reconnecting().await;
                }

                let account_pipeline = Pipeline::new(account_parser, [handler.clone()]);
                let instruction_pipeline = Pipeline::new(instruction_parser, [handler]);

                if let Some(ref health) = health_monitor {
                    health.record_connection().await;
                }

                let result = hyperstack::runtime::yellowstone_vixen::Runtime::<YellowstoneGrpcSource>::builder()
                    .account(account_pipeline)
                    .instruction(instruction_pipeline)
                    .build(vixen_config)
                    .try_run_async()
                    .await;

                if let Err(e) = result {
                    hyperstack::runtime::tracing::error!("Vixen runtime error: {:?}", e);
                }

                attempt += 1;

                if let Some(max) = reconnection_config.max_attempts {
                    if attempt >= max {
                        hyperstack::runtime::tracing::error!("Max reconnection attempts ({}) reached, giving up", max);
                        if let Some(ref health) = health_monitor {
                            health.record_error("Max reconnection attempts reached".into()).await;
                        }
                        return Err(hyperstack::runtime::anyhow::anyhow!("Max reconnection attempts reached"));
                    }
                }

                hyperstack::runtime::tracing::warn!(
                    "gRPC stream disconnected. Reconnecting in {:?} (attempt {})",
                    backoff,
                    attempt
                );

                if let Some(ref health) = health_monitor {
                    health.record_disconnection().await;
                }

                hyperstack::runtime::tokio::time::sleep(backoff).await;

                backoff = reconnection_config.next_backoff(backoff);
            }
        }
    }
}
