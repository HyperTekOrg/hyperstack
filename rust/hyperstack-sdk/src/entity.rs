//! Entity trait for typed HyperStack entities.
//!
//! The `Entity` trait is implemented by generated code for each entity type,
//! providing type-safe access to HyperStack views.

use serde::{de::DeserializeOwned, Serialize};

/// Marker trait for HyperStack entities.
///
/// This trait is implemented by generated code (via `hyperstack sdk create rust`)
/// for each entity type defined in a HyperStack spec.
///
/// # Example (Generated Code)
///
/// ```ignore
/// pub struct PumpfunTokenEntity;
///
/// impl Entity for PumpfunTokenEntity {
///     type Data = PumpfunToken;
///     
///     const NAME: &'static str = "PumpfunToken";
///     
///     fn state_view() -> &'static str { "PumpfunToken/state" }
///     fn list_view() -> &'static str { "PumpfunToken/list" }
///     fn kv_view() -> &'static str { "PumpfunToken/kv" }
/// }
/// ```
///
/// # Usage
///
/// ```ignore
/// use hyperstack_sdk::HyperStack;
/// use my_stack::PumpfunTokenEntity;
///
/// let hs = HyperStack::connect("wss://example.com").await?;
/// let token = hs.get::<PumpfunTokenEntity>("mint_address").await;
/// ```
pub trait Entity: Sized + Send + Sync + 'static {
    /// The data type this entity deserializes to.
    ///
    /// This is the struct containing all entity fields (id, info, trading, etc.).
    type Data: Serialize + DeserializeOwned + Clone + Send + Sync + 'static;

    /// Entity name (e.g., "PumpfunToken", "SettlementGame").
    ///
    /// This matches the entity name defined in the HyperStack spec.
    const NAME: &'static str;

    /// View path for single-entity state subscriptions.
    ///
    /// Returns a path like "EntityName/state" for subscribing to
    /// a single entity's complete state by key.
    fn state_view() -> &'static str;

    /// View path for list subscriptions.
    ///
    /// Returns a path like "EntityName/list" for subscribing to
    /// all entities of this type.
    fn list_view() -> &'static str;

    /// View path for key-value lookups.
    ///
    /// Returns a path like "EntityName/kv" for subscribing to
    /// specific entities by key with efficient updates.
    fn kv_view() -> &'static str;
}

/// Optional trait for entities that support server-side filtering.
///
/// Implement this trait to enable filtered list queries.
pub trait Filterable: Entity {
    /// Filter configuration type for this entity.
    type Filter: Default + Clone + Send + Sync;

    /// Convert filter to query parameters.
    fn filter_to_params(filter: &Self::Filter) -> std::collections::HashMap<String, String>;
}
