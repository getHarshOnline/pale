//! Separate administrative and query boundaries for source-intelligence providers.
//!
//! Lifecycle implementations build, publish, invalidate, retire, or remove a
//! deletable derived index. Query implementations answer one already-admitted
//! structural question from an immutable ready generation. Keeping those ports
//! distinct prevents a model-facing query from acquiring index administration.

/// Administrative lifecycle of one replaceable source-index implementation.
///
/// Associated types let the application retain its phase-typed candidate and
/// receipt records without making this neutral port own public wire schemas.
/// In particular, [`Self::Building`] is not a valid query input.
pub trait SourceIndexLifecyclePort {
    /// Exact candidate admitted before provider work begins.
    type Candidate;
    /// In-progress generation, deliberately unavailable to the query port.
    type Building;
    /// Immutable ready-generation coordinate returned after atomic publication.
    type Ready;
    /// Typed non-query lifecycle transition.
    type Transition;
    /// Lifecycle failure.
    type Error;

    /// Admit a candidate into the unqueryable building phase.
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] when the candidate is malformed, collides with
    /// existing lifecycle state, or cannot enter the building phase.
    fn begin(&self, candidate: Self::Candidate) -> Result<Self::Building, Self::Error>;

    /// Consume one building generation and publish it atomically as immutable.
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] when graph construction or atomic publication
    /// fails. No partially published generation may become queryable.
    fn publish(&self, building: Self::Building) -> Result<Self::Ready, Self::Error>;

    /// Apply an admitted activation, rollback, invalidation, retirement, or removal.
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] when the transition does not follow the exact
    /// current lifecycle frontier.
    fn transition(&self, transition: Self::Transition) -> Result<(), Self::Error>;
}

/// Operation-bound queries over immutable ready source-index generations.
///
/// This port deliberately has no initialization, synchronization, health,
/// activation, deletion, or provider-configuration method. A query result is
/// structural evidence only; it cannot establish source truth, correctness,
/// acceptance, authority, release, or native effect.
pub trait SourceIntelligenceQueryPort {
    /// Request already admitted by application policy.
    type Request;
    /// Bounded evidence returned from the selected ready generation.
    type Result;
    /// Query failure.
    type Error;

    /// Answer one exact admitted structural question.
    ///
    /// # Errors
    ///
    /// Returns [`Self::Error`] when no exact ready generation exists, a scope
    /// or frontier fence differs, or the provider cannot satisfy the declared
    /// bound.
    fn query(&self, request: &Self::Request) -> Result<Self::Result, Self::Error>;
}
