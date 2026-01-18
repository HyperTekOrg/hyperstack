//! MutationBatch - Envelope type for propagating trace context across async boundaries.

use hyperstack_interpreter::Mutation;
use smallvec::SmallVec;
use tracing::Span;

/// Envelope type that carries mutations along with their originating span context.
///
/// This enables trace context propagation across the mpsc channel boundary
/// from the Vixen parser to the Projector.
#[derive(Debug)]
pub struct MutationBatch {
    /// The span from which these mutations originated
    pub span: Span,
    /// The mutations to process
    pub mutations: SmallVec<[Mutation; 6]>,
}

impl MutationBatch {
    pub fn new(mutations: SmallVec<[Mutation; 6]>) -> Self {
        Self {
            span: Span::current(),
            mutations,
        }
    }

    pub fn with_span(span: Span, mutations: SmallVec<[Mutation; 6]>) -> Self {
        Self { span, mutations }
    }

    pub fn len(&self) -> usize {
        self.mutations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.mutations.is_empty()
    }
}
