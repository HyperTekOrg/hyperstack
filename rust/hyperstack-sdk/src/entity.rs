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

/// Trait that maps a Data type back to its Entity type.
///
/// This enables type inference from return type instead of requiring turbofish syntax.
///
/// # Example
///
/// ```ignore
/// // With EntityData implemented:
/// let token: PumpfunToken = hs.get_data("mint").await?;
///
/// // Without EntityData (original API still works):
/// let token = hs.get::<PumpfunTokenEntity>("mint").await;
/// ```
///
/// The generated SDK code automatically implements this trait for each entity's data type.
pub trait EntityData: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {
    /// The Entity type that produces this Data type.
    type Entity: Entity<Data = Self>;
}
