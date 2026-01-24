//! Unified Vixen runtime generation.
//!
//! This module consolidates VmHandler and runtime loop generation that was previously
//! duplicated across `vm_handler.rs`, `spec_fn.rs`, and `idl_vixen_gen.rs`.
//!
//! Key unification:
//! - Single VmHandler definition with MutationBatch + SlotContext
//! - Single runtime loop with configurable logging verbosity
//! - Config-driven generation for different code paths

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Configuration for runtime code generation.
#[derive(Debug, Clone, Default)]
pub struct RuntimeGenConfig {
    /// Include verbose bytecode logging at startup
    pub verbose_bytecode_logging: bool,
    /// Include parser registration logging
    pub verbose_parser_logging: bool,
    /// Include views in spec() function
    pub include_views: bool,
}

impl RuntimeGenConfig {
    /// Configuration for IDL-based generation (more verbose, includes views)
    pub fn for_idl() -> Self {
        Self {
            verbose_bytecode_logging: true,
            verbose_parser_logging: true,
            include_views: true,
        }
    }

    /// Configuration for generate_all path (minimal logging)
    pub fn for_generate_all() -> Self {
        Self {
            verbose_bytecode_logging: false,
            verbose_parser_logging: false,
            include_views: true,
        }
    }
}

/// Generate the VmHandler struct and its Handler trait implementations.
///
/// This is the single source of truth for VmHandler generation.
/// Uses MutationBatch with SlotContext for proper slot tracking.
pub fn generate_vm_handler(
    state_enum_name: &str,
    instruction_enum_name: &str,
    entity_name: &str,
) -> TokenStream {
    let state_enum = format_ident!("{}", state_enum_name);
    let instruction_enum = format_ident!("{}", instruction_enum_name);
    let entity_name_lit = entity_name;

    quote! {
        #[derive(Clone)]
        pub struct VmHandler {
            vm: std::sync::Arc<std::sync::Mutex<hyperstack::runtime::hyperstack_interpreter::vm::VmContext>>,
            bytecode: std::sync::Arc<hyperstack::runtime::hyperstack_interpreter::compiler::MultiEntityBytecode>,
            mutations_tx: hyperstack::runtime::tokio::sync::mpsc::Sender<hyperstack::runtime::hyperstack_server::MutationBatch>,
            health_monitor: Option<hyperstack::runtime::hyperstack_server::HealthMonitor>,
            slot_tracker: hyperstack::runtime::hyperstack_server::SlotTracker,
        }

        impl std::fmt::Debug for VmHandler {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("VmHandler")
                    .field("vm", &"<VmContext>")
                    .field("bytecode", &"<MultiEntityBytecode>")
                    .finish()
            }
        }

        impl VmHandler {
            pub fn new(
                vm: std::sync::Arc<std::sync::Mutex<hyperstack::runtime::hyperstack_interpreter::vm::VmContext>>,
                bytecode: std::sync::Arc<hyperstack::runtime::hyperstack_interpreter::compiler::MultiEntityBytecode>,
                mutations_tx: hyperstack::runtime::tokio::sync::mpsc::Sender<hyperstack::runtime::hyperstack_server::MutationBatch>,
                health_monitor: Option<hyperstack::runtime::hyperstack_server::HealthMonitor>,
                slot_tracker: hyperstack::runtime::hyperstack_server::SlotTracker,
            ) -> Self {
                Self {
                    vm,
                    bytecode,
                    mutations_tx,
                    health_monitor,
                    slot_tracker,
                }
            }

            #[inline]
            async fn send_mutations_with_context(&self, mutations: Vec<hyperstack::runtime::hyperstack_interpreter::Mutation>, slot: u64, ordering: u64) {
                if !mutations.is_empty() {
                    let slot_context = hyperstack::runtime::hyperstack_server::SlotContext::new(slot, ordering);
                    let batch = hyperstack::runtime::hyperstack_server::MutationBatch::with_slot_context(
                        hyperstack::runtime::smallvec::SmallVec::from_vec(mutations),
                        slot_context,
                    );
                    let _ = self.mutations_tx.send(batch).await;
                }
            }
        }

        impl hyperstack::runtime::yellowstone_vixen::Handler<parsers::#state_enum, hyperstack::runtime::yellowstone_vixen_core::AccountUpdate> for VmHandler {
            async fn handle(
                &self,
                value: &parsers::#state_enum,
                raw_update: &hyperstack::runtime::yellowstone_vixen_core::AccountUpdate,
            ) -> hyperstack::runtime::yellowstone_vixen::HandlerResult<()> {
                let slot = raw_update.slot;
                let account = raw_update.account.as_ref().unwrap();
                let write_version = account.write_version;
                let signature = hyperstack::runtime::bs58::encode(account.txn_signature.as_ref().unwrap()).into_string();

                if let Some(ref health) = self.health_monitor {
                    health.record_event().await;
                }

                let account_address = hyperstack::runtime::bs58::encode(&account.pubkey).into_string();

                let event_type = value.event_type();
                let mut event_value = value.to_value();

                if let Some(obj) = event_value.as_object_mut() {
                    obj.insert("__account_address".to_string(), hyperstack::runtime::serde_json::json!(account_address));
                }

                let resolver_result = {
                    let mut vm = self.vm.lock().unwrap();

                    if let Some(state_table) = vm.get_state_table_mut(0) {
                        let mut ctx = hyperstack::runtime::hyperstack_interpreter::resolvers::ResolveContext::new(
                            0,
                            slot,
                            signature.clone(),
                            &mut state_table.pda_reverse_lookups,
                        );

                        if let Some(resolver_fn) = get_resolver_for_account_type(event_type) {
                            resolver_fn(&account_address, &event_value, &mut ctx)
                        } else {
                            hyperstack::runtime::hyperstack_interpreter::resolvers::KeyResolution::Found(String::new())
                        }
                    } else {
                        hyperstack::runtime::hyperstack_interpreter::resolvers::KeyResolution::Found(String::new())
                    }
                };

                match resolver_result {
                    hyperstack::runtime::hyperstack_interpreter::resolvers::KeyResolution::Found(resolved_key) => {
                        if !resolved_key.is_empty() {
                            if let Some(obj) = event_value.as_object_mut() {
                                obj.insert("__resolved_primary_key".to_string(), hyperstack::runtime::serde_json::json!(resolved_key));
                            }
                        }
                    }
                    hyperstack::runtime::hyperstack_interpreter::resolvers::KeyResolution::QueueUntil(_discriminators) => {
                        let mut vm = self.vm.lock().unwrap();

                        let _ = vm.queue_account_update(
                            0,
                            hyperstack::runtime::hyperstack_interpreter::QueuedAccountUpdate {
                                pda_address: account_address.clone(),
                                account_type: event_type.to_string(),
                                account_data: event_value,
                                slot,
                                write_version,
                                signature,
                            },
                        );
                        return Ok(());
                    }
                    hyperstack::runtime::hyperstack_interpreter::resolvers::KeyResolution::Skip => {
                        return Ok(());
                    }
                }

                let mutations_result = {
                    let mut vm = self.vm.lock().unwrap();

                    let context = hyperstack::runtime::hyperstack_interpreter::UpdateContext::new_account(slot, signature.clone(), write_version);

                    vm.process_event(&self.bytecode, event_value, event_type, Some(&context), None)
                        .map_err(|e| e.to_string())
                };

                match mutations_result {
                    Ok(mutations) => {
                        self.slot_tracker.record(slot);
                        self.send_mutations_with_context(mutations, slot, write_version).await;
                        Ok(())
                    }
                    Err(e) => {
                        if let Some(ref health) = self.health_monitor {
                            health.record_error(format!("VM error for {}: {}", event_type, e)).await;
                        }
                        Ok(())
                    }
                }
            }
        }

        impl hyperstack::runtime::yellowstone_vixen::Handler<parsers::#instruction_enum, hyperstack::runtime::yellowstone_vixen_core::instruction::InstructionUpdate> for VmHandler {
            async fn handle(
                &self,
                value: &parsers::#instruction_enum,
                raw_update: &hyperstack::runtime::yellowstone_vixen_core::instruction::InstructionUpdate,
            ) -> hyperstack::runtime::yellowstone_vixen::HandlerResult<()> {
                let slot = raw_update.shared.slot;
                let txn_index = raw_update.shared.txn_index;
                let signature = hyperstack::runtime::bs58::encode(&raw_update.shared.signature).into_string();

                if let Some(ref health) = self.health_monitor {
                    health.record_event().await;
                }

                let static_keys_vec = &raw_update.accounts;
                let event_type = value.event_type();
                let event_value = value.to_value_with_accounts(static_keys_vec);

                let bytecode = self.bytecode.clone();
                let mutations_result = {
                    let mut vm = self.vm.lock().unwrap();

                    let context = hyperstack::runtime::hyperstack_interpreter::UpdateContext::new_instruction(slot, signature.clone(), txn_index);

                    let mut result = vm.process_event(&bytecode, event_value.clone(), event_type, Some(&context), None)
                        .map_err(|e| e.to_string());

                    if result.is_ok() {
                        let hooks = get_instruction_hooks(event_type);
                        if !hooks.is_empty() {
                            let accounts = event_value.get("accounts")
                                .and_then(|a| a.as_object())
                                .map(|obj| {
                                    obj.iter()
                                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                                        .collect::<std::collections::HashMap<String, String>>()
                                })
                                .unwrap_or_default();

                            let instruction_data = event_value.get("data").unwrap_or(&hyperstack::runtime::serde_json::Value::Null);

                            let timestamp = vm.current_context()
                                .map(|ctx| ctx.timestamp())
                                .unwrap_or_else(|| std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs() as i64);

                            // SAFETY: Carefully splitting mutable borrow into disjoint parts
                            let vm_ptr: *mut hyperstack::runtime::hyperstack_interpreter::vm::VmContext = &mut *vm as *mut hyperstack::runtime::hyperstack_interpreter::vm::VmContext;

                            let mut ctx = hyperstack::runtime::hyperstack_interpreter::resolvers::InstructionContext::with_metrics(
                                accounts,
                                0,
                                &mut *vm,
                                unsafe { (*vm_ptr).registers_mut() },
                                2,
                                unsafe { (*vm_ptr).path_cache() },
                                instruction_data,
                                Some(context.slot.unwrap_or(0)),
                                context.signature.clone(),
                                timestamp,
                            );

                            for hook_fn in hooks.iter() {
                                hook_fn(&mut ctx);
                            }

                            let pending_updates = ctx.take_pending_updates();

                            drop(ctx);

                            // Process pending account updates from instruction hooks
                            if !pending_updates.is_empty() {
                                for update in pending_updates {
                                    let resolved_key = vm.try_pda_reverse_lookup(0, "default_pda_lookup", &update.pda_address);

                                    let mut account_data = update.account_data;
                                    if let Some(key) = resolved_key {
                                        if let Some(obj) = account_data.as_object_mut() {
                                            obj.insert("__resolved_primary_key".to_string(), hyperstack::runtime::serde_json::json!(key));
                                        }
                                    }

                                    let update_context = hyperstack::runtime::hyperstack_interpreter::UpdateContext::new_account(
                                        update.slot,
                                        update.signature.clone(),
                                        update.write_version,
                                    );

                                    match vm.process_event(&bytecode, account_data, &update.account_type, Some(&update_context), None) {
                                        Ok(pending_mutations) => {
                                            if let Ok(ref mut mutations) = result {
                                                mutations.extend(pending_mutations);
                                            }
                                        }
                                        Err(_e) => {}
                                    }
                                }
                            }
                        }

                        // Periodic cleanup
                        if vm.instructions_executed % 1000 == 0 {
                            let _ = vm.cleanup_all_expired(0);
                            let stats = vm.get_memory_stats(0);
                            hyperstack::runtime::hyperstack_interpreter::vm_metrics::record_memory_stats(&stats, #entity_name_lit);
                        }
                    }

                    result
                };

                match mutations_result {
                    Ok(mutations) => {
                        self.slot_tracker.record(slot);
                        self.send_mutations_with_context(mutations, slot, txn_index as u64).await;
                        Ok(())
                    }
                    Err(e) => {
                        if let Some(ref health) = self.health_monitor {
                            health.record_error(format!("VM error for {}: {}", event_type, e)).await;
                        }
                        Ok(())
                    }
                }
            }
        }
    }
}

