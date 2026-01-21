//! VmHandler generation for routing Vixen parser outputs to the bytecode VM.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

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
            mutations_tx: hyperstack::runtime::tokio::sync::mpsc::Sender<hyperstack::runtime::smallvec::SmallVec<[hyperstack::runtime::hyperstack_interpreter::Mutation; 6]>>,
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
                mutations_tx: hyperstack::runtime::tokio::sync::mpsc::Sender<hyperstack::runtime::smallvec::SmallVec<[hyperstack::runtime::hyperstack_interpreter::Mutation; 6]>>,
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

                        if let Err(_e) = vm.queue_account_update(
                            0,
                            hyperstack::runtime::hyperstack_interpreter::QueuedAccountUpdate {
                                pda_address: account_address.clone(),
                                account_type: event_type.to_string(),
                                account_data: event_value,
                                slot,
                                write_version,
                                signature,
                            },
                        ) {
                        }
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
                            let _ = self.mutations_tx.send(hyperstack::runtime::smallvec::SmallVec::from_vec(mutations)).await;
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

                            let dirty_fields: std::collections::HashSet<String> = ctx.dirty_tracker().dirty_paths();
                            let pending_updates = ctx.take_pending_updates();

                            drop(ctx);

                            if !dirty_fields.is_empty() {
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
                                                            if let Err(_e) = evaluate_computed_fields(&mut full_state_for_eval) {
                                                            }
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
                                                    export: #entity_name_lit.to_string(),
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
                        if !mutations.is_empty() {
                            let _ = self.mutations_tx.send(hyperstack::runtime::smallvec::SmallVec::from_vec(mutations)).await;
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
