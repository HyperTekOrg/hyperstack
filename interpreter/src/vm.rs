use crate::ast::{
    BinaryOp, ComparisonOp, ComputedExpr, ComputedFieldSpec, FieldPath, Transformation,
};
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
    /// Write version for account updates (monotonically increasing per account within a slot)
    /// Used for staleness detection to reject out-of-order updates
    pub write_version: Option<u64>,
    /// Transaction index for instruction updates (orders transactions within a slot)
    /// Used for staleness detection to reject out-of-order updates
    pub txn_index: Option<u64>,
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
            write_version: None,
            txn_index: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a new UpdateContext with slot, signature, and timestamp
    pub fn with_timestamp(slot: u64, signature: String, timestamp: i64) -> Self {
        Self {
            slot: Some(slot),
            signature: Some(signature),
            timestamp: Some(timestamp),
            write_version: None,
            txn_index: None,
            metadata: HashMap::new(),
        }
    }

    /// Create context for account updates with write_version for staleness detection
    pub fn new_account(slot: u64, signature: String, write_version: u64) -> Self {
        Self {
            slot: Some(slot),
            signature: Some(signature),
            timestamp: None,
            write_version: Some(write_version),
            txn_index: None,
            metadata: HashMap::new(),
        }
    }

    /// Create context for instruction updates with txn_index for staleness detection
    pub fn new_instruction(slot: u64, signature: String, txn_index: u64) -> Self {
        Self {
            slot: Some(slot),
            signature: Some(signature),
            timestamp: None,
            write_version: None,
            txn_index: Some(txn_index),
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
    /// Returns true if this is an account update context (has write_version, no txn_index)
    pub fn is_account_update(&self) -> bool {
        self.write_version.is_some() && self.txn_index.is_none()
    }

    /// Returns true if this is an instruction update context (has txn_index, no write_version)
    pub fn is_instruction_update(&self) -> bool {
        self.txn_index.is_some() && self.write_version.is_none()
    }

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
const MAX_PENDING_UPDATES_TOTAL: usize = 2_500;
const MAX_PENDING_UPDATES_PER_PDA: usize = 50;
const PENDING_UPDATE_TTL_SECONDS: i64 = 300; // 5 minutes

// Temporal index configuration - prevents unbounded history growth
const TEMPORAL_HISTORY_TTL_SECONDS: i64 = 300; // 5 minutes, matches pending queue TTL
const MAX_TEMPORAL_ENTRIES_PER_KEY: usize = 250;

// State table configuration - aligned with downstream EntityCache (500 per view)
const DEFAULT_MAX_STATE_TABLE_ENTRIES: usize = 2_500;
const DEFAULT_MAX_ARRAY_LENGTH: usize = 100;

const DEFAULT_MAX_LOOKUP_INDEX_ENTRIES: usize = 2_500;

const DEFAULT_MAX_VERSION_TRACKER_ENTRIES: usize = 2_500;

// Smaller cache for instruction deduplication - provides shorter effective TTL
// since we don't expect instruction duplicates to arrive much later
const DEFAULT_MAX_INSTRUCTION_DEDUP_ENTRIES: usize = 500;

const DEFAULT_MAX_TEMPORAL_INDEX_KEYS: usize = 2_500;

const DEFAULT_MAX_PDA_REVERSE_LOOKUP_ENTRIES: usize = 2_500;

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

/// Represents the type of change made to a field for granular dirty tracking.
/// This enables emitting only the actual changes rather than entire field values.
#[derive(Debug, Clone)]
pub enum FieldChange {
    /// Field was replaced with a new value (emit the full value from state)
    Replaced,
    /// Items were appended to an array field (emit only the new items)
    Appended(Vec<Value>),
}

/// Tracks field modifications during handler execution with granular change information.
/// This replaces the simple HashSet<String> approach to enable delta-only emissions.
#[derive(Debug, Clone, Default)]
pub struct DirtyTracker {
    changes: HashMap<String, FieldChange>,
}

impl DirtyTracker {
    /// Create a new empty DirtyTracker
    pub fn new() -> Self {
        Self {
            changes: HashMap::new(),
        }
    }

    /// Mark a field as replaced (full value will be emitted)
    pub fn mark_replaced(&mut self, path: &str) {
        // If there was an append, it's now superseded by a full replacement
        self.changes.insert(path.to_string(), FieldChange::Replaced);
    }

    /// Record an appended value for a field
    pub fn mark_appended(&mut self, path: &str, value: Value) {
        match self.changes.get_mut(path) {
            Some(FieldChange::Appended(values)) => {
                // Add to existing appended values
                values.push(value);
            }
            Some(FieldChange::Replaced) => {
                // Field was already replaced, keep it as replaced
                // (the full value including the append will be emitted)
            }
            None => {
                // First append to this field
                self.changes
                    .insert(path.to_string(), FieldChange::Appended(vec![value]));
            }
        }
    }

    /// Check if there are any changes tracked
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Get the number of changed fields
    pub fn len(&self) -> usize {
        self.changes.len()
    }

    /// Iterate over all changes
    pub fn iter(&self) -> impl Iterator<Item = (&String, &FieldChange)> {
        self.changes.iter()
    }

    /// Get a set of all dirty field paths (for backward compatibility)
    pub fn dirty_paths(&self) -> HashSet<String> {
        self.changes.keys().cloned().collect()
    }

    /// Consume the tracker and return the changes map
    pub fn into_changes(self) -> HashMap<String, FieldChange> {
        self.changes
    }

    /// Get a reference to the changes map
    pub fn changes(&self) -> &HashMap<String, FieldChange> {
        &self.changes
    }

    /// Get paths that were appended (not replaced)
    pub fn appended_paths(&self) -> Vec<String> {
        self.changes
            .iter()
            .filter_map(|(path, change)| match change {
                FieldChange::Appended(_) => Some(path.clone()),
                FieldChange::Replaced => None,
            })
            .collect()
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
    current_context: Option<UpdateContext>,
    warnings: Vec<String>,
    last_pda_lookup_miss: Option<String>,
    last_pda_registered: Option<String>,
    last_lookup_index_keys: Vec<String>,
}

#[derive(Debug)]
pub struct LookupIndex {
    index: std::sync::Mutex<LruCache<String, Value>>,
}

impl LookupIndex {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_MAX_LOOKUP_INDEX_ENTRIES)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        LookupIndex {
            index: std::sync::Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity).expect("capacity must be > 0"),
            )),
        }
    }

    pub fn lookup(&self, lookup_value: &Value) -> Option<Value> {
        let key = value_to_cache_key(lookup_value);
        self.index.lock().unwrap().get(&key).cloned()
    }

    pub fn insert(&self, lookup_value: Value, primary_key: Value) {
        let key = value_to_cache_key(&lookup_value);
        self.index.lock().unwrap().put(key, primary_key);
    }

    pub fn len(&self) -> usize {
        self.index.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.lock().unwrap().is_empty()
    }
}

impl Default for LookupIndex {
    fn default() -> Self {
        Self::new()
    }
}

fn value_to_cache_key(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| "unknown".to_string()),
    }
}

