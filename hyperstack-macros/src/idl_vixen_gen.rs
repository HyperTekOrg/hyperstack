//! IDL-based Vixen runtime generation.
//!
//! This module generates Vixen runtime integration code for stream_spec processing.
//! Some functions here are kept for backward compatibility but may be unused.

#![allow(dead_code)]

use crate::parse::idl::*;
use crate::parse::{ResolverHookKind, ResolverHookSpec};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Path;

/// Extract the type name from a path and convert it to the event type string
/// For accounts: generated_sdk::accounts::BondingCurve -> "BondingCurveState"
/// For instructions: generated_sdk::instructions::Create -> "CreateIxState"
fn path_to_event_type(path: &Path, is_instruction: bool) -> String {
    // Get the last segment (the actual type name)
    let type_name = path
        .segments
        .last()
        .map(|seg| seg.ident.to_string())
        .unwrap_or_default();

    if is_instruction {
        // Instructions use IxState suffix
        format!("{}IxState", type_name)
    } else {
        // Accounts use State suffix
        format!("{}State", type_name)
    }
}

/// Generate both resolver registries (public function for use in lib.rs)
pub fn generate_resolver_registries(resolver_hooks: &[ResolverHookSpec]) -> TokenStream {
    let resolver_registry = generate_resolver_registry(resolver_hooks);
    let instruction_hook_registry = generate_instruction_hook_registry(resolver_hooks);

    quote! {
        #resolver_registry
        #instruction_hook_registry
    }
}

/// Generate resolver registry for account types
fn generate_resolver_registry(resolver_hooks: &[ResolverHookSpec]) -> TokenStream {
    let key_resolvers: Vec<_> = resolver_hooks
        .iter()
        .filter(|hook| matches!(hook.kind, ResolverHookKind::KeyResolver))
        .collect();

    if key_resolvers.is_empty() {
        // Generate empty default function when no resolvers
        return quote! {
            /// Get resolver function for a given account type (no resolvers registered)
            fn get_resolver_for_account_type(_account_type: &str) -> Option<fn(&str, &serde_json::Value, &mut hyperstack_interpreter::resolvers::ResolveContext) -> hyperstack_interpreter::resolvers::KeyResolution> {
                None
            }
        };
    }

    // Generate match arms for each resolver
    let resolver_arms = key_resolvers.iter().map(|hook| {
        // Convert path to event type (e.g., generated_sdk::accounts::BondingCurve -> "BondingCurveState")
        let event_type = path_to_event_type(&hook.account_type_path, false);
        let fn_name = &hook.fn_name;

        quote! {
            #event_type => {
                // Call user's resolver function
                Some(#fn_name)
            }
        }
    });

    quote! {
        /// Get resolver function for a given account type
        fn get_resolver_for_account_type(account_type: &str) -> Option<fn(&str, &serde_json::Value, &mut hyperstack_interpreter::resolvers::ResolveContext) -> hyperstack_interpreter::resolvers::KeyResolution> {
            match account_type {
                #(#resolver_arms)*
                _ => None
            }
        }
    }
}

/// Generate instruction hook registry
fn generate_instruction_hook_registry(resolver_hooks: &[ResolverHookSpec]) -> TokenStream {
    let instruction_hooks: Vec<_> = resolver_hooks
        .iter()
        .filter(|hook| matches!(hook.kind, ResolverHookKind::AfterInstruction))
        .collect();

    if instruction_hooks.is_empty() {
        // Generate empty default function when no hooks
        return quote! {
            /// Get instruction hooks for a given instruction type (no hooks registered)
            fn get_instruction_hooks(_instruction_type: &str) -> Vec<fn(&mut hyperstack_interpreter::resolvers::InstructionContext)> {
                Vec::new()
            }
        };
    }

    // Group hooks by instruction type to avoid duplicate match arms
    use std::collections::HashMap as StdHashMap;
    let mut hooks_by_instruction: StdHashMap<String, Vec<&syn::Ident>> = StdHashMap::new();

    for hook in &instruction_hooks {
        let event_type = path_to_event_type(&hook.account_type_path, true);
        hooks_by_instruction
            .entry(event_type)
            .or_default()
            .push(&hook.fn_name);
    }

    // Generate match arms with all hooks for each instruction type
    let hook_arms = hooks_by_instruction.iter().map(|(event_type, hook_fns)| {
        quote! {
            #event_type => {
                vec![#(#hook_fns),*]
            }
        }
    });

    quote! {
        /// Get instruction hooks for a given instruction type
        fn get_instruction_hooks(instruction_type: &str) -> Vec<fn(&mut hyperstack_interpreter::resolvers::InstructionContext)> {
            match instruction_type {
                #(#hook_arms)*
                _ => Vec::new()
            }
        }
    }
}

