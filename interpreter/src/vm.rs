use crate::ast::{BinaryOp, ComparisonOp, ComputedExpr, ComputedFieldSpec, FieldPath, Transformation};
use crate::compiler::{MultiEntityBytecode, OpCode};
use crate::Mutation;
use dashmap::DashMap;
use lru::LruCache;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::num::NonZeroUsize;

#[cfg(feature = "otel")]
use tracing::instrument;

/// Context metadata for blockchain updates (accounts and instructions)
/// This structure is designed to be extended over time with additional metadata
#[derive(Debug, Clone, Default)]
pub struct UpdateContext {
    /// Blockchain slot number
    pub slot: Option<u64>,
    /// Transaction signature
    pub signature: Option<String>,
    /// Unix timestamp (seconds since epoch)
    /// If not provided, will default to current system time when accessed
    pub timestamp: Option<i64>,
    /// Additional custom metadata that can be added without breaking changes
    pub metadata: HashMap<String, Value>,
}

impl UpdateContext {
    /// Create a new UpdateContext with slot and signature
    pub fn new(slot: u64, signature: String) -> Self {
        Self {
            slot: Some(slot),
            signature: Some(signature),
            timestamp: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new UpdateContext with slot, signature, and timestamp
    pub fn with_timestamp(slot: u64, signature: String, timestamp: i64) -> Self {
        Self {
            slot: Some(slot),
            signature: Some(signature),
            timestamp: Some(timestamp),
            metadata: HashMap::new(),
        }
    }

    /// Get the timestamp, falling back to current system time if not set
    pub fn timestamp(&self) -> i64 {
        self.timestamp.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
        })
    }

    /// Create an empty context (for testing or when context is not available)
    pub fn empty() -> Self {
        Self::default()
    }

    /// Add custom metadata
    pub fn with_metadata(mut self, key: String, value: Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&Value> {
        self.metadata.get(key)
    }

    /// Convert context to JSON value for injection into event data
    pub fn to_value(&self) -> Value {
        let mut obj = serde_json::Map::new();
        if let Some(slot) = self.slot {
            obj.insert("slot".to_string(), json!(slot));
        }
        if let Some(ref sig) = self.signature {
            obj.insert("signature".to_string(), json!(sig));
        }
        // Always include timestamp (use current time if not set)
        obj.insert("timestamp".to_string(), json!(self.timestamp()));
        for (key, value) in &self.metadata {
            obj.insert(key.clone(), value.clone());
        }
        Value::Object(obj)
    }
}

pub type Register = usize;
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub type RegisterValue = Value;

/// Trait for evaluating computed fields
/// Implement this in your generated spec to enable computed field evaluation
pub trait ComputedFieldsEvaluator {
    fn evaluate(&self, state: &mut Value) -> Result<()>;
}

// Pending queue configuration
const MAX_PENDING_UPDATES_TOTAL: usize = 10_000;
const MAX_PENDING_UPDATES_PER_PDA: usize = 10;
const PENDING_UPDATE_TTL_SECONDS: i64 = 300; // 5 minutes

/// Estimate the size of a JSON value in bytes
fn estimate_json_size(value: &Value) -> usize {
    match value {
        Value::Null => 4,
        Value::Bool(_) => 5,
        Value::Number(_) => 8,
        Value::String(s) => s.len() + 2,
        Value::Array(arr) => 2 + arr.iter().map(|v| estimate_json_size(v) + 1).sum::<usize>(),
        Value::Object(obj) => {
            2 + obj
                .iter()
                .map(|(k, v)| k.len() + 3 + estimate_json_size(v) + 1)
                .sum::<usize>()
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompiledPath {
    pub segments: std::sync::Arc<[String]>,
}

impl CompiledPath {
    pub fn new(path: &str) -> Self {
        let segments: Vec<String> = path.split('.').map(|s| s.to_string()).collect();
        CompiledPath {
            segments: segments.into(),
        }
    }

    fn segments(&self) -> &[String] {
        &self.segments
    }
}

pub struct VmContext {
    registers: Vec<RegisterValue>,
    states: HashMap<u32, StateTable>,
    pub instructions_executed: u64,
    pub cache_hits: u64,
    path_cache: HashMap<String, CompiledPath>,
    pub pda_cache_hits: u64,
    pub pda_cache_misses: u64,
    pub pending_queue_size: u64,
    /// Current update context (set during execute_handler)
    current_context: Option<UpdateContext>,
}

#[derive(Debug)]
pub struct LookupIndex {
    index: DashMap<Value, Value>,
}

impl Default for LookupIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl LookupIndex {
    pub fn new() -> Self {
        LookupIndex {
            index: DashMap::new(),
        }
    }

    pub fn lookup(&self, lookup_value: &Value) -> Option<Value> {
        self.index.get(lookup_value).map(|v| v.clone())
    }

    pub fn insert(&self, lookup_value: Value, primary_key: Value) {
        self.index.insert(lookup_value, primary_key);
    }

    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }
}

#[derive(Debug)]
pub struct TemporalIndex {
    index: DashMap<Value, Vec<(Value, i64)>>,
}

impl Default for TemporalIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl TemporalIndex {
    pub fn new() -> Self {
        TemporalIndex {
            index: DashMap::new(),
        }
    }

    pub fn lookup(&self, lookup_value: &Value, timestamp: i64) -> Option<Value> {
        if let Some(entries) = self.index.get(lookup_value) {
            let entries_vec = entries.value();
            for i in (0..entries_vec.len()).rev() {
                if entries_vec[i].1 <= timestamp {
                    return Some(entries_vec[i].0.clone());
                }
            }
        }
        None
    }

    pub fn lookup_latest(&self, lookup_value: &Value) -> Option<Value> {
        if let Some(entries) = self.index.get(lookup_value) {
            let entries_vec = entries.value();
            if let Some(last) = entries_vec.last() {
                return Some(last.0.clone());
            }
        }
        None
    }

    pub fn insert(&self, lookup_value: Value, primary_key: Value, timestamp: i64) {
        let lookup_key = lookup_value.clone();
        self.index
            .entry(lookup_value)
            .or_default()
            .push((primary_key, timestamp));

        if let Some(mut entries) = self.index.get_mut(&lookup_key) {
            entries.sort_by_key(|(_, ts)| *ts);
        }
    }
}

#[derive(Debug)]
pub struct PdaReverseLookup {
    // Maps: PDA address -> seed value (e.g., bonding_curve_addr -> mint)
    index: LruCache<String, String>,
}

impl PdaReverseLookup {
    pub fn new(capacity: usize) -> Self {
        PdaReverseLookup {
            index: LruCache::new(NonZeroUsize::new(capacity).unwrap()),
        }
    }

    pub fn lookup(&mut self, pda_address: &str) -> Option<String> {
        self.index.get(pda_address).cloned()
    }

    pub fn insert(&mut self, pda_address: String, seed_value: String) -> Option<String> {
        // Check if we're at capacity and will evict
        let evicted = if self.index.len() >= self.index.cap().get() {
            // Get the LRU key that will be evicted
            self.index.peek_lru().map(|(k, _)| k.clone())
        } else {
            None
        };

        self.index.put(pda_address, seed_value);
        evicted
    }
}

#[derive(Debug, Clone)]
pub struct PendingAccountUpdate {
    pub account_type: String,
    pub pda_address: String,
    pub account_data: Value,
    pub slot: u64,
    pub signature: String,
    pub queued_at: i64,
}

#[derive(Debug, Clone)]
pub struct PendingQueueStats {
    pub total_updates: usize,
    pub unique_pdas: usize,
    pub oldest_age_seconds: i64,
    pub largest_pda_queue_size: usize,
    pub estimated_memory_bytes: usize,
}

#[derive(Debug)]
pub struct StateTable {
    pub data: DashMap<Value, Value>,
    pub lookup_indexes: HashMap<String, LookupIndex>,
    pub temporal_indexes: HashMap<String, TemporalIndex>,
    pub pda_reverse_lookups: HashMap<String, PdaReverseLookup>,
    pub pending_updates: DashMap<String, Vec<PendingAccountUpdate>>,
}

impl VmContext {
    pub fn new() -> Self {
        let mut vm = VmContext {
            registers: vec![Value::Null; 256],
            states: HashMap::new(),
            instructions_executed: 0,
            cache_hits: 0,
            path_cache: HashMap::new(),
            pda_cache_hits: 0,
            pda_cache_misses: 0,
            pending_queue_size: 0,
            current_context: None,
        };
        vm.states.insert(
            0,
            StateTable {
                data: DashMap::new(),
                lookup_indexes: HashMap::new(),
                temporal_indexes: HashMap::new(),
                pda_reverse_lookups: HashMap::new(),
                pending_updates: DashMap::new(),
            },
        );
        vm
    }

    /// Get a mutable reference to a state table by ID
    /// Returns None if the state ID doesn't exist
    pub fn get_state_table_mut(&mut self, state_id: u32) -> Option<&mut StateTable> {
        self.states.get_mut(&state_id)
    }

    /// Get public access to registers (for metrics context)
    pub fn registers_mut(&mut self) -> &mut Vec<RegisterValue> {
        &mut self.registers
    }

    /// Get public access to path cache (for metrics context)
    pub fn path_cache(&self) -> &HashMap<String, CompiledPath> {
        &self.path_cache
    }

    /// Get the current update context
    pub fn current_context(&self) -> Option<&UpdateContext> {
        self.current_context.as_ref()
    }

    /// Update the state table with the current value in a register
    /// This allows imperative hooks to persist their changes to the state table
    pub fn update_state_from_register(
        &mut self,
        state_id: u32,
        key: Value,
        register: Register,
    ) -> Result<()> {
        let state = self.states.get(&state_id).ok_or("State table not found")?;
        let value = self.registers[register].clone();
        state.data.insert(key, value);
        Ok(())
    }

    fn reset_registers(&mut self) {
        for reg in &mut self.registers {
            *reg = Value::Null;
        }
    }

    /// Extract only the dirty fields from state (public for use by instruction hooks)
    pub fn extract_partial_state(
        &self,
        state_reg: Register,
        dirty_fields: &HashSet<String>,
    ) -> Result<Value> {
        let full_state = &self.registers[state_reg];

        if dirty_fields.is_empty() {
            return Ok(json!({}));
        }

        let mut partial = serde_json::Map::new();

        for path in dirty_fields {
            let segments: Vec<&str> = path.split('.').collect();

            let mut current = full_state;
            let mut found = true;

            for segment in &segments {
                match current.get(segment) {
                    Some(v) => current = v,
                    None => {
                        found = false;
                        break;
                    }
                }
            }

            if !found {
                continue;
            }

            let mut target = &mut partial;
            for (i, segment) in segments.iter().enumerate() {
                if i == segments.len() - 1 {
                    target.insert(segment.to_string(), current.clone());
                } else {
                    target
                        .entry(segment.to_string())
                        .or_insert_with(|| json!({}));
                    target = target
                        .get_mut(*segment)
                        .and_then(|v| v.as_object_mut())
                        .ok_or("Failed to build nested structure")?;
                }
            }
        }

        Ok(Value::Object(partial))
    }

    fn get_compiled_path(&mut self, path: &str) -> CompiledPath {
        if let Some(compiled) = self.path_cache.get(path) {
            self.cache_hits += 1;
            return compiled.clone();
        }
        let compiled = CompiledPath::new(path);
        self.path_cache.insert(path.to_string(), compiled.clone());
        compiled
    }

    /// Process an event with optional context metadata
    #[cfg_attr(feature = "otel", instrument(
        name = "vm.process_event",
        skip(self, bytecode, event_value, context),
        fields(
            event_type = %event_type,
            slot = context.as_ref().and_then(|c| c.slot),
        )
    ))]
    pub fn process_event_with_context(
        &mut self,
        bytecode: &MultiEntityBytecode,
        mut event_value: Value,
        event_type: &str,
        context: Option<&UpdateContext>,
    ) -> Result<Vec<Mutation>> {
        // Store context for use during handler execution
        self.current_context = context.cloned();

        // Inject context metadata into event value if provided
        if let Some(ctx) = context {
            if let Some(obj) = event_value.as_object_mut() {
                obj.insert("__update_context".to_string(), ctx.to_value());
            }
        }

        let mut all_mutations = Vec::new();

        if let Some(entity_names) = bytecode.event_routing.get(event_type) {
            tracing::debug!(
                "🔀 Event type '{}' routes to {} entity/entities",
                event_type,
                entity_names.len()
            );

            for entity_name in entity_names {
                if let Some(entity_bytecode) = bytecode.entities.get(entity_name) {
                    if let Some(handler) = entity_bytecode.handlers.get(event_type) {
                        tracing::debug!(
                            "   ▶️  Executing handler for entity '{}', event '{}'",
                            entity_name,
                            event_type
                        );
                        tracing::debug!("      Handler has {} opcodes", handler.len());

                        let mutations = self.execute_handler(
                            handler,
                            event_value.clone(),
                            event_type,
                            entity_bytecode.state_id,
                            entity_bytecode.computed_fields_evaluator.as_ref(),
                        )?;

                        tracing::debug!("      Handler produced {} mutation(s)", mutations.len());
                        all_mutations.extend(mutations);
                    } else {
                        tracing::debug!(
                            "   ⊘ No handler found for entity '{}', event '{}'",
                            entity_name,
                            event_type
                        );
                    }
                } else {
                    tracing::debug!("   ⊘ Entity '{}' not found in bytecode", entity_name);
                }
            }
        } else {
            tracing::debug!("⊘ No event routing found for event type '{}'", event_type);
        }

        Ok(all_mutations)
    }