#[derive(Debug)]
pub struct TemporalIndex {
    index: std::sync::Mutex<LruCache<String, Vec<(Value, i64)>>>,
}

impl Default for TemporalIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl TemporalIndex {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_MAX_TEMPORAL_INDEX_KEYS)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        TemporalIndex {
            index: std::sync::Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity).expect("capacity must be > 0"),
            )),
        }
    }

    pub fn lookup(&self, lookup_value: &Value, timestamp: i64) -> Option<Value> {
        let key = value_to_cache_key(lookup_value);
        let mut cache = self.index.lock().unwrap();
        if let Some(entries) = cache.get(&key) {
            for i in (0..entries.len()).rev() {
                if entries[i].1 <= timestamp {
                    return Some(entries[i].0.clone());
                }
            }
        }
        None
    }

    pub fn lookup_latest(&self, lookup_value: &Value) -> Option<Value> {
        let key = value_to_cache_key(lookup_value);
        let mut cache = self.index.lock().unwrap();
        if let Some(entries) = cache.get(&key) {
            if let Some(last) = entries.last() {
                return Some(last.0.clone());
            }
        }
        None
    }

    pub fn insert(&self, lookup_value: Value, primary_key: Value, timestamp: i64) {
        let key = value_to_cache_key(&lookup_value);
        let mut cache = self.index.lock().unwrap();

        let entries = cache.get_or_insert_mut(key, Vec::new);
        entries.push((primary_key, timestamp));
        entries.sort_by_key(|(_, ts)| *ts);

        let cutoff = timestamp - TEMPORAL_HISTORY_TTL_SECONDS;
        entries.retain(|(_, ts)| *ts >= cutoff);

        if entries.len() > MAX_TEMPORAL_ENTRIES_PER_KEY {
            let excess = entries.len() - MAX_TEMPORAL_ENTRIES_PER_KEY;
            entries.drain(0..excess);
        }
    }

    pub fn len(&self) -> usize {
        self.index.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.lock().unwrap().is_empty()
    }

    pub fn total_entries(&self) -> usize {
        self.index
            .lock()
            .unwrap()
            .iter()
            .map(|(_, entries)| entries.len())
            .sum()
    }

    pub fn cleanup_expired(&self, cutoff_timestamp: i64) -> usize {
        let mut cache = self.index.lock().unwrap();
        let mut total_removed = 0;

        for (_, entries) in cache.iter_mut() {
            let original_len = entries.len();
            entries.retain(|(_, ts)| *ts >= cutoff_timestamp);
            total_removed += original_len - entries.len();
        }

        total_removed
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
        let evicted = if self.index.len() >= self.index.cap().get() {
            self.index.peek_lru().map(|(k, _)| k.clone())
        } else {
            None
        };

        self.index.put(pda_address, seed_value);
        evicted
    }

    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }
}

/// Input for queueing an account update.
#[derive(Debug, Clone)]
pub struct QueuedAccountUpdate {
    pub pda_address: String,
    pub account_type: String,
    pub account_data: Value,
    pub slot: u64,
    pub write_version: u64,
    pub signature: String,
}

/// Internal representation of a pending account update with queue metadata.
#[derive(Debug, Clone)]
pub struct PendingAccountUpdate {
    pub account_type: String,
    pub pda_address: String,
    pub account_data: Value,
    pub slot: u64,
    pub write_version: u64,
    pub signature: String,
    pub queued_at: i64,
}

/// Input for queueing an instruction event when PDA lookup fails.
#[derive(Debug, Clone)]
pub struct QueuedInstructionEvent {
    pub pda_address: String,
    pub event_type: String,
    pub event_data: Value,
    pub slot: u64,
    pub signature: String,
}

/// Internal representation of a pending instruction event with queue metadata.
#[derive(Debug, Clone)]
pub struct PendingInstructionEvent {
    pub event_type: String,
    pub pda_address: String,
    pub event_data: Value,
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

#[derive(Debug, Clone, Default)]
pub struct VmMemoryStats {
    pub state_table_entity_count: usize,
    pub state_table_max_entries: usize,
    pub state_table_at_capacity: bool,
    pub lookup_index_count: usize,
    pub lookup_index_total_entries: usize,
    pub temporal_index_count: usize,
    pub temporal_index_total_entries: usize,
    pub pda_reverse_lookup_count: usize,
    pub pda_reverse_lookup_total_entries: usize,
    pub version_tracker_entries: usize,
    pub pending_queue_stats: Option<PendingQueueStats>,
    pub path_cache_size: usize,
}

#[derive(Debug, Clone, Default)]
pub struct CleanupResult {
    pub pending_updates_removed: usize,
    pub temporal_entries_removed: usize,
}

#[derive(Debug, Clone)]
pub struct CapacityWarning {
    pub current_entries: usize,
    pub max_entries: usize,
    pub entries_over_limit: usize,
}

#[derive(Debug, Clone)]
pub struct StateTableConfig {
    pub max_entries: usize,
    pub max_array_length: usize,
}

impl Default for StateTableConfig {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_MAX_STATE_TABLE_ENTRIES,
            max_array_length: DEFAULT_MAX_ARRAY_LENGTH,
        }
    }
}

#[derive(Debug)]
pub struct VersionTracker {
    cache: std::sync::Mutex<LruCache<String, (u64, u64)>>,
}

impl VersionTracker {
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_MAX_VERSION_TRACKER_ENTRIES)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        VersionTracker {
            cache: std::sync::Mutex::new(LruCache::new(
                NonZeroUsize::new(capacity).expect("capacity must be > 0"),
            )),
        }
    }

    fn make_key(primary_key: &Value, event_type: &str) -> String {
        format!("{}:{}", primary_key, event_type)
    }

    pub fn get(&self, primary_key: &Value, event_type: &str) -> Option<(u64, u64)> {
        let key = Self::make_key(primary_key, event_type);
        self.cache.lock().unwrap().get(&key).copied()
    }

    pub fn insert(&self, primary_key: &Value, event_type: &str, slot: u64, ordering_value: u64) {
        let key = Self::make_key(primary_key, event_type);
        self.cache.lock().unwrap().put(key, (slot, ordering_value));
    }

    pub fn len(&self) -> usize {
        self.cache.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.lock().unwrap().is_empty()
    }
}

impl Default for VersionTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct StateTable {
    pub data: DashMap<Value, Value>,
    access_times: DashMap<Value, i64>,
    pub lookup_indexes: HashMap<String, LookupIndex>,
    pub temporal_indexes: HashMap<String, TemporalIndex>,
    pub pda_reverse_lookups: HashMap<String, PdaReverseLookup>,
    pub pending_updates: DashMap<String, Vec<PendingAccountUpdate>>,
    pub pending_instruction_events: DashMap<String, Vec<PendingInstructionEvent>>,
    version_tracker: VersionTracker,
    instruction_dedup_cache: VersionTracker,
    config: StateTableConfig,
    #[cfg_attr(not(feature = "otel"), allow(dead_code))]
    entity_name: String,
}

impl StateTable {
    pub fn is_at_capacity(&self) -> bool {
        self.data.len() >= self.config.max_entries
    }

