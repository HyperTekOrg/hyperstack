use std::collections::HashMap;

/// Context provided to primary key resolver functions
pub struct ResolveContext<'a> {
    #[allow(dead_code)]
    pub(crate) state_id: u32,
    pub(crate) slot: u64,
    pub(crate) signature: String,
    pub(crate) reverse_lookups:
        &'a mut std::collections::HashMap<String, crate::vm::PdaReverseLookup>,
}

impl<'a> ResolveContext<'a> {
    /// Create a new ResolveContext (primarily for use by generated code)
    pub fn new(
        state_id: u32,
        slot: u64,
        signature: String,
        reverse_lookups: &'a mut std::collections::HashMap<String, crate::vm::PdaReverseLookup>,
    ) -> Self {
        Self {
            state_id,
            slot,
            signature,
            reverse_lookups,
        }
    }

    /// Try to reverse lookup a PDA address to find the seed value
    /// This is typically used to find the primary key from a PDA account address
    pub fn pda_reverse_lookup(&mut self, pda_address: &str) -> Option<String> {
        // Default lookup name - could be made configurable
        let lookup_name = "default_pda_lookup";

        if let Some(lookup_table) = self.reverse_lookups.get_mut(lookup_name) {
            let result = lookup_table.lookup(pda_address);
            if result.is_some() {
                tracing::debug!("✓ PDA reverse lookup hit: {} -> {:?}", pda_address, result);
            } else {
                tracing::debug!("✗ PDA reverse lookup miss: {}", pda_address);
            }
            result
        } else {
            tracing::debug!("✗ PDA reverse lookup table '{}' not found", lookup_name);
            None
        }
    }

    pub fn slot(&self) -> u64 {
        self.slot
    }

    pub fn signature(&self) -> &str {
        &self.signature
    }
}

/// Result of attempting to resolve a primary key
pub enum KeyResolution {
    /// Primary key successfully resolved
    Found(String),

    /// Queue this update until we see one of these instruction discriminators
    /// The discriminators identify which instructions can populate the reverse lookup
    QueueUntil(&'static [u8]),

    /// Skip this update entirely (don't queue)
    Skip,
}

/// Context provided to instruction hook functions
pub struct InstructionContext<'a> {
    pub(crate) accounts: HashMap<String, String>,
    #[allow(dead_code)]
    pub(crate) state_id: u32,
    pub(crate) reverse_lookup_tx: &'a mut dyn ReverseLookupUpdater,
    pub(crate) pending_updates: Vec<crate::vm::PendingAccountUpdate>,
    pub(crate) registers: Option<&'a mut Vec<crate::vm::RegisterValue>>,
    pub(crate) state_reg: Option<crate::vm::Register>,
    #[allow(dead_code)]
    pub(crate) compiled_paths: Option<&'a HashMap<String, crate::metrics_context::CompiledPath>>,
    pub(crate) instruction_data: Option<&'a serde_json::Value>,
    pub(crate) slot: Option<u64>,
    pub(crate) signature: Option<String>,
    pub(crate) timestamp: Option<i64>,
    pub(crate) dirty_tracker: crate::vm::DirtyTracker,
}

pub trait ReverseLookupUpdater {
    fn update(
        &mut self,
        pda_address: String,
        seed_value: String,
    ) -> Vec<crate::vm::PendingAccountUpdate>;
    fn flush_pending(&mut self, pda_address: &str) -> Vec<crate::vm::PendingAccountUpdate>;
}

