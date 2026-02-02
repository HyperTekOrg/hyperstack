use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::marker::PhantomData;

// ============================================================================
// IDL Snapshot Types - Embedded IDL for AST-only compilation
// ============================================================================

/// Snapshot of an Anchor IDL for embedding in the AST
/// Contains all information needed to generate parsers and SDK types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlSnapshot {
    /// Program name (e.g., "pump")
    pub name: String,
    /// Program ID this IDL belongs to
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub program_id: Option<String>,
    /// Program version
    pub version: String,
    /// Account type definitions
    pub accounts: Vec<IdlAccountSnapshot>,
    /// Instruction definitions
    pub instructions: Vec<IdlInstructionSnapshot>,
    /// Type definitions (structs, enums)
    #[serde(default)]
    pub types: Vec<IdlTypeDefSnapshot>,
    /// Event definitions
    #[serde(default)]
    pub events: Vec<IdlEventSnapshot>,
    /// Error definitions
    #[serde(default)]
    pub errors: Vec<IdlErrorSnapshot>,
    /// Discriminant size in bytes (1 for Steel, 8 for Anchor)
    /// Defaults to 8 (Anchor) for backwards compatibility
    #[serde(default = "default_discriminant_size")]
    pub discriminant_size: usize,
}

fn default_discriminant_size() -> usize {
    8
}

/// Account definition from IDL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlAccountSnapshot {
    /// Account name (e.g., "BondingCurve")
    pub name: String,
    /// 8-byte discriminator
    pub discriminator: Vec<u8>,
    /// Documentation
    #[serde(default)]
    pub docs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serialization: Option<IdlSerializationSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IdlSerializationSnapshot {
    Borsh,
    Bytemuck,
    #[serde(alias = "bytemuckunsafe")]
    BytemuckUnsafe,
}

/// Instruction definition from IDL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlInstructionSnapshot {
    /// Instruction name (e.g., "buy", "sell", "create")
    pub name: String,
    /// 8-byte discriminator
    pub discriminator: Vec<u8>,
    /// Documentation
    #[serde(default)]
    pub docs: Vec<String>,
    /// Account arguments
    pub accounts: Vec<IdlInstructionAccountSnapshot>,
    /// Data arguments
    pub args: Vec<IdlFieldSnapshot>,
}

/// Account argument in an instruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlInstructionAccountSnapshot {
    /// Account name (e.g., "mint", "user")
    pub name: String,
    /// Whether this account is writable
    #[serde(default)]
    pub writable: bool,
    /// Whether this account is a signer
    #[serde(default)]
    pub signer: bool,
    /// Optional - if the account is optional
    #[serde(default)]
    pub optional: bool,
    /// Fixed address constraint (if any)
    #[serde(default)]
    pub address: Option<String>,
    /// Documentation
    #[serde(default)]
    pub docs: Vec<String>,
}

/// Field definition (used in instructions, accounts, types)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlFieldSnapshot {
    /// Field name
    pub name: String,
    /// Field type
    #[serde(rename = "type")]
    pub type_: IdlTypeSnapshot,
}

/// Type representation from IDL
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IdlTypeSnapshot {
    /// Simple types: "u64", "bool", "string", "pubkey", etc.
    Simple(String),
    /// Array type: { "array": ["u8", 32] }
    Array(IdlArrayTypeSnapshot),
    /// Option type: { "option": "u64" }
    Option(IdlOptionTypeSnapshot),
    /// Vec type: { "vec": "u8" }
    Vec(IdlVecTypeSnapshot),
    /// Defined/custom type: { "defined": { "name": "MyStruct" } }
    Defined(IdlDefinedTypeSnapshot),
}

/// Array type representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlArrayTypeSnapshot {
    /// [element_type, size]
    pub array: Vec<IdlArrayElementSnapshot>,
}

/// Array element (can be type or size)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IdlArrayElementSnapshot {
    /// Nested type
    Type(IdlTypeSnapshot),
    /// Type name as string
    TypeName(String),
    /// Array size
    Size(u32),
}

/// Option type representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlOptionTypeSnapshot {
    pub option: Box<IdlTypeSnapshot>,
}

/// Vec type representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlVecTypeSnapshot {
    pub vec: Box<IdlTypeSnapshot>,
}

/// Defined/custom type reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlDefinedTypeSnapshot {
    pub defined: IdlDefinedInnerSnapshot,
}

/// Inner defined type (can be named or simple string)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IdlDefinedInnerSnapshot {
    /// Named: { "name": "MyStruct" }
    Named { name: String },
    /// Simple string reference
    Simple(String),
}