    /// Process an event without context (backward compatibility)
    pub fn process_event(
        &mut self,
        bytecode: &MultiEntityBytecode,
        event_value: Value,
        event_type: &str,
    ) -> Result<Vec<Mutation>> {
        self.process_event_with_context(bytecode, event_value, event_type, None)
    }

    pub fn process_any(
        &mut self,
        bytecode: &MultiEntityBytecode,
        any: prost_types::Any,
    ) -> Result<Vec<Mutation>> {
        let (event_value, event_type) = bytecode.proto_router.decode(any)?;
        self.process_event(bytecode, event_value, &event_type)
    }

    #[cfg_attr(feature = "otel", instrument(
        name = "vm.execute_handler", 
        skip(self, handler, event_value, entity_evaluator),
        level = "debug",
        fields(
            event_type = %event_type,
            handler_opcodes = handler.len(),
        )
    ))]
    #[allow(clippy::type_complexity)]
    fn execute_handler(
        &mut self,
        handler: &[OpCode],
        event_value: Value,
        event_type: &str,
        override_state_id: u32,
        entity_evaluator: Option<&Box<dyn Fn(&mut Value) -> Result<()> + Send + Sync>>,
    ) -> Result<Vec<Mutation>> {
        self.reset_registers();

        tracing::trace!(
            "Executing handler: event_type={}, handler_opcodes={}, event_value={:?}",
            event_type,
            handler.len(),
            event_value
        );

        let mut pc: usize = 0;
        let mut output = Vec::new();
        let mut dirty_fields: HashSet<String> = HashSet::new();

        while pc < handler.len() {
            match &handler[pc] {
                OpCode::LoadEventField {
                    path,
                    dest,
                    default,
                } => {
                    let value = self.load_field(&event_value, path, default.as_ref())?;
                    tracing::trace!(
                        "LoadEventField: path={:?}, dest={}, value={:?}",
                        path.segments,
                        dest,
                        value
                    );
                    // Warn if accounts.* path returns null - this helps debug key resolution issues
                    if value.is_null() && path.segments.len() >= 2 && path.segments[0] == "accounts" {
                        tracing::warn!(
                            "⚠️ LoadEventField returned NULL for accounts path: {:?} -> dest={}",
                            path.segments, dest
                        );
                    }
                    self.registers[*dest] = value;
                    pc += 1;
                }
                OpCode::LoadConstant { value, dest } => {
                    self.registers[*dest] = value.clone();
                    pc += 1;
                }
                OpCode::CopyRegister { source, dest } => {
                    let value = self.registers[*source].clone();
                    tracing::trace!(
                        "CopyRegister: source={}, dest={}, value={:?}",
                        source,
                        dest,
                        if value.is_null() { "NULL".to_string() } else { format!("{}", value) }
                    );
                    self.registers[*dest] = value;
                    pc += 1;
                }
                OpCode::CopyRegisterIfNull { source, dest } => {
                    if self.registers[*dest].is_null() {
                        self.registers[*dest] = self.registers[*source].clone();
                        tracing::trace!(
                            "CopyRegisterIfNull: copied from reg {} to reg {} (value={:?})",
                            source,
                            dest,
                            self.registers[*source]
                        );
                    } else {
                        tracing::trace!(
                            "CopyRegisterIfNull: dest reg {} not null, skipping copy",
                            dest
                        );
                    }
                    pc += 1;
                }
                OpCode::GetEventType { dest } => {
                    self.registers[*dest] = json!(event_type);
                    pc += 1;
                }
                OpCode::CreateObject { dest } => {
                    self.registers[*dest] = json!({});
                    pc += 1;
                }
                OpCode::SetField {
                    object,
                    path,
                    value,
                } => {
                    let val = self.registers[*value].clone();
                    tracing::trace!("SetField: path={}, value={:?}", path, val);

                    self.set_field_auto_vivify(*object, path, *value)?;
                    dirty_fields.insert(path.to_string());
                    pc += 1;
                }
                OpCode::SetFields { object, fields } => {
                    for (path, value_reg) in fields {
                        self.set_field_auto_vivify(*object, path, *value_reg)?;
                        dirty_fields.insert(path.to_string());
                    }
                    pc += 1;
                }
                OpCode::GetField { object, path, dest } => {
                    let value = self.get_field(*object, path)?;
                    self.registers[*dest] = value;
                    pc += 1;
                }
                OpCode::ReadOrInitState {
                    state_id: _,
                    key,
                    default,
                    dest,
                } => {
                    let actual_state_id = override_state_id;
                    self.states
                        .entry(actual_state_id)
                        .or_insert_with(|| StateTable {
                            data: DashMap::new(),
                            lookup_indexes: HashMap::new(),
                            temporal_indexes: HashMap::new(),
                            pda_reverse_lookups: HashMap::new(),
                            pending_updates: DashMap::new(),
                        });
                    let state = self
                        .states
                        .get(&actual_state_id)
                        .ok_or("State table not found")?;
                    let key_value = &self.registers[*key];
                    if key_value.is_null() {
                        // Only warn for account state updates (not instruction hooks that just register PDAs)
                        // Instruction types ending in "IxState" that don't have lookup_by are expected to have null keys
                        if event_type.ends_with("State") && !event_type.ends_with("IxState") {
                            tracing::warn!(
                                "ReadOrInitState: key register {} is NULL for account state, event_type={}",
                                key,
                                event_type
                            );
                        } else {
                            tracing::debug!(
                                "ReadOrInitState: key register {} is NULL (expected for PDA registration hook), event_type={}",
                                key,
                                event_type
                            );
                        }
                    }
                    let value = state
                        .data
                        .get(key_value)
                        .map(|v| v.clone())
                        .unwrap_or_else(|| default.clone());

                    self.registers[*dest] = value;
                    pc += 1;
                }
                OpCode::UpdateState {
                    state_id: _,
                    key,
                    value,
                } => {
                    let actual_state_id = override_state_id;
                    let state = self
                        .states
                        .get(&actual_state_id)
                        .ok_or("State table not found")?;
                    let key_value = self.registers[*key].clone();
                    let value_data = self.registers[*value].clone();

                    state.data.insert(key_value, value_data);
                    pc += 1;
                }
                OpCode::AppendToArray {
                    object,
                    path,
                    value,
                } => {
                    tracing::trace!("AppendToArray: path={}, value={:?}", path, self.registers[*value]);
                    self.append_to_array(*object, path, *value)?;
                    dirty_fields.insert(path.to_string());
                    pc += 1;
                }
                OpCode::GetCurrentTimestamp { dest } => {
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64;
                    self.registers[*dest] = json!(timestamp);
                    pc += 1;
                }
                OpCode::CreateEvent { dest, event_value } => {
                    tracing::trace!("CreateEvent: event_value_reg={}, event_value={:?}", event_value, self.registers[*event_value]);
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64;

                    // Filter out __update_context from the event data
                    let mut event_data = self.registers[*event_value].clone();
                    if let Some(obj) = event_data.as_object_mut() {
                        obj.remove("__update_context");
                    }

                    // Create event with timestamp, data, and optional slot/signature from context
                    let mut event = serde_json::Map::new();
                    event.insert("timestamp".to_string(), json!(timestamp));
                    event.insert("data".to_string(), event_data);

                    // Add slot and signature if available from current context
                    if let Some(ref ctx) = self.current_context {
                        if let Some(slot) = ctx.slot {
                            event.insert("slot".to_string(), json!(slot));
                        }
                        if let Some(ref signature) = ctx.signature {
                            event.insert("signature".to_string(), json!(signature));
                        }
                    }

                    self.registers[*dest] = Value::Object(event);
                    pc += 1;
                }
                OpCode::CreateCapture {
                    dest,
                    capture_value,
                } => {
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs() as i64;

                    // Get the capture data (already filtered by load_field)
                    let capture_data = self.registers[*capture_value].clone();

                    // Extract account_address from the original event if available
                    let account_address = event_value
                        .get("__account_address")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    // Create capture wrapper with timestamp, account_address, data, and optional slot/signature
                    let mut capture = serde_json::Map::new();
                    capture.insert("timestamp".to_string(), json!(timestamp));
                    capture.insert("account_address".to_string(), json!(account_address));
                    capture.insert("data".to_string(), capture_data);

                    // Add slot and signature if available from current context
                    if let Some(ref ctx) = self.current_context {
                        if let Some(slot) = ctx.slot {
                            capture.insert("slot".to_string(), json!(slot));
                        }
                        if let Some(ref signature) = ctx.signature {
                            capture.insert("signature".to_string(), json!(signature));
                        }
                    }

                    self.registers[*dest] = Value::Object(capture);
                    pc += 1;
                }
                OpCode::Transform {
                    source,
                    dest,
                    transformation,
                } => {
                    if source == dest {
                        let result = self.transform_in_place(*source, transformation);
                        if let Err(ref e) = result {
                            tracing::error!(
                                "Transform {:?} failed at pc={}: {} (source_reg={}, source_value={:?})",
                                transformation, pc, e, source, &self.registers[*source]
                            );
                        }
                        result?;
                    } else {
                        let source_value = &self.registers[*source];
                        let result = self.apply_transformation(source_value, transformation);
                        if let Err(ref e) = result {
                            tracing::error!(
                                "Transform {:?} failed at pc={}: {} (source_reg={}, source_value={:?})",
                                transformation, pc, e, source, source_value
                            );
                        }
                        let value = result?;
                        self.registers[*dest] = value;
                    }
                    pc += 1;
                }
                OpCode::EmitMutation {
                    entity_name,
                    key,
                    state,
                } => {
                    let primary_key = self.registers[*key].clone();

                    if primary_key.is_null() || dirty_fields.is_empty() {
                        // Skip mutation if no primary key or no dirty fields
                        tracing::warn!(
                            "   ⊘ [VM] Skipping mutation for entity '{}': primary_key={}, dirty_fields.len()={}",
                            entity_name,
                            if primary_key.is_null() { "NULL" } else { "present" },
                            dirty_fields.len()
                        );
                        if dirty_fields.is_empty() {
                            tracing::warn!("      Reason: No fields were modified by this handler");
                        }
                        if primary_key.is_null() {
                            tracing::warn!("      Reason: Primary key could not be resolved (check __resolved_primary_key or key_resolution)");
                            // Debug: dump register 15, 17, 19, 20 to understand key loading state
                            tracing::warn!("      Debug: reg[15]={:?}, reg[17]={:?}, reg[19]={:?}, reg[20]={:?}",
                                self.registers.get(15).map(|v| if v.is_null() { "NULL".to_string() } else { format!("{:?}", v) }),
                                self.registers.get(17).map(|v| if v.is_null() { "NULL".to_string() } else { format!("{:?}", v) }),
                                self.registers.get(19).map(|v| if v.is_null() { "NULL".to_string() } else { format!("{:?}", v) }),
                                self.registers.get(20).map(|v| if v.is_null() { "NULL".to_string() } else { format!("{:?}", v) })
                            );
                        }
                    } else {
                        let patch = self.extract_partial_state(*state, &dirty_fields)?;
                        tracing::debug!(
                            "   Patch structure: {}",
                            serde_json::to_string_pretty(&patch).unwrap_or_default()
                        );
                        let mutation = Mutation {
                            export: entity_name.clone(),
                            key: primary_key,
                            patch,
                        };
                        output.push(mutation);
                    }
                    pc += 1;
                }
                OpCode::SetFieldIfNull {
                    object,
                    path,
                    value,
                } => {
                    let val = self.registers[*value].clone();
                    let was_set = self.set_field_if_null(*object, path, *value)?;
                    tracing::trace!(
                        "SetFieldIfNull: path={}, value={:?}, was_set={}",
                        path,
                        val,
                        was_set
                    );
                    if was_set {
                        dirty_fields.insert(path.to_string());
                    }
                    pc += 1;
                }
                OpCode::SetFieldMax {
                    object,
                    path,
                    value,
                } => {
                    let was_updated = self.set_field_max(*object, path, *value)?;
                    if was_updated {
                        dirty_fields.insert(path.to_string());
                    }
                    pc += 1;
                }
                OpCode::UpdateTemporalIndex {
                    state_id: _,
                    index_name,
                    lookup_value,
                    primary_key,
                    timestamp,
                } => {
                    let actual_state_id = override_state_id;
                    let state = self
                        .states
                        .get_mut(&actual_state_id)
                        .ok_or("State table not found")?;
                    let index = state
                        .temporal_indexes
                        .entry(index_name.clone())
                        .or_insert_with(TemporalIndex::new);

                    let lookup_val = self.registers[*lookup_value].clone();
                    let pk_val = self.registers[*primary_key].clone();
                    let ts_val = if let Some(val) = self.registers[*timestamp].as_i64() {
                        val
                    } else if let Some(val) = self.registers[*timestamp].as_u64() {
                        val as i64
                    } else {
                        return Err(format!(
                            "Timestamp must be a number (i64 or u64), got: {:?}",
                            self.registers[*timestamp]
                        )
                        .into());
                    };

                    index.insert(lookup_val, pk_val, ts_val);
                    pc += 1;
                }
                OpCode::LookupTemporalIndex {
                    state_id: _,
                    index_name,
                    lookup_value,
                    timestamp,
                    dest,
                } => {
                    let actual_state_id = override_state_id;
                    let state = self
                        .states
                        .get(&actual_state_id)
                        .ok_or("State table not found")?;
                    let lookup_val = &self.registers[*lookup_value];

                    let result = if self.registers[*timestamp].is_null() {
                        if let Some(index) = state.temporal_indexes.get(index_name) {
                            index.lookup_latest(lookup_val).unwrap_or(Value::Null)
                        } else {
                            Value::Null
                        }
                    } else {
                        let ts_val = if let Some(val) = self.registers[*timestamp].as_i64() {
                            val
                        } else if let Some(val) = self.registers[*timestamp].as_u64() {
                            val as i64
                        } else {
                            return Err(format!(
                                "Timestamp must be a number (i64 or u64), got: {:?}",
                                self.registers[*timestamp]
                            )
                            .into());
                        };

                        if let Some(index) = state.temporal_indexes.get(index_name) {
                            index.lookup(lookup_val, ts_val).unwrap_or(Value::Null)
                        } else {
                            Value::Null
                        }
                    };

                    self.registers[*dest] = result;
                    pc += 1;
                }
                OpCode::UpdateLookupIndex {
                    state_id: _,
                    index_name,
                    lookup_value,
                    primary_key,
                } => {
                    let actual_state_id = override_state_id;
                    let state = self
                        .states
                        .get_mut(&actual_state_id)
                        .ok_or("State table not found")?;
                    let index = state
                        .lookup_indexes
                        .entry(index_name.clone())
                        .or_insert_with(LookupIndex::new);

                    let lookup_val = self.registers[*lookup_value].clone();
                    let pk_val = self.registers[*primary_key].clone();

                    tracing::debug!(
                        "📝 UpdateLookupIndex: {} -> {} (index: {})",
                        lookup_val, pk_val, index_name
                    );

                    index.insert(lookup_val, pk_val);
                    pc += 1;
                }
                OpCode::LookupIndex {
                    state_id: _,
                    index_name,
                    lookup_value,
                    dest,
                } => {
                    let actual_state_id = override_state_id;
                    let lookup_val = self.registers[*lookup_value].clone();

                    let result = {
                        let state = self
                            .states
                            .get(&actual_state_id)
                            .ok_or("State table not found")?;
                        
                        if let Some(index) = state.lookup_indexes.get(index_name) {
                            let found = index.lookup(&lookup_val).unwrap_or(Value::Null);
                            tracing::debug!(
                                "🔍 LookupIndex: {} -> {} (index: {}, entries: {})",
                                lookup_val, found, index_name, index.len()
                            );
                            found
                        } else {
                            Value::Null
                        }
                    };
                    
                    // Fallback: Check PDA reverse lookup table if regular index returned NULL or doesn't exist
                    // This handles cases where Lookup key resolution uses PDA addresses
                    let final_result = if result.is_null() {
                        if let Some(pda_str) = lookup_val.as_str() {
                            let state = self
                                .states
                                .get_mut(&actual_state_id)
                                .ok_or("State table not found")?;
                                
                            if let Some(pda_lookup) = state.pda_reverse_lookups.get_mut("default_pda_lookup") {
                                if let Some(resolved) = pda_lookup.lookup(pda_str) {
                                    tracing::info!(
                                        "🔍 LookupIndex (PDA fallback): {} -> {} (via default_pda_lookup)",
                                        &pda_str[..pda_str.len().min(8)], resolved
                                    );
                                    Value::String(resolved)
                                } else {
                                    tracing::debug!(
                                        "🔍 LookupIndex (PDA fallback): {} -> NOT FOUND in PDA reverse lookup",
                                        &pda_str[..pda_str.len().min(8)]
                                    );
                                    Value::Null
                                }
                            } else {
                                tracing::debug!(
                                    "🔍 LookupIndex: {} -> NOT FOUND (index: {} does not exist, no PDA lookup table)",
                                    lookup_val, index_name
                                );
                                Value::Null
                            }
                        } else {
                            tracing::debug!(
                                "🔍 LookupIndex: {} -> NOT FOUND (index: {} does not exist)",
                                lookup_val, index_name
                            );
                            Value::Null
                        }
                    } else {
                        result
                    };

                    self.registers[*dest] = final_result;
                    pc += 1;
                }
                OpCode::SetFieldSum {
                    object,
                    path,
                    value,
                } => {
                    let was_updated = self.set_field_sum(*object, path, *value)?;
                    if was_updated {
                        dirty_fields.insert(path.to_string());
                    }
                    pc += 1;
                }
                OpCode::SetFieldIncrement { object, path } => {
                    let was_updated = self.set_field_increment(*object, path)?;
                    if was_updated {
                        dirty_fields.insert(path.to_string());
                    }
                    pc += 1;
                }
                OpCode::SetFieldMin {
                    object,
                    path,
                    value,
                } => {
                    let was_updated = self.set_field_min(*object, path, *value)?;
                    if was_updated {
                        dirty_fields.insert(path.to_string());
                    }
                    pc += 1;
                }
                OpCode::AddToUniqueSet {
                    state_id: _,
                    set_name,
                    value,
                    count_object,
                    count_path,
                } => {
                    let value_to_add = self.registers[*value].clone();

                    // Store the unique set within the entity object, not in the state table
                    // This ensures each entity instance has its own unique set
                    let set_field_path = format!("__unique_set:{}", set_name);

                    // Get or create the unique set from the entity object
                    let mut set: HashSet<Value> =
                        if let Ok(existing) = self.get_field(*count_object, &set_field_path) {
                            if !existing.is_null() {
                                serde_json::from_value(existing).unwrap_or_default()
                            } else {
                                HashSet::new()
                            }
                        } else {
                            HashSet::new()
                        };

                    // Add value to set
                    let was_new = set.insert(value_to_add);

                    // Store updated set back in the entity object
                    let set_as_vec: Vec<Value> = set.iter().cloned().collect();
                    self.registers[100] = serde_json::to_value(set_as_vec)?;
                    self.set_field_auto_vivify(*count_object, &set_field_path, 100)?;

                    // Update the count field in the object
                    if was_new {
                        self.registers[100] = Value::Number(serde_json::Number::from(set.len()));
                        self.set_field_auto_vivify(*count_object, count_path, 100)?;
                        dirty_fields.insert(count_path.to_string());
                    }

                    pc += 1;
                }
                OpCode::ConditionalSetField {
                    object,
                    path,
                    value,
                    condition_field,
                    condition_op,
                    condition_value,
                } => {
                    // Load the field value from the event
                    let field_value = self.load_field(&event_value, condition_field, None)?;

                    // Evaluate the condition
                    let condition_met =
                        self.evaluate_comparison(&field_value, condition_op, condition_value)?;

                    if condition_met {
                        let val = self.registers[*value].clone();
                        tracing::trace!(
                            "ConditionalSetField: condition met, setting {}={:?}",
                            path,
                            val
                        );
                        self.set_field_auto_vivify(*object, path, *value)?;
                        dirty_fields.insert(path.to_string());
                    } else {
                        tracing::trace!(
                            "ConditionalSetField: condition not met, skipping {}",
                            path
                        );
                    }
                    pc += 1;
                }
                OpCode::ConditionalIncrement {
                    object,
                    path,
                    condition_field,
                    condition_op,
                    condition_value,
                } => {
                    // Load the field value from the event
                    let field_value = self.load_field(&event_value, condition_field, None)?;

                    // Evaluate the condition
                    let condition_met =
                        self.evaluate_comparison(&field_value, condition_op, condition_value)?;

                    if condition_met {
                        tracing::trace!(
                            "ConditionalIncrement: condition met, incrementing {}",
                            path
                        );
                        let was_updated = self.set_field_increment(*object, path)?;
                        if was_updated {
                            dirty_fields.insert(path.to_string());
                        }
                    } else {
                        tracing::trace!(
                            "ConditionalIncrement: condition not met, skipping {}",
                            path
                        );
                    }
                    pc += 1;
                }
                OpCode::EvaluateComputedFields {
                    state,
                    computed_paths,
                } => {
                    // Call the registered evaluator if one exists
                    if let Some(evaluator) = entity_evaluator {
                        let state_value = &mut self.registers[*state];
                        match evaluator(state_value) {
                            Ok(()) => {
                                // Mark computed fields as dirty so they're included in mutations
                                for path in computed_paths {
                                    dirty_fields.insert(path.clone());
                                }
                            }
                            Err(_e) => {
                                // Silently ignore evaluation errors
                            }
                        }
                    }
                    pc += 1;
                }
                OpCode::UpdatePdaReverseLookup {
                    state_id: _,
                    lookup_name,
                    pda_address,
                    primary_key,
                } => {
                    let actual_state_id = override_state_id;
                    let state = self
                        .states
                        .get_mut(&actual_state_id)
                        .ok_or("State table not found")?;
                    
                    let pda_val = self.registers[*pda_address].clone();
                    let pk_val = self.registers[*primary_key].clone();
                    
                    // Only update if both values are non-null strings
                    if let (Some(pda_str), Some(pk_str)) = (pda_val.as_str(), pk_val.as_str()) {
                        // Get or create the PDA reverse lookup table
                        let pda_lookup = state
                            .pda_reverse_lookups
                            .entry(lookup_name.clone())
                            .or_insert_with(|| PdaReverseLookup::new(10000));
                        
                        pda_lookup.insert(pda_str.to_string(), pk_str.to_string());
                        
                        tracing::debug!(
                            "📝 UpdatePdaReverseLookup: {} -> {} (lookup: {})",
                            pda_str, pk_str, lookup_name
                        );
                    } else if !pk_val.is_null() {
                        // Primary key might be a u64, convert to string
                        if let Some(pk_num) = pk_val.as_u64() {
                            if let Some(pda_str) = pda_val.as_str() {
                                let pda_lookup = state
                                    .pda_reverse_lookups
                                    .entry(lookup_name.clone())
                                    .or_insert_with(|| PdaReverseLookup::new(10000));
                                
                                pda_lookup.insert(pda_str.to_string(), pk_num.to_string());
                                
                                tracing::debug!(
                                    "📝 UpdatePdaReverseLookup: {} -> {} (lookup: {}, pk was u64)",
                                    pda_str, pk_num, lookup_name
                                );
                            }
                        }
                    }
                    
                    pc += 1;
                }
            }

            self.instructions_executed += 1;
        }

        Ok(output)
    }

    fn load_field(
        &self,
        event_value: &Value,
        path: &FieldPath,
        default: Option<&Value>,
    ) -> Result<Value> {
        // Warn if path contains empty segments - this indicates a bug in AST generation
        if path.segments.iter().any(|s| s.is_empty()) {
            tracing::warn!(
                "load_field: path contains empty segment! path={:?}",
                path.segments
            );
        }
        
        if path.segments.is_empty() {
            // For WholeSource (empty path), filter out internal metadata fields
            // This prevents __account_address, __resolved_primary_key, __update_context
            // from appearing in captured account data
            if let Some(obj) = event_value.as_object() {
                let filtered: serde_json::Map<String, Value> = obj
                    .iter()
                    .filter(|(k, _)| !k.starts_with("__"))
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                return Ok(Value::Object(filtered));
            }
            return Ok(event_value.clone());
        }

        let mut current = event_value;
        for (i, segment) in path.segments.iter().enumerate() {
            current = match current.get(segment) {
                Some(v) => v,
                None => {
                    // Log when we can't find a path segment - this helps debug key loading issues
                    if path.segments.len() == 2 && path.segments[0] == "accounts" {
                        tracing::warn!(
                            "load_field: could not find segment '{}' at index {} in path {:?}",
                            segment,
                            i,
                            path.segments,
                        );
                        tracing::warn!(
                            "  event has top-level keys: {:?}",
                            event_value.as_object().map(|o| o.keys().collect::<Vec<_>>())
                        );
                        // Show what's in the accounts object if it exists
                        if let Some(accounts) = event_value.get("accounts") {
                            tracing::warn!(
                                "  'accounts' exists with keys: {:?}",
                                accounts.as_object().map(|o| o.keys().collect::<Vec<_>>())
                            );
                        } else {
                            tracing::warn!("  'accounts' key does NOT exist in event_value");
                        }
                    }
                    return Ok(default.cloned().unwrap_or(Value::Null));
                }
            };
        }

        Ok(current.clone())
    }

    fn set_field_auto_vivify(
        &mut self,
        object_reg: Register,
        path: &str,
        value_reg: Register,
    ) -> Result<()> {
        let compiled = self.get_compiled_path(path);
        let segments = compiled.segments();
        let value = self.registers[value_reg].clone();

        if !self.registers[object_reg].is_object() {
            self.registers[object_reg] = json!({});
        }

        let obj = self.registers[object_reg]
            .as_object_mut()
            .ok_or("Not an object")?;

        let mut current = obj;
        for (i, segment) in segments.iter().enumerate() {
            if i == segments.len() - 1 {
                current.insert(segment.to_string(), value.clone());
            } else {
                current
                    .entry(segment.to_string())
                    .or_insert_with(|| json!({}));
                current = current
                    .get_mut(segment)
                    .and_then(|v| v.as_object_mut())
                    .ok_or("Path collision: expected object")?;
            }
        }

        Ok(())
    }

    fn set_field_if_null(
        &mut self,
        object_reg: Register,
        path: &str,
        value_reg: Register,
    ) -> Result<bool> {
        let compiled = self.get_compiled_path(path);
        let segments = compiled.segments();
        let value = self.registers[value_reg].clone();

        if !self.registers[object_reg].is_object() {
            self.registers[object_reg] = json!({});
        }

        let obj = self.registers[object_reg]
            .as_object_mut()
            .ok_or("Not an object")?;

        let mut current = obj;
        let mut was_set = false;
        for (i, segment) in segments.iter().enumerate() {
            if i == segments.len() - 1 {
                if !current.contains_key(segment) || current.get(segment).unwrap().is_null() {
                    current.insert(segment.to_string(), value.clone());
                    was_set = true;
                }
            } else {
                current
                    .entry(segment.to_string())
                    .or_insert_with(|| json!({}));
                current = current
                    .get_mut(segment)
                    .and_then(|v| v.as_object_mut())
                    .ok_or("Path collision: expected object")?;
            }
        }

        Ok(was_set)
    }

    fn set_field_max(
        &mut self,
        object_reg: Register,
        path: &str,
        value_reg: Register,
    ) -> Result<bool> {
        let compiled = self.get_compiled_path(path);
        let segments = compiled.segments();
        let new_value = self.registers[value_reg].clone();

        if !self.registers[object_reg].is_object() {
            self.registers[object_reg] = json!({});
        }

        let obj = self.registers[object_reg]
            .as_object_mut()
            .ok_or("Not an object")?;

        let mut current = obj;
        let mut was_updated = false;
        for (i, segment) in segments.iter().enumerate() {
            if i == segments.len() - 1 {
                let should_update = if let Some(current_value) = current.get(segment) {
                    if current_value.is_null() {
                        true
                    } else {
                        match (current_value.as_i64(), new_value.as_i64()) {
                            (Some(current_val), Some(new_val)) => new_val > current_val,
                            (Some(current_val), None) if new_value.as_u64().is_some() => {
                                new_value.as_u64().unwrap() as i64 > current_val
                            }
                            (None, Some(new_val)) if current_value.as_u64().is_some() => {
                                new_val > current_value.as_u64().unwrap() as i64
                            }
                            (None, None) => match (current_value.as_u64(), new_value.as_u64()) {
                                (Some(current_val), Some(new_val)) => new_val > current_val,
                                _ => match (current_value.as_f64(), new_value.as_f64()) {
                                    (Some(current_val), Some(new_val)) => new_val > current_val,
                                    _ => false,
                                },
                            },
                            _ => false,
                        }
                    }
                } else {
                    true
                };

                if should_update {
                    current.insert(segment.to_string(), new_value.clone());
                    was_updated = true;
                }
            } else {
                current
                    .entry(segment.to_string())
                    .or_insert_with(|| json!({}));
                current = current
                    .get_mut(segment)
                    .and_then(|v| v.as_object_mut())
                    .ok_or("Path collision: expected object")?;
            }
        }

        Ok(was_updated)
    }

    fn set_field_sum(
        &mut self,
        object_reg: Register,
        path: &str,
        value_reg: Register,
    ) -> Result<bool> {
        let compiled = self.get_compiled_path(path);
        let segments = compiled.segments();
        let new_value = self.registers[value_reg].clone();

        if !self.registers[object_reg].is_object() {
            self.registers[object_reg] = json!({});
        }

        let obj = self.registers[object_reg]
            .as_object_mut()
            .ok_or("Not an object")?;

        let mut current = obj;
        for (i, segment) in segments.iter().enumerate() {
            if i == segments.len() - 1 {
                // Get current value (default to 0 if null/missing)
                let current_val = current
                    .get(segment)
                    .and_then(|v| {
                        if v.is_null() {
                            None
                        } else {
                            v.as_i64().or_else(|| v.as_u64().map(|n| n as i64))
                        }
                    })
                    .unwrap_or(0);

                // Add new value
                let new_val_num = new_value
                    .as_i64()
                    .or_else(|| new_value.as_u64().map(|n| n as i64))
                    .ok_or("Sum requires numeric value")?;

                let sum = current_val + new_val_num;
                current.insert(segment.to_string(), json!(sum));
                return Ok(true);
            } else {
                current
                    .entry(segment.to_string())
                    .or_insert_with(|| json!({}));
                current = current
                    .get_mut(segment)
                    .and_then(|v| v.as_object_mut())
                    .ok_or("Path collision: expected object")?;
            }
        }

        Ok(false)
    }

    fn set_field_increment(&mut self, object_reg: Register, path: &str) -> Result<bool> {
        let compiled = self.get_compiled_path(path);
        let segments = compiled.segments();

        if !self.registers[object_reg].is_object() {
            self.registers[object_reg] = json!({});
        }

        let obj = self.registers[object_reg]
            .as_object_mut()
            .ok_or("Not an object")?;

        let mut current = obj;
        for (i, segment) in segments.iter().enumerate() {
            if i == segments.len() - 1 {
                // Get current value (default to 0 if null/missing)
                let current_val = current
                    .get(segment)
                    .and_then(|v| {
                        if v.is_null() {
                            None
                        } else {
                            v.as_i64().or_else(|| v.as_u64().map(|n| n as i64))
                        }
                    })
                    .unwrap_or(0);

                let incremented = current_val + 1;
                current.insert(segment.to_string(), json!(incremented));
                return Ok(true);
            } else {
                current
                    .entry(segment.to_string())
                    .or_insert_with(|| json!({}));
                current = current
                    .get_mut(segment)
                    .and_then(|v| v.as_object_mut())
                    .ok_or("Path collision: expected object")?;
            }
        }

        Ok(false)
    }

    fn set_field_min(
        &mut self,
        object_reg: Register,
        path: &str,
        value_reg: Register,
    ) -> Result<bool> {
        let compiled = self.get_compiled_path(path);
        let segments = compiled.segments();
        let new_value = self.registers[value_reg].clone();

        if !self.registers[object_reg].is_object() {
            self.registers[object_reg] = json!({});
        }

        let obj = self.registers[object_reg]
            .as_object_mut()
            .ok_or("Not an object")?;

        let mut current = obj;
        let mut was_updated = false;
        for (i, segment) in segments.iter().enumerate() {
            if i == segments.len() - 1 {
                let should_update = if let Some(current_value) = current.get(segment) {
                    if current_value.is_null() {
                        true
                    } else {
                        match (current_value.as_i64(), new_value.as_i64()) {
                            (Some(current_val), Some(new_val)) => new_val < current_val,
                            (Some(current_val), None) if new_value.as_u64().is_some() => {
                                (new_value.as_u64().unwrap() as i64) < current_val
                            }
                            (None, Some(new_val)) if current_value.as_u64().is_some() => {
                                new_val < current_value.as_u64().unwrap() as i64
                            }
                            (None, None) => match (current_value.as_u64(), new_value.as_u64()) {
                                (Some(current_val), Some(new_val)) => new_val < current_val,
                                _ => match (current_value.as_f64(), new_value.as_f64()) {
                                    (Some(current_val), Some(new_val)) => new_val < current_val,
                                    _ => false,
                                },
                            },
                            _ => false,
                        }
                    }
                } else {
                    true
                };

                if should_update {
                    current.insert(segment.to_string(), new_value.clone());
                    was_updated = true;
                }
            } else {
                current
                    .entry(segment.to_string())
                    .or_insert_with(|| json!({}));
                current = current
                    .get_mut(segment)
                    .and_then(|v| v.as_object_mut())
                    .ok_or("Path collision: expected object")?;
            }
        }

        Ok(was_updated)
    }

    fn get_field(&mut self, object_reg: Register, path: &str) -> Result<Value> {
        let compiled = self.get_compiled_path(path);
        let segments = compiled.segments();
        let mut current = &self.registers[object_reg];

        for segment in segments {
            current = current
                .get(segment)
                .ok_or_else(|| format!("Field not found: {}", segment))?;
        }

        Ok(current.clone())
    }

    fn append_to_array(
        &mut self,
        object_reg: Register,
        path: &str,
        value_reg: Register,
    ) -> Result<()> {
        let compiled = self.get_compiled_path(path);
        let segments = compiled.segments();
        let value = self.registers[value_reg].clone();

        if !self.registers[object_reg].is_object() {
            self.registers[object_reg] = json!({});
        }

        let obj = self.registers[object_reg]
            .as_object_mut()
            .ok_or("Not an object")?;

        let mut current = obj;
        for (i, segment) in segments.iter().enumerate() {
            if i == segments.len() - 1 {
                current
                    .entry(segment.to_string())
                    .or_insert_with(|| json!([]));
                let arr = current
                    .get_mut(segment)
                    .and_then(|v| v.as_array_mut())
                    .ok_or("Path is not an array")?;
                arr.push(value.clone());
            } else {
                current
                    .entry(segment.to_string())
                    .or_insert_with(|| json!({}));
                current = current
                    .get_mut(segment)
                    .and_then(|v| v.as_object_mut())
                    .ok_or("Path collision: expected object")?;
            }
        }

        Ok(())
    }

    fn transform_in_place(&mut self, reg: Register, transformation: &Transformation) -> Result<()> {
        let value = &self.registers[reg];
        let transformed = self.apply_transformation(value, transformation)?;
        self.registers[reg] = transformed;
        Ok(())
    }

    fn apply_transformation(
        &self,
        value: &Value,
        transformation: &Transformation,
    ) -> Result<Value> {
        match transformation {
            Transformation::HexEncode => {
                if let Some(arr) = value.as_array() {
                    let bytes: Vec<u8> = arr
                        .iter()
                        .filter_map(|v| v.as_u64().map(|n| n as u8))
                        .collect();
                    let hex = hex::encode(&bytes);
                    Ok(json!(hex))
                } else {
                    Err("HexEncode requires an array of numbers".into())
                }
            }
            Transformation::HexDecode => {
                if let Some(s) = value.as_str() {
                    let s = s.strip_prefix("0x").unwrap_or(s);
                    let bytes = hex::decode(s).map_err(|e| format!("Hex decode error: {}", e))?;
                    Ok(json!(bytes))
                } else {
                    Err("HexDecode requires a string".into())
                }
            }
            Transformation::Base58Encode => {
                if let Some(arr) = value.as_array() {
                    let bytes: Vec<u8> = arr
                        .iter()
                        .filter_map(|v| v.as_u64().map(|n| n as u8))
                        .collect();
                    let encoded = bs58::encode(&bytes).into_string();
                    Ok(json!(encoded))
                } else if value.is_string() {
                    // Value is already a string (likely already base58 encoded), return as-is
                    tracing::debug!("Base58Encode: value is already a string, passing through: {:?}", value);
                    Ok(value.clone())
                } else {
                    tracing::error!("Base58Encode failed: value type is {:?}, value: {:?}", 
                        if value.is_null() { "null" }
                        else if value.is_boolean() { "boolean" }
                        else if value.is_number() { "number" }
                        else if value.is_object() { "object" }
                        else { "unknown" },
                        value
                    );
                    Err("Base58Encode requires an array of numbers".into())
                }
            }
            Transformation::Base58Decode => {
                if let Some(s) = value.as_str() {
                    let bytes = bs58::decode(s).into_vec().map_err(|e| format!("Base58 decode error: {}", e))?;
                    Ok(json!(bytes))
                } else {
                    Err("Base58Decode requires a string".into())
                }
            }
            Transformation::ToString => Ok(json!(value.to_string())),
            Transformation::ToNumber => {
                if let Some(s) = value.as_str() {
                    let n = s
                        .parse::<i64>()
                        .map_err(|e| format!("Parse error: {}", e))?;
                    Ok(json!(n))
                } else {
                    Ok(value.clone())
                }
            }
        }
    }

    fn evaluate_comparison(
        &self,
        field_value: &Value,
        op: &ComparisonOp,
        condition_value: &Value,
    ) -> Result<bool> {
        use ComparisonOp::*;

        match op {
            Equal => Ok(field_value == condition_value),
            NotEqual => Ok(field_value != condition_value),
            GreaterThan => {
                // Try to compare as numbers
                match (field_value.as_i64(), condition_value.as_i64()) {
                    (Some(a), Some(b)) => Ok(a > b),
                    _ => match (field_value.as_u64(), condition_value.as_u64()) {
                        (Some(a), Some(b)) => Ok(a > b),
                        _ => match (field_value.as_f64(), condition_value.as_f64()) {
                            (Some(a), Some(b)) => Ok(a > b),
                            _ => Err("Cannot compare non-numeric values with GreaterThan".into()),
                        },
                    },
                }
            }
            GreaterThanOrEqual => match (field_value.as_i64(), condition_value.as_i64()) {
                (Some(a), Some(b)) => Ok(a >= b),
                _ => match (field_value.as_u64(), condition_value.as_u64()) {
                    (Some(a), Some(b)) => Ok(a >= b),
                    _ => match (field_value.as_f64(), condition_value.as_f64()) {
                        (Some(a), Some(b)) => Ok(a >= b),
                        _ => {
                            Err("Cannot compare non-numeric values with GreaterThanOrEqual".into())
                        }
                    },
                },
            },
            LessThan => match (field_value.as_i64(), condition_value.as_i64()) {
                (Some(a), Some(b)) => Ok(a < b),
                _ => match (field_value.as_u64(), condition_value.as_u64()) {
                    (Some(a), Some(b)) => Ok(a < b),
                    _ => match (field_value.as_f64(), condition_value.as_f64()) {
                        (Some(a), Some(b)) => Ok(a < b),
                        _ => Err("Cannot compare non-numeric values with LessThan".into()),
                    },
                },
            },
            LessThanOrEqual => match (field_value.as_i64(), condition_value.as_i64()) {
                (Some(a), Some(b)) => Ok(a <= b),
                _ => match (field_value.as_u64(), condition_value.as_u64()) {
                    (Some(a), Some(b)) => Ok(a <= b),
                    _ => match (field_value.as_f64(), condition_value.as_f64()) {
                        (Some(a), Some(b)) => Ok(a <= b),
                        _ => Err("Cannot compare non-numeric values with LessThanOrEqual".into()),
                    },
                },
            },
        }
    }

    /// Update a PDA reverse lookup and return pending updates for reprocessing
    ///
    /// After registering the PDA reverse lookup, this returns any pending account updates
    /// that were queued for this PDA. The caller should reprocess these through the VM
    /// by calling process_event() for each update.
    ///
    /// # Example
    /// ```ignore
    /// let pending = vm.update_pda_reverse_lookup(state_id, lookup_name, pda_addr, seed)?;
    /// for update in pending {
    ///     vm.process_event(&bytecode, update.account_data, &update.account_type)?;
    /// }
    /// ```
    #[cfg_attr(feature = "otel", instrument(
        name = "vm.update_pda_lookup",
        skip(self),
        fields(
            pda = %pda_address,
            seed = %seed_value,
        )
    ))]
    pub fn update_pda_reverse_lookup(
        &mut self,
        state_id: u32,
        lookup_name: &str,
        pda_address: String,
        seed_value: String,
    ) -> Result<Vec<PendingAccountUpdate>> {
        let state = self
            .states
            .get_mut(&state_id)
            .ok_or("State table not found")?;

        let lookup = state
            .pda_reverse_lookups
            .entry(lookup_name.to_string())
            .or_insert_with(|| PdaReverseLookup::new(10000));

        // Insert and check if an entry was evicted
        let evicted_pda = lookup.insert(pda_address.clone(), seed_value);

        // Clean up pending updates for the evicted PDA
        if let Some(ref evicted) = evicted_pda {
            if let Some((_, evicted_updates)) = state.pending_updates.remove(evicted) {
                let count = evicted_updates.len();
                self.pending_queue_size = self.pending_queue_size.saturating_sub(count as u64);
                tracing::info!(
                    "Cleaned up {} pending updates for evicted PDA {} from LRU cache",
                    count,
                    evicted
                );
            }
        }

        // Flush and return pending updates for this PDA
        self.flush_pending_updates(state_id, &pda_address)
    }

    /// Clean up expired pending updates that are older than the TTL
    ///
    /// Returns the number of updates that were removed.
    /// This should be called periodically to prevent memory leaks from orphaned updates.
    pub fn cleanup_expired_pending_updates(&mut self, state_id: u32) -> usize {
        let state = match self.states.get_mut(&state_id) {
            Some(s) => s,
            None => return 0,
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mut removed_count = 0;

        // Iterate through all pending updates and remove expired ones
        state.pending_updates.retain(|_pda_address, updates| {
            let original_len = updates.len();

            updates.retain(|update| {
                let age = now - update.queued_at;
                age <= PENDING_UPDATE_TTL_SECONDS
            });

            removed_count += original_len - updates.len();

            // Remove the entry entirely if no updates remain
            !updates.is_empty()
        });

        // Update the global counter
        self.pending_queue_size = self.pending_queue_size.saturating_sub(removed_count as u64);

        if removed_count > 0 {
            tracing::info!(
                "Cleaned up {} expired pending updates (TTL: {}s)",
                removed_count,
                PENDING_UPDATE_TTL_SECONDS
            );
        }

        removed_count
    }

    /// Queue an account update for later processing when PDA reverse lookup is not yet available
    ///
    /// # Workflow
    ///
    /// This implements a deferred processing pattern for account updates when the PDA reverse
    /// lookup needed to resolve the primary key is not yet available:
    ///
    /// 1. **Initial Account Update**: When an account update arrives but the PDA reverse lookup
    ///    is not available, call `queue_account_update()` to queue it for later.
    ///
    /// 2. **Register PDA Mapping**: When the instruction that establishes the PDA mapping is
    ///    processed, call `update_pda_reverse_lookup()` which returns pending updates.
    ///
    /// 3. **Reprocess Pending Updates**: Process the returned pending updates through the VM:
    ///    ```ignore
    ///    let pending = vm.update_pda_reverse_lookup(state_id, lookup_name, pda_addr, seed)?;
    ///    for update in pending {
    ///        let mutations = vm.process_event(
    ///            &bytecode,
    ///            update.account_data,
    ///            &update.account_type
    ///        )?;
    ///        // Handle mutations...
    ///    }
    ///    ```
    ///
    /// # Arguments
    ///
    /// * `state_id` - The state table ID
    /// * `pda_address` - The PDA address that needs reverse lookup
    /// * `account_type` - The event type name for reprocessing
    /// * `account_data` - The account data (event value) for reprocessing
    /// * `slot` - The slot number when this update occurred
    /// * `signature` - The transaction signature
    #[cfg_attr(feature = "otel", instrument(
        name = "vm.queue_account_update",
        skip(self, account_data),
        fields(
            pda = %pda_address,
            account_type = %account_type,
            slot = slot,
        )
    ))]
    pub fn queue_account_update(
        &mut self,
        state_id: u32,
        pda_address: String,
        account_type: String,
        account_data: Value,
        slot: u64,
        signature: String,
    ) -> Result<()> {
        // Check if we've exceeded the global limit
        if self.pending_queue_size >= MAX_PENDING_UPDATES_TOTAL as u64 {
            tracing::warn!(
                "Pending queue size limit reached ({}), cleaning up expired updates",
                MAX_PENDING_UPDATES_TOTAL
            );
            let removed = self.cleanup_expired_pending_updates(state_id);

            // If still at limit after cleanup, drop the oldest update
            if self.pending_queue_size >= MAX_PENDING_UPDATES_TOTAL as u64 {
                tracing::warn!(
                    "Still at limit after cleanup (removed {}), will drop oldest update",
                    removed
                );
                self.drop_oldest_pending_update(state_id)?;
            }
        }

        let state = self
            .states
            .get_mut(&state_id)
            .ok_or("State table not found")?;

        let pending = PendingAccountUpdate {
            account_type,
            pda_address: pda_address.clone(),
            account_data,
            slot,
            signature,
            queued_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        };

        // Get or create the vector for this PDA and enforce limits
        {
            let mut updates = state
                .pending_updates
                .entry(pda_address.clone())
                .or_insert_with(Vec::new);

            // Deduplication: Remove any updates with the same or older slot
            // Keep only updates with newer slots than the incoming one
            let original_len = updates.len();
            updates.retain(|existing| {
                // Keep if existing has a newer slot
                existing.slot > slot
            });
            let removed_by_dedup = original_len - updates.len();

            if removed_by_dedup > 0 {
                self.pending_queue_size = self
                    .pending_queue_size
                    .saturating_sub(removed_by_dedup as u64);
                tracing::debug!(
                    "Deduplicated {} older update(s) for PDA {} (new slot: {})",
                    removed_by_dedup,
                    pda_address,
                    slot
                );
            }

            // Enforce per-PDA limit after deduplication
            if updates.len() >= MAX_PENDING_UPDATES_PER_PDA {
                tracing::warn!(
                    "Per-PDA limit reached for {} ({} updates), dropping oldest",
                    pda_address,
                    updates.len()
                );
                updates.remove(0);
                self.pending_queue_size = self.pending_queue_size.saturating_sub(1);
            }

            updates.push(pending);
            self.pending_queue_size += 1;
        }

        Ok(())
    }

    /// Get statistics about the pending queue for monitoring
    pub fn get_pending_queue_stats(&self, state_id: u32) -> Option<PendingQueueStats> {
        let state = self.states.get(&state_id)?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let mut total_updates = 0;
        let mut oldest_timestamp = now;
        let mut largest_pda_queue = 0;
        let mut estimated_memory = 0;

        for entry in state.pending_updates.iter() {
            let (_, updates) = entry.pair();
            total_updates += updates.len();
            largest_pda_queue = largest_pda_queue.max(updates.len());

            for update in updates.iter() {
                oldest_timestamp = oldest_timestamp.min(update.queued_at);
                // Rough memory estimate
                estimated_memory += update.account_type.len() +
                                   update.pda_address.len() +
                                   update.signature.len() +
                                   16 + // slot + queued_at
                                   estimate_json_size(&update.account_data);
            }
        }

        Some(PendingQueueStats {
            total_updates,
            unique_pdas: state.pending_updates.len(),
            oldest_age_seconds: now - oldest_timestamp,
            largest_pda_queue_size: largest_pda_queue,
            estimated_memory_bytes: estimated_memory,
        })
    }

    /// Drop the oldest pending update across all PDAs
    fn drop_oldest_pending_update(&mut self, state_id: u32) -> Result<()> {
        let state = self
            .states
            .get_mut(&state_id)
            .ok_or("State table not found")?;

        let mut oldest_pda: Option<String> = None;
        let mut oldest_timestamp = i64::MAX;

        // Find the PDA with the oldest update
        for entry in state.pending_updates.iter() {
            let (pda, updates) = entry.pair();
            if let Some(update) = updates.first() {
                if update.queued_at < oldest_timestamp {
                    oldest_timestamp = update.queued_at;
                    oldest_pda = Some(pda.clone());
                }
            }
        }

        // Remove the oldest update
        if let Some(pda) = oldest_pda {
            if let Some(mut updates) = state.pending_updates.get_mut(&pda) {
                if !updates.is_empty() {
                    updates.remove(0);
                    self.pending_queue_size = self.pending_queue_size.saturating_sub(1);

                    // Remove the entry if it's now empty
                    if updates.is_empty() {
                        drop(updates);
                        state.pending_updates.remove(&pda);
                    }
                }
            }
        }

        Ok(())
    }

    /// Flush and return pending updates for a PDA for external reprocessing
    ///
    /// Returns the pending updates that were queued for this PDA address.
    /// The caller should reprocess these through the VM using process_event().
    fn flush_pending_updates(
        &mut self,
        state_id: u32,
        pda_address: &str,
    ) -> Result<Vec<PendingAccountUpdate>> {
        let state = self
            .states
            .get_mut(&state_id)
            .ok_or("State table not found")?;

        if let Some((_, pending_updates)) = state.pending_updates.remove(pda_address) {
            let count = pending_updates.len();
            self.pending_queue_size = self.pending_queue_size.saturating_sub(count as u64);
            Ok(pending_updates)
        } else {
            Ok(Vec::new())
        }
    }

    /// Try to resolve a primary key via PDA reverse lookup
    pub fn try_pda_reverse_lookup(
        &mut self,
        state_id: u32,
        lookup_name: &str,
        pda_address: &str,
    ) -> Option<String> {
        let state = self.states.get_mut(&state_id)?;

        if let Some(lookup) = state.pda_reverse_lookups.get_mut(lookup_name) {
            if let Some(value) = lookup.lookup(pda_address) {
                self.pda_cache_hits += 1;
                return Some(value);
            }
        }

        self.pda_cache_misses += 1;
        None
    }

    // ============================================================================
    // Computed Expression Evaluator (Task 5)
    // ============================================================================

    /// Evaluate a computed expression AST against the current state
    /// This is the core runtime evaluator for computed fields from the AST
    pub fn evaluate_computed_expr(&self, expr: &ComputedExpr, state: &Value) -> Result<Value> {
        self.evaluate_computed_expr_with_env(expr, state, &std::collections::HashMap::new())
    }

    /// Evaluate a computed expression with a variable environment (for let bindings)
    fn evaluate_computed_expr_with_env(
        &self,
        expr: &ComputedExpr,
        state: &Value,
        env: &std::collections::HashMap<String, Value>,
    ) -> Result<Value> {
        match expr {
            ComputedExpr::FieldRef { path } => {
                self.get_field_from_state(state, path)
            }

            ComputedExpr::Var { name } => {
                env.get(name)
                    .cloned()
                    .ok_or_else(|| format!("Undefined variable: {}", name).into())
            }

            ComputedExpr::Let { name, value, body } => {
                let val = self.evaluate_computed_expr_with_env(value, state, env)?;
                let mut new_env = env.clone();
                new_env.insert(name.clone(), val);
                self.evaluate_computed_expr_with_env(body, state, &new_env)
            }

            ComputedExpr::If { condition, then_branch, else_branch } => {
                let cond_val = self.evaluate_computed_expr_with_env(condition, state, env)?;
                if self.value_to_bool(&cond_val) {
                    self.evaluate_computed_expr_with_env(then_branch, state, env)
                } else {
                    self.evaluate_computed_expr_with_env(else_branch, state, env)
                }
            }

            ComputedExpr::None => Ok(Value::Null),

            ComputedExpr::Some { value } => {
                self.evaluate_computed_expr_with_env(value, state, env)
            }

            ComputedExpr::Slice { expr, start, end } => {
                let val = self.evaluate_computed_expr_with_env(expr, state, env)?;
                match val {
                    Value::Array(arr) => {
                        let slice: Vec<Value> = arr.get(*start..*end)
                            .unwrap_or(&[])
                            .to_vec();
                        Ok(Value::Array(slice))
                    }
                    _ => Err(format!("Cannot slice non-array value: {:?}", val).into()),
                }
            }

            ComputedExpr::Index { expr, index } => {
                let val = self.evaluate_computed_expr_with_env(expr, state, env)?;
                match val {
                    Value::Array(arr) => {
                        Ok(arr.get(*index).cloned().unwrap_or(Value::Null))
                    }
                    _ => Err(format!("Cannot index non-array value: {:?}", val).into()),
                }
            }

            ComputedExpr::U64FromLeBytes { bytes } => {
                let val = self.evaluate_computed_expr_with_env(bytes, state, env)?;
                let byte_vec = self.value_to_bytes(&val)?;
                if byte_vec.len() < 8 {
                    return Err(format!("u64::from_le_bytes requires 8 bytes, got {}", byte_vec.len()).into());
                }
                let arr: [u8; 8] = byte_vec[..8].try_into()
                    .map_err(|_| "Failed to convert to [u8; 8]")?;
                Ok(json!(u64::from_le_bytes(arr)))
            }

            ComputedExpr::U64FromBeBytes { bytes } => {
                let val = self.evaluate_computed_expr_with_env(bytes, state, env)?;
                let byte_vec = self.value_to_bytes(&val)?;
                if byte_vec.len() < 8 {
                    return Err(format!("u64::from_be_bytes requires 8 bytes, got {}", byte_vec.len()).into());
                }
                let arr: [u8; 8] = byte_vec[..8].try_into()
                    .map_err(|_| "Failed to convert to [u8; 8]")?;
                Ok(json!(u64::from_be_bytes(arr)))
            }

            ComputedExpr::ByteArray { bytes } => {
                Ok(Value::Array(bytes.iter().map(|b| json!(*b)).collect()))
            }

            ComputedExpr::Closure { param, body } => {
                // Closures are stored as-is; they're evaluated when used in map()
                // Return a special representation
                Ok(json!({
                    "__closure": {
                        "param": param,
                        "body": serde_json::to_value(body).unwrap_or(Value::Null)
                    }
                }))
            }

            ComputedExpr::Unary { op, expr } => {
                let val = self.evaluate_computed_expr_with_env(expr, state, env)?;
                self.apply_unary_op(op, &val)
            }

            ComputedExpr::JsonToBytes { expr } => {
                let val = self.evaluate_computed_expr_with_env(expr, state, env)?;
                // Convert JSON array of numbers to byte array
                let bytes = self.value_to_bytes(&val)?;
                Ok(Value::Array(bytes.iter().map(|b| json!(*b)).collect()))
            }

            ComputedExpr::UnwrapOr { expr, default } => {
                let val = self.evaluate_computed_expr_with_env(expr, state, env)?;
                if val.is_null() {
                    Ok(default.clone())
                } else {
                    Ok(val)
                }
            }

            ComputedExpr::Binary { op, left, right } => {
                let l = self.evaluate_computed_expr_with_env(left, state, env)?;
                let r = self.evaluate_computed_expr_with_env(right, state, env)?;
                self.apply_binary_op(op, &l, &r)
            }

            ComputedExpr::Cast { expr, to_type } => {
                let val = self.evaluate_computed_expr_with_env(expr, state, env)?;
                self.apply_cast(&val, to_type)
            }

            ComputedExpr::MethodCall { expr, method, args } => {
                let val = self.evaluate_computed_expr_with_env(expr, state, env)?;
                // Special handling for map() with closures
                if method == "map" && args.len() == 1 {
                    if let ComputedExpr::Closure { param, body } = &args[0] {
                        // If the value is null, return null (Option::None.map returns None)
                        if val.is_null() {
                            return Ok(Value::Null);
                        }
                        // Evaluate the closure body with the value bound to param
                        let mut closure_env = env.clone();
                        closure_env.insert(param.clone(), val);
                        return self.evaluate_computed_expr_with_env(body, state, &closure_env);
                    }
                }
                let evaluated_args: Vec<Value> = args
                    .iter()
                    .map(|a| self.evaluate_computed_expr_with_env(a, state, env))
                    .collect::<Result<Vec<_>>>()?;
                self.apply_method_call(&val, method, &evaluated_args)
            }

            ComputedExpr::Literal { value } => {
                Ok(value.clone())
            }

            ComputedExpr::Paren { expr } => {
                self.evaluate_computed_expr_with_env(expr, state, env)
            }
        }
    }

    /// Convert a JSON value to a byte vector
    fn value_to_bytes(&self, val: &Value) -> Result<Vec<u8>> {
        match val {
            Value::Array(arr) => {
                arr.iter()
                    .map(|v| v.as_u64().map(|n| n as u8).ok_or_else(|| "Array element not a valid byte".into()))
                    .collect()
            }
            Value::String(s) => {
                // Try to decode as hex
                if s.starts_with("0x") || s.starts_with("0X") {
                    hex::decode(&s[2..]).map_err(|e| format!("Invalid hex string: {}", e).into())
                } else {
                    hex::decode(s).map_err(|e| format!("Invalid hex string: {}", e).into())
                }
            }
            _ => Err(format!("Cannot convert {:?} to bytes", val).into()),
        }
    }

    /// Apply a unary operation
    fn apply_unary_op(&self, op: &crate::ast::UnaryOp, val: &Value) -> Result<Value> {
        use crate::ast::UnaryOp;
        match op {
            UnaryOp::Not => {
                Ok(json!(!self.value_to_bool(val)))
            }
            UnaryOp::ReverseBits => {
                match val.as_u64() {
                    Some(n) => Ok(json!(n.reverse_bits())),
                    None => match val.as_i64() {
                        Some(n) => Ok(json!((n as u64).reverse_bits())),
                        None => Err("reverse_bits requires an integer".into()),
                    }
                }
            }
        }
    }

    /// Get a field value from state by path (e.g., "section.field" or just "field")
    fn get_field_from_state(&self, state: &Value, path: &str) -> Result<Value> {
        let segments: Vec<&str> = path.split('.').collect();
        let mut current = state;

        for segment in segments {
            match current.get(segment) {
                Some(v) => current = v,
                None => return Ok(Value::Null),
            }
        }

        Ok(current.clone())
    }

    /// Apply a binary operation to two values
    fn apply_binary_op(&self, op: &BinaryOp, left: &Value, right: &Value) -> Result<Value> {
        match op {
            // Arithmetic operations
            BinaryOp::Add => self.numeric_op(left, right, |a, b| a + b, |a, b| a + b),
            BinaryOp::Sub => self.numeric_op(left, right, |a, b| a - b, |a, b| a - b),
            BinaryOp::Mul => self.numeric_op(left, right, |a, b| a * b, |a, b| a * b),
            BinaryOp::Div => {
                // Check for division by zero
                if let Some(r) = right.as_i64() {
                    if r == 0 {
                        return Err("Division by zero".into());
                    }
                }
                if let Some(r) = right.as_f64() {
                    if r == 0.0 {
                        return Err("Division by zero".into());
                    }
                }
                self.numeric_op(left, right, |a, b| a / b, |a, b| a / b)
            }
            BinaryOp::Mod => {
                // Modulo - only for integers
                match (left.as_i64(), right.as_i64()) {
                    (Some(a), Some(b)) if b != 0 => Ok(json!(a % b)),
                    (None, _) | (_, None) => {
                        match (left.as_u64(), right.as_u64()) {
                            (Some(a), Some(b)) if b != 0 => Ok(json!(a % b)),
                            _ => Err("Modulo requires non-zero integer operands".into()),
                        }
                    }
                    _ => Err("Modulo by zero".into()),
                }
            }

            // Comparison operations
            BinaryOp::Gt => self.comparison_op(left, right, |a, b| a > b, |a, b| a > b),
            BinaryOp::Lt => self.comparison_op(left, right, |a, b| a < b, |a, b| a < b),
            BinaryOp::Gte => self.comparison_op(left, right, |a, b| a >= b, |a, b| a >= b),
            BinaryOp::Lte => self.comparison_op(left, right, |a, b| a <= b, |a, b| a <= b),
            BinaryOp::Eq => Ok(json!(left == right)),
            BinaryOp::Ne => Ok(json!(left != right)),

            // Logical operations
            BinaryOp::And => {
                let l_bool = self.value_to_bool(left);
                let r_bool = self.value_to_bool(right);
                Ok(json!(l_bool && r_bool))
            }
            BinaryOp::Or => {
                let l_bool = self.value_to_bool(left);
                let r_bool = self.value_to_bool(right);
                Ok(json!(l_bool || r_bool))
            }

            // Bitwise operations
            BinaryOp::Xor => {
                match (left.as_u64(), right.as_u64()) {
                    (Some(a), Some(b)) => Ok(json!(a ^ b)),
                    _ => match (left.as_i64(), right.as_i64()) {
                        (Some(a), Some(b)) => Ok(json!(a ^ b)),
                        _ => Err("XOR requires integer operands".into()),
                    }
                }
            }
            BinaryOp::BitAnd => {
                match (left.as_u64(), right.as_u64()) {
                    (Some(a), Some(b)) => Ok(json!(a & b)),
                    _ => match (left.as_i64(), right.as_i64()) {
                        (Some(a), Some(b)) => Ok(json!(a & b)),
                        _ => Err("BitAnd requires integer operands".into()),
                    }
                }
            }
            BinaryOp::BitOr => {
                match (left.as_u64(), right.as_u64()) {
                    (Some(a), Some(b)) => Ok(json!(a | b)),
                    _ => match (left.as_i64(), right.as_i64()) {
                        (Some(a), Some(b)) => Ok(json!(a | b)),
                        _ => Err("BitOr requires integer operands".into()),
                    }
                }
            }
            BinaryOp::Shl => {
                match (left.as_u64(), right.as_u64()) {
                    (Some(a), Some(b)) => Ok(json!(a << b)),
                    _ => match (left.as_i64(), right.as_i64()) {
                        (Some(a), Some(b)) => Ok(json!(a << b)),
                        _ => Err("Shl requires integer operands".into()),
                    }
                }
            }
            BinaryOp::Shr => {
                match (left.as_u64(), right.as_u64()) {
                    (Some(a), Some(b)) => Ok(json!(a >> b)),
                    _ => match (left.as_i64(), right.as_i64()) {
                        (Some(a), Some(b)) => Ok(json!(a >> b)),
                        _ => Err("Shr requires integer operands".into()),
                    }
                }
            }
        }
    }

    /// Helper for numeric operations that can work on integers or floats
    fn numeric_op<F1, F2>(
        &self,
        left: &Value,
        right: &Value,
        int_op: F1,
        float_op: F2,
    ) -> Result<Value>
    where
        F1: Fn(i64, i64) -> i64,
        F2: Fn(f64, f64) -> f64,
    {
        // Try i64 first
        if let (Some(a), Some(b)) = (left.as_i64(), right.as_i64()) {
            return Ok(json!(int_op(a, b)));
        }

        // Try u64
        if let (Some(a), Some(b)) = (left.as_u64(), right.as_u64()) {
            // For u64, we need to be careful with underflow in subtraction
            return Ok(json!(int_op(a as i64, b as i64)));
        }

        // Try f64
        if let (Some(a), Some(b)) = (left.as_f64(), right.as_f64()) {
            return Ok(json!(float_op(a, b)));
        }

        // If either is null, return null
        if left.is_null() || right.is_null() {
            return Ok(Value::Null);
        }

        Err(format!(
            "Cannot perform numeric operation on {:?} and {:?}",
            left, right
        )
        .into())
    }

    /// Helper for comparison operations
    fn comparison_op<F1, F2>(
        &self,
        left: &Value,
        right: &Value,
        int_cmp: F1,
        float_cmp: F2,
    ) -> Result<Value>
    where
        F1: Fn(i64, i64) -> bool,
        F2: Fn(f64, f64) -> bool,
    {
        // Try i64 first
        if let (Some(a), Some(b)) = (left.as_i64(), right.as_i64()) {
            return Ok(json!(int_cmp(a, b)));
        }

        // Try u64
        if let (Some(a), Some(b)) = (left.as_u64(), right.as_u64()) {
            return Ok(json!(int_cmp(a as i64, b as i64)));
        }

        // Try f64
        if let (Some(a), Some(b)) = (left.as_f64(), right.as_f64()) {
            return Ok(json!(float_cmp(a, b)));
        }

        // If either is null, comparison returns false
        if left.is_null() || right.is_null() {
            return Ok(json!(false));
        }

        Err(format!(
            "Cannot compare {:?} and {:?}",
            left, right
        )
        .into())
    }

    /// Convert a value to boolean for logical operations
    fn value_to_bool(&self, value: &Value) -> bool {
        match value {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    i != 0
                } else if let Some(f) = n.as_f64() {
                    f != 0.0
                } else {
                    true
                }
            }
            Value::String(s) => !s.is_empty(),
            Value::Array(arr) => !arr.is_empty(),
            Value::Object(obj) => !obj.is_empty(),
        }
    }

    /// Apply a type cast to a value
    fn apply_cast(&self, value: &Value, to_type: &str) -> Result<Value> {
        match to_type {
            "i8" | "i16" | "i32" | "i64" | "isize" => {
                if let Some(n) = value.as_i64() {
                    Ok(json!(n))
                } else if let Some(n) = value.as_u64() {
                    Ok(json!(n as i64))
                } else if let Some(n) = value.as_f64() {
                    Ok(json!(n as i64))
                } else if let Some(s) = value.as_str() {
                    s.parse::<i64>()
                        .map(|n| json!(n))
                        .map_err(|e| format!("Cannot parse '{}' as integer: {}", s, e).into())
                } else {
                    Err(format!("Cannot cast {:?} to {}", value, to_type).into())
                }
            }
            "u8" | "u16" | "u32" | "u64" | "usize" => {
                if let Some(n) = value.as_u64() {
                    Ok(json!(n))
                } else if let Some(n) = value.as_i64() {
                    Ok(json!(n as u64))
                } else if let Some(n) = value.as_f64() {
                    Ok(json!(n as u64))
                } else if let Some(s) = value.as_str() {
                    s.parse::<u64>()
                        .map(|n| json!(n))
                        .map_err(|e| format!("Cannot parse '{}' as unsigned integer: {}", s, e).into())
                } else {
                    Err(format!("Cannot cast {:?} to {}", value, to_type).into())
                }
            }
            "f32" | "f64" => {
                if let Some(n) = value.as_f64() {
                    Ok(json!(n))
                } else if let Some(n) = value.as_i64() {
                    Ok(json!(n as f64))
                } else if let Some(n) = value.as_u64() {
                    Ok(json!(n as f64))
                } else if let Some(s) = value.as_str() {
                    s.parse::<f64>()
                        .map(|n| json!(n))
                        .map_err(|e| format!("Cannot parse '{}' as float: {}", s, e).into())
                } else {
                    Err(format!("Cannot cast {:?} to {}", value, to_type).into())
                }
            }
            "String" | "string" => {
                Ok(json!(value.to_string()))
            }
            "bool" => {
                Ok(json!(self.value_to_bool(value)))
            }
            _ => {
                // Unknown type, return value as-is
                Ok(value.clone())
            }
        }
    }

    /// Apply a method call to a value
    fn apply_method_call(&self, value: &Value, method: &str, args: &[Value]) -> Result<Value> {
        match method {
            "unwrap_or" => {
                if value.is_null() && !args.is_empty() {
                    Ok(args[0].clone())
                } else {
                    Ok(value.clone())
                }
            }
            "unwrap_or_default" => {
                if value.is_null() {
                    // Return default for common types
                    Ok(json!(0))
                } else {
                    Ok(value.clone())
                }
            }
            "is_some" => {
                Ok(json!(!value.is_null()))
            }
            "is_none" => {
                Ok(json!(value.is_null()))
            }
            "abs" => {
                if let Some(n) = value.as_i64() {
                    Ok(json!(n.abs()))
                } else if let Some(n) = value.as_f64() {
                    Ok(json!(n.abs()))
                } else {
                    Err(format!("Cannot call abs() on {:?}", value).into())
                }
            }
            "len" => {
                if let Some(s) = value.as_str() {
                    Ok(json!(s.len()))
                } else if let Some(arr) = value.as_array() {
                    Ok(json!(arr.len()))
                } else if let Some(obj) = value.as_object() {
                    Ok(json!(obj.len()))
                } else {
                    Err(format!("Cannot call len() on {:?}", value).into())
                }
            }
            "to_string" => {
                Ok(json!(value.to_string()))
            }
            "min" => {
                if args.is_empty() {
                    return Err("min() requires an argument".into());
                }
                let other = &args[0];
                if let (Some(a), Some(b)) = (value.as_i64(), other.as_i64()) {
                    Ok(json!(a.min(b)))
                } else if let (Some(a), Some(b)) = (value.as_f64(), other.as_f64()) {
                    Ok(json!(a.min(b)))
                } else {
                    Err(format!("Cannot call min() on {:?} and {:?}", value, other).into())
                }
            }
            "max" => {
                if args.is_empty() {
                    return Err("max() requires an argument".into());
                }
                let other = &args[0];
                if let (Some(a), Some(b)) = (value.as_i64(), other.as_i64()) {
                    Ok(json!(a.max(b)))
                } else if let (Some(a), Some(b)) = (value.as_f64(), other.as_f64()) {
                    Ok(json!(a.max(b)))
                } else {
                    Err(format!("Cannot call max() on {:?} and {:?}", value, other).into())
                }
            }
            "saturating_add" => {
                if args.is_empty() {
                    return Err("saturating_add() requires an argument".into());
                }
                let other = &args[0];
                if let (Some(a), Some(b)) = (value.as_i64(), other.as_i64()) {
                    Ok(json!(a.saturating_add(b)))
                } else if let (Some(a), Some(b)) = (value.as_u64(), other.as_u64()) {
                    Ok(json!(a.saturating_add(b)))
                } else {
                    Err(format!("Cannot call saturating_add() on {:?} and {:?}", value, other).into())
                }
            }
            "saturating_sub" => {
                if args.is_empty() {
                    return Err("saturating_sub() requires an argument".into());
                }
                let other = &args[0];
                if let (Some(a), Some(b)) = (value.as_i64(), other.as_i64()) {
                    Ok(json!(a.saturating_sub(b)))
                } else if let (Some(a), Some(b)) = (value.as_u64(), other.as_u64()) {
                    Ok(json!(a.saturating_sub(b)))
                } else {
                    Err(format!("Cannot call saturating_sub() on {:?} and {:?}", value, other).into())
                }
            }
            _ => {
                // Unknown method - return value unchanged with a warning
                tracing::warn!("Unknown method call: {}() on {:?}", method, value);
                Ok(value.clone())
            }
        }
    }

    /// Evaluate all computed fields for an entity and update the state
    /// This takes a list of ComputedFieldSpec from the AST and applies them
    pub fn evaluate_computed_fields_from_ast(
        &self,
        state: &mut Value,
        computed_field_specs: &[ComputedFieldSpec],
    ) -> Result<Vec<String>> {
        let mut updated_paths = Vec::new();

        for spec in computed_field_specs {
            match self.evaluate_computed_expr(&spec.expression, state) {
                Ok(result) => {
                    // Set the computed value in state
                    self.set_field_in_state(state, &spec.target_path, result)?;
                    updated_paths.push(spec.target_path.clone());
                }
                Err(e) => {
                    // Log error but continue with other computed fields
                    tracing::warn!(
                        "Failed to evaluate computed field '{}': {}",
                        spec.target_path,
                        e
                    );
                }
            }
        }

        Ok(updated_paths)
    }

    /// Set a field value in state by path (e.g., "section.field")
    fn set_field_in_state(&self, state: &mut Value, path: &str, value: Value) -> Result<()> {
        let segments: Vec<&str> = path.split('.').collect();
        
        if segments.is_empty() {
            return Err("Empty path".into());
        }

        // Navigate to parent, creating intermediate objects as needed
        let mut current = state;
        for (i, segment) in segments.iter().enumerate() {
            if i == segments.len() - 1 {
                // Last segment - set the value
                if let Some(obj) = current.as_object_mut() {
                    obj.insert(segment.to_string(), value);
                    return Ok(());
                } else {
                    return Err(format!("Cannot set field '{}' on non-object", segment).into());
                }
            } else {
                // Intermediate segment - navigate or create
                if !current.is_object() {
                    *current = json!({});
                }
                let obj = current.as_object_mut().unwrap();
                current = obj
                    .entry(segment.to_string())
                    .or_insert_with(|| json!({}));
            }
        }

        Ok(())
    }

    /// Create a computed fields evaluator closure from AST specs
    /// This returns a function that can be passed to the bytecode builder
    pub fn create_evaluator_from_specs(
        specs: Vec<ComputedFieldSpec>,
    ) -> impl Fn(&mut Value) -> Result<()> + Send + Sync + 'static {
        move |state: &mut Value| {
            // Create a temporary VmContext just for evaluation
            // (We only need the expression evaluation methods)
            let vm = VmContext::new();
            vm.evaluate_computed_fields_from_ast(state, &specs)?;
            Ok(())
        }
    }
}

impl Default for VmContext {
    fn default() -> Self {
        Self::new()
    }
}

// Implement the ReverseLookupUpdater trait for VmContext
impl crate::resolvers::ReverseLookupUpdater for VmContext {
    fn update(&mut self, pda_address: String, seed_value: String) -> Vec<PendingAccountUpdate> {
        // Use default state_id=0 and default lookup name
        self.update_pda_reverse_lookup(0, "default_pda_lookup", pda_address, seed_value)
            .unwrap_or_else(|e| {
                tracing::error!("Failed to update PDA reverse lookup: {}", e);
                Vec::new()
            })
    }

    fn flush_pending(&mut self, pda_address: &str) -> Vec<PendingAccountUpdate> {
        // Flush is handled inside update_pda_reverse_lookup, but we can also call it directly
        self.flush_pending_updates(0, pda_address)
            .unwrap_or_else(|e| {
                tracing::error!("Failed to flush pending updates: {}", e);
                Vec::new()
            })
    }
}