/// Generate the complete spec() function with runtime setup.
///
/// This consolidates the runtime loop generation that was previously duplicated
/// in `spec_fn.rs` and `idl_vixen_gen.rs`.
pub fn generate_spec_function(
    state_enum_name: &str,
    instruction_enum_name: &str,
    program_name: &str,
    config: &RuntimeGenConfig,
) -> TokenStream {
    let _state_enum = format_ident!("{}", state_enum_name);
    let _instruction_enum = format_ident!("{}", instruction_enum_name);

    let views_call = if config.include_views {
        quote! { .with_views(get_view_definitions()) }
    } else {
        quote! {}
    };

    let bytecode_logging = if config.verbose_bytecode_logging {
        quote! {
            hyperstack::runtime::tracing::info!("Bytecode Handler Details:");
            for (entity_name, entity_bytecode) in &bytecode.entities {
                hyperstack::runtime::tracing::info!("   Entity: {}", entity_name);
                for (event_type, handler_opcodes) in &entity_bytecode.handlers {
                    hyperstack::runtime::tracing::info!("      {} -> {} opcodes", event_type, handler_opcodes.len());
                }
            }
        }
    } else {
        quote! {}
    };

    let parser_logging = if config.verbose_parser_logging {
        quote! {
            hyperstack::runtime::tracing::info!("Registering parsers:");
            hyperstack::runtime::tracing::info!("   - Account Parser ID: {}", hyperstack::runtime::yellowstone_vixen_core::Parser::id(&account_parser));
            hyperstack::runtime::tracing::info!("   - Instruction Parser ID: {}", hyperstack::runtime::yellowstone_vixen_core::Parser::id(&instruction_parser));
        }
    } else {
        quote! {}
    };

    quote! {
        pub fn spec() -> hyperstack::runtime::hyperstack_server::Spec {
            let bytecode = create_multi_entity_bytecode();
            let program_id = parsers::PROGRAM_ID_STR.to_string();

            hyperstack::runtime::hyperstack_server::Spec::new(bytecode, program_id)
                .with_parser_setup(create_parser_setup())
                #views_call
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
            mutations_tx: hyperstack::runtime::tokio::sync::mpsc::Sender<hyperstack::runtime::hyperstack_server::MutationBatch>,
            health_monitor: Option<hyperstack::runtime::hyperstack_server::HealthMonitor>,
            reconnection_config: hyperstack::runtime::hyperstack_server::ReconnectionConfig,
        ) -> hyperstack::runtime::anyhow::Result<()> {
            use hyperstack::runtime::yellowstone_vixen::config::{BufferConfig, VixenConfig};
            use hyperstack::runtime::yellowstone_vixen_yellowstone_grpc_source::YellowstoneGrpcConfig;
            use hyperstack::runtime::yellowstone_vixen_yellowstone_grpc_source::YellowstoneGrpcSource;
            use hyperstack::runtime::yellowstone_vixen::Pipeline;
            use std::sync::{Arc, Mutex};

            // Load environment variables
            let env_loaded = hyperstack::runtime::dotenvy::from_filename(".env.local").is_ok()
                || hyperstack::runtime::dotenvy::from_filename(".env").is_ok()
                || hyperstack::runtime::dotenvy::dotenv().is_ok();

            if !env_loaded {
                hyperstack::runtime::tracing::warn!("No .env file found. Make sure environment variables are set.");
            }

            let endpoint = std::env::var("YELLOWSTONE_ENDPOINT")
                .map_err(|_| hyperstack::runtime::anyhow::anyhow!(
                    "YELLOWSTONE_ENDPOINT environment variable must be set.\n\
                     Example: export YELLOWSTONE_ENDPOINT=http://localhost:10000"
                ))?;
            let x_token = std::env::var("YELLOWSTONE_X_TOKEN").ok();

            let slot_tracker = hyperstack::runtime::hyperstack_server::SlotTracker::new();
            let mut attempt = 0u32;
            let mut backoff = reconnection_config.initial_delay;

            let bytecode = create_multi_entity_bytecode();

            #bytecode_logging

            let vm = Arc::new(Mutex::new(hyperstack::runtime::hyperstack_interpreter::vm::VmContext::new()));
            let bytecode_arc = Arc::new(bytecode);

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
                    bytecode_arc.clone(),
                    mutations_tx.clone(),
                    health_monitor.clone(),
                    slot_tracker.clone(),
                );

                let account_parser = parsers::AccountParser;
                let instruction_parser = parsers::InstructionParser;

                if attempt == 0 {
                    hyperstack::runtime::tracing::info!("Starting yellowstone-vixen runtime for {} program", #program_name);
                    hyperstack::runtime::tracing::info!("Program ID: {}", parsers::PROGRAM_ID_STR);
                    #parser_logging
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

/// Generate both VmHandler and spec function together.
///
/// This is a convenience function that combines `generate_vm_handler` and
/// `generate_spec_function` into a single output.
#[allow(dead_code)]
pub fn generate_runtime(
    state_enum_name: &str,
    instruction_enum_name: &str,
    entity_name: &str,
    config: &RuntimeGenConfig,
) -> TokenStream {
    let vm_handler = generate_vm_handler(state_enum_name, instruction_enum_name, entity_name);
    let spec_fn =
        generate_spec_function(state_enum_name, instruction_enum_name, entity_name, config);

    quote! {
        #vm_handler
        #spec_fn
    }
}
