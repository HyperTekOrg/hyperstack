//! IDL-based Vixen runtime generation.

#![allow(dead_code)]

use crate::parse::idl::*;
use crate::parse::{ResolverHookKind, ResolverHookSpec};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Path;

fn path_to_event_type(path: &Path, is_instruction: bool) -> String {
    let type_name = path
        .segments
        .last()
        .map(|seg| seg.ident.to_string())
        .unwrap_or_default();

    if is_instruction {
        format!("{}IxState", type_name)
    } else {
        format!("{}State", type_name)
    }
}

pub fn generate_resolver_registries(resolver_hooks: &[ResolverHookSpec]) -> TokenStream {
    let resolver_registry = generate_resolver_registry(resolver_hooks);
    let instruction_hook_registry = generate_instruction_hook_registry(resolver_hooks);

    quote! {
        #resolver_registry
        #instruction_hook_registry
    }
}

fn generate_resolver_registry(resolver_hooks: &[ResolverHookSpec]) -> TokenStream {
    let key_resolvers: Vec<_> = resolver_hooks
        .iter()
        .filter(|hook| matches!(hook.kind, ResolverHookKind::KeyResolver))
        .collect();

    if key_resolvers.is_empty() {
        return quote! {
            fn get_resolver_for_account_type(_account_type: &str) -> Option<fn(&str, &hyperstack::runtime::serde_json::Value, &mut hyperstack::runtime::hyperstack_interpreter::resolvers::ResolveContext) -> hyperstack::runtime::hyperstack_interpreter::resolvers::KeyResolution> {
                None
            }
        };
    }

    let resolver_arms = key_resolvers.iter().map(|hook| {
        let event_type = path_to_event_type(&hook.account_type_path, false);
        let fn_name = &hook.fn_name;

        quote! {
            #event_type => {
                Some(#fn_name)
            }
        }
    });

    quote! {
        fn get_resolver_for_account_type(account_type: &str) -> Option<fn(&str, &hyperstack::runtime::serde_json::Value, &mut hyperstack::runtime::hyperstack_interpreter::resolvers::ResolveContext) -> hyperstack::runtime::hyperstack_interpreter::resolvers::KeyResolution> {
            match account_type {
                #(#resolver_arms)*
                _ => None
            }
        }
    }
}

fn generate_instruction_hook_registry(resolver_hooks: &[ResolverHookSpec]) -> TokenStream {
    let instruction_hooks: Vec<_> = resolver_hooks
        .iter()
        .filter(|hook| matches!(hook.kind, ResolverHookKind::AfterInstruction))
        .collect();

    if instruction_hooks.is_empty() {
        return quote! {
            fn get_instruction_hooks(_instruction_type: &str) -> Vec<fn(&mut hyperstack::runtime::hyperstack_interpreter::resolvers::InstructionContext)> {
                Vec::new()
            }
        };
    }

    use std::collections::HashMap as StdHashMap;
    let mut hooks_by_instruction: StdHashMap<String, Vec<&syn::Ident>> = StdHashMap::new();

    for hook in &instruction_hooks {
        let event_type = path_to_event_type(&hook.account_type_path, true);
        hooks_by_instruction
            .entry(event_type)
            .or_default()
            .push(&hook.fn_name);
    }

    let hook_arms = hooks_by_instruction.iter().map(|(event_type, hook_fns)| {
        quote! {
            #event_type => {
                vec![#(#hook_fns),*]
            }
        }
    });

    quote! {
        fn get_instruction_hooks(instruction_type: &str) -> Vec<fn(&mut hyperstack::runtime::hyperstack_interpreter::resolvers::InstructionContext)> {
            match instruction_type {
                #(#hook_arms)*
                _ => Vec::new()
            }
        }
    }
}

pub fn generate_spec_function(
    idl: &IdlSpec,
    program_id: &str,
    resolver_hooks: &[ResolverHookSpec],
) -> TokenStream {
    let registries = generate_resolver_registries(resolver_hooks);
    let spec_fn = generate_spec_function_without_registries(idl, program_id);

    quote! {
        #registries
        #spec_fn
    }
}