/// Type definition (struct or enum)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlTypeDefSnapshot {
    /// Type name
    pub name: String,
    /// Documentation
    #[serde(default)]
    pub docs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serialization: Option<IdlSerializationSnapshot>,
    /// Type definition (struct or enum)
    #[serde(rename = "type")]
    pub type_def: IdlTypeDefKindSnapshot,
}

/// Type definition kind (struct or enum)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IdlTypeDefKindSnapshot {
    /// Struct: { "kind": "struct", "fields": [...] }
    Struct {
        kind: String,
        fields: Vec<IdlFieldSnapshot>,
    },
    /// Tuple struct: { "kind": "struct", "fields": ["type1", "type2"] }
    TupleStruct {
        kind: String,
        fields: Vec<IdlTypeSnapshot>,
    },
    /// Enum: { "kind": "enum", "variants": [...] }
    Enum {
        kind: String,
        variants: Vec<IdlEnumVariantSnapshot>,
    },
}

/// Enum variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlEnumVariantSnapshot {
    pub name: String,
}

/// Event definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlEventSnapshot {
    /// Event name
    pub name: String,
    /// 8-byte discriminator
    pub discriminator: Vec<u8>,
    /// Documentation
    #[serde(default)]
    pub docs: Vec<String>,
}

/// Error definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdlErrorSnapshot {
    /// Error code
    pub code: u32,
    /// Error name
    pub name: String,
    /// Error message (optional - some IDLs omit this)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
}

impl IdlTypeSnapshot {
    /// Convert IDL type to Rust type string
    pub fn to_rust_type_string(&self) -> String {
        match self {
            IdlTypeSnapshot::Simple(s) => Self::map_simple_type(s),
            IdlTypeSnapshot::Array(arr) => {
                if arr.array.len() == 2 {
                    match (&arr.array[0], &arr.array[1]) {
                        (
                            IdlArrayElementSnapshot::TypeName(t),
                            IdlArrayElementSnapshot::Size(size),
                        ) => {
                            format!("[{}; {}]", Self::map_simple_type(t), size)
                        }
                        (
                            IdlArrayElementSnapshot::Type(nested),
                            IdlArrayElementSnapshot::Size(size),
                        ) => {
                            format!("[{}; {}]", nested.to_rust_type_string(), size)
                        }
                        _ => "Vec<u8>".to_string(),
                    }
                } else {
                    "Vec<u8>".to_string()
                }
            }
            IdlTypeSnapshot::Option(opt) => {
                format!("Option<{}>", opt.option.to_rust_type_string())
            }
            IdlTypeSnapshot::Vec(vec) => {
                format!("Vec<{}>", vec.vec.to_rust_type_string())
            }
            IdlTypeSnapshot::Defined(def) => match &def.defined {
                IdlDefinedInnerSnapshot::Named { name } => name.clone(),
                IdlDefinedInnerSnapshot::Simple(s) => s.clone(),
            },
        }
    }

