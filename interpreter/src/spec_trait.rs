use crate::ast::TypedStreamSpec;

/// Trait for providing stream specifications from different sources
/// 
/// This trait enables dynamic loading of specs without the CLI needing
/// to directly depend on all atom crates.
pub trait SpecProvider {
    /// Get the name of this spec (e.g., "settlement-game")
    fn spec_name(&self) -> &str;
    
    /// Get the entity name (e.g., "SettlementGame")
    fn entity_name(&self) -> &str;
    
    /// Get the stream spec with type information
    fn get_spec(&self) -> Box<dyn std::any::Any>;
    
    /// Get description of this spec
    fn description(&self) -> Option<&str> {
        None
    }
}

/// Type-erased stream spec that can be used across crate boundaries
/// 
/// This allows the CLI to work with specs without knowing the concrete state type
pub struct ErasedStreamSpec {
    pub spec_name: String,
    pub entity_name: String,
    pub description: Option<String>,
    // Store the actual spec as Any for type erasure
    spec_any: Box<dyn std::any::Any>,
}

impl ErasedStreamSpec {
    pub fn new<S: 'static>(
        spec_name: String,
        entity_name: String,
        spec: TypedStreamSpec<S>,
        description: Option<String>,
    ) -> Self {
        ErasedStreamSpec {
            spec_name,
            entity_name,
            description,
            spec_any: Box::new(spec),
        }
    }
    
    /// Try to downcast to a specific spec type
    pub fn downcast<S: 'static>(&self) -> Option<&TypedStreamSpec<S>> {
        self.spec_any.downcast_ref::<TypedStreamSpec<S>>()
    }
}