pub fn generate_spec_function_without_registries(idl: &IdlSpec, _program_id: &str) -> TokenStream {
    let program_name = idl.get_name();
    let state_enum_name = format_ident!("{}State", to_pascal_case(program_name));
    let instruction_enum_name = format_ident!("{}Instruction", to_pascal_case(program_name));

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
            mutations_tx: hyperstack::runtime::tokio::sync::mpsc::Sender<hyperstack::runtime::hyperstack_server::MutationBatch>,
            health_monitor: Option<hyperstack::runtime::hyperstack_server::HealthMonitor>,
            reconnection_config: hyperstack::runtime::hyperstack_server::ReconnectionConfig,
        ) -> hyperstack::runtime::anyhow::Result<()> {
            use hyperstack::runtime::yellowstone_vixen::config::{BufferConfig, VixenConfig};
            use hyperstack::runtime::yellowstone_vixen_yellowstone_grpc_source::YellowstoneGrpcConfig;
            use hyperstack::runtime::yellowstone_vixen_yellowstone_grpc_source::YellowstoneGrpcSource;
            use hyperstack::runtime::yellowstone_vixen::Pipeline;
            use std::sync::{Arc, Mutex};

            let env_loaded = hyperstack::runtime::dotenvy::from_filename(".env.local").is_ok()
                || hyperstack::runtime::dotenvy::from_filename("backend/tenant-runtime/.env").is_ok()
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

            hyperstack::runtime::tracing::info!("Bytecode Handler Details:");
            for (entity_name, entity_bytecode) in &bytecode.entities {
                hyperstack::runtime::tracing::info!("   Entity: {}", entity_name);
                for (event_type, handler_opcodes) in &entity_bytecode.handlers {
                    hyperstack::runtime::tracing::info!("      {} -> {} opcodes", event_type, handler_opcodes.len());
                    if event_type == "BuyIxState" {
                        hyperstack::runtime::tracing::info!("         Opcode types:");
                        for (idx, opcode) in handler_opcodes.iter().enumerate() {
                            hyperstack::runtime::tracing::info!("            [{}] {:?}", idx, opcode);
                        }
                    }
                }
            }

            let vm = Arc::new(Mutex::new(hyperstack::runtime::hyperstack_interpreter::vm::VmContext::new()));
            let bytecode_arc = Arc::new(bytecode);

            #[derive(Clone)]
            struct VmHandler {
                vm: Arc<Mutex<hyperstack::runtime::hyperstack_interpreter::vm::VmContext>>,
                bytecode: Arc<hyperstack::runtime::hyperstack_interpreter::compiler::MultiEntityBytecode>,
                mutations_tx: hyperstack::runtime::tokio::sync::mpsc::Sender<hyperstack::runtime::hyperstack_server::MutationBatch>,
                health_monitor: Option<hyperstack::runtime::hyperstack_server::HealthMonitor>,
                slot_tracker: hyperstack::runtime::hyperstack_server::SlotTracker,
            }

            impl VmHandler {
                fn new(
                    vm: Arc<Mutex<hyperstack::runtime::hyperstack_interpreter::vm::VmContext>>,
                    bytecode: Arc<hyperstack::runtime::hyperstack_interpreter::compiler::MultiEntityBytecode>,
                    mutations_tx: hyperstack::runtime::tokio::sync::mpsc::Sender<hyperstack::runtime::hyperstack_server::MutationBatch>,
                    health_monitor: Option<hyperstack::runtime::hyperstack_server::HealthMonitor>,
                    slot_tracker: hyperstack::runtime::hyperstack_server::SlotTracker,
                ) -> Self {
                    Self { vm, bytecode, mutations_tx, health_monitor, slot_tracker }
                }
            }

            impl std::fmt::Debug for VmHandler {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.debug_struct("VmHandler")
                        .field("vm", &"<VmContext>")
                        .field("bytecode", &"<MultiEntityBytecode>")
                        .finish()
                }
            }

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

            impl hyperstack::runtime::yellowstone_vixen::Handler<parsers::#state_enum_name, hyperstack::runtime::yellowstone_vixen_core::AccountUpdate> for VmHandler {
                async fn handle(&self, value: &parsers::#state_enum_name, raw_update: &hyperstack::runtime::yellowstone_vixen_core::AccountUpdate)
                    -> hyperstack::runtime::yellowstone_vixen::HandlerResult<()>
                {
                    let slot = raw_update.slot;
                    let account = raw_update.account.as_ref().unwrap();
                    let write_version = account.write_version;
                    let signature = hyperstack::runtime::bs58::encode(account.txn_signature.as_ref().unwrap()).into_string();
                    let account_address = hyperstack::runtime::bs58::encode(&account.pubkey).into_string();
                    let event_type = value.event_type();

                    if let Some(ref health) = self.health_monitor {
                        health.record_event().await;
                    }

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
                            if !mutations.is_empty() {
                                let batch = hyperstack::runtime::hyperstack_server::MutationBatch::new(
                                    hyperstack::runtime::smallvec::SmallVec::from_vec(mutations)
                                );
                                let _ = self.mutations_tx.send(batch).await;
                            }
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

            impl hyperstack::runtime::yellowstone_vixen::Handler<parsers::#instruction_enum_name, hyperstack::runtime::yellowstone_vixen_core::instruction::InstructionUpdate> for VmHandler {
                async fn handle(&self, value: &parsers::#instruction_enum_name, raw_update: &hyperstack::runtime::yellowstone_vixen_core::instruction::InstructionUpdate)
                    -> hyperstack::runtime::yellowstone_vixen::HandlerResult<()>
                {
                    let slot = raw_update.shared.slot;
                    let txn_index = raw_update.shared.txn_index;
                    let signature = hyperstack::runtime::bs58::encode(&raw_update.shared.signature).into_string();
                    let event_type = value.event_type();

                    if let Some(ref health) = self.health_monitor {
                        health.record_event().await;
                    }

                    let static_keys_vec = &raw_update.accounts;
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

                                let dirty_fields: std::collections::HashSet<String> = ctx.dirty_tracker().dirty_paths();
                                let pending_updates = ctx.take_pending_updates();
                                drop(ctx);

                                if !dirty_fields.is_empty() {
                                    if let Ok(ref mutations) = result {
                                        if let Some(first_mutation) = mutations.first() {
                                            let _ = vm.update_state_from_register(0, first_mutation.key.clone(), 2);
                                        }
                                    }

                                    if let Ok(patch) = vm.extract_partial_state(2, &dirty_fields) {
                                        if let Some(mint) = event_value.get("accounts").and_then(|a| a.get("mint")).and_then(|m| m.as_str()) {
                                            if let Ok(ref mut mutations) = result {
                                                let mint_value = hyperstack::runtime::serde_json::Value::String(mint.to_string());
                                                let found = mutations.iter_mut()
                                                    .find(|m| m.key == mint_value)
                                                    .map(|m| {
                                                        if let hyperstack::runtime::serde_json::Value::Object(ref mut existing_patch_obj) = m.patch {
                                                            if let hyperstack::runtime::serde_json::Value::Object(new_patch_obj) = patch.clone() {
                                                                for (section_key, new_section_value) in new_patch_obj {
                                                                    if let Some(existing_section) = existing_patch_obj.get_mut(&section_key) {
                                                                        if let (Some(existing_obj), Some(new_obj)) =
                                                                            (existing_section.as_object_mut(), new_section_value.as_object()) {
                                                                            for (field_key, field_value) in new_obj {
                                                                                existing_obj.insert(field_key.clone(), field_value.clone());
                                                                            }
                                                                        } else {
                                                                            *existing_section = new_section_value.clone();
                                                                        }
                                                                    } else {
                                                                        existing_patch_obj.insert(section_key.clone(), new_section_value.clone());
                                                                    }
                                                                }

                                                                let mut full_state_for_eval = vm.registers_mut()[2].clone();
                                                                let _ = evaluate_computed_fields(&mut full_state_for_eval);
                                                                for path in computed_field_paths() {
                                                                    let parts: Vec<&str> = path.split('.').collect();
                                                                    if parts.len() >= 2 {
                                                                        let section = parts[0];
                                                                        let field = parts[1];
                                                                        if let Some(value) = full_state_for_eval
                                                                            .get(section)
                                                                            .and_then(|s| s.get(field))
                                                                        {
                                                                            if !existing_patch_obj.contains_key(section) {
                                                                                existing_patch_obj.insert(
                                                                                    section.to_string(),
                                                                                    hyperstack::runtime::serde_json::json!({})
                                                                                );
                                                                            }
                                                                            if let Some(patch_section) = existing_patch_obj.get_mut(section) {
                                                                                if let Some(obj) = patch_section.as_object_mut() {
                                                                                    obj.insert(field.to_string(), value.clone());
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    });

                                                if found.is_none() {
                                                    mutations.push(hyperstack::runtime::hyperstack_interpreter::Mutation {
                                                        export: "Token".to_string(),
                                                        key: mint_value,
                                                        patch: patch.clone(),
                                                        append: Vec::new(),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }

                                if !pending_updates.is_empty() {
                                    for update in pending_updates.into_iter() {
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

                                        if let Ok(pending_mutations) = vm.process_event(&bytecode, account_data, &update.account_type, Some(&update_context), None) {
                                            if !pending_mutations.is_empty() {
                                                if let Ok(ref mut mutations) = result {
                                                    mutations.extend(pending_mutations);
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            if vm.instructions_executed % 1000 == 0 {
                                let _ = vm.cleanup_all_expired(0);
                                let stats = vm.get_memory_stats(0);
                                hyperstack::runtime::hyperstack_interpreter::vm_metrics::record_memory_stats(&stats, #program_name);
                            }
                        }

                        result
                    };

                    match mutations_result {
                        Ok(mutations) => {
                            self.slot_tracker.record(slot);
                            if !mutations.is_empty() {
                                let batch = hyperstack::runtime::hyperstack_server::MutationBatch::new(
                                    hyperstack::runtime::smallvec::SmallVec::from_vec(mutations)
                                );
                                let _ = self.mutations_tx.send(batch).await;
                            }
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
                hyperstack::runtime::tracing::info!("Registering parsers:");
                hyperstack::runtime::tracing::info!("   - Account Parser ID: {}", hyperstack::runtime::yellowstone_vixen_core::Parser::id(&account_parser));
                hyperstack::runtime::tracing::info!("   - Instruction Parser ID: {}", hyperstack::runtime::yellowstone_vixen_core::Parser::id(&instruction_parser));
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
