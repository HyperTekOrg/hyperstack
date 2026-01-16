//! VmHandler generation for routing Vixen parser outputs to the bytecode VM.
//!
//! This generates the complex handler that includes:
//! - Resolver integration for account types
//! - Instruction hook execution
//! - Unsafe borrow splitting for the VM context

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generate VmHandler implementation for processing account and instruction updates.
///
/// This is the complex handler that includes:
/// - Resolver integration for account types
/// - Instruction hook execution
/// - Unsafe borrow splitting for the VM context
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
            vm: std::sync::Arc<std::sync::Mutex<hyperstack_interpreter::vm::VmContext>>,
            bytecode: std::sync::Arc<hyperstack_interpreter::compiler::MultiEntityBytecode>,
            mutations_tx: tokio::sync::mpsc::Sender<smallvec::SmallVec<[hyperstack_interpreter::Mutation; 6]>>,
            health_monitor: Option<hyperstack_server::HealthMonitor>,
            slot_tracker: hyperstack_server::SlotTracker,
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
                bytecode: hyperstack_interpreter::compiler::MultiEntityBytecode,
                mutations_tx: tokio::sync::mpsc::Sender<smallvec::SmallVec<[hyperstack_interpreter::Mutation; 6]>>,
                health_monitor: Option<hyperstack_server::HealthMonitor>,
                slot_tracker: hyperstack_server::SlotTracker,
            ) -> Self {
                Self {
                    vm: std::sync::Arc::new(std::sync::Mutex::new(hyperstack_interpreter::vm::VmContext::new())),
                    bytecode: std::sync::Arc::new(bytecode),
                    mutations_tx,
                    health_monitor,
                    slot_tracker,
                }
            }
        }

        // Account handler implementation
        impl yellowstone_vixen::Handler<parsers::#state_enum, yellowstone_vixen_core::AccountUpdate> for VmHandler {
            async fn handle(
                &self,
                value: &parsers::#state_enum,
                raw_update: &yellowstone_vixen_core::AccountUpdate,
            ) -> yellowstone_vixen::HandlerResult<()> {
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
                            resolver_fn(&account_address, &event_value, &mut ctx)
                        } else {
                            // No resolver defined, process normally
                            hyperstack_interpreter::resolvers::KeyResolution::Found(String::new())
                        }
                    } else {
                        // No state table, process normally
                        hyperstack_interpreter::resolvers::KeyResolution::Found(String::new())
                    }
                };

                // Handle the resolution result
                match resolver_result {
                    hyperstack_interpreter::resolvers::KeyResolution::Found(resolved_key) => {
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
                        let mut vm = self.vm.lock().unwrap();

                        if let Err(_e) = vm.queue_account_update(
                            0, // state_id
                            account_address.clone(),
                            event_type.to_string(),
                            event_value,
                            slot,
                            signature,
                        ) {
                            // Silently ignore queue errors
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

        impl yellowstone_vixen::Handler<parsers::#instruction_enum, yellowstone_vixen_core::instruction::InstructionUpdate> for VmHandler {
            async fn handle(
                &self,
                value: &parsers::#instruction_enum,
                raw_update: &yellowstone_vixen_core::instruction::InstructionUpdate,
            ) -> yellowstone_vixen::HandlerResult<()> {
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
                let event_value = value.to_value_with_accounts(static_keys_vec);

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

                            // Extract instruction data
                            let instruction_data = event_value.get("data").unwrap_or(&serde_json::Value::Null);

                            // Get timestamp from context
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
                                2, // state_reg - register 2 holds the entity state
                                unsafe { (*vm_ptr).path_cache() },
                                instruction_data,
                                Some(context.slot.unwrap_or(0)),
                                context.signature.clone(),
                                timestamp,
                            );

                            // Call each registered hook
                            for hook_fn in hooks.iter() {
                                hook_fn(&mut ctx);
                            }

                            // Collect data from ctx before dropping it
                            let dirty_fields: std::collections::HashSet<String> = ctx.dirty_fields().clone();
                            let pending_updates = ctx.take_pending_updates();

                            // Drop ctx to release the mutable borrows before we use vm again
                            drop(ctx);

                            // Generate additional mutations from fields modified by hooks
                            if !dirty_fields.is_empty() {
                                // Extract the dirty fields from state to create a patch
                                if let Ok(patch) = vm.extract_partial_state(2, &dirty_fields) {
                                    // Find or create mutation for the current entity
                                    if let Some(mint) = event_value.get("accounts").and_then(|a| a.get("mint")).and_then(|m| m.as_str()) {
                                        // Merge this patch into mutations from result
                                        if let Ok(ref mut mutations) = result {
                                            let mint_value = serde_json::Value::String(mint.to_string());
                                            let found = mutations.iter_mut()
                                                .find(|m| m.key == mint_value)
                                                .map(|m| {
                                                    // Deep merge patch into existing mutation's patch
                                                    if let serde_json::Value::Object(ref mut existing_patch_obj) = m.patch {
                                                        if let serde_json::Value::Object(new_patch_obj) = patch.clone() {
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

                                            if found.is_none() {
                                                mutations.push(hyperstack_interpreter::Mutation {
                                                    export: #entity_name_lit.to_string(),
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
                                for update in pending_updates {
                                    let resolved_key = vm.try_pda_reverse_lookup(0, "default_pda_lookup", &update.pda_address);

                                    let mut account_data = update.account_data;
                                    if let Some(key) = resolved_key {
                                        if let Some(obj) = account_data.as_object_mut() {
                                            obj.insert("__resolved_primary_key".to_string(), serde_json::json!(key));
                                        }
                                    }

                                    let update_context = hyperstack_interpreter::UpdateContext::with_timestamp(
                                        update.slot,
                                        update.signature.clone(),
                                        update.queued_at,
                                    );

                                    match vm.process_event_with_context(&bytecode, account_data, &update.account_type, Some(&update_context)) {
                                        Ok(pending_mutations) => {
                                            if let Ok(ref mut mutations) = result {
                                                mutations.extend(pending_mutations);
                                            }
                                        }
                                        Err(_e) => {
                                            // Ignore errors
                                        }
                                    }
                                }
                            }
                        }

                        // Periodically clean up expired pending updates
                        if vm.instructions_executed % 1000 == 0 {
                            let _ = vm.cleanup_expired_pending_updates(0);
                        }
                    }

                    result
                };

                match mutations_result {
                    Ok(mutations) => {
                        self.slot_tracker.record(slot);
                        if !mutations.is_empty() {
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
    }
}
