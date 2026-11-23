//! Development-only resident conformance support shared by Pale interfaces.
//!
//! This crate builds on `pale-application`, `pale-cli`, `pale-contracts`,
//! `pale-core` and `pale-ports`, and the tests of `pale-mcp` use it. It
//! supplies bounded in-memory physical ports plus SDK-side carrier and
//! response-admission models; resident semantics stay with the application.
//! Tests carry frames to an application-owned service that this crate does not
//! assemble.
//!
//! The deterministic clock, entropy, signatures, and anti-rollback store are
//! test models. They do not qualify production cryptography, time, randomness,
//! persistence, endpoint discovery, authentication, or independent storage.

mod physical;
mod resident_client;
mod transport;

pub use physical::{
    deterministic_signature, DeterministicClock, DeterministicEntropy,
    EmptyPrincipalProducerReader, InMemoryCheckpointSetReader,
    InMemoryDeploymentFinalizationReader, InMemoryResidentWriterReader, SimulatedIndependentAnchor,
    TestWriterGuard,
};
pub use resident_client::{
    AdmittedResidentResponse, DeterministicResponseAdmission, InMemoryResidentCarrier,
};
pub use transport::{
    serve_exchange, serve_exchange_with_failed_response, serve_exchange_with_response,
    BoundedFrameClient, BoundedFrameClientError, PendingResidentExchange,
};
