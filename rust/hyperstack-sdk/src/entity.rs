/// Stack definition trait - defines the shape of a HyperStack deployment.
///
/// ```ignore
/// use hyperstack_sdk::{Stack, Views};
///
/// pub struct OreStack;
///
/// impl Stack for OreStack {
///     type Views = OreRoundViews;
///
///     fn name() -> &'static str { "ore-round" }
///     fn url() -> &'static str { "wss://ore.stack.usehyperstack.com" }
/// }
///
/// // Usage
/// let hs = HyperStack::<OreStack>::connect().await?;
/// let rounds = hs.views.latest().get().await;
/// ```
pub trait Stack: Sized + Send + Sync + 'static {
    type Views: crate::view::Views;

    fn name() -> &'static str;
    fn url() -> &'static str;
}