impl<'a> InstructionContext<'a> {
    pub fn new(
        accounts: HashMap<String, String>,
        state_id: u32,
        reverse_lookup_tx: &'a mut dyn ReverseLookupUpdater,
    ) -> Self {
        Self {
            accounts,
            state_id,
            reverse_lookup_tx,
            pending_updates: Vec::new(),
            registers: None,
            state_reg: None,
            compiled_paths: None,
            instruction_data: None,
            slot: None,
            signature: None,
            timestamp: None,
            dirty_tracker: crate::vm::DirtyTracker::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_metrics(
        accounts: HashMap<String, String>,
        state_id: u32,
        reverse_lookup_tx: &'a mut dyn ReverseLookupUpdater,
        registers: &'a mut Vec<crate::vm::RegisterValue>,
        state_reg: crate::vm::Register,
        compiled_paths: &'a HashMap<String, crate::metrics_context::CompiledPath>,
        instruction_data: &'a serde_json::Value,
        slot: Option<u64>,
        signature: Option<String>,
        timestamp: i64,
    ) -> Self {
        Self {
            accounts,
            state_id,
            reverse_lookup_tx,
            pending_updates: Vec::new(),
            registers: Some(registers),
            state_reg: Some(state_reg),
            compiled_paths: Some(compiled_paths),
            instruction_data: Some(instruction_data),
            slot,
            signature,
            timestamp: Some(timestamp),
            dirty_tracker: crate::vm::DirtyTracker::new(),
        }
    }

    /// Get an account address by its name from the instruction
    pub fn account(&self, name: &str) -> Option<String> {
        self.accounts.get(name).cloned()
    }

    /// Register a reverse lookup: PDA address -> seed value
    /// This also flushes any pending account updates for this PDA
    ///
    /// The pending account updates are accumulated internally and can be retrieved
    /// via `take_pending_updates()` after all hooks have executed.
    pub fn register_pda_reverse_lookup(&mut self, pda_address: &str, seed_value: &str) {
        tracing::info!(
            "📝 Registering PDA reverse lookup: {} -> {}",
            pda_address,
            seed_value
        );
        let pending = self
            .reverse_lookup_tx
            .update(pda_address.to_string(), seed_value.to_string());
        if !pending.is_empty() {
            tracing::info!(
                "   🔄 Flushed {} pending account update(s) for this PDA",
                pending.len()
            );
        }
        self.pending_updates.extend(pending);
    }

    /// Take all accumulated pending updates
    ///
    /// This should be called after all instruction hooks have executed to retrieve
    /// any pending account updates that need to be reprocessed.
    pub fn take_pending_updates(&mut self) -> Vec<crate::vm::PendingAccountUpdate> {
        std::mem::take(&mut self.pending_updates)
    }

    pub fn dirty_tracker(&self) -> &crate::vm::DirtyTracker {
        &self.dirty_tracker
    }

    pub fn dirty_tracker_mut(&mut self) -> &mut crate::vm::DirtyTracker {
        &mut self.dirty_tracker
    }

    /// Get the current state register value (for generating mutations)
    pub fn state_value(&self) -> Option<&serde_json::Value> {
        if let (Some(registers), Some(state_reg)) = (self.registers.as_ref(), self.state_reg) {
            Some(&registers[state_reg])
        } else {
            None
        }
    }

    /// Get a field value from the entity state
    /// This allows reading aggregated values or other entity fields
    pub fn get<T: serde::de::DeserializeOwned>(&self, field_path: &str) -> Option<T> {
        if let (Some(registers), Some(state_reg)) = (self.registers.as_ref(), self.state_reg) {
            let state = &registers[state_reg];
            self.get_nested_value(state, field_path)
                .and_then(|v| serde_json::from_value(v.clone()).ok())
        } else {
            None
        }
    }

    pub fn set<T: serde::Serialize>(&mut self, field_path: &str, value: T) {
        if let (Some(registers), Some(state_reg)) = (self.registers.as_mut(), self.state_reg) {
            let serialized = serde_json::to_value(value).ok();
            if let Some(val) = serialized {
                Self::set_nested_value_static(&mut registers[state_reg], field_path, val);
                self.dirty_tracker.mark_replaced(field_path);
                println!("      ✓ Set field '{}' and marked as dirty", field_path);
            }
        } else {
            println!("      ⚠️  Cannot set field '{}': metrics not configured (registers={}, state_reg={:?})", 
                field_path, self.registers.is_some(), self.state_reg);
        }
    }

    pub fn increment(&mut self, field_path: &str, amount: i64) {
        let current = self.get::<i64>(field_path).unwrap_or(0);
        self.set(field_path, current + amount);
    }

    pub fn append<T: serde::Serialize>(&mut self, field_path: &str, value: T) {
        if let (Some(registers), Some(state_reg)) = (self.registers.as_mut(), self.state_reg) {
            let serialized = serde_json::to_value(&value).ok();
            if let Some(val) = serialized {
                Self::append_to_array_static(&mut registers[state_reg], field_path, val.clone());
                self.dirty_tracker.mark_appended(field_path, val);
                println!(
                    "      ✓ Appended to '{}' and marked as appended",
                    field_path
                );
            }
        } else {
            println!(
                "      ⚠️  Cannot append to '{}': metrics not configured",
                field_path
            );
        }
    }

    fn append_to_array_static(
        value: &mut serde_json::Value,
        path: &str,
        new_value: serde_json::Value,
    ) {
        let segments: Vec<&str> = path.split('.').collect();
        if segments.is_empty() {
            return;
        }

        let mut current = value;
        for segment in &segments[..segments.len() - 1] {
            if !current.is_object() {
                *current = serde_json::json!({});
            }
            let obj = current.as_object_mut().unwrap();
            current = obj
                .entry(segment.to_string())
                .or_insert(serde_json::json!({}));
        }

        let last_segment = segments[segments.len() - 1];
        if !current.is_object() {
            *current = serde_json::json!({});
        }
        let obj = current.as_object_mut().unwrap();
        let arr = obj
            .entry(last_segment.to_string())
            .or_insert_with(|| serde_json::json!([]));
        if let Some(arr) = arr.as_array_mut() {
            arr.push(new_value);
        }
    }

    fn get_nested_value<'b>(
        &self,
        value: &'b serde_json::Value,
        path: &str,
    ) -> Option<&'b serde_json::Value> {
        let mut current = value;
        for segment in path.split('.') {
            current = current.get(segment)?;
        }
        Some(current)
    }

    fn set_nested_value_static(
        value: &mut serde_json::Value,
        path: &str,
        new_value: serde_json::Value,
    ) {
        let segments: Vec<&str> = path.split('.').collect();
        if segments.is_empty() {
            return;
        }

        let mut current = value;
        for segment in &segments[..segments.len() - 1] {
            if !current.is_object() {
                *current = serde_json::json!({});
            }
            let obj = current.as_object_mut().unwrap();
            current = obj
                .entry(segment.to_string())
                .or_insert(serde_json::json!({}));
        }

        if !current.is_object() {
            *current = serde_json::json!({});
        }
        if let Some(obj) = current.as_object_mut() {
            obj.insert(segments[segments.len() - 1].to_string(), new_value);
        }
    }

    /// Access instruction data field
    pub fn data<T: serde::de::DeserializeOwned>(&self, field: &str) -> Option<T> {
        self.instruction_data
            .and_then(|data| data.get(field))
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Get the current timestamp
    pub fn timestamp(&self) -> i64 {
        self.timestamp.unwrap_or(0)
    }

    /// Get the current slot
    pub fn slot(&self) -> Option<u64> {
        self.slot
    }

    /// Get the current signature
    pub fn signature(&self) -> Option<&str> {
        self.signature.as_deref()
    }
}