    pub fn entries_over_limit(&self) -> usize {
        self.data.len().saturating_sub(self.config.max_entries)
    }

    pub fn max_array_length(&self) -> usize {
        self.config.max_array_length
    }

    fn touch(&self, key: &Value) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        self.access_times.insert(key.clone(), now);
    }

    fn evict_lru(&self, count: usize) -> usize {
        if count == 0 || self.data.is_empty() {
            return 0;
        }

        let mut entries: Vec<(Value, i64)> = self
            .access_times
            .iter()
            .map(|entry| (entry.key().clone(), *entry.value()))
            .collect();

        entries.sort_by_key(|(_, ts)| *ts);

        let to_evict: Vec<Value> = entries.iter().take(count).map(|(k, _)| k.clone()).collect();

        let mut evicted = 0;
        for key in to_evict {
            self.data.remove(&key);
            self.access_times.remove(&key);
            evicted += 1;
        }

        #[cfg(feature = "otel")]
        if evicted > 0 {
            crate::vm_metrics::record_state_table_eviction(evicted as u64, &self.entity_name);
        }

        evicted
    }

    pub fn insert_with_eviction(&self, key: Value, value: Value) {
        if self.data.len() >= self.config.max_entries && !self.data.contains_key(&key) {
            #[cfg(feature = "otel")]
            crate::vm_metrics::record_state_table_at_capacity(&self.entity_name);
            let to_evict = (self.data.len() + 1).saturating_sub(self.config.max_entries);
            self.evict_lru(to_evict.max(1));
        }
        self.data.insert(key.clone(), value);
        self.touch(&key);
    }

    pub fn get_and_touch(&self, key: &Value) -> Option<Value> {
        let result = self.data.get(key).map(|v| v.clone());
        if result.is_some() {
            self.touch(key);
        }
        result
    }

    /// Check if an update is fresh and update the version tracker.
    /// Returns true if the update should be processed (is fresh).
    /// Returns false if the update is stale and should be skipped.
    ///
    /// Comparison is lexicographic on (slot, ordering_value):
    /// (100, 5) > (100, 3) > (99, 999)
    pub fn is_fresh_update(
        &self,
        primary_key: &Value,
        event_type: &str,
        slot: u64,
        ordering_value: u64,
    ) -> bool {
        let dominated = self
            .version_tracker
            .get(primary_key, event_type)
            .map(|(last_slot, last_version)| (slot, ordering_value) <= (last_slot, last_version))
            .unwrap_or(false);

        if dominated {
            return false;
        }

        self.version_tracker
            .insert(primary_key, event_type, slot, ordering_value);
        true
    }

    /// Check if an instruction is a duplicate of one we've seen recently.
    /// Returns true if this exact instruction has been seen before (is a duplicate).
    /// Returns false if this is a new instruction that should be processed.
    ///
    /// Unlike account updates, instructions don't use recency checks - all
    /// unique instructions are processed. Only exact duplicates are skipped.
    /// Uses a smaller cache capacity for shorter effective TTL.
    pub fn is_duplicate_instruction(
        &self,
        primary_key: &Value,
        event_type: &str,
        slot: u64,
        txn_index: u64,
    ) -> bool {
        // Check if we've seen this exact instruction before
        let is_duplicate = self
            .instruction_dedup_cache
            .get(primary_key, event_type)
            .map(|(last_slot, last_txn_index)| slot == last_slot && txn_index == last_txn_index)
            .unwrap_or(false);

        if is_duplicate {
            return true;
        }

        // Record this instruction for deduplication
        self.instruction_dedup_cache
            .insert(primary_key, event_type, slot, txn_index);
        false
    }
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
            warnings: Vec::new(),
            last_pda_lookup_miss: None,
            last_pda_registered: None,
            last_lookup_index_keys: Vec::new(),
        };
        vm.states.insert(
            0,
            StateTable {
                data: DashMap::new(),
                access_times: DashMap::new(),
                lookup_indexes: HashMap::new(),
                temporal_indexes: HashMap::new(),
                pda_reverse_lookups: HashMap::new(),
                pending_updates: DashMap::new(),
                pending_instruction_events: DashMap::new(),
                version_tracker: VersionTracker::new(),
                instruction_dedup_cache: VersionTracker::with_capacity(
                    DEFAULT_MAX_INSTRUCTION_DEDUP_ENTRIES,
                ),
                config: StateTableConfig::default(),
                entity_name: "default".to_string(),
            },
        );
        vm
    }

    pub fn new_with_config(state_config: StateTableConfig) -> Self {
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
            warnings: Vec::new(),
            last_pda_lookup_miss: None,
            last_pda_registered: None,
            last_lookup_index_keys: Vec::new(),
        };
        vm.states.insert(
            0,
            StateTable {
                data: DashMap::new(),
                access_times: DashMap::new(),
                lookup_indexes: HashMap::new(),
                temporal_indexes: HashMap::new(),
                pda_reverse_lookups: HashMap::new(),
                pending_updates: DashMap::new(),
                pending_instruction_events: DashMap::new(),
                version_tracker: VersionTracker::new(),
                instruction_dedup_cache: VersionTracker::with_capacity(
                    DEFAULT_MAX_INSTRUCTION_DEDUP_ENTRIES,
                ),
                config: state_config,
                entity_name: "default".to_string(),
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

    fn add_warning(&mut self, msg: String) {
        self.warnings.push(msg);
    }

    pub fn take_warnings(&mut self) -> Vec<String> {
        std::mem::take(&mut self.warnings)
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn update_state_from_register(
        &mut self,
        state_id: u32,
        key: Value,
        register: Register,
    ) -> Result<()> {
        let state = self.states.get(&state_id).ok_or("State table not found")?;
        let value = self.registers[register].clone();
        state.insert_with_eviction(key, value);
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

    /// Extract a patch from state based on the DirtyTracker.
    /// For Replaced fields: extracts the full value from state.
    /// For Appended fields: emits only the appended values as an array.
    pub fn extract_partial_state_with_tracker(
        &self,
        state_reg: Register,
        tracker: &DirtyTracker,
    ) -> Result<Value> {
        let full_state = &self.registers[state_reg];

        if tracker.is_empty() {
            return Ok(json!({}));
        }

        let mut partial = serde_json::Map::new();

        for (path, change) in tracker.iter() {
            let segments: Vec<&str> = path.split('.').collect();

            let value_to_insert = match change {
                FieldChange::Replaced => {
                    let mut current = full_state;
                    let mut found = true;

                    for segment in &segments {
                        match current.get(*segment) {
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
                    current.clone()
                }
                FieldChange::Appended(values) => Value::Array(values.clone()),
            };

            let mut target = &mut partial;
            for (i, segment) in segments.iter().enumerate() {
                if i == segments.len() - 1 {
                    target.insert(segment.to_string(), value_to_insert.clone());
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
            #[cfg(feature = "otel")]
            crate::vm_metrics::record_path_cache_hit();
            return compiled.clone();
        }
        #[cfg(feature = "otel")]
        crate::vm_metrics::record_path_cache_miss();
        let compiled = CompiledPath::new(path);
        self.path_cache.insert(path.to_string(), compiled.clone());
        compiled
    }

    /// Process an event with optional context metadata
    #[cfg_attr(feature = "otel", instrument(
        name = "vm.process_event",
        skip(self, bytecode, event_value, log),
        level = "info",
        fields(
            event_type = %event_type,
            slot = context.as_ref().and_then(|c| c.slot),
        )
    ))]
    pub fn process_event(
        &mut self,
        bytecode: &MultiEntityBytecode,
        event_value: Value,
        event_type: &str,
        context: Option<&UpdateContext>,
        mut log: Option<&mut crate::canonical_log::CanonicalLog>,
    ) -> Result<Vec<Mutation>> {
        self.current_context = context.cloned();

        let mut event_value = event_value;
        if let Some(ctx) = context {
            if let Some(obj) = event_value.as_object_mut() {
                obj.insert("__update_context".to_string(), ctx.to_value());
            }
        }

        let mut all_mutations = Vec::new();

        if let Some(entity_names) = bytecode.event_routing.get(event_type) {
            for entity_name in entity_names {
                if let Some(entity_bytecode) = bytecode.entities.get(entity_name) {
                    if let Some(handler) = entity_bytecode.handlers.get(event_type) {
                        if let Some(ref mut log) = log {
                            log.set("entity", entity_name.clone());
                            log.inc("handlers", 1);
                        }

                        let opcodes_before = self.instructions_executed;
                        let cache_before = self.cache_hits;
                        let pda_hits_before = self.pda_cache_hits;
                        let pda_misses_before = self.pda_cache_misses;

                        let mutations = self.execute_handler(
                            handler,
                            &event_value,
                            event_type,
                            entity_bytecode.state_id,
                            entity_name,
                            entity_bytecode.computed_fields_evaluator.as_ref(),
                        )?;

                        if let Some(ref mut log) = log {
                            log.inc(
                                "opcodes",
                                (self.instructions_executed - opcodes_before) as i64,
                            );
                            log.inc("cache_hits", (self.cache_hits - cache_before) as i64);
                            log.inc("pda_hits", (self.pda_cache_hits - pda_hits_before) as i64);
                            log.inc(
                                "pda_misses",
                                (self.pda_cache_misses - pda_misses_before) as i64,
                            );
                        }

                        if mutations.is_empty() {
                            if let Some(missed_pda) = self.take_last_pda_lookup_miss() {
                                if event_type.ends_with("IxState") {
                                    let slot = context.and_then(|c| c.slot).unwrap_or(0);
                                    let signature = context
                                        .and_then(|c| c.signature.clone())
                                        .unwrap_or_default();
                                    let _ = self.queue_instruction_event(
                                        entity_bytecode.state_id,
                                        QueuedInstructionEvent {
                                            pda_address: missed_pda,
                                            event_type: event_type.to_string(),
                                            event_data: event_value.clone(),
                                            slot,
                                            signature,
                                        },
                                    );
                                }
                            }
                        }

                        all_mutations.extend(mutations);

                        if let Some(registered_pda) = self.take_last_pda_registered() {
                            let pending_events = self.flush_pending_instruction_events(
                                entity_bytecode.state_id,
                                &registered_pda,
                            );
                            for pending in pending_events {
                                if let Some(pending_handler) =
                                    entity_bytecode.handlers.get(&pending.event_type)
                                {
                                    if let Ok(reprocessed_mutations) = self.execute_handler(
                                        pending_handler,
                                        &pending.event_data,
                                        &pending.event_type,
                                        entity_bytecode.state_id,
                                        entity_name,
                                        entity_bytecode.computed_fields_evaluator.as_ref(),
                                    ) {
                                        all_mutations.extend(reprocessed_mutations);
                                    }
                                }
                            }
                        }

                        let lookup_keys = self.take_last_lookup_index_keys();
                        for lookup_key in lookup_keys {
                            if let Ok(pending_updates) =
                                self.flush_pending_updates(entity_bytecode.state_id, &lookup_key)
                            {
                                for pending in pending_updates {
                                    if let Some(pending_handler) =
                                        entity_bytecode.handlers.get(&pending.account_type)
                                    {
                                        self.current_context = Some(UpdateContext::new_account(
                                            pending.slot,
                                            pending.signature.clone(),
                                            pending.write_version,
                                        ));
                                        if let Ok(reprocessed) = self.execute_handler(
                                            pending_handler,
                                            &pending.account_data,
                                            &pending.account_type,
                                            entity_bytecode.state_id,
                                            entity_name,
                                            entity_bytecode.computed_fields_evaluator.as_ref(),
                                        ) {
                                            all_mutations.extend(reprocessed);
                                        }
                                    }
                                }
                            }
                        }
                    } else if let Some(ref mut log) = log {
                        log.set("skip_reason", "no_handler");
                    }
                } else if let Some(ref mut log) = log {
                    log.set("skip_reason", "entity_not_found");
                }
            }
        } else if let Some(ref mut log) = log {
            log.set("skip_reason", "no_event_routing");
        }

        if let Some(log) = log {
            log.set("mutations", all_mutations.len() as i64);
            if let Some(first) = all_mutations.first() {
                if let Some(key_str) = first.key.as_str() {
                    log.set("primary_key", key_str);
                } else if let Some(key_num) = first.key.as_u64() {
                    log.set("primary_key", key_num as i64);
                }
            }
            if let Some(state) = self.states.get(&0) {
                log.set("state_table_size", state.data.len() as i64);
            }

            let warnings = self.take_warnings();
            if !warnings.is_empty() {
                log.set("warnings", warnings.len() as i64);
                log.set(
                    "warning_messages",
                    Value::Array(warnings.into_iter().map(Value::String).collect()),
                );
                log.set_level(crate::canonical_log::LogLevel::Warn);
            }
        } else {
            self.warnings.clear();
        }

        Ok(all_mutations)
    }

    pub fn process_any(
        &mut self,
        bytecode: &MultiEntityBytecode,
        any: prost_types::Any,
    ) -> Result<Vec<Mutation>> {
        let (event_value, event_type) = bytecode.proto_router.decode(any)?;
        self.process_event(bytecode, event_value, &event_type, None, None)
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
        event_value: &Value,
        event_type: &str,
        override_state_id: u32,
        entity_name: &str,
        entity_evaluator: Option<&Box<dyn Fn(&mut Value) -> Result<()> + Send + Sync>>,
    ) -> Result<Vec<Mutation>> {
        self.reset_registers();
        self.last_pda_lookup_miss = None;

        let mut pc: usize = 0;
        let mut output = Vec::new();
        let mut dirty_tracker = DirtyTracker::new();

        while pc < handler.len() {
            match &handler[pc] {
                OpCode::LoadEventField {
                    path,
                    dest,
                    default,
                } => {
                    let value = self.load_field(event_value, path, default.as_ref())?;
                    self.registers[*dest] = value;
                    pc += 1;
                }
                OpCode::LoadConstant { value, dest } => {
                    self.registers[*dest] = value.clone();
                    pc += 1;
                }
                OpCode::CopyRegister { source, dest } => {
                    self.registers[*dest] = self.registers[*source].clone();
                    pc += 1;
                }
                OpCode::CopyRegisterIfNull { source, dest } => {
                    if self.registers[*dest].is_null() {
                        self.registers[*dest] = self.registers[*source].clone();
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
                    self.set_field_auto_vivify(*object, path, *value)?;
                    dirty_tracker.mark_replaced(path);
                    pc += 1;
                }
                OpCode::SetFields { object, fields } => {
                    for (path, value_reg) in fields {
                        self.set_field_auto_vivify(*object, path, *value_reg)?;
                        dirty_tracker.mark_replaced(path);
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
                    let entity_name_owned = entity_name.to_string();
                    self.states
                        .entry(actual_state_id)
                        .or_insert_with(|| StateTable {
                            data: DashMap::new(),
                            access_times: DashMap::new(),
                            lookup_indexes: HashMap::new(),
                            temporal_indexes: HashMap::new(),
                            pda_reverse_lookups: HashMap::new(),
                            pending_updates: DashMap::new(),
                            pending_instruction_events: DashMap::new(),
                            version_tracker: VersionTracker::new(),
                            instruction_dedup_cache: VersionTracker::with_capacity(
                                DEFAULT_MAX_INSTRUCTION_DEDUP_ENTRIES,
                            ),
                            config: StateTableConfig::default(),
                            entity_name: entity_name_owned,
                        });
                    let key_value = self.registers[*key].clone();
                    let warn_null_key = key_value.is_null()
                        && event_type.ends_with("State")
                        && !event_type.ends_with("IxState");

                    if warn_null_key {
                        self.add_warning(format!(
                            "ReadOrInitState: key register {} is NULL for account state, event_type={}",
                            key, event_type
                        ));
                    }

                    let state = self
                        .states
                        .get(&actual_state_id)
                        .ok_or("State table not found")?;

                    if !key_value.is_null() {
                        if let Some(ctx) = &self.current_context {
                            // Account updates: use recency check to discard stale updates
                            if ctx.is_account_update() {
                                if let (Some(slot), Some(write_version)) =
                                    (ctx.slot, ctx.write_version)
                                {
                                    if !state.is_fresh_update(
                                        &key_value,
                                        event_type,
                                        slot,
                                        write_version,
                                    ) {
                                        self.add_warning(format!(
                                            "Stale account update skipped: slot={}, write_version={}",
                                            slot, write_version
                                        ));
                                        return Ok(Vec::new());
                                    }
                                }
                            }
                            // Instruction updates: process all, but skip exact duplicates
                            else if ctx.is_instruction_update() {
                                if let (Some(slot), Some(txn_index)) = (ctx.slot, ctx.txn_index) {
                                    if state.is_duplicate_instruction(
                                        &key_value, event_type, slot, txn_index,
                                    ) {
                                        self.add_warning(format!(
                                            "Duplicate instruction skipped: slot={}, txn_index={}",
                                            slot, txn_index
                                        ));
                                        return Ok(Vec::new());
                                    }
                                }
                            }
                        }
                    }
                    let value = state
                        .get_and_touch(&key_value)
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

                    state.insert_with_eviction(key_value, value_data);
                    pc += 1;
                }
                OpCode::AppendToArray {
                    object,
                    path,
                    value,
                } => {
                    let appended_value = self.registers[*value].clone();
                    let max_len = self
                        .states
                        .get(&override_state_id)
                        .map(|s| s.max_array_length())
                        .unwrap_or(DEFAULT_MAX_ARRAY_LENGTH);
                    self.append_to_array(*object, path, *value, max_len)?;
                    dirty_tracker.mark_appended(path, appended_value);
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
                        self.transform_in_place(*source, transformation)?;
                    } else {
                        let source_value = &self.registers[*source];
                        let value = self.apply_transformation(source_value, transformation)?;
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

                    if primary_key.is_null() || dirty_tracker.is_empty() {
                        let reason = if dirty_tracker.is_empty() {
                            "no_fields_modified"
                        } else {
                            "null_primary_key"
                        };
                        self.add_warning(format!(
                            "Skipping mutation for entity '{}': {} (dirty_fields={})",
                            entity_name,
                            reason,
                            dirty_tracker.len()
                        ));
                    } else {
                        let patch =
                            self.extract_partial_state_with_tracker(*state, &dirty_tracker)?;

                        let append = dirty_tracker.appended_paths();
                        let mutation = Mutation {
                            export: entity_name.clone(),
                            key: primary_key,
                            patch,
                            append,
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
                    let was_set = self.set_field_if_null(*object, path, *value)?;
                    if was_set {
                        dirty_tracker.mark_replaced(path);
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
                        dirty_tracker.mark_replaced(path);
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

                    index.insert(lookup_val.clone(), pk_val);

                    // Track lookup keys so process_event can flush queued account updates
                    if let Some(key_str) = lookup_val.as_str() {
                        self.last_lookup_index_keys.push(key_str.to_string());
                    }

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
                            #[cfg(feature = "otel")]
                            if found.is_null() {
                                crate::vm_metrics::record_lookup_index_miss(index_name);
                            } else {
                                crate::vm_metrics::record_lookup_index_hit(index_name);
                            }
                            found
                        } else {
                            Value::Null
                        }
                    };

                    let final_result = if result.is_null() {
                        if let Some(pda_str) = lookup_val.as_str() {
                            let state = self
                                .states
                                .get_mut(&actual_state_id)
                                .ok_or("State table not found")?;

                            if let Some(pda_lookup) =
                                state.pda_reverse_lookups.get_mut("default_pda_lookup")
                            {
                                if let Some(resolved) = pda_lookup.lookup(pda_str) {
                                    Value::String(resolved)
                                } else {
                                    self.last_pda_lookup_miss = Some(pda_str.to_string());
                                    Value::Null
                                }
                            } else {
                                self.last_pda_lookup_miss = Some(pda_str.to_string());
                                Value::Null
                            }
                        } else {
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
                        dirty_tracker.mark_replaced(path);
                    }
                    pc += 1;
                }
                OpCode::SetFieldIncrement { object, path } => {
                    let was_updated = self.set_field_increment(*object, path)?;
                    if was_updated {
                        dirty_tracker.mark_replaced(path);
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
                        dirty_tracker.mark_replaced(path);
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
                        dirty_tracker.mark_replaced(count_path);
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
                    let field_value = self.load_field(event_value, condition_field, None)?;
                    let condition_met =
                        self.evaluate_comparison(&field_value, condition_op, condition_value)?;

                    if condition_met {
                        self.set_field_auto_vivify(*object, path, *value)?;
                        dirty_tracker.mark_replaced(path);
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
                    let field_value = self.load_field(event_value, condition_field, None)?;
                    let condition_met =
                        self.evaluate_comparison(&field_value, condition_op, condition_value)?;

                    if condition_met {
                        let was_updated = self.set_field_increment(*object, path)?;
                        if was_updated {
                            dirty_tracker.mark_replaced(path);
                        }
                    }
                    pc += 1;
                }
                OpCode::EvaluateComputedFields {
                    state,
                    computed_paths,
                } => {
                    if let Some(evaluator) = entity_evaluator {
                        let old_values: Vec<_> = computed_paths
                            .iter()
                            .map(|path| Self::get_value_at_path(&self.registers[*state], path))
                            .collect();

                        let state_value = &mut self.registers[*state];
                        let eval_result = evaluator(state_value);

                        if eval_result.is_ok() {
                            for (path, old_value) in computed_paths.iter().zip(old_values.iter()) {
                                let new_value =
                                    Self::get_value_at_path(&self.registers[*state], path);

                                if new_value != *old_value {
                                    dirty_tracker.mark_replaced(path);
                                }
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

                    if let (Some(pda_str), Some(pk_str)) = (pda_val.as_str(), pk_val.as_str()) {
                        let pda_lookup = state
                            .pda_reverse_lookups
                            .entry(lookup_name.clone())
                            .or_insert_with(|| {
                                PdaReverseLookup::new(DEFAULT_MAX_PDA_REVERSE_LOOKUP_ENTRIES)
                            });

                        pda_lookup.insert(pda_str.to_string(), pk_str.to_string());
                        self.last_pda_registered = Some(pda_str.to_string());
                    } else if !pk_val.is_null() {
                        if let Some(pk_num) = pk_val.as_u64() {
                            if let Some(pda_str) = pda_val.as_str() {
                                let pda_lookup = state
                                    .pda_reverse_lookups
                                    .entry(lookup_name.clone())
                                    .or_insert_with(|| {
                                        PdaReverseLookup::new(
                                            DEFAULT_MAX_PDA_REVERSE_LOOKUP_ENTRIES,
                                        )
                                    });

                                pda_lookup.insert(pda_str.to_string(), pk_num.to_string());
                                self.last_pda_registered = Some(pda_str.to_string());
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
        if path.segments.is_empty() {
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
        for segment in path.segments.iter() {
            current = match current.get(segment) {
                Some(v) => v,
                None => return Ok(default.cloned().unwrap_or(Value::Null)),
            };
        }

        Ok(current.clone())
    }

    fn get_value_at_path(value: &Value, path: &str) -> Option<Value> {
        let mut current = value;
        for segment in path.split('.') {
            current = current.get(segment)?;
        }
        Some(current.clone())
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
                current.insert(segment.to_string(), value);
                return Ok(());
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

        // SetOnce should only set meaningful values. A null source typically means
        // the field doesn't exist in this event type (e.g., instruction events don't
        // have account data). Skip to preserve any existing value.
        if value.is_null() {
            return Ok(false);
        }

        if !self.registers[object_reg].is_object() {
            self.registers[object_reg] = json!({});
        }

        let obj = self.registers[object_reg]
            .as_object_mut()
            .ok_or("Not an object")?;

        let mut current = obj;
        for (i, segment) in segments.iter().enumerate() {
            if i == segments.len() - 1 {
                if !current.contains_key(segment) || current.get(segment).unwrap().is_null() {
                    current.insert(segment.to_string(), value);
                    return Ok(true);
                }
                return Ok(false);
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
                    current.insert(segment.to_string(), new_value);
                    return Ok(true);
                }
                return Ok(false);
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

    fn set_field_sum(
        &mut self,
        object_reg: Register,
        path: &str,
        value_reg: Register,
    ) -> Result<bool> {
        let compiled = self.get_compiled_path(path);
        let segments = compiled.segments();
        let new_value = &self.registers[value_reg];

        // Extract numeric value before borrowing object_reg mutably
        let new_val_num = new_value
            .as_i64()
            .or_else(|| new_value.as_u64().map(|n| n as i64))
            .ok_or("Sum requires numeric value")?;

        if !self.registers[object_reg].is_object() {
            self.registers[object_reg] = json!({});
        }

        let obj = self.registers[object_reg]
            .as_object_mut()
            .ok_or("Not an object")?;

        let mut current = obj;
        for (i, segment) in segments.iter().enumerate() {
            if i == segments.len() - 1 {
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
                    current.insert(segment.to_string(), new_value);
                    return Ok(true);
                }
                return Ok(false);
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
        max_length: usize,
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

                if arr.len() > max_length {
                    let excess = arr.len() - max_length;
                    arr.drain(0..excess);
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
                    Ok(value.clone())
                } else {
                    Err("Base58Encode requires an array of numbers".into())
                }
            }
            Transformation::Base58Decode => {
                if let Some(s) = value.as_str() {
                    let bytes = bs58::decode(s)
                        .into_vec()
                        .map_err(|e| format!("Base58 decode error: {}", e))?;
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

    /// Update a PDA reverse lookup and return pending updates for reprocessing.
    /// Returns any pending account updates that were queued for this PDA.
    /// ```ignore
    /// let pending = vm.update_pda_reverse_lookup(state_id, lookup_name, pda_addr, seed)?;
    /// for update in pending {
    ///     vm.process_event(&bytecode, update.account_data, &update.account_type, None, None)?;
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
            .or_insert_with(|| PdaReverseLookup::new(DEFAULT_MAX_PDA_REVERSE_LOOKUP_ENTRIES));

        let evicted_pda = lookup.insert(pda_address.clone(), seed_value);

        if let Some(ref evicted) = evicted_pda {
            if let Some((_, evicted_updates)) = state.pending_updates.remove(evicted) {
                let count = evicted_updates.len();
                self.pending_queue_size = self.pending_queue_size.saturating_sub(count as u64);
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
            #[cfg(feature = "otel")]
            crate::vm_metrics::record_pending_updates_expired(
                removed_count as u64,
                &state.entity_name,
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
    ///            &bytecode, update.account_data, &update.account_type, None, None
    ///        )?;
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
        skip(self, update),
        fields(
            pda = %update.pda_address,
            account_type = %update.account_type,
            slot = update.slot,
        )
    ))]
    pub fn queue_account_update(
        &mut self,
        state_id: u32,
        update: QueuedAccountUpdate,
    ) -> Result<()> {
        if self.pending_queue_size >= MAX_PENDING_UPDATES_TOTAL as u64 {
            self.cleanup_expired_pending_updates(state_id);
            if self.pending_queue_size >= MAX_PENDING_UPDATES_TOTAL as u64 {
                self.drop_oldest_pending_update(state_id)?;
            }
        }

        let state = self
            .states
            .get_mut(&state_id)
            .ok_or("State table not found")?;

        let pending = PendingAccountUpdate {
            account_type: update.account_type,
            pda_address: update.pda_address.clone(),
            account_data: update.account_data,
            slot: update.slot,
            write_version: update.write_version,
            signature: update.signature,
            queued_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        };

        let pda_address = pending.pda_address.clone();
        let slot = pending.slot;

        let mut updates = state
            .pending_updates
            .entry(pda_address.clone())
            .or_insert_with(Vec::new);

        let original_len = updates.len();
        updates.retain(|existing| existing.slot > slot);
        let removed_by_dedup = original_len - updates.len();

        if removed_by_dedup > 0 {
            self.pending_queue_size = self
                .pending_queue_size
                .saturating_sub(removed_by_dedup as u64);
        }

        if updates.len() >= MAX_PENDING_UPDATES_PER_PDA {
            updates.remove(0);
            self.pending_queue_size = self.pending_queue_size.saturating_sub(1);
        }

        updates.push(pending);
        #[cfg(feature = "otel")]
        crate::vm_metrics::record_pending_update_queued(&state.entity_name);

        Ok(())
    }

    pub fn queue_instruction_event(
        &mut self,
        state_id: u32,
        event: QueuedInstructionEvent,
    ) -> Result<()> {
        let state = self
            .states
            .get_mut(&state_id)
            .ok_or("State table not found")?;

        let pda_address = event.pda_address.clone();

        let pending = PendingInstructionEvent {
            event_type: event.event_type,
            pda_address: event.pda_address,
            event_data: event.event_data,
            slot: event.slot,
            signature: event.signature,
            queued_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        };

        let mut events = state
            .pending_instruction_events
            .entry(pda_address)
            .or_insert_with(Vec::new);

        if events.len() >= MAX_PENDING_UPDATES_PER_PDA {
            events.remove(0);
        }

        events.push(pending);

        Ok(())
    }

    pub fn take_last_pda_lookup_miss(&mut self) -> Option<String> {
        self.last_pda_lookup_miss.take()
    }

    pub fn take_last_pda_registered(&mut self) -> Option<String> {
        self.last_pda_registered.take()
    }

    pub fn take_last_lookup_index_keys(&mut self) -> Vec<String> {
        std::mem::take(&mut self.last_lookup_index_keys)
    }

    pub fn flush_pending_instruction_events(
        &mut self,
        state_id: u32,
        pda_address: &str,
    ) -> Vec<PendingInstructionEvent> {
        let state = match self.states.get_mut(&state_id) {
            Some(s) => s,
            None => return Vec::new(),
        };

        if let Some((_, events)) = state.pending_instruction_events.remove(pda_address) {
            events
        } else {
            Vec::new()
        }
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

    pub fn get_memory_stats(&self, state_id: u32) -> VmMemoryStats {
        let mut stats = VmMemoryStats {
            path_cache_size: self.path_cache.len(),
            ..Default::default()
        };

        if let Some(state) = self.states.get(&state_id) {
            stats.state_table_entity_count = state.data.len();
            stats.state_table_max_entries = state.config.max_entries;
            stats.state_table_at_capacity = state.is_at_capacity();

            stats.lookup_index_count = state.lookup_indexes.len();
            stats.lookup_index_total_entries =
                state.lookup_indexes.values().map(|idx| idx.len()).sum();

            stats.temporal_index_count = state.temporal_indexes.len();
            stats.temporal_index_total_entries = state
                .temporal_indexes
                .values()
                .map(|idx| idx.total_entries())
                .sum();

            stats.pda_reverse_lookup_count = state.pda_reverse_lookups.len();
            stats.pda_reverse_lookup_total_entries = state
                .pda_reverse_lookups
                .values()
                .map(|lookup| lookup.len())
                .sum();

            stats.version_tracker_entries = state.version_tracker.len();

            stats.pending_queue_stats = self.get_pending_queue_stats(state_id);
        }

        stats
    }

    pub fn cleanup_all_expired(&mut self, state_id: u32) -> CleanupResult {
        let pending_removed = self.cleanup_expired_pending_updates(state_id);
        let temporal_removed = self.cleanup_temporal_indexes(state_id);

        #[cfg(feature = "otel")]
        if let Some(state) = self.states.get(&state_id) {
            crate::vm_metrics::record_cleanup(
                pending_removed,
                temporal_removed,
                &state.entity_name,
            );
        }

        CleanupResult {
            pending_updates_removed: pending_removed,
            temporal_entries_removed: temporal_removed,
        }
    }

    fn cleanup_temporal_indexes(&mut self, state_id: u32) -> usize {
        let state = match self.states.get_mut(&state_id) {
            Some(s) => s,
            None => return 0,
        };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let cutoff = now - TEMPORAL_HISTORY_TTL_SECONDS;
        let mut total_removed = 0;

        for (_, index) in state.temporal_indexes.iter_mut() {
            total_removed += index.cleanup_expired(cutoff);
        }

        total_removed
    }

    pub fn check_state_table_capacity(&self, state_id: u32) -> Option<CapacityWarning> {
        let state = self.states.get(&state_id)?;

        if state.is_at_capacity() {
            Some(CapacityWarning {
                current_entries: state.data.len(),
                max_entries: state.config.max_entries,
                entries_over_limit: state.entries_over_limit(),
            })
        } else {
            None
        }
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
            #[cfg(feature = "otel")]
            crate::vm_metrics::record_pending_updates_flushed(count as u64, &state.entity_name);
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
            ComputedExpr::FieldRef { path } => self.get_field_from_state(state, path),

            ComputedExpr::Var { name } => env
                .get(name)
                .cloned()
                .ok_or_else(|| format!("Undefined variable: {}", name).into()),

            ComputedExpr::Let { name, value, body } => {
                let val = self.evaluate_computed_expr_with_env(value, state, env)?;
                let mut new_env = env.clone();
                new_env.insert(name.clone(), val);
                self.evaluate_computed_expr_with_env(body, state, &new_env)
            }

            ComputedExpr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_val = self.evaluate_computed_expr_with_env(condition, state, env)?;
                if self.value_to_bool(&cond_val) {
                    self.evaluate_computed_expr_with_env(then_branch, state, env)
                } else {
                    self.evaluate_computed_expr_with_env(else_branch, state, env)
                }
            }

            ComputedExpr::None => Ok(Value::Null),

            ComputedExpr::Some { value } => self.evaluate_computed_expr_with_env(value, state, env),

            ComputedExpr::Slice { expr, start, end } => {
                let val = self.evaluate_computed_expr_with_env(expr, state, env)?;
                match val {
                    Value::Array(arr) => {
                        let slice: Vec<Value> = arr.get(*start..*end).unwrap_or(&[]).to_vec();
                        Ok(Value::Array(slice))
                    }
                    _ => Err(format!("Cannot slice non-array value: {:?}", val).into()),
                }
            }

            ComputedExpr::Index { expr, index } => {
                let val = self.evaluate_computed_expr_with_env(expr, state, env)?;
                match val {
                    Value::Array(arr) => Ok(arr.get(*index).cloned().unwrap_or(Value::Null)),
                    _ => Err(format!("Cannot index non-array value: {:?}", val).into()),
                }
            }

            ComputedExpr::U64FromLeBytes { bytes } => {
                let val = self.evaluate_computed_expr_with_env(bytes, state, env)?;
                let byte_vec = self.value_to_bytes(&val)?;
                if byte_vec.len() < 8 {
                    return Err(format!(
                        "u64::from_le_bytes requires 8 bytes, got {}",
                        byte_vec.len()
                    )
                    .into());
                }
                let arr: [u8; 8] = byte_vec[..8]
                    .try_into()
                    .map_err(|_| "Failed to convert to [u8; 8]")?;
                Ok(json!(u64::from_le_bytes(arr)))
            }

            ComputedExpr::U64FromBeBytes { bytes } => {
                let val = self.evaluate_computed_expr_with_env(bytes, state, env)?;
                let byte_vec = self.value_to_bytes(&val)?;
                if byte_vec.len() < 8 {
                    return Err(format!(
                        "u64::from_be_bytes requires 8 bytes, got {}",
                        byte_vec.len()
                    )
                    .into());
                }
                let arr: [u8; 8] = byte_vec[..8]
                    .try_into()
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

            ComputedExpr::Literal { value } => Ok(value.clone()),

            ComputedExpr::Paren { expr } => self.evaluate_computed_expr_with_env(expr, state, env),
        }
    }

    /// Convert a JSON value to a byte vector
    fn value_to_bytes(&self, val: &Value) -> Result<Vec<u8>> {
        match val {
            Value::Array(arr) => arr
                .iter()
                .map(|v| {
                    v.as_u64()
                        .map(|n| n as u8)
                        .ok_or_else(|| "Array element not a valid byte".into())
                })
                .collect(),
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
            UnaryOp::Not => Ok(json!(!self.value_to_bool(val))),
            UnaryOp::ReverseBits => match val.as_u64() {
                Some(n) => Ok(json!(n.reverse_bits())),
                None => match val.as_i64() {
                    Some(n) => Ok(json!((n as u64).reverse_bits())),
                    None => Err("reverse_bits requires an integer".into()),
                },
            },
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
                    (None, _) | (_, None) => match (left.as_u64(), right.as_u64()) {
                        (Some(a), Some(b)) if b != 0 => Ok(json!(a % b)),
                        _ => Err("Modulo requires non-zero integer operands".into()),
                    },
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
            BinaryOp::Xor => match (left.as_u64(), right.as_u64()) {
                (Some(a), Some(b)) => Ok(json!(a ^ b)),
                _ => match (left.as_i64(), right.as_i64()) {
                    (Some(a), Some(b)) => Ok(json!(a ^ b)),
                    _ => Err("XOR requires integer operands".into()),
                },
            },
            BinaryOp::BitAnd => match (left.as_u64(), right.as_u64()) {
                (Some(a), Some(b)) => Ok(json!(a & b)),
                _ => match (left.as_i64(), right.as_i64()) {
                    (Some(a), Some(b)) => Ok(json!(a & b)),
                    _ => Err("BitAnd requires integer operands".into()),
                },
            },
            BinaryOp::BitOr => match (left.as_u64(), right.as_u64()) {
                (Some(a), Some(b)) => Ok(json!(a | b)),
                _ => match (left.as_i64(), right.as_i64()) {
                    (Some(a), Some(b)) => Ok(json!(a | b)),
                    _ => Err("BitOr requires integer operands".into()),
                },
            },
            BinaryOp::Shl => match (left.as_u64(), right.as_u64()) {
                (Some(a), Some(b)) => Ok(json!(a << b)),
                _ => match (left.as_i64(), right.as_i64()) {
                    (Some(a), Some(b)) => Ok(json!(a << b)),
                    _ => Err("Shl requires integer operands".into()),
                },
            },
            BinaryOp::Shr => match (left.as_u64(), right.as_u64()) {
                (Some(a), Some(b)) => Ok(json!(a >> b)),
                _ => match (left.as_i64(), right.as_i64()) {
                    (Some(a), Some(b)) => Ok(json!(a >> b)),
                    _ => Err("Shr requires integer operands".into()),
                },
            },
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

        Err(format!("Cannot compare {:?} and {:?}", left, right).into())
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
                    s.parse::<u64>().map(|n| json!(n)).map_err(|e| {
                        format!("Cannot parse '{}' as unsigned integer: {}", s, e).into()
                    })
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
            "String" | "string" => Ok(json!(value.to_string())),
            "bool" => Ok(json!(self.value_to_bool(value))),
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
            "is_some" => Ok(json!(!value.is_null())),
            "is_none" => Ok(json!(value.is_null())),
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
            "to_string" => Ok(json!(value.to_string())),
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
                    Err(format!(
                        "Cannot call saturating_add() on {:?} and {:?}",
                        value, other
                    )
                    .into())
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
                    Err(format!(
                        "Cannot call saturating_sub() on {:?} and {:?}",
                        value, other
                    )
                    .into())
                }
            }
            _ => Err(format!("Unknown method call: {}()", method).into()),
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
            if let Ok(result) = self.evaluate_computed_expr(&spec.expression, state) {
                self.set_field_in_state(state, &spec.target_path, result)?;
                updated_paths.push(spec.target_path.clone());
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
                current = obj.entry(segment.to_string()).or_insert_with(|| json!({}));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{BinaryOp, ComputedExpr, ComputedFieldSpec};

    #[test]
    fn test_computed_field_preserves_integer_type() {
        let vm = VmContext::new();

        let mut state = serde_json::json!({
            "trading": {
                "total_buy_volume": 20000000000_i64,
                "total_sell_volume": 17951316474_i64
            }
        });

        let spec = ComputedFieldSpec {
            target_path: "trading.total_volume".to_string(),
            result_type: "Option<u64>".to_string(),
            expression: ComputedExpr::Binary {
                op: BinaryOp::Add,
                left: Box::new(ComputedExpr::UnwrapOr {
                    expr: Box::new(ComputedExpr::FieldRef {
                        path: "trading.total_buy_volume".to_string(),
                    }),
                    default: serde_json::json!(0),
                }),
                right: Box::new(ComputedExpr::UnwrapOr {
                    expr: Box::new(ComputedExpr::FieldRef {
                        path: "trading.total_sell_volume".to_string(),
                    }),
                    default: serde_json::json!(0),
                }),
            },
        };

        vm.evaluate_computed_fields_from_ast(&mut state, &[spec])
            .unwrap();

        let total_volume = state
            .get("trading")
            .and_then(|t| t.get("total_volume"))
            .expect("total_volume should exist");

        let serialized = serde_json::to_string(total_volume).unwrap();
        assert!(
            !serialized.contains('.'),
            "Integer should not have decimal point: {}",
            serialized
        );
        assert_eq!(
            total_volume.as_i64(),
            Some(37951316474),
            "Value should be correct sum"
        );
    }

    #[test]
    fn test_set_field_sum_preserves_integer_type() {
        let mut vm = VmContext::new();
        vm.registers[0] = serde_json::json!({});
        vm.registers[1] = serde_json::json!(20000000000_i64);
        vm.registers[2] = serde_json::json!(17951316474_i64);

        vm.set_field_sum(0, "trading.total_buy_volume", 1).unwrap();
        vm.set_field_sum(0, "trading.total_sell_volume", 2).unwrap();

        let state = &vm.registers[0];
        let buy_vol = state
            .get("trading")
            .and_then(|t| t.get("total_buy_volume"))
            .unwrap();
        let sell_vol = state
            .get("trading")
            .and_then(|t| t.get("total_sell_volume"))
            .unwrap();

        let buy_serialized = serde_json::to_string(buy_vol).unwrap();
        let sell_serialized = serde_json::to_string(sell_vol).unwrap();

        assert!(
            !buy_serialized.contains('.'),
            "Buy volume should not have decimal: {}",
            buy_serialized
        );
        assert!(
            !sell_serialized.contains('.'),
            "Sell volume should not have decimal: {}",
            sell_serialized
        );
    }
}
