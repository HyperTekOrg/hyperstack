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
        /// Creates a hyperstack-server Spec with bytecode and parsers
        pub fn spec() -> hyperstack_server::Spec {
            let bytecode = create_multi_entity_bytecode();
            let program_id = parsers::PROGRAM_ID_STR.to_string();

            hyperstack_server::Spec::new(bytecode, program_id)
                .with_parser_setup(create_parser_setup())
        }

        /// Creates the parser setup function for Vixen runtime integration
        fn create_parser_setup() -> hyperstack_server::ParserSetupFn {
            use std::sync::Arc;

            Arc::new(|mutations_tx, health_monitor| {
                Box::pin(async move {
                    run_vixen_runtime_with_channel(mutations_tx, health_monitor).await
                })
            })
        }

        /// Runs the Vixen runtime and sends mutations to the provided channel
        async fn run_vixen_runtime_with_channel(
            mutations_tx: tokio::sync::mpsc::Sender<smallvec::SmallVec<[hyperstack_interpreter::Mutation; 6]>>,
            health_monitor: Option<hyperstack_server::HealthMonitor>,
        ) -> anyhow::Result<()> {
            use yellowstone_vixen::config::{BufferConfig, VixenConfig};
            use yellowstone_vixen_yellowstone_grpc_source::YellowstoneGrpcConfig;
            use yellowstone_vixen_yellowstone_grpc_source::YellowstoneGrpcSource;
            use yellowstone_vixen::Pipeline;

            // Try loading .env file from common locations
            let _ = dotenvy::from_filename(".env.local")
                .or_else(|_| dotenvy::from_filename(".env"))
                .or_else(|_| dotenvy::dotenv());

            let endpoint = std::env::var("YELLOWSTONE_ENDPOINT")
                .map_err(|_| anyhow::anyhow!(
                    "YELLOWSTONE_ENDPOINT environment variable must be set"
                ))?;
            let x_token = std::env::var("YELLOWSTONE_X_TOKEN").ok();

            let vixen_config = VixenConfig {
                source: YellowstoneGrpcConfig {
                    endpoint,
                    x_token,
                    timeout: 60,
                    commitment_level: None,
                    from_slot: None,
                    accept_compression: None,
                    max_decoding_message_size: None,
                },
                buffer: BufferConfig::default(),
            };

            // Create bytecode VM handler
            let bytecode = create_multi_entity_bytecode();
            let handler = VmHandler::new(bytecode, mutations_tx, health_monitor.clone());

            let account_parser = parsers::AccountParser;
            let instruction_parser = parsers::InstructionParser;

            tracing::info!("Starting yellowstone-vixen runtime for {} program", #program_name);
            tracing::info!("Program ID: {}", parsers::PROGRAM_ID_STR);

            // Record connection attempt
            if let Some(ref health) = health_monitor {
                health.record_reconnecting().await;
            }

            let account_pipeline = Pipeline::new(account_parser, [handler.clone()]);
            let instruction_pipeline = Pipeline::new(instruction_parser, [handler]);

            yellowstone_vixen::Runtime::<YellowstoneGrpcSource>::builder()
                .account(account_pipeline)
                .instruction(instruction_pipeline)
                .build(vixen_config)
                .run_async()
                .await;

            // Record disconnection
            if let Some(ref health) = health_monitor {
                health.record_disconnection().await;
            }

            Ok(())
        }
    }
}