    fn map_simple_type(idl_type: &str) -> String {
        match idl_type {
            "u8" => "u8".to_string(),
            "u16" => "u16".to_string(),
            "u32" => "u32".to_string(),
            "u64" => "u64".to_string(),
            "u128" => "u128".to_string(),
            "i8" => "i8".to_string(),
            "i16" => "i16".to_string(),
            "i32" => "i32".to_string(),
            "i64" => "i64".to_string(),
            "i128" => "i128".to_string(),
            "bool" => "bool".to_string(),
            "string" => "String".to_string(),
            "publicKey" | "pubkey" => "solana_pubkey::Pubkey".to_string(),
            "bytes" => "Vec<u8>".to_string(),
            _ => idl_type.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldPath {
    pub segments: Vec<String>,
    pub offsets: Option<Vec<usize>>,
}

impl FieldPath {
    pub fn new(segments: &[&str]) -> Self {
        FieldPath {
            segments: segments.iter().map(|s| s.to_string()).collect(),
            offsets: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Transformation {
    HexEncode,
    HexDecode,
    Base58Encode,
    Base58Decode,
    ToString,
    ToNumber,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PopulationStrategy {
    SetOnce,
    LastWrite,
    Append,
    Merge,
    Max,
    /// Sum numeric values (accumulator pattern for aggregations)
    Sum,
    /// Count occurrences (increments by 1 for each update)
    Count,
    /// Track minimum value
    Min,
    /// Track unique values and store the count
    /// Internally maintains a HashSet, exposes only the count
    UniqueCount,
}

// ============================================================================
// Computed Field Expression AST
// ============================================================================

/// Specification for a computed/derived field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputedFieldSpec {
    /// Target field path (e.g., "trading.total_volume")
    pub target_path: String,
    /// Expression AST
    pub expression: ComputedExpr,
    /// Result type (e.g., "Option<u64>", "Option<f64>")
    pub result_type: String,
}

/// AST for computed field expressions
/// Supports a subset of Rust expressions needed for computed fields:
/// - Field references (possibly from other sections)
/// - Unwrap with defaults
/// - Basic arithmetic and comparisons
/// - Type casts
/// - Method calls
/// - Let bindings and conditionals
/// - Byte array operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComputedExpr {
    // Existing variants
    /// Reference to a field: "field_name" or "section.field_name"
    FieldRef {
        path: String,
    },

    /// Unwrap with default: expr.unwrap_or(default)
    UnwrapOr {
        expr: Box<ComputedExpr>,
        default: serde_json::Value,
    },

    /// Binary operation: left op right
    Binary {
        op: BinaryOp,
        left: Box<ComputedExpr>,
        right: Box<ComputedExpr>,
    },

    /// Type cast: expr as type
    Cast {
        expr: Box<ComputedExpr>,
        to_type: String,
    },

    /// Method call: expr.method(args)
    MethodCall {
        expr: Box<ComputedExpr>,
        method: String,
        args: Vec<ComputedExpr>,
    },

    /// Literal value: numbers, booleans, strings
    Literal {
        value: serde_json::Value,
    },

    /// Parenthesized expression for grouping
    Paren {
        expr: Box<ComputedExpr>,
    },

    // Variable reference (for let bindings)
    Var {
        name: String,
    },

    // Let binding: let name = value; body
    Let {
        name: String,
        value: Box<ComputedExpr>,
        body: Box<ComputedExpr>,
    },

    // Conditional: if condition { then_branch } else { else_branch }
    If {
        condition: Box<ComputedExpr>,
        then_branch: Box<ComputedExpr>,
        else_branch: Box<ComputedExpr>,
    },

    // Option constructors
    None,
    Some {
        value: Box<ComputedExpr>,
    },

    // Byte/array operations
    Slice {
        expr: Box<ComputedExpr>,
        start: usize,
        end: usize,
    },
    Index {
        expr: Box<ComputedExpr>,
        index: usize,
    },

    // Byte conversion functions
    U64FromLeBytes {
        bytes: Box<ComputedExpr>,
    },
    U64FromBeBytes {
        bytes: Box<ComputedExpr>,
    },

    // Byte array literals: [0u8; 32] or [1, 2, 3]
    ByteArray {
        bytes: Vec<u8>,
    },

    // Closure for map operations: |x| body
    Closure {
        param: String,
        body: Box<ComputedExpr>,
    },

    // Unary operations
    Unary {
        op: UnaryOp,
        expr: Box<ComputedExpr>,
    },

    // JSON array to bytes conversion (for working with captured byte arrays)
    JsonToBytes {
        expr: Box<ComputedExpr>,
    },
}

/// Binary operators for computed expressions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    // Comparison
    Gt,
    Lt,
    Gte,
    Lte,
    Eq,
    Ne,
    // Logical
    And,
    Or,
    // Bitwise
    Xor,
    BitAnd,
    BitOr,
    Shl,
    Shr,
}

/// Unary operators for computed expressions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnaryOp {
    Not,
    ReverseBits,
}

/// Serializable version of StreamSpec without phantom types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableStreamSpec {
    pub state_name: String,
    /// Program ID (Solana address) - extracted from IDL
    #[serde(default)]
    pub program_id: Option<String>,
    /// Embedded IDL for AST-only compilation
    #[serde(default)]
    pub idl: Option<IdlSnapshot>,
    pub identity: IdentitySpec,
    pub handlers: Vec<SerializableHandlerSpec>,
    pub sections: Vec<EntitySection>,
    pub field_mappings: BTreeMap<String, FieldTypeInfo>,
    pub resolver_hooks: Vec<ResolverHook>,
    pub instruction_hooks: Vec<InstructionHook>,
    /// Computed field paths (legacy, for backward compatibility)
    #[serde(default)]
    pub computed_fields: Vec<String>,
    /// Computed field specifications with full expression AST
    #[serde(default)]
    pub computed_field_specs: Vec<ComputedFieldSpec>,
    /// Deterministic content hash (SHA256 of canonical JSON, excluding this field)
    /// Used for deduplication and version tracking
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    /// View definitions for derived/projected views
    #[serde(default)]
    pub views: Vec<ViewDef>,
}

#[derive(Debug, Clone)]
pub struct TypedStreamSpec<S> {
    pub state_name: String,
    pub identity: IdentitySpec,
    pub handlers: Vec<TypedHandlerSpec<S>>,
    pub sections: Vec<EntitySection>, // NEW: Complete structural information
    pub field_mappings: BTreeMap<String, FieldTypeInfo>, // NEW: All field type info by target path
    pub resolver_hooks: Vec<ResolverHook>, // NEW: Resolver hooks for PDA key resolution
    pub instruction_hooks: Vec<InstructionHook>, // NEW: Instruction hooks for PDA registration
    pub computed_fields: Vec<String>, // List of computed field paths
    _phantom: PhantomData<S>,
}

impl<S> TypedStreamSpec<S> {
    pub fn new(
        state_name: String,
        identity: IdentitySpec,
        handlers: Vec<TypedHandlerSpec<S>>,
    ) -> Self {
        TypedStreamSpec {
            state_name,
            identity,
            handlers,
            sections: Vec::new(),
            field_mappings: BTreeMap::new(),
            resolver_hooks: Vec::new(),
            instruction_hooks: Vec::new(),
            computed_fields: Vec::new(),
            _phantom: PhantomData,
        }
    }

    /// Enhanced constructor with type information
    pub fn with_type_info(
        state_name: String,
        identity: IdentitySpec,
        handlers: Vec<TypedHandlerSpec<S>>,
        sections: Vec<EntitySection>,
        field_mappings: BTreeMap<String, FieldTypeInfo>,
    ) -> Self {
        TypedStreamSpec {
            state_name,
            identity,
            handlers,
            sections,
            field_mappings,
            resolver_hooks: Vec::new(),
            instruction_hooks: Vec::new(),
            computed_fields: Vec::new(),
            _phantom: PhantomData,
        }
    }

    /// Get type information for a specific field path
    pub fn get_field_type(&self, path: &str) -> Option<&FieldTypeInfo> {
        self.field_mappings.get(path)
    }

    /// Get all fields for a specific section
    pub fn get_section_fields(&self, section_name: &str) -> Option<&Vec<FieldTypeInfo>> {
        self.sections
            .iter()
            .find(|s| s.name == section_name)
            .map(|s| &s.fields)
    }

    /// Get all section names
    pub fn get_section_names(&self) -> Vec<&String> {
        self.sections.iter().map(|s| &s.name).collect()
    }

    /// Convert to serializable format
    pub fn to_serializable(&self) -> SerializableStreamSpec {
        let mut spec = SerializableStreamSpec {
            state_name: self.state_name.clone(),
            program_id: None,
            idl: None,
            identity: self.identity.clone(),
            handlers: self.handlers.iter().map(|h| h.to_serializable()).collect(),
            sections: self.sections.clone(),
            field_mappings: self.field_mappings.clone(),
            resolver_hooks: self.resolver_hooks.clone(),
            instruction_hooks: self.instruction_hooks.clone(),
            computed_fields: self.computed_fields.clone(),
            computed_field_specs: Vec::new(),
            content_hash: None,
            views: Vec::new(),
        };
        spec.content_hash = Some(spec.compute_content_hash());
        spec
    }

    /// Create from serializable format
    pub fn from_serializable(spec: SerializableStreamSpec) -> Self {
        TypedStreamSpec {
            state_name: spec.state_name,
            identity: spec.identity,
            handlers: spec
                .handlers
                .into_iter()
                .map(|h| TypedHandlerSpec::from_serializable(h))
                .collect(),
            sections: spec.sections,
            field_mappings: spec.field_mappings,
            resolver_hooks: spec.resolver_hooks,
            instruction_hooks: spec.instruction_hooks,
            computed_fields: spec.computed_fields,
            _phantom: PhantomData,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentitySpec {
    pub primary_keys: Vec<String>,
    pub lookup_indexes: Vec<LookupIndexSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupIndexSpec {
    pub field_name: String,
    pub temporal_field: Option<String>,
}

// ============================================================================
// Level 1: Declarative Hook Extensions
// ============================================================================

/// Declarative resolver hook specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolverHook {
    /// Account type this resolver applies to (e.g., "BondingCurveState")
    pub account_type: String,

    /// Resolution strategy
    pub strategy: ResolverStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResolverStrategy {
    /// Look up PDA in reverse lookup table, queue if not found
    PdaReverseLookup {
        lookup_name: String,
        /// Instruction discriminators to queue until (8 bytes each)
        queue_discriminators: Vec<Vec<u8>>,
    },

    /// Extract primary key directly from account data (future)
    DirectField { field_path: FieldPath },
}

/// Declarative instruction hook specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionHook {
    /// Instruction type this hook applies to (e.g., "CreateIxState")
    pub instruction_type: String,

    /// Actions to perform when this instruction is processed
    pub actions: Vec<HookAction>,

    /// Lookup strategy for finding the entity
    pub lookup_by: Option<FieldPath>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HookAction {
    /// Register a PDA mapping for reverse lookup
    RegisterPdaMapping {
        pda_field: FieldPath,
        seed_field: FieldPath,
        lookup_name: String,
    },

    /// Set a field value (for #[track_from])
    SetField {
        target_field: String,
        source: MappingSource,
        condition: Option<ConditionExpr>,
    },

    /// Increment a field value (for conditional aggregations)
    IncrementField {
        target_field: String,
        increment_by: i64,
        condition: Option<ConditionExpr>,
    },
}

/// Simple condition expression (Level 1 - basic comparisons only)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConditionExpr {
    /// Expression as string (will be parsed and validated)
    pub expression: String,

    /// Parsed representation (for validation and execution)
    pub parsed: Option<ParsedCondition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParsedCondition {
    /// Binary comparison: field op value
    Comparison {
        field: FieldPath,
        op: ComparisonOp,
        value: serde_json::Value,
    },

    /// Logical AND/OR
    Logical {
        op: LogicalOp,
        conditions: Vec<ParsedCondition>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOp {
    Equal,
    NotEqual,
    GreaterThan,
    GreaterThanOrEqual,
    LessThan,
    LessThanOrEqual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogicalOp {
    And,
    Or,
}

/// Serializable version of HandlerSpec without phantom types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableHandlerSpec {
    pub source: SourceSpec,
    pub key_resolution: KeyResolutionStrategy,
    pub mappings: Vec<SerializableFieldMapping>,
    pub conditions: Vec<Condition>,
    pub emit: bool,
}

#[derive(Debug, Clone)]
pub struct TypedHandlerSpec<S> {
    pub source: SourceSpec,
    pub key_resolution: KeyResolutionStrategy,
    pub mappings: Vec<TypedFieldMapping<S>>,
    pub conditions: Vec<Condition>,
    pub emit: bool,
    _phantom: PhantomData<S>,
}

impl<S> TypedHandlerSpec<S> {
    pub fn new(
        source: SourceSpec,
        key_resolution: KeyResolutionStrategy,
        mappings: Vec<TypedFieldMapping<S>>,
        emit: bool,
    ) -> Self {
        TypedHandlerSpec {
            source,
            key_resolution,
            mappings,
            conditions: vec![],
            emit,
            _phantom: PhantomData,
        }
    }

    /// Convert to serializable format
    pub fn to_serializable(&self) -> SerializableHandlerSpec {
        SerializableHandlerSpec {
            source: self.source.clone(),
            key_resolution: self.key_resolution.clone(),
            mappings: self.mappings.iter().map(|m| m.to_serializable()).collect(),
            conditions: self.conditions.clone(),
            emit: self.emit,
        }
    }

    /// Create from serializable format
    pub fn from_serializable(spec: SerializableHandlerSpec) -> Self {
        TypedHandlerSpec {
            source: spec.source,
            key_resolution: spec.key_resolution,
            mappings: spec
                .mappings
                .into_iter()
                .map(|m| TypedFieldMapping::from_serializable(m))
                .collect(),
            conditions: spec.conditions,
            emit: spec.emit,
            _phantom: PhantomData,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyResolutionStrategy {
    Embedded {
        primary_field: FieldPath,
    },
    Lookup {
        primary_field: FieldPath,
    },
    Computed {
        primary_field: FieldPath,
        compute_partition: ComputeFunction,
    },
    TemporalLookup {
        lookup_field: FieldPath,
        timestamp_field: FieldPath,
        index_name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceSpec {
    Source {
        program_id: Option<String>,
        discriminator: Option<Vec<u8>>,
        type_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        serialization: Option<IdlSerializationSnapshot>,
    },
}

/// Serializable version of FieldMapping without phantom types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableFieldMapping {
    pub target_path: String,
    pub source: MappingSource,
    pub transform: Option<Transformation>,
    pub population: PopulationStrategy,
}

#[derive(Debug, Clone)]
pub struct TypedFieldMapping<S> {
    pub target_path: String,
    pub source: MappingSource,
    pub transform: Option<Transformation>,
    pub population: PopulationStrategy,
    _phantom: PhantomData<S>,
}

impl<S> TypedFieldMapping<S> {
    pub fn new(target_path: String, source: MappingSource, population: PopulationStrategy) -> Self {
        TypedFieldMapping {
            target_path,
            source,
            transform: None,
            population,
            _phantom: PhantomData,
        }
    }

    pub fn with_transform(mut self, transform: Transformation) -> Self {
        self.transform = Some(transform);
        self
    }

    /// Convert to serializable format
    pub fn to_serializable(&self) -> SerializableFieldMapping {
        SerializableFieldMapping {
            target_path: self.target_path.clone(),
            source: self.source.clone(),
            transform: self.transform.clone(),
            population: self.population.clone(),
        }
    }

    /// Create from serializable format
    pub fn from_serializable(mapping: SerializableFieldMapping) -> Self {
        TypedFieldMapping {
            target_path: mapping.target_path,
            source: mapping.source,
            transform: mapping.transform,
            population: mapping.population,
            _phantom: PhantomData,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MappingSource {
    FromSource {
        path: FieldPath,
        default: Option<Value>,
        transform: Option<Transformation>,
    },
    Constant(Value),
    Computed {
        inputs: Vec<FieldPath>,
        function: ComputeFunction,
    },
    FromState {
        path: String,
    },
    AsEvent {
        fields: Vec<Box<MappingSource>>,
    },
    WholeSource,
    /// Similar to WholeSource but with field-level transformations
    /// Used by #[capture] macro to apply transforms to specific fields in an account
    AsCapture {
        field_transforms: BTreeMap<String, Transformation>,
    },
    /// From instruction context (timestamp, slot, signature)
    /// Used by #[track_from] with special fields like __timestamp
    FromContext {
        field: String,
    },
}

impl MappingSource {
    pub fn with_transform(self, transform: Transformation) -> Self {
        match self {
            MappingSource::FromSource {
                path,
                default,
                transform: _,
            } => MappingSource::FromSource {
                path,
                default,
                transform: Some(transform),
            },
            other => other,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComputeFunction {
    Sum,
    Concat,
    Format(String),
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub field: FieldPath,
    pub operator: ConditionOp,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConditionOp {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    Contains,
    Exists,
}

/// Language-agnostic type information for fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldTypeInfo {
    pub field_name: String,
    pub rust_type_name: String, // Full Rust type: "Option<i64>", "Vec<Value>", etc.
    pub base_type: BaseType,    // Fundamental type classification
    pub is_optional: bool,      // true for Option<T>
    pub is_array: bool,         // true for Vec<T>
    pub inner_type: Option<String>, // For Option<T> or Vec<T>, store the inner type
    pub source_path: Option<String>, // Path to source field if this is mapped
    /// Resolved type information for complex types (instructions, accounts, custom types)
    #[serde(default)]
    pub resolved_type: Option<ResolvedStructType>,
}

/// Resolved structure type with field information from IDL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedStructType {
    pub type_name: String,
    pub fields: Vec<ResolvedField>,
    pub is_instruction: bool,
    pub is_account: bool,
    pub is_event: bool,
    /// If true, this is an enum type and enum_variants should be used instead of fields
    #[serde(default)]
    pub is_enum: bool,
    /// For enum types, list of variant names
    #[serde(default)]
    pub enum_variants: Vec<String>,
}

/// A resolved field within a complex type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedField {
    pub field_name: String,
    pub field_type: String,
    pub base_type: BaseType,
    pub is_optional: bool,
    pub is_array: bool,
}

/// Language-agnostic base type classification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BaseType {
    // Numeric types
    Integer, // i8, i16, i32, i64, u8, u16, u32, u64, usize, isize
    Float,   // f32, f64
    // Text types
    String, // String, &str
    // Boolean
    Boolean, // bool
    // Complex types
    Object, // Custom structs, HashMap, etc.
    Array,  // Vec<T>, arrays
    Binary, // Bytes, binary data
    // Special types
    Timestamp, // Detected from field names ending in _at, _time, etc.
    Pubkey,    // Solana public key (Base58 encoded)
    Any,       // serde_json::Value, unknown types
}

/// Represents a logical section/group of fields in the entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySection {
    pub name: String,
    pub fields: Vec<FieldTypeInfo>,
    pub is_nested_struct: bool,
    pub parent_field: Option<String>, // If this section comes from a nested struct field
}

impl FieldTypeInfo {
    pub fn new(field_name: String, rust_type_name: String) -> Self {
        let (base_type, is_optional, is_array, inner_type) =
            Self::analyze_rust_type(&rust_type_name);

        FieldTypeInfo {
            field_name: field_name.clone(),
            rust_type_name,
            base_type: Self::infer_semantic_type(&field_name, base_type),
            is_optional,
            is_array,
            inner_type,
            source_path: None,
            resolved_type: None,
        }
    }

    pub fn with_source_path(mut self, source_path: String) -> Self {
        self.source_path = Some(source_path);
        self
    }

    /// Analyze a Rust type string and extract structural information
    fn analyze_rust_type(rust_type: &str) -> (BaseType, bool, bool, Option<String>) {
        let type_str = rust_type.trim();

        // Handle Option<T>
        if let Some(inner) = Self::extract_generic_inner(type_str, "Option") {
            let (inner_base_type, _, inner_is_array, inner_inner_type) =
                Self::analyze_rust_type(&inner);
            return (
                inner_base_type,
                true,
                inner_is_array,
                inner_inner_type.or(Some(inner)),
            );
        }

        // Handle Vec<T>
        if let Some(inner) = Self::extract_generic_inner(type_str, "Vec") {
            let (_inner_base_type, inner_is_optional, _, inner_inner_type) =
                Self::analyze_rust_type(&inner);
            return (
                BaseType::Array,
                inner_is_optional,
                true,
                inner_inner_type.or(Some(inner)),
            );
        }

        // Handle primitive types
        let base_type = match type_str {
            "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => {
                BaseType::Integer
            }
            "f32" | "f64" => BaseType::Float,
            "bool" => BaseType::Boolean,
            "String" | "&str" | "str" => BaseType::String,
            "Value" | "serde_json::Value" => BaseType::Any,
            "Pubkey" | "solana_pubkey::Pubkey" => BaseType::Pubkey,
            _ => {
                // Check for binary types
                if type_str.contains("Bytes") || type_str.contains("bytes") {
                    BaseType::Binary
                } else if type_str.contains("Pubkey") {
                    BaseType::Pubkey
                } else {
                    BaseType::Object
                }
            }
        };

        (base_type, false, false, None)
    }

    /// Extract inner type from generic like "Option<T>" -> "T"
    fn extract_generic_inner(type_str: &str, generic_name: &str) -> Option<String> {
        let pattern = format!("{}<", generic_name);
        if type_str.starts_with(&pattern) && type_str.ends_with('>') {
            let start = pattern.len();
            let end = type_str.len() - 1;
            if end > start {
                return Some(type_str[start..end].trim().to_string());
            }
        }
        None
    }

    /// Infer semantic type based on field name patterns
    fn infer_semantic_type(field_name: &str, base_type: BaseType) -> BaseType {
        let lower_name = field_name.to_lowercase();

        // If already classified as integer, check if it should be timestamp
        if base_type == BaseType::Integer
            && (lower_name.ends_with("_at")
                || lower_name.ends_with("_time")
                || lower_name.contains("timestamp")
                || lower_name.contains("created")
                || lower_name.contains("settled")
                || lower_name.contains("activated"))
        {
            return BaseType::Timestamp;
        }

        base_type
    }
}

pub trait FieldAccessor<S> {
    fn path(&self) -> String;
}

// ============================================================================
// SerializableStreamSpec Implementation
// ============================================================================

impl SerializableStreamSpec {
    /// Compute deterministic content hash (SHA256 of canonical JSON).
    ///
    /// The hash is computed over the entire spec except the content_hash field itself,
    /// ensuring the same AST always produces the same hash regardless of when it was
    /// generated or by whom.
    pub fn compute_content_hash(&self) -> String {
        use sha2::{Digest, Sha256};

        // Clone and clear the hash field for computation
        let mut spec_for_hash = self.clone();
        spec_for_hash.content_hash = None;

        // Serialize to JSON (serde_json produces consistent output for the same struct)
        let json =
            serde_json::to_string(&spec_for_hash).expect("Failed to serialize spec for hashing");

        // Compute SHA256 hash
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        let result = hasher.finalize();

        // Return hex-encoded hash
        hex::encode(result)
    }

    /// Verify that the content_hash matches the computed hash.
    /// Returns true if hash is valid or not set.
    pub fn verify_content_hash(&self) -> bool {
        match &self.content_hash {
            Some(hash) => {
                let computed = self.compute_content_hash();
                hash == &computed
            }
            None => true, // No hash to verify
        }
    }

    /// Set the content_hash field to the computed hash.
    pub fn with_content_hash(mut self) -> Self {
        self.content_hash = Some(self.compute_content_hash());
        self
    }
}

// ============================================================================
// Stack Spec — Unified multi-entity AST format
// ============================================================================

/// A unified stack specification containing all entities.
/// Written to `.hyperstack/{StackName}.stack.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableStackSpec {
    /// Stack name (PascalCase, derived from module ident)
    pub stack_name: String,
    /// Program IDs (one per IDL, in order)
    #[serde(default)]
    pub program_ids: Vec<String>,
    /// IDL snapshots (one per program)
    #[serde(default)]
    pub idls: Vec<IdlSnapshot>,
    /// All entity specifications in this stack
    pub entities: Vec<SerializableStreamSpec>,
    /// Deterministic content hash of the entire stack
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

impl SerializableStackSpec {
    /// Compute deterministic content hash (SHA256 of canonical JSON).
    pub fn compute_content_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut spec_for_hash = self.clone();
        spec_for_hash.content_hash = None;
        let json = serde_json::to_string(&spec_for_hash)
            .expect("Failed to serialize stack spec for hashing");
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        hex::encode(hasher.finalize())
    }

    pub fn with_content_hash(mut self) -> Self {
        self.content_hash = Some(self.compute_content_hash());
        self
    }
}

// ============================================================================
// View Pipeline Types - Composable View Definitions
// ============================================================================

/// Sort order for view transforms
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

/// Comparison operators for predicates
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    Ne,
    Gt,
    Gte,
    Lt,
    Lte,
}

/// Value in a predicate comparison
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PredicateValue {
    /// Literal JSON value
    Literal(serde_json::Value),
    /// Dynamic runtime value (e.g., "now()" for current timestamp)
    Dynamic(String),
    /// Reference to another field
    Field(FieldPath),
}

/// Predicate for filtering entities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Predicate {
    /// Field comparison: field op value
    Compare {
        field: FieldPath,
        op: CompareOp,
        value: PredicateValue,
    },
    /// Logical AND of predicates
    And(Vec<Predicate>),
    /// Logical OR of predicates
    Or(Vec<Predicate>),
    /// Negation
    Not(Box<Predicate>),
    /// Field exists (is not null)
    Exists { field: FieldPath },
}

/// Transform operation in a view pipeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ViewTransform {
    /// Filter entities matching a predicate
    Filter { predicate: Predicate },

    /// Sort entities by a field
    Sort {
        key: FieldPath,
        #[serde(default)]
        order: SortOrder,
    },

    /// Take first N entities (after sort)
    Take { count: usize },

    /// Skip first N entities
    Skip { count: usize },

    /// Take only the first entity (after sort) - produces Single output
    First,

    /// Take only the last entity (after sort) - produces Single output
    Last,

    /// Get entity with maximum value for field - produces Single output
    MaxBy { key: FieldPath },

    /// Get entity with minimum value for field - produces Single output
    MinBy { key: FieldPath },
}

/// Source for a view definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ViewSource {
    /// Derive directly from entity mutations
    Entity { name: String },
    /// Derive from another view's output
    View { id: String },
}

/// Output mode for a view
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum ViewOutput {
    /// Multiple entities (list-like semantics)
    #[default]
    Collection,
    /// Single entity (state-like semantics)
    Single,
    /// Keyed lookup by a specific field
    Keyed { key_field: FieldPath },
}

/// Definition of a view in the pipeline
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ViewDef {
    /// Unique view identifier (e.g., "OreRound/latest")
    pub id: String,

    /// Source this view derives from
    pub source: ViewSource,

    /// Pipeline of transforms to apply (in order)
    #[serde(default)]
    pub pipeline: Vec<ViewTransform>,

    /// Output mode for this view
    #[serde(default)]
    pub output: ViewOutput,
}

impl ViewDef {
    /// Create a new list view for an entity
    pub fn list(entity_name: &str) -> Self {
        ViewDef {
            id: format!("{}/list", entity_name),
            source: ViewSource::Entity {
                name: entity_name.to_string(),
            },
            pipeline: vec![],
            output: ViewOutput::Collection,
        }
    }

    /// Create a new state view for an entity
    pub fn state(entity_name: &str, key_field: &[&str]) -> Self {
        ViewDef {
            id: format!("{}/state", entity_name),
            source: ViewSource::Entity {
                name: entity_name.to_string(),
            },
            pipeline: vec![],
            output: ViewOutput::Keyed {
                key_field: FieldPath::new(key_field),
            },
        }
    }

    /// Check if this view produces a single entity
    pub fn is_single(&self) -> bool {
        matches!(self.output, ViewOutput::Single)
    }

    /// Check if any transform in the pipeline produces a single result
    pub fn has_single_transform(&self) -> bool {
        self.pipeline.iter().any(|t| {
            matches!(
                t,
                ViewTransform::First
                    | ViewTransform::Last
                    | ViewTransform::MaxBy { .. }
                    | ViewTransform::MinBy { .. }
            )
        })
    }
}

#[macro_export]
macro_rules! define_accessor {
    ($name:ident, $state:ty, $path:expr) => {
        pub struct $name;

        impl $crate::ast::FieldAccessor<$state> for $name {
            fn path(&self) -> String {
                $path.to_string()
            }
        }
    };
}
