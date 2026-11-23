//! Outbound ports through which the Pale application reaches files, processes,
//! time, randomness, storage, anti-rollback anchors, local IPC and A2A.
//!
//! This crate builds on `pale-core`. `pale-application` calls these ports, and
//! the adapter crates implement them, keeping the mechanics behind each port.
//! Each port describes an observation or exchange that application policy
//! needs. Ports neither expose operating-system handles nor grant authority.
//!
//! Bounded one-shot composition mechanics that have no stable resident contract
//! remain private to the composition shell rather than becoming speculative
//! public ports.

mod a2a_exchange;
mod accumulator;
mod ambient_session_authority;
mod anti_rollback;
mod clock_and_randomness;
mod cold_start;
mod control_store;
mod deployment_initialization;
mod durability;
mod effect_case;
mod event;
mod local_ipc;
mod machine_filesystem;
mod materialization_effect;
mod operation;
mod principal_bootstrap;
mod principal_control_checkpoint;
mod principal_producer;
mod principal_resident_writer;
mod principal_trust;
mod process;
mod provider_invocation;
mod resident_hook;
mod session_lease_custody;
mod source;
mod source_guidance;
mod source_intelligence;

pub use source_guidance::{
    AdmittedSourceGuidanceEffect, AdmittedSourceGuidanceRecovery, SourceGuidanceEffectFile,
    SourceGuidanceFileChange, SourceGuidanceRecoveryProposal, MAX_SOURCE_GUIDANCE_OWNED_FILES,
    MAX_SOURCE_GUIDANCE_TRANSACTION_FILES,
};