/// Generates the spec function for hyperstack-server integration (with registries)
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

/// Generates the spec function for hyperstack-server integration without resolver registries
pub fn generate_spec_function_without_registries(idl: &IdlSpec, _program_id: &str) -> TokenStream {
    let program_name = idl.get_name();
    let state_enum_name = format_ident!("{}State", to_pascal_case(program_name));
    let instruction_enum_name = format_ident!("{}Instruction", to_pascal_case(program_name));

    quote! {

        /// Creates a hyperstack-server Spec with bytecode and parsers
        ///
        /// This function returns a complete specification that can be used
        /// with the hyperstack-server builder API.
        ///
        /// # Example
        /// ```ignore
        /// use hyperstack_server::Server;
        ///
        /// #[tokio::main]
        /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
        ///     Server::builder()
        ///         .spec(spec())
        ///         .websocket()
        ///         .bind("[::]:8877")
        ///         .start()
        ///         .await
        /// }
        /// ```
        pub fn spec() -> hyperstack_server::Spec {
            let bytecode = create_multi_entity_bytecode();
            let program_id = parsers::PROGRAM_ID_STR.to_string();

            hyperstack_server::Spec::new(bytecode, program_id)
                .with_parser_setup(create_parser_setup())
        }

        /// Creates the parser setup function for Vixen runtime integration
        fn create_parser_setup() -> hyperstack_server::ParserSetupFn {
            use std::sync::Arc;

            Arc::new(|mutations_tx, health_monitor, reconnection_config| {
                Box::pin(async move {
                    run_vixen_runtime_with_channel(mutations_tx, health_monitor, reconnection_config).await
                })
            })
        }

        /// Runs the Vixen runtime with reconnection support
        async fn run_vixen_runtime_with_channel(
            mutations_tx: tokio::sync::mpsc::Sender<smallvec::SmallVec<[hyperstack_interpreter::Mutation; 6]>>,
            health_monitor: Option<hyperstack_server::HealthMonitor>,
            reconnection_config: hyperstack_server::ReconnectionConfig,
        ) -> anyhow::Result<()> {
            use yellowstone_vixen::config::{BufferConfig, VixenConfig};
            use yellowstone_vixen_yellowstone_grpc_source::YellowstoneGrpcConfig;
            use yellowstone_vixen_yellowstone_grpc_source::YellowstoneGrpcSource;
            use yellowstone_vixen::Pipeline;
            use std::sync::{Arc, Mutex};

            let env_loaded = dotenvy::from_filename(".env.local").is_ok()
                || dotenvy::from_filename("backend/tenant-runtime/.env").is_ok()
                || dotenvy::from_filename(".env").is_ok()
                || dotenvy::dotenv().is_ok();

            if !env_loaded {
                tracing::warn!("No .env file found. Make sure environment variables are set.");
            }

            let endpoint = std::env::var("YELLOWSTONE_ENDPOINT")
                .map_err(|_| anyhow::anyhow!(
                    "YELLOWSTONE_ENDPOINT environment variable must be set.\n\
                     Example: export YELLOWSTONE_ENDPOINT=http://localhost:10000"
                ))?;
            let x_token = std::env::var("YELLOWSTONE_X_TOKEN").ok();

            let slot_tracker = hyperstack_server::SlotTracker::new();
            let mut attempt = 0u32;
            let mut backoff = reconnection_config.initial_delay;

            loop {
                let from_slot = {
                    let last = slot_tracker.get();
                    if last > 0 { Some(last) } else { None }
                };

                if from_slot.is_some() {
                    tracing::info!("Resuming from slot {}", from_slot.unwrap());
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

                let bytecode = create_multi_entity_bytecode();

                if attempt == 0 {
                    tracing::info!("🔍 Bytecode Handler Details:");
                    for (entity_name, entity_bytecode) in &bytecode.entities {
                        tracing::info!("   Entity: {}", entity_name);
                        for (event_type, handler_opcodes) in &entity_bytecode.handlers {
                            tracing::info!("      {} -> {} opcodes", event_type, handler_opcodes.len());
                            if event_type == "BuyIxState" {
                                tracing::info!("         Opcode types:");
                                for (idx, opcode) in handler_opcodes.iter().enumerate() {
                                    tracing::info!("            [{}] {:?}", idx, opcode);
                                }
                            }
                        }
                    }
                }

                let vm = Arc::new(Mutex::new(hyperstack_interpreter::vm::VmContext::new()));
                let bytecode_arc = Arc::new(bytecode);

            #[derive(Clone)]
            struct VmHandler {
                vm: Arc<Mutex<hyperstack_interpreter::vm::VmContext>>,
                bytecode: Arc<hyperstack_interpreter::compiler::MultiEntityBytecode>,
                mutations_tx: tokio::sync::mpsc::Sender<smallvec::SmallVec<[hyperstack_interpreter::Mutation; 6]>>,
                health_monitor: Option<hyperstack_server::HealthMonitor>,
                slot_tracker: hyperstack_server::SlotTracker,
            }

            impl VmHandler {
                fn new(
                    vm: Arc<Mutex<hyperstack_interpreter::vm::VmContext>>,
                    bytecode: Arc<hyperstack_interpreter::compiler::MultiEntityBytecode>,
                    mutations_tx: tokio::sync::mpsc::Sender<smallvec::SmallVec<[hyperstack_interpreter::Mutation; 6]>>,
                    health_monitor: Option<hyperstack_server::HealthMonitor>,
                    slot_tracker: hyperstack_server::SlotTracker,
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

            impl yellowstone_vixen::Handler<parsers::#state_enum_name, yellowstone_vixen_core::AccountUpdate> for VmHandler {
                async fn handle(&self, value: &parsers::#state_enum_name, raw_update: &yellowstone_vixen_core::AccountUpdate)
                    -> yellowstone_vixen::HandlerResult<()>
                {
                    let slot = raw_update.slot;
                    let account = raw_update.account.as_ref().unwrap();
                    let signature = bs58::encode(account.txn_signature.as_ref().unwrap()).into_string();
                    // Record event received for health monitoring
                    if let Some(ref health) = self.health_monitor {
                        health.record_event().await;
                    }

                    // Extract account address from raw_update
                    let account_address = bs58::encode(&account.pubkey).into_string();

                    let event_type = value.event_type();
                    let mut event_value = value.to_value();

                    // Add account address to event value
                    if let Some(obj) = event_value.as_object_mut() {
                        obj.insert("__account_address".to_string(), serde_json::json!(account_address));
                    }

                    // Check if this account type has a resolver and handle the resolution
                    tracing::debug!("🔍 Account update: type={}, address={}", event_type, account_address);
                    let resolver_result = {
                        let mut vm = self.vm.lock().unwrap();

                        // Get state table to access reverse lookups
                        if let Some(state_table) = vm.get_state_table_mut(0) {
                            let mut ctx = hyperstack_interpreter::resolvers::ResolveContext::new(
                                0,
                                slot,
                                signature.clone(),
                                &mut state_table.pda_reverse_lookups,
                            );

                            // Call the resolver if one exists for this account type
                            if let Some(resolver_fn) = get_resolver_for_account_type(event_type) {
                                tracing::debug!("   Has resolver, attempting lookup");
                                resolver_fn(&account_address, &event_value, &mut ctx)
                            } else {
                                tracing::debug!("   No resolver, processing normally");
                                // No resolver defined, process normally
                                hyperstack_interpreter::resolvers::KeyResolution::Found(String::new())
                            }
                        } else {
                            tracing::warn!("   No state table found!");
                            // No state table, process normally
                            hyperstack_interpreter::resolvers::KeyResolution::Found(String::new())
                        }
                    };

                    // Handle the resolution result
                    match resolver_result {
                        hyperstack_interpreter::resolvers::KeyResolution::Found(resolved_key) => {
                            tracing::debug!("   ✓ Resolved key: {}", if resolved_key.is_empty() { "(empty)" } else { &resolved_key });
                            // If a primary key was resolved, override it in the event value
                            if !resolved_key.is_empty() {
                                if let Some(obj) = event_value.as_object_mut() {
                                    obj.insert("__resolved_primary_key".to_string(), serde_json::json!(resolved_key));
                                }
                            }
                            // Continue with normal processing
                        }
                        hyperstack_interpreter::resolvers::KeyResolution::QueueUntil(_discriminators) => {
                            // Queue this update for later processing
                            tracing::info!("⏳ Queuing {} account update for PDA {}, waiting for reverse lookup", event_type, account_address);
                            let mut vm = self.vm.lock().unwrap();

                            if let Err(e) = vm.queue_account_update(
                                0, // state_id
                                account_address.clone(),
                                event_type.to_string(),
                                event_value,
                                slot,
                                signature,
                            ) {
                                tracing::warn!("Failed to queue account update: {}", e);
                            }
                            return Ok(()); // Don't process now, wait for queue flush
                        }
                        hyperstack_interpreter::resolvers::KeyResolution::Skip => {
                            // Skip this update entirely
                            return Ok(());
                        }
                    }

                    let mutations_result = {
                        let mut vm = self.vm.lock().unwrap();

                        // Create update context with slot and signature
                        let context = hyperstack_interpreter::UpdateContext::new(slot, signature.clone());

                        vm.process_event_with_context(&self.bytecode, event_value, event_type, Some(&context))
                            .map_err(|e| e.to_string())
                    };

                    match mutations_result {
                        Ok(mutations) => {
                            self.slot_tracker.record(slot);
                            if !mutations.is_empty() {
                                for (i, m) in mutations.iter().enumerate() {
                                    tracing::info!("      Mutation {}: key={}, patch_keys={:?}",
                                        i + 1,
                                        m.key,
                                        m.patch.as_object().map(|o| o.keys().collect::<Vec<_>>()).unwrap_or_default()
                                    );
                                }
                                let _ = self.mutations_tx.send(smallvec::SmallVec::from_vec(mutations)).await;
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

            impl yellowstone_vixen::Handler<parsers::#instruction_enum_name, yellowstone_vixen_core::instruction::InstructionUpdate> for VmHandler {
                async fn handle(&self, value: &parsers::#instruction_enum_name, raw_update: &yellowstone_vixen_core::instruction::InstructionUpdate)
                    -> yellowstone_vixen::HandlerResult<()>
                {
                    // Extract and log slot, signature, and accounts from shared field in raw_update
                    let slot = raw_update.shared.slot;
                    let signature = bs58::encode(&raw_update.shared.signature).into_string();

                    // Record event received for health monitoring
                    if let Some(ref health) = self.health_monitor {
                        health.record_event().await;
                    }

                    // raw_update.accounts are already KeyBytes<32>, no conversion needed
                    let static_keys_vec = &raw_update.accounts;
                    let event_type = value.event_type();

                    // Use to_value_with_accounts to get event value with named accounts from IDL
                    // Pass the inner instruction accounts from raw_update
                    let event_value = value.to_value_with_accounts(static_keys_vec);

                    // Log instruction processing details
                    if event_type.ends_with("IxState") {
                        let account_keys: Vec<String> = event_value.get("accounts")
                            .and_then(|a| a.as_object())
                            .map(|obj| obj.keys().cloned().collect())
                            .unwrap_or_default();
                        tracing::info!(
                            "📥 [INSTRUCTION] type={} slot={} sig={}... accounts=[{}]",
                            event_type, slot, &signature[..8], account_keys.join(", ")
                        );
                    }

                    let bytecode = self.bytecode.clone();
                    let mutations_result = {
                        let mut vm = self.vm.lock().unwrap();

                        // Create update context with slot and signature
                        let context = hyperstack_interpreter::UpdateContext::new(slot, signature.clone());

                        let mut result = vm.process_event_with_context(&bytecode, event_value.clone(), event_type, Some(&context))
                            .map_err(|e| e.to_string());

                        // After processing instruction, call any registered after-instruction hooks
                        if result.is_ok() {
                            let hooks = get_instruction_hooks(event_type);
                            if !hooks.is_empty() {

                                // Extract accounts map from event_value
                                let accounts = event_value.get("accounts")
                                    .and_then(|a| a.as_object())
                                    .map(|obj| {
                                        obj.iter()
                                            .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                                            .collect::<std::collections::HashMap<String, String>>()
                                    })
                                    .unwrap_or_default();

                                // Extract instruction data (the "data" field contains instruction args)
                                let instruction_data = event_value.get("data").unwrap_or(&serde_json::Value::Null);

                                // Get timestamp from context (defaults to current time if not set)
                                let timestamp = vm.current_context()
                                    .map(|ctx| ctx.timestamp())
                                    .unwrap_or_else(|| std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap()
                                        .as_secs() as i64);

                                // SAFETY: We're carefully splitting the mutable borrow of vm into disjoint parts.
                                let vm_ptr: *mut hyperstack_interpreter::vm::VmContext = &mut *vm as *mut hyperstack_interpreter::vm::VmContext;

                                // Build InstructionContext with metrics support
                                let mut ctx = hyperstack_interpreter::resolvers::InstructionContext::with_metrics(
                                    accounts,
                                    0, // state_id
                                    &mut *vm,
                                    unsafe { (*vm_ptr).registers_mut() },
                                    2, // state_reg - register 2 holds the entity state (see compiler.rs line 384)
                                    unsafe { (*vm_ptr).path_cache() },
                                    instruction_data,
                                    Some(context.slot.unwrap_or(0)),
                                    context.signature.clone(),
                                    timestamp,
                                );

                                // Call each registered hook
                                for (idx, hook_fn) in hooks.iter().enumerate() {
                                    hook_fn(&mut ctx);
                                }

                                // Collect data from ctx before dropping it
                                let dirty_fields: std::collections::HashSet<String> = ctx.dirty_fields().clone();
                                let pending_updates = ctx.take_pending_updates();

                                // Drop ctx to release the mutable borrows before we use vm again
                                drop(ctx);

                                // Generate additional mutations from fields modified by hooks
                                if !dirty_fields.is_empty() {
                                    tracing::info!("   📝 Hooks modified {} field(s): {:?}", dirty_fields.len(), dirty_fields);

                                    // IMPORTANT: Persist hook changes back to state table so they're available for future mutations
                                    // The state is in register 2 (see compiler.rs line 384), and the key should be the mint
                                    if let Ok(ref mutations) = result {
                                        if let Some(first_mutation) = mutations.first() {
                                            tracing::debug!("      💾 Persisting hook changes to state table for key: {:?}", first_mutation.key);
                                            // Manually call UpdateState to persist the modified state back to the state table
                                            if let Err(e) = vm.update_state_from_register(0, first_mutation.key.clone(), 2) {
                                            } else {
                                            }
                                        }
                                    }

                                    // Extract the dirty fields from state to create a patch
                                    if let Ok(patch) = vm.extract_partial_state(2, &dirty_fields) {
                                        // Find or create mutation for the current entity
                                        if let Some(mint) = event_value.get("accounts").and_then(|a| a.get("mint")).and_then(|m| m.as_str()) {

                                            // Merge this patch into mutations from result
                                            if let Ok(ref mut mutations) = result {
                                                // Find existing mutation for this mint, or create new one
                                                let mint_value = serde_json::Value::String(mint.to_string());
                                                let found = mutations.iter_mut()
                                                    .find(|m| m.key == mint_value)
                                                    .map(|m| {
                                                        // Deep merge patch into existing mutation's patch
                                                        if let serde_json::Value::Object(ref mut existing_patch_obj) = m.patch {
                                                            if let serde_json::Value::Object(new_patch_obj) = patch.clone() {
                                                                for (section_key, new_section_value) in new_patch_obj {
                                                                    if let Some(existing_section) = existing_patch_obj.get_mut(&section_key) {
                                                                        // Section exists - deep merge objects
                                                                        if let (Some(existing_obj), Some(new_obj)) =
                                                                            (existing_section.as_object_mut(), new_section_value.as_object()) {
                                                                            // Merge field by field within the section
                                                                            for (field_key, field_value) in new_obj {
                                                                                existing_obj.insert(field_key.clone(), field_value.clone());
                                                                            }
                                                                        } else {
                                                                            // Not both objects - replace entirely
                                                                            *existing_section = new_section_value.clone();
                                                                        }
                                                                    } else {
                                                                        // Section doesn't exist - insert it
                                                                        existing_patch_obj.insert(section_key.clone(), new_section_value.clone());
                                                                    }
                                                                }

                                                                // Re-evaluate computed fields using full accumulated state
                                                                // The patch only contains dirty fields, but computed fields may reference
                                                                // other sections (e.g., last_trade_price references reserves.*)
                                                                // So we evaluate on full state and copy computed values back to patch
                                                                let mut full_state_for_eval = vm.registers_mut()[2].clone();
                                                                if let Err(_e) = evaluate_computed_fields(&mut full_state_for_eval) {
                                                                    // Ignore errors
                                                                }
                                                                // Copy only computed field values from full state back to patch
                                                                // Uses computed_field_paths() to know which fields to copy
                                                                for path in computed_field_paths() {
                                                                    let parts: Vec<&str> = path.split('.').collect();
                                                                    if parts.len() >= 2 {
                                                                        let section = parts[0];
                                                                        let field = parts[1];
                                                                        // Get value from full state
                                                                        if let Some(value) = full_state_for_eval
                                                                            .get(section)
                                                                            .and_then(|s| s.get(field))
                                                                        {
                                                                            // Ensure section exists in patch
                                                                            if !existing_patch_obj.contains_key(section) {
                                                                                existing_patch_obj.insert(
                                                                                    section.to_string(),
                                                                                    serde_json::json!({})
                                                                                );
                                                                            }
                                                                            // Insert computed value into patch
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

                                                // If no mutation existed for this key, create one
                                                if found.is_none() {
                                                    mutations.push(hyperstack_interpreter::Mutation {
                                                        export: "Token".to_string(), // TODO: Get export name from event type
                                                        key: mint_value,
                                                        patch: patch.clone(),
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }

                                // Reprocess any pending updates that were queued
                                if !pending_updates.is_empty() {
                                    for (idx, update) in pending_updates.into_iter().enumerate() {
                                        // Resolve the primary key using PDA reverse lookup
                                        let resolved_key = vm.try_pda_reverse_lookup(0, "default_pda_lookup", &update.pda_address);

                                        // Inject the resolved key into the account data
                                        let mut account_data = update.account_data;
                                        if let Some(key) = resolved_key {
                                            if let Some(obj) = account_data.as_object_mut() {
                                                obj.insert("__resolved_primary_key".to_string(), serde_json::json!(key));
                                            }
                                        }

                                        // Create update context with slot and signature
                                        let update_context = hyperstack_interpreter::UpdateContext::with_timestamp(
                                            update.slot,
                                            update.signature.clone(),
                                            update.queued_at
                                        );

                                        match vm.process_event_with_context(&bytecode, account_data, &update.account_type, Some(&update_context)) {
                                            Ok(pending_mutations) => {
                                                if !pending_mutations.is_empty() {
                                                    // Merge pending mutations into the main result
                                                    if let Ok(ref mut mutations) = result {
                                                        mutations.extend(pending_mutations);
                                                    }
                                                } else {
                                                }
                                            }
                                            Err(e) => {
                                            }
                                        }
                                    }
                                }
                            }

                            if vm.instructions_executed % 1000 == 0 {
                                let cleanup_result = vm.cleanup_all_expired(0);
                                if cleanup_result.pending_updates_removed > 0 || cleanup_result.temporal_entries_removed > 0 {
                                    tracing::info!(
                                        "Cleanup: {} pending updates, {} temporal entries removed",
                                        cleanup_result.pending_updates_removed,
                                        cleanup_result.temporal_entries_removed
                                    );
                                }

                                let stats = vm.get_memory_stats(0);
                                if stats.state_table_at_capacity {
                                    tracing::warn!(
                                        "State table at capacity: {}/{} entities",
                                        stats.state_table_entity_count,
                                        stats.state_table_max_entries
                                    );
                                }
                                if let Some(ref pending) = stats.pending_queue_stats {
                                    if pending.total_updates > 100 {
                                        tracing::warn!(
                                            "Large pending queue: {} updates across {} PDAs (oldest: {}s, est memory: {}KB)",
                                            pending.total_updates,
                                            pending.unique_pdas,
                                            pending.oldest_age_seconds,
                                            pending.estimated_memory_bytes / 1024
                                        );
                                    }
                                }
                            }
                        }

                        result
                    };

                    match mutations_result {
                        Ok(mutations) => {
                            self.slot_tracker.record(slot);
                            if !mutations.is_empty() {
                                for (i, m) in mutations.iter().enumerate() {
                                    tracing::info!("      Mutation {}: key={}, patch_keys={:?}",
                                        i + 1,
                                        m.key,
                                        m.patch.as_object().map(|o| o.keys().collect::<Vec<_>>()).unwrap_or_default()
                                    );
                                }
                                let _ = self.mutations_tx.send(smallvec::SmallVec::from_vec(mutations)).await;
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
                tracing::info!("🚀 Starting yellowstone-vixen runtime for {} program", #program_name);
                tracing::info!("📍 Program ID: {}", parsers::PROGRAM_ID_STR);
                tracing::info!("📊 Registering parsers:");
                tracing::info!("   - Account Parser ID: {}", yellowstone_vixen_core::Parser::id(&account_parser));
                tracing::info!("   - Instruction Parser ID: {}", yellowstone_vixen_core::Parser::id(&instruction_parser));
            }

            if let Some(ref health) = health_monitor {
                health.record_reconnecting().await;
            }

            let account_pipeline = Pipeline::new(account_parser, [handler.clone()]);
            let instruction_pipeline = Pipeline::new(instruction_parser, [handler]);

            if let Some(ref health) = health_monitor {
                health.record_connection().await;
            }

            yellowstone_vixen::Runtime::<YellowstoneGrpcSource>::builder()
                .account(account_pipeline)
                .instruction(instruction_pipeline)
                .build(vixen_config)
                .run_async()
                .await;

            attempt += 1;

            if let Some(max) = reconnection_config.max_attempts {
                if attempt >= max {
                    tracing::error!("Max reconnection attempts ({}) reached, giving up", max);
                    if let Some(ref health) = health_monitor {
                        health.record_error("Max reconnection attempts reached".into()).await;
                    }
                    return Err(anyhow::anyhow!("Max reconnection attempts reached"));
                }
            }

            tracing::warn!(
                "gRPC stream disconnected. Reconnecting in {:?} (attempt {})",
                backoff,
                attempt
            );

            if let Some(ref health) = health_monitor {
                health.record_disconnection().await;
            }

            tokio::time::sleep(backoff).await;

            backoff = reconnection_config.next_backoff(backoff);
            }
        }
    }
}