pub use a2a_exchange::{
    A2aCancelTaskEvidencePort, A2aCancelTaskRequestEvidencePort, A2aExchange, A2aExchangeCommand,
    A2aExchangeContext, A2aExchangeCoordinate, A2aExchangeError, A2aExchangeEvidence,
    A2aExchangeFacts, A2aExchangeInput, A2aExchangeLimits, A2aExchangeParents, A2aExchangePort,
    A2aExchangeRequest, A2aGetTaskEvidencePort, A2aPreparedExchange, A2aSendMessageEvidencePort,
};
pub use accumulator::membership::{
    AccumulatorMembershipInputError, AccumulatorMembershipLeaf, AccumulatorMembershipLimits,
    OperationAccumulatorMembership, VerifiedOperationAccumulatorMembership,
};
pub use accumulator::preparation::{
    AccumulatorImplementation, AccumulatorInitialDomain, AccumulatorLeafChange,
    AccumulatorPreparationBasis, AccumulatorPreparationInputError, AccumulatorPreparationLimits,
    OperationAccumulatorCheckpoint, OperationAccumulatorPreparation,
    OperationAccumulatorPreparationError, PreparedOperationAccumulator, StateLeafChanges,
    UnresolvedCustodyLeafChanges,
};
pub use accumulator::{
    AccumulatorBatchError, AccumulatorCheckpointBinding, AccumulatorCheckpointPhase,
    AccumulatorMutation, AccumulatorPosition, OperationAccumulatorBatch, StateAccumulatorBatch,
    UnresolvedCustodyAccumulatorBatch,
};
pub use ambient_session_authority::{AmbientSessionAuthorityError, AmbientSessionAuthorityPort};
pub use anti_rollback::{
    AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError,
    AntiRollbackAnchorErrorKind, AntiRollbackAnchorKey, AntiRollbackAnchorObservation,
    AntiRollbackAnchorPort, AntiRollbackAnchorTransitionError, ControlDomainAntiRollbackAnchor,
    PreparedAntiRollbackAnchorTransition, PrincipalTrustMinimumAnchor,
};
pub use clock_and_randomness::{
    CryptographicRandomSourceError, CryptographicRandomSourcePort, WallClockSourceError,
    WallClockSourcePort,
};
pub use cold_start::{
    DeploymentControlStoreGenesisReadLimits, DeploymentControlStoreGenesisReadPort,
    DeploymentControlStoreGenesisSnapshot, DeploymentControlStoreGenesisSnapshotError,
    DeploymentFinalizationSetReadLimits, DeploymentFinalizationSetReadPort,
    DeploymentFinalizationSetSnapshot, DeploymentFinalizationSetSnapshotError,
    DeploymentFinalizationSnapshot, DeploymentFinalizationSnapshotError,
    PrincipalResidentWriterReadPort, PrincipalResidentWriterRecordSetReadLimits,
    PrincipalResidentWriterRecordSetReadPort, PrincipalResidentWriterRecordSetSnapshot,
    PrincipalResidentWriterRecordSetSnapshotError, PrincipalResidentWriterSnapshot,
    PrincipalResidentWriterSnapshotError, PrincipalResidentWriterStoredRecord,
};
pub use control_store::{
    ControlCheckpointHead, ControlRecord, ControlRecordBytes, ControlRecordCoordinate,
    ControlRecordDigest, ControlRecordError, ControlStoreError, ControlStoreErrorKind,
    ControlTransactionPort, ControlTransactionPreparation,
};
pub use deployment_initialization::{
    AnchoredDeploymentFinalizationCapability, DatabaseCommittedDeploymentControlStoreGenesis,
    DatabaseCommittedDeploymentFinalization, DeploymentControlStoreBinding,
    DeploymentControlStoreGenesisCandidateError, DeploymentControlStoreGenesisCommitDisposition,
    DeploymentControlStoreGenesisCommitEvidence, DeploymentControlStoreGenesisCommitFailure,
    DeploymentControlStoreGenesisCommitFailureReason, DeploymentControlStoreGenesisCustodyPort,
    DeploymentControlStoreGenesisDatabaseObservation, DeploymentControlStoreGenesisRecords,
    DeploymentFinalizationAnchorEvidence, DeploymentFinalizationAnchorFailure,
    DeploymentFinalizationAnchorFailureReason, DeploymentFinalizationAnchorOwner,
    DeploymentFinalizationAnchorsPending, DeploymentFinalizationCandidateError,
    DeploymentFinalizationCommitDisposition, DeploymentFinalizationCommitEvidence,
    DeploymentFinalizationCommitFailure, DeploymentFinalizationCommitFailureReason,
    DeploymentFinalizationCustodyPort, DeploymentFinalizationDatabaseObservation,
    DeploymentFinalizationRecords, DeploymentFinalizationReplayIdentity,
    PreparedDeploymentControlStoreGenesisAtom, PreparedDeploymentFinalizationAtom,
};
pub use durability::{
    AuthorityByteClass, AuthorityRecoveryReadLimits, AuthorityStoreCoordinate,
    BoundedAuthorityEntryBytes, BoundedRecoveryCheckpointBytes, DurableAuthorityAppendFailure,
    DurableAuthorityCheckpointFailure, DurableAuthorityStoreError, DurableAuthorityStoreErrorKind,
    DurableAuthorityStorePort, JournalTailPage, JournalTailPageBoundary, JournalTailPageRead,
    PreparedAuthorityAppend, PreparedRecoveryCheckpoint, RecoveryCheckpointCandidate,
    RecoveryCheckpointPhysicalRejection, RecoveryCheckpointRead, SequencedAuthorityEntry,
    StoredRecoveryCheckpoint,
};
pub use effect_case::{
    EffectCaseActionAdmissionKind, EffectCaseCandidateError, EffectCaseCheckpointMutation,
    EffectCaseCheckpointTransition, EffectCaseCommitOutcome, EffectCaseCustodyPort,
    EffectCaseIndexAssertion, EffectCaseIndexKey, EffectCaseIndexMutation,
    EffectCaseIndexObservation, EffectCaseIndexObservationError, EffectCaseIndexPredecessor,
    EffectCaseIndexQuery, EffectCaseReadBackOutcome, EffectCaseReconciliationAdvanceKind,
    EffectCaseRecordObservation, EffectCaseTransitionCandidate, EffectCaseTransitionKind,
    EffectCaseTransitionMember, EffectCaseTransitionMemberKind,
};
pub use event::{
    EventCustodyCandidateError, EventCustodyCommitOutcome, EventCustodyHistoryMember,
    EventCustodyHistoryObservation, EventCustodyHistoryRead, EventCustodyPort,
    EventCustodyReadBackOutcome, EventCustodyTransition, EventCustodyTransitionKind,
    EventInboxCursor, EventInboxPage, EventInboxPageBoundary, EventInboxPageEntry,
    EventInboxPageRead, EventIndexAssertion, EventIndexKey, EventIndexMutation,
    EventIndexObservation, EventIndexPredecessor, EventIndexQuery, EventObservationError,
    EventPageError, EventPublicationRole, EventRecordObservation, EventTransitionMember,
    EventTransitionMemberKind, PaleDerivedOutboxCursor, PaleDerivedOutboxPage,
    PaleDerivedOutboxPageBoundary, PaleDerivedOutboxPageEntry, PaleDerivedOutboxPageError,
    PaleDerivedOutboxPageRead,
};
pub use local_ipc::{
    LocalConnectionSignerPort, LocalConnectionSigningError, LocalIpcCarrierError,
    LocalIpcCarrierOperation, LocalIpcFrameLimits, LocalIpcFramePort, LocalIpcFrameSection,
    LocalIpcPeerCredentials, ResidentObserveFrameFailure, ResidentObserveFrameRequest,
    ResidentObserveFrameResponse, ResidentOperationFrameCancellationDisposition,
    ResidentOperationFrameFailure, ResidentOperationFrameLocator, ResidentOperationFrameRecords,
    ResidentOperationFrameRequest, ResidentOperationFrameResponse,
    ResidentOperationFrameUnsupportedReason,
};
pub use machine_filesystem::{
    MachineFilesystemError, MachineFilesystemOperation, MachineFilesystemPort, MachinePathKind,
    MachinePathObservation,
};
pub use materialization_effect::{
    AdmittedMaterializationEffect, MachineLocalEffectCandidate, MachineLocalFileObservation,
    MaterializationPayload,
};
pub use operation::{
    OperationA2aBindingCursor, OperationA2aBindingScope, OperationA2aBindingSource,
    OperationA2aBindings, OperationA2aDependencies, OperationA2aEvidence, OperationCheckpointPhase,
    OperationCollectionFrontierEvidence, OperationCollectionHeadObservation,
    OperationCollectionJournal, OperationCollectionJournalError, OperationCollectionSnapshot,
    OperationCustodyCandidate, OperationCustodyCandidateError, OperationCustodyCheckpoint,
    OperationCustodyCommitOutcome, OperationCustodyIndexKey, OperationCustodyIndexMutation,
    OperationCustodyIndexObservation, OperationCustodyIndexPredecessor, OperationCustodyIndexQuery,
    OperationCustodyObservationError, OperationCustodyPort, OperationCustodyReadBackOutcome,
    OperationCustodyRecord, OperationCustodyRecordKind, OperationCustodyTransitionKind,
    OperationPublicationCursor, OperationPublicationReadError, OperationPublicationSource,
    OperationPublicationSourceKind, OperationPublicationSources, OperationSessionBasis,
    OperationStartCursor, OperationStartReadError, OperationStartSource, OperationStartSources,
    MAX_OPERATION_A2A_BINDINGS, MAX_OPERATION_A2A_BINDING_BYTES,
    MAX_OPERATION_PUBLICATION_BODY_BYTES, MAX_OPERATION_PUBLICATION_SOURCES,
    MAX_OPERATION_SESSION_BASIS_BYTES, MAX_OPERATION_START_SOURCES,
    MAX_OPERATION_START_SOURCE_BYTES, PRIVATE_OPERATION_A2A_GET_TASK_LOCAL_DISPOSITION_V1_SCHEMA,
    PRIVATE_OPERATION_A2A_GET_TASK_PREPARATION_V1_SCHEMA,
    PRIVATE_OPERATION_A2A_SEND_PREPARATION_V1_SCHEMA,
    PRIVATE_OPERATION_ATTEMPT_SELECTION_V1_SCHEMA, PRIVATE_OPERATION_COLLECTION_FRONTIER_V1_SCHEMA,
    PRIVATE_OPERATION_HOOK_RESPONSE_V1_SCHEMA, PRIVATE_OPERATION_NATIVE_OUTCOME_V1_SCHEMA,
};
pub use principal_bootstrap::{
    PreparedPrincipalBootstrapAtom, PrincipalBootstrapCandidateError,
    PrincipalBootstrapCommitDisposition, PrincipalBootstrapControlCheckpoint,
    PrincipalBootstrapCustodyPort, PrincipalBootstrapDatabaseObservation,
    PrincipalBootstrapFixedRecords, PrincipalBootstrapHead,
};
pub use principal_control_checkpoint::{
    PrincipalControlCheckpointChainReadLimits, PrincipalControlCheckpointChainReadPort,
    PrincipalControlCheckpointChainSnapshot, PrincipalControlCheckpointChainSnapshotError,
    PrincipalControlCheckpointStoredRecord,
};
pub use principal_producer::{
    PreparedPrincipalProducerIncarnationTransition, PrincipalProducerIncarnationCandidateError,
    PrincipalProducerIncarnationCommitDisposition, PrincipalProducerIncarnationCustodyPort,
    PrincipalProducerIncarnationDatabaseObservation, PrincipalProducerIncarnationExpectedFrontier,
    PrincipalProducerIncarnationHead, PrincipalProducerIncarnationHeadCursor,
    PrincipalProducerIncarnationHeadPage, PrincipalProducerIncarnationHeadPageBoundary,
    PrincipalProducerIncarnationHeadPageEntry, PrincipalProducerIncarnationHeadPageError,
    PrincipalProducerIncarnationHeadPageRead, PrincipalProducerIncarnationReadError,
    PrincipalProducerIncarnationReadObservation, PrincipalProducerIncarnationReadPort,
    PrincipalProducerIncarnationRecordSet, PrincipalProducerIncarnationSuccessorFrontier,
    PrincipalProducerIncarnationTransitionKind,
};
#[doc(no_inline)]
pub use principal_resident_writer::{
    PreparedPrincipalResidentWriterTransition, PrincipalResidentWriterCandidateError,
    PrincipalResidentWriterCommitDisposition, PrincipalResidentWriterCustodyPort,
    PrincipalResidentWriterDatabaseObservation, PrincipalResidentWriterHead,
    PrincipalResidentWriterTransitionKind, PrincipalWriterGuardPort,
};
pub use principal_trust::{
    PrincipalCheckpointPreparationPort, PrincipalTrustCheckpointHead,
    PrincipalTrustCheckpointRecord, PrincipalTrustCheckpointSetReadLimits,
    PrincipalTrustCheckpointSetReadPort, PrincipalTrustCheckpointSetSnapshot,
    PrincipalTrustCheckpointSetSnapshotError, PrincipalTrustReadPort, PrincipalTrustSnapshot,
    PrincipalTrustSnapshotError, PrincipalTrustStoreBinding,
};
pub use process::{
    DirectChildProcessIdentity, ExecutableIdentityError, ExecutableIdentityPort,
    ExecutableIdentityRequest, ExecutableObservation, HostProcessGroupId, HostProcessId,
    HostProcessStartIdentity, HostProcessStartObservation, IdentityProbeRequest, ProbeLimits,
    ProbeObservation, ProbeStream, ProcessEnvironmentEntry, ProcessProbeError, ProcessProbePort,
    ProcessTerminationIssue, ProcessTerminationMode, ProcessTerminationPolicy,
    ProcessTerminationStage, ProcessTerminationUncertainty, SupervisedProcess,
    SupervisedProcessExit, SupervisedProcessObservation, SupervisedProcessObservationError,
    SupervisedProcessPort, SupervisedProcessRequest, SupervisedProcessStartError,
};
pub use provider_invocation::{
    ProviderInvocationCoordinate, ProviderInvocationCoordinateError,
    ProviderInvocationEvidenceFrontier, ProviderInvocationEvidenceSubject,
    ProviderInvocationFailureWire, ProviderInvocationImplementationCoordinate,
    ProviderInvocationObservation, ProviderInvocationObservationCoordinate, ProviderInvocationPort,
    ProviderInvocationRequest, ProviderInvocationSuccessWire, ProviderInvocationUnavailability,
};
pub use resident_hook::{
    ResidentHookFrameRequest, ResidentHookFrameResponse, ResidentHookResponseRecord,
};
pub use session_lease_custody::{
    SessionLeaseBody, SessionLeaseBodyRead, SessionLeaseBodyRetention, SessionLeaseCustodyPort,
};
pub use source::{
    SourceFilesystemError, SourceFilesystemPort, SourceReadLimit, SourceTreeLimits,
    SourceTreeSnapshot,
};
pub use source_intelligence::{SourceIndexLifecyclePort, SourceIntelligenceQueryPort};
