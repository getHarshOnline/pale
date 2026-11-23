//! Linear process-owned composition of an already-selected Consult start.
//!
//! This module joins existing durable owners. It does not select a roster,
//! contributor, stopping rule, provider, synthesis step, or transport.

use pale_core::records::{
    ArtifactDigest, CanonicalSignatureV1, CommonRequestEnvelopeV1,
    ControlCheckpointSigningAuthorityAdmissionContextV1, ControlStateCheckpointDigest,
    ControlStateCheckpointPreimageV1, ControlStateCheckpointV1, InvalidResidentRecordV1,
    OperationHandleDigest, OperationHandlePreimageV1, OperationLeaseV1, OperationObservationDigest,
    OperationObservationPreimageV1, OperationObservationV1, ResidentRecordRefV1, UnixTimeV1,
};

use super::*;
use crate::resident::{
    consultation::{
        ConsultationContributionSource, ConsultationFreshChildPlan, ConsultationStartIntent,
    },
    recover_local_operation_from_handle, ConsultationCollectionObservationInput,
    LocalOperationHandleRecoveryInput, LocalOperationRecoveryAdmission,
    LocalOperationRecoveryContext, OperationControlRecoveryAdmission,
};

/// Already-admitted collection meaning for the durable parent owned by a start phase.
pub(super) struct ResidentProcessConsultationCollectionInput<'a> {
    pub(super) collection: ConsultationCollection,
    pub(super) parent_observation: (&'a ResidentRecordRefV1, &'a OperationObservationV1),
    pub(super) parent_checkpoint: (&'a ResidentRecordRefV1, &'a ControlStateCheckpointV1),
    pub(super) observation: ConsultationCollectionObservationInput<'a>,
    pub(super) limits: ConsultationCollectionLimits,
    pub(super) checkpoint: ConsultationSuccessorCommitInput<'a>,
}

/// A durable Consult parent and the retained prefix of its planned children.
///
/// The vector is preallocated before parent custody begins. A child is retained
/// here only after its database and independent-anchor custody completed. This
/// phase does not select roster meaning or prove child execution. The parent's
/// retained start intent remains the sole ordered source for child recovery.
#[must_use = "open and recover the exact initial collection or retain this phase"]
pub(super) struct ResidentProcessConsultationStarted {
    selector: ResidentDeploymentAuthoritySelector,
    store: AuthorityStoreCoordinate,
    parent: super::super::IssuedLocalOperation,
    children: Vec<super::super::IssuedLocalOperation>,
    maximum_children: usize,
}

/// The next durable boundary while a Consult prepares its initial collection.
///
/// Each value permits exactly one kind of work. Starting the final child yields
/// `OpenCollection`; it never opens the collection in the same call. Recovery
/// can therefore resume both between the parent and first child and between the
/// final child and collection custody.
#[must_use = "advance or retain the exact Consult initialization boundary"]
pub(super) enum ResidentProcessConsultationInitialization {
    /// One exact fresh child is next in retained roster order.
    StartChild(ResidentProcessConsultationChildPending),
    /// Every fresh child is durable; the initial collection is still unopened.
    OpenCollection(ResidentProcessConsultationCollectionPending),
}

/// A Consult start phase that permits one exact next child start.
#[must_use = "start the next retained child or preserve this boundary"]
pub(super) struct ResidentProcessConsultationChildPending {
    started: ResidentProcessConsultationStarted,
}

/// A Consult start phase whose exact initial collection is not yet durable.
#[must_use = "open the exact initial collection or preserve this boundary"]
pub(super) struct ResidentProcessConsultationCollectionPending {
    started: ResidentProcessConsultationStarted,
}

/// One child start failed while preserving the exact retry boundary.
#[must_use = "preserve the pending child phase before retry or recovery"]
pub(super) struct ResidentProcessConsultationInitializationChildFailure {
    pub(super) pending: Box<ResidentProcessConsultationChildPending>,
    pub(super) reason: ResidentProcessConsultationChildStartError,
}

impl ResidentProcessConsultationStarted {
    /// Retain a newly issued parent after its child capacity was reserved.
    pub(super) fn from_started_parent(
        selector: ResidentDeploymentAuthoritySelector,
        store: AuthorityStoreCoordinate,
        parent: super::super::IssuedLocalOperation,
        children: Vec<super::super::IssuedLocalOperation>,
        maximum_children: usize,
    ) -> Self {
        debug_assert!(children.capacity() >= maximum_children);
        Self {
            selector,
            store,
            parent,
            children,
            maximum_children,
        }
    }

    /// Borrow the deployment that admitted this exact start phase.
    pub(super) const fn selector(&self) -> &ResidentDeploymentAuthoritySelector {
        &self.selector
    }

    /// Borrow the authority store that owns this exact start phase.
    pub(super) const fn store(&self) -> &AuthorityStoreCoordinate {
        &self.store
    }

    /// Borrow the durable parent without granting continuation authority.
    pub(super) const fn parent(&self) -> &super::super::IssuedLocalOperation {
        &self.parent
    }

    /// Borrow the durable children in the selected order.
    pub(super) fn children(&self) -> &[super::super::IssuedLocalOperation] {
        &self.children
    }

    /// Retain one child already recovered from complete durable custody.
    pub(super) fn retain_recovered_child(
        &mut self,
        child: super::super::IssuedLocalOperation,
    ) -> Result<(), ResidentProcessConsultationChildStartError> {
        self.ensure_child_capacity()?;
        let Some(expected) = self
            .parent
            .consultation_start_intent()
            .and_then(|intent| intent.fresh_children().nth(self.children.len()))
        else {
            return Err(ResidentProcessConsultationChildStartError::BoundExceeded);
        };
        let expected_parent = child.handle().1.parent_operation().as_ref().ok_or(
            ResidentProcessConsultationChildStartError::Start(
                ResidentProcessConsultStartError::Selection(
                    ResidentProcessHostSelectionError::ConsultationChildAssociation,
                ),
            ),
        )?;
        let (parent_ref, parent_handle) = self.parent.handle();
        if !expected.matches_operation(&child)
            || !self.selector.matches_operation(&child)
            || !child.checkpoint_selection().matches_store(&self.store)
            || !matches!(
                child.handle().1.operation_class(),
                OperationClassV1::Invoke | OperationClassV1::Consult
            )
            || expected_parent.operation_handle_ref != *parent_ref
            || expected_parent.operation_handle_id != *parent_handle.operation_handle_id()
            || expected_parent.operation_handle_sha256.as_str()
                != parent_handle.canonical_operation_handle_sha256().as_str()
            || self.children.iter().any(|retained| {
                retained.handle().0 == child.handle().0
                    && retained
                        .handle()
                        .1
                        .canonical_operation_handle_sha256()
                        .as_str()
                        == child
                            .handle()
                            .1
                            .canonical_operation_handle_sha256()
                            .as_str()
            })
        {
            return Err(ResidentProcessConsultationChildStartError::Start(
                ResidentProcessConsultStartError::Selection(
                    ResidentProcessHostSelectionError::ConsultationChildAssociation,
                ),
            ));
        }
        self.children.push(child);
        Ok(())
    }

    /// Return the durable operations without detaching their retained intent.
    pub(super) fn into_operations(
        self,
    ) -> (
        super::super::IssuedLocalOperation,
        Vec<super::super::IssuedLocalOperation>,
    ) {
        (self.parent, self.children)
    }

    fn ensure_child_capacity(&self) -> Result<(), ResidentProcessConsultationChildStartError> {
        if self.children.len() == self.maximum_children {
            Err(ResidentProcessConsultationChildStartError::BoundExceeded)
        } else {
            Ok(())
        }
    }

    /// Return remaining capacity before another child start can use any port.
    pub(super) fn remaining_child_capacity(&self) -> usize {
        self.maximum_children - self.children.len()
    }

    /// Classify the next bounded initialization step without using a port.
    ///
    /// The retained intent is the ordered source for this decision. A recovered
    /// child prefix reaches the same boundary as a live prefix of equal length.
    pub(super) fn into_initialization(self) -> ResidentProcessConsultationInitialization {
        if self.remaining_child_capacity() == 0 {
            ResidentProcessConsultationInitialization::OpenCollection(
                ResidentProcessConsultationCollectionPending { started: self },
            )
        } else {
            ResidentProcessConsultationInitialization::StartChild(
                ResidentProcessConsultationChildPending { started: self },
            )
        }
    }

    /// Start one already-selected narrower Invoke or nested Consult child.
    ///
    /// Bound exhaustion is rejected before authority, custody, signing, or
    /// anchor work. A failed start leaves this phase and its completed children
    /// available to the caller; any linear lower-phase continuation stays in the
    /// returned error.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn start_child<
        'store,
        WriterGuard,
        Authority,
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: ResidentLocalOperationStartInput<'_>,
        parent: PreparedLocalChildParent,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<(), ResidentProcessConsultationChildStartError>
    where
        Authority: super::super::AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::super::AdmittedAmbientSession,
            &super::super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage: FnOnce(
            &super::super::PreparedLocalOperationCheckpoint,
        ) -> Result<
            ControlStateCheckpointPreimageV1,
            super::super::LocalOperationStartError,
        >,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.ensure_child_capacity()?;
        if access.store != &self.store {
            return Err(ResidentProcessConsultationChildStartError::Start(
                ResidentProcessConsultStartError::Selection(
                    ResidentProcessHostSelectionError::AccessStoreAssociation,
                ),
            ));
        }
        let Some(expected) = self
            .parent
            .consultation_start_intent()
            .and_then(|intent| intent.fresh_children().nth(self.children.len()))
        else {
            return Err(ResidentProcessConsultationChildStartError::BoundExceeded);
        };
        if !expected.matches_input(&input).map_err(|_| {
            ResidentProcessConsultationChildStartError::Start(
                ResidentProcessConsultStartError::Selection(
                    ResidentProcessHostSelectionError::ConsultationChildAssociation,
                ),
            )
        })? {
            return Err(ResidentProcessConsultationChildStartError::Start(
                ResidentProcessConsultStartError::Selection(
                    ResidentProcessHostSelectionError::ConsultationChildAssociation,
                ),
            ));
        }
        let child = host
            .start_consultation_child(
                &self.selector,
                access,
                authority,
                input,
                parent,
                admit_compiled_authority,
                sign_handle,
                sign_observation,
                prepare_checkpoint_preimage,
                sign_checkpoint,
            )
            .map_err(ResidentProcessConsultationChildStartError::Start)?;
        self.children.push(child);
        Ok(())
    }

    /// Commit the exact initial collection after all fresh children are durable.
    ///
    /// Fresh-child coordinates must equal the started children in the same
    /// roster order. Evidence-resolved slots do not consume child capacity.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn open_collection<
        'store,
        WriterGuard,
        Authority,
        AdmitCollection,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: ResidentProcessConsultationCollectionInput<'_>,
        resolve_store: &dyn super::super::OperationObservationAuthorityStoreResolver,
        admit_collection: AdmitCollection,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ResidentProcessConsultationOpened, ResidentProcessConsultationOpenJourneyFailure>
    where
        AdmitCollection: FnOnce(&ConsultationCollection) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        if !self.matches_collection(&input.collection)
            || self
                .parent
                .consultation_start_intent()
                .is_none_or(|intent| intent.limits().ok() != Some(input.limits))
        {
            return Err(ResidentProcessConsultationOpenJourneyFailure {
                started: Box::new(self),
                reason: ResidentProcessConsultationOpenJourneyError::ChildAssociation,
            });
        }
        let input = ResidentConsultationOpenInput {
            collection: input.collection,
            parent: &self.parent,
            parent_observation: input.parent_observation,
            parent_checkpoint: input.parent_checkpoint,
            observation: input.observation,
            limits: input.limits,
            checkpoint: input.checkpoint,
        };
        let completed = match host.open_consultation_collection(
            &self.selector,
            access,
            authority,
            input,
            resolve_store,
            admit_collection,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        ) {
            Ok(completed) => completed,
            Err(reason) => {
                return Err(ResidentProcessConsultationOpenJourneyFailure {
                    started: Box::new(self),
                    reason: ResidentProcessConsultationOpenJourneyError::Open(reason),
                });
            }
        };
        Ok(ResidentProcessConsultationOpened {
            started: self,
            completed,
        })
    }

    pub(super) fn matches_collection(&self, collection: &ConsultationCollection) -> bool {
        if !self
            .parent
            .consultation_start_intent()
            .is_some_and(|intent| intent.matches_collection(collection))
        {
            return false;
        }
        let fresh_children = collection.slots().iter().filter_map(|slot| {
            let ConsultationContributionSource::FreshChild { child } = slot.roster().source()
            else {
                return None;
            };
            Some(child)
        });
        let mut count = 0usize;
        for (expected, issued) in fresh_children.zip(self.children.iter()) {
            let (reference, handle) = issued.handle();
            if expected.record_ref != *reference
                || expected.record_sha256.as_str()
                    != handle.canonical_operation_handle_sha256().as_str()
            {
                return false;
            }
            count += 1;
        }
        count == self.children.len()
            && collection
                .slots()
                .iter()
                .filter(|slot| {
                    matches!(
                        slot.roster().source(),
                        ConsultationContributionSource::FreshChild { .. }
                    )
                })
                .count()
                == self.children.len()
    }
}

impl ResidentProcessConsultationChildPending {
    /// Borrow the durable parent whose signed identity fixed this child plan.
    pub(super) const fn parent(&self) -> &super::super::IssuedLocalOperation {
        self.started.parent()
    }

    /// Borrow the durable child prefix in retained roster order.
    pub(super) fn children(&self) -> &[super::super::IssuedLocalOperation] {
        self.started.children()
    }

    /// Borrow the exact next child plan from the parent's retained intent.
    pub(super) fn child(&self) -> &ConsultationFreshChildPlan {
        self.started
            .parent
            .consultation_start_intent()
            .and_then(|intent| intent.fresh_children().nth(self.started.children.len()))
            .expect("a child-pending phase has one retained next child")
    }

    /// Recover the underlying start phase without advancing it.
    pub(super) fn into_started(self) -> ResidentProcessConsultationStarted {
        self.started
    }

    /// Start exactly one child, then return the next durable boundary.
    ///
    /// Existing child-start admission compares the supplied input with
    /// [`Self::child`] before authority, signing, or custody work. Failure keeps
    /// this same pending-child boundary. Success cannot open the collection.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn start<
        'store,
        WriterGuard,
        Authority,
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: ResidentLocalOperationStartInput<'_>,
        parent: PreparedLocalChildParent,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<
        ResidentProcessConsultationInitialization,
        ResidentProcessConsultationInitializationChildFailure,
    >
    where
        Authority: super::super::AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::super::AdmittedAmbientSession,
            &super::super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage: FnOnce(
            &super::super::PreparedLocalOperationCheckpoint,
        ) -> Result<
            ControlStateCheckpointPreimageV1,
            super::super::LocalOperationStartError,
        >,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let mut started = self.started;
        if let Err(reason) = started.start_child(
            host,
            access,
            authority,
            input,
            parent,
            admit_compiled_authority,
            sign_handle,
            sign_observation,
            prepare_checkpoint_preimage,
            sign_checkpoint,
        ) {
            return Err(ResidentProcessConsultationInitializationChildFailure {
                pending: Box::new(Self { started }),
                reason,
            });
        }
        Ok(started.into_initialization())
    }
}

impl ResidentProcessConsultationCollectionPending {
    /// Borrow the durable parent without claiming collection custody.
    pub(super) const fn parent(&self) -> &super::super::IssuedLocalOperation {
        self.started.parent()
    }

    /// Borrow the complete durable child prefix in retained roster order.
    pub(super) fn children(&self) -> &[super::super::IssuedLocalOperation] {
        self.started.children()
    }

    /// Rebuild the collection selected by the parent's retained start intent.
    ///
    /// This performs no write and allocates no identity. Fresh physical record
    /// coordinates and signers remain inputs to [`Self::open`].
    pub(super) fn admit_initial_collection(
        &self,
    ) -> Result<(ConsultationCollection, ConsultationCollectionLimits), ConsultationCollectionError>
    {
        self.parent()
            .consultation_start_intent()
            .ok_or(ConsultationCollectionError::Association)?
            .admit_initial_collection(self.started.store(), self.parent(), self.children())
    }

    /// Recover the underlying start phase without opening the collection.
    pub(super) fn into_started(self) -> ResidentProcessConsultationStarted {
        self.started
    }

    /// Open the exact initial collection as a separate durable step.
    ///
    /// Collection custody does not imply that any child ran or contributed, or
    /// that the consultation's completion policy is satisfied.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn open<
        'store,
        WriterGuard,
        Authority,
        AdmitCollection,
        Rotation,
        ObservationSign,
        CheckpointSign,
    >(
        self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: ResidentProcessConsultationCollectionInput<'_>,
        resolve_store: &dyn super::super::OperationObservationAuthorityStoreResolver,
        admit_collection: AdmitCollection,
        admit_rotation: Rotation,
        sign_observation: ObservationSign,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ResidentProcessConsultationOpened, ResidentProcessConsultationOpenJourneyFailure>
    where
        AdmitCollection: FnOnce(&ConsultationCollection) -> Result<(), InvalidResidentRecordV1>,
        Rotation: FnOnce(
            &ResidentRecordRefV1,
            &ArtifactDigest,
            &ResidentRecordRefV1,
            &ArtifactDigest,
        ) -> Result<(), InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        self.started.open_collection(
            host,
            access,
            authority,
            input,
            resolve_store,
            admit_collection,
            admit_rotation,
            sign_observation,
            sign_checkpoint,
        )
    }
}

impl<'store, WriterGuard> ResidentProcessHost<'store, WriterGuard> {
    /// Start one root Consult and retain capacity for its selected fresh children.
    ///
    /// Capacity is reserved before any authority or custody port is used. This
    /// starts no child and selects no consultation meaning.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn begin_consultation_start<
        Authority,
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: ResidentLocalOperationStartInput<'_>,
        intent: ConsultationStartIntent,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ResidentProcessConsultationStarted, ResidentProcessConsultationBeginError>
    where
        Authority: super::super::AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::super::AdmittedAmbientSession,
            &super::super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        CheckpointPreimage: FnOnce(
            &super::super::PreparedLocalOperationCheckpoint,
        ) -> Result<
            ControlStateCheckpointPreimageV1,
            super::super::LocalOperationStartError,
        >,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let maximum_children = intent.fresh_child_count();
        let mut children = Vec::new();
        children
            .try_reserve_exact(maximum_children)
            .map_err(|_| ResidentProcessConsultationBeginError::Capacity)?;
        let store = access.store.clone();
        let parent = self
            .start_consult(
                selector,
                access,
                authority,
                input,
                Some(&intent),
                admit_compiled_authority,
                sign_handle,
                sign_observation,
                prepare_checkpoint_preimage,
                sign_checkpoint,
            )
            .map_err(ResidentProcessConsultationBeginError::Start)?;
        Ok(ResidentProcessConsultationStarted {
            selector: selector.clone(),
            store,
            parent,
            children,
            maximum_children,
        })
    }

    /// Start a Consult whose child identities depend on its signed parent handle.
    ///
    /// The child vector is reserved first. The builder may only return retained
    /// intent; it cannot retain or dispatch a child. Parent custody begins only
    /// after that intent has the declared number of fresh children.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn begin_consultation_start_with_intent_builder<
        Authority,
        CompiledAuthority,
        HandleSign,
        ObservationSign,
        Intent,
        CheckpointPreimage,
        CheckpointSign,
    >(
        &mut self,
        selector: &ResidentDeploymentAuthoritySelector,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        input: ResidentLocalOperationStartInput<'_>,
        maximum_children: usize,
        build_intent: Intent,
        admit_compiled_authority: CompiledAuthority,
        sign_handle: HandleSign,
        sign_observation: ObservationSign,
        prepare_checkpoint_preimage: CheckpointPreimage,
        sign_checkpoint: CheckpointSign,
    ) -> Result<ResidentProcessConsultationStarted, ResidentProcessConsultationBeginError>
    where
        Authority: super::super::AmbientSessionAuthorityPort,
        CompiledAuthority: FnOnce(
            &CommonRequestEnvelopeV1,
            &super::super::AdmittedAmbientSession,
            &super::super::AmbientSessionAdmissionContext<'_>,
            &OperationHandlePreimageV1,
            &OperationLeaseV1,
            &crate::semantic::ResolvedOperation,
            &super::super::LocalOperationPublicationPolicyInput<'_>,
            &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
        ) -> Result<(), InvalidResidentRecordV1>,
        HandleSign: FnOnce(
            &[u8],
            &OperationHandlePreimageV1,
            &OperationHandleDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        ObservationSign: FnOnce(
            &[u8],
            &OperationObservationPreimageV1,
            &OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
        Intent: FnOnce(
            &AuthorityStoreCoordinate,
            &ResidentRecordRefV1,
            &pale_core::records::OperationHandleV1,
        )
            -> Result<ConsultationStartIntent, super::super::LocalOperationStartError>,
        CheckpointPreimage: FnOnce(
            &super::super::PreparedLocalOperationCheckpoint,
        ) -> Result<
            ControlStateCheckpointPreimageV1,
            super::super::LocalOperationStartError,
        >,
        CheckpointSign: FnOnce(
            &[u8],
            &ControlStateCheckpointPreimageV1,
            &ControlStateCheckpointDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let mut children = Vec::new();
        children
            .try_reserve_exact(maximum_children)
            .map_err(|_| ResidentProcessConsultationBeginError::Capacity)?;
        let store = access.store.clone();
        let parent = self
            .start_consult_with_intent_builder(
                selector,
                access,
                authority,
                input,
                admit_compiled_authority,
                sign_handle,
                sign_observation,
                |store, reference, handle| {
                    let intent = build_intent(store, reference, handle)?;
                    if intent.fresh_child_count() != maximum_children {
                        return Err(super::super::LocalOperationStartError::Coordinate);
                    }
                    Ok(intent)
                },
                prepare_checkpoint_preimage,
                sign_checkpoint,
            )
            .map_err(ResidentProcessConsultationBeginError::Start)?;
        Ok(ResidentProcessConsultationStarted {
            selector: selector.clone(),
            store,
            parent,
            children,
            maximum_children,
        })
    }

    /// Rebuild a start phase from the parent's retained intent.
    ///
    /// The retained intent fixes every child identity and its order. The resolver
    /// performs point recovery for those identities; `None` leaves that child and
    /// every later child unretained.
    pub(super) fn rebuild_consultation_start_from_retained_intent<Recover>(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        store: &AuthorityStoreCoordinate,
        parent: super::super::IssuedLocalOperation,
        mut recover: Recover,
    ) -> Result<ResidentProcessConsultationStarted, ResidentProcessConsultationResumeError>
    where
        Recover: FnMut(
            &ConsultationFreshChildPlan,
        ) -> Result<
            Option<super::super::IssuedLocalOperation>,
            super::super::LocalOperationRecoveryError,
        >,
    {
        let index = self
            .deployment_authority_index(selector)
            .map_err(ResidentProcessConsultationResumeError::Selection)?;
        if self.deployments[index].store() != store
            || !selector.matches_operation(&parent)
            || !parent.checkpoint_selection().matches_store(store)
            || parent.handle().1.operation_class() != &OperationClassV1::Consult
        {
            return Err(ResidentProcessConsultationResumeError::Selection(
                ResidentProcessHostSelectionError::ConsultationParentAuthority,
            ));
        }
        let intent = parent.consultation_start_intent().ok_or(
            ResidentProcessConsultationResumeError::Selection(
                ResidentProcessHostSelectionError::ConsultationParentAuthority,
            ),
        )?;
        if intent.validate_parent(store, &parent).is_err() {
            return Err(ResidentProcessConsultationResumeError::Selection(
                ResidentProcessHostSelectionError::ConsultationParentAuthority,
            ));
        }
        let maximum_children = intent.fresh_child_count();
        let mut recovered = Vec::new();
        recovered
            .try_reserve_exact(maximum_children)
            .map_err(|_| ResidentProcessConsultationResumeError::Capacity)?;
        let mut started = ResidentProcessConsultationStarted {
            selector: selector.clone(),
            store: store.clone(),
            parent,
            children: recovered,
            maximum_children,
        };
        let plans = started
            .parent
            .consultation_start_intent()
            .expect("validated intent remains retained")
            .fresh_children()
            .cloned()
            .collect::<Vec<_>>();
        let mut missing = false;
        for plan in &plans {
            match recover(plan).map_err(ResidentProcessConsultationResumeError::Recovery)? {
                Some(child) if !missing => started
                    .retain_recovered_child(child)
                    .map_err(ResidentProcessConsultationResumeError::Child)?,
                Some(_) => {
                    return Err(ResidentProcessConsultationResumeError::Selection(
                        ResidentProcessHostSelectionError::ConsultationChildAssociation,
                    ));
                }
                None => missing = true,
            }
        }
        Ok(started)
    }

    /// Recover the retained child prefix through each exact protected handle.
    ///
    /// An absent handle is an unstarted suffix. Once a later planned handle is
    /// present, or any present handle lacks complete protected evidence, recovery
    /// fails instead of omitting or recreating a child.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn rebuild_consultation_start_from_protected_intent(
        &self,
        selector: &ResidentDeploymentAuthoritySelector,
        store: &AuthorityStoreCoordinate,
        parent: super::super::IssuedLocalOperation,
        context: LocalOperationRecoveryContext<'_>,
        maximum_parent_depth: usize,
        operation_port: &mut dyn pale_ports::OperationCustodyPort,
        session_port: &dyn pale_ports::SessionLeaseCustodyPort,
        anchor_port: &mut dyn pale_ports::AntiRollbackAnchorPort,
        admission: OperationControlRecoveryAdmission<'_>,
    ) -> Result<ResidentProcessConsultationStarted, ResidentProcessConsultationResumeError> {
        let OperationControlRecoveryAdmission {
            admit_request,
            admit_handle_authority,
            verify_handle_signature,
            admit_checkpoint,
            resolve_observation_signer,
            admit_observation_state,
            admit_rotation,
            verify_observation_signature,
            verify_checkpoint_signature,
        } = admission;
        self.rebuild_consultation_start_from_retained_intent(selector, store, parent, |plan| {
            let expected_handle = plan.handle_coordinate();
            let selected = operation_port
                .read_operation_record(store, &expected_handle, context.maximum_record_bytes)
                .map_err(super::super::LocalOperationRecoveryError::Store)?;
            let Some(selected) = selected else {
                return Ok(None);
            };
            if selected.coordinate() != &expected_handle {
                return Err(super::super::LocalOperationRecoveryError::Association);
            }
            recover_local_operation_from_handle(
                LocalOperationHandleRecoveryInput {
                    context,
                    expected_handle: &expected_handle,
                    maximum_parent_depth,
                },
                operation_port,
                session_port,
                anchor_port,
                &mut *admit_request,
                LocalOperationRecoveryAdmission {
                    admit_historical_authority: &mut *admit_handle_authority,
                    verify_handle_signature: &mut *verify_handle_signature,
                    verify_checkpoint_signature: &mut *verify_checkpoint_signature,
                    initial_observation: super::super::OperationObservationRecoveryAdmission {
                        admit_checkpoint: &mut *admit_checkpoint,
                        resolve_signer: &mut *resolve_observation_signer,
                        admit_state: &mut *admit_observation_state,
                        admit_rotation: &mut *admit_rotation,
                        verify_signature: &mut *verify_observation_signature,
                    },
                },
            )
            .map(|recovered| Some(recovered.into_operation()))
        })
    }
}

/// Collection custody completed for one exact started Consult.
#[must_use = "recover the current collection before retaining a dispatch batch"]
pub(super) struct ResidentProcessConsultationOpened {
    started: ResidentProcessConsultationStarted,
    completed: CompletedConsultationCollection,
}

impl ResidentProcessConsultationOpened {
    /// Borrow the durable Consult parent.
    pub(super) const fn parent(&self) -> &super::super::IssuedLocalOperation {
        self.started.parent()
    }

    /// Borrow the completed collection custody evidence.
    pub(super) const fn completed(&self) -> &CompletedConsultationCollection {
        &self.completed
    }

    /// Recover the current collection from the exact protected parent head.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn recover<'store, WriterGuard, Authority, AdmitCollection>(
        self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
        protected: &RecoveredOperationObservation,
        limits: ConsultationCollectionLimits,
        resolve_store: &dyn super::super::OperationObservationAuthorityStoreResolver,
        admit_collection: AdmitCollection,
    ) -> Result<ResidentProcessConsultationRecovered, ResidentProcessConsultationRecoveryFailure>
    where
        AdmitCollection: FnOnce(&ConsultationCollection) -> Result<(), InvalidResidentRecordV1>,
    {
        let current = match host.recover_consultation_collection(
            &self.started.selector,
            access,
            authority,
            &self.started.parent,
            protected,
            limits,
            resolve_store,
            admit_collection,
        ) {
            Ok(Some(current)) => current,
            Ok(None) => {
                return Err(ResidentProcessConsultationRecoveryFailure {
                    opened: Box::new(self),
                    reason: ResidentProcessConsultationJourneyRecoveryError::Absent,
                });
            }
            Err(reason) => {
                return Err(ResidentProcessConsultationRecoveryFailure {
                    opened: Box::new(self),
                    reason: ResidentProcessConsultationJourneyRecoveryError::Recovery(reason),
                });
            }
        };
        Ok(ResidentProcessConsultationRecovered {
            opened: self,
            current,
        })
    }
}

/// A current collection recovered after exact durable opening.
#[must_use = "retain its finite first batch or preserve this phase"]
pub(super) struct ResidentProcessConsultationRecovered {
    opened: ResidentProcessConsultationOpened,
    current: RecoveredConsultationCollection,
}

impl ResidentProcessConsultationRecovered {
    /// Borrow the exact re-admitted collection.
    pub(super) const fn current(&self) -> &RecoveredConsultationCollection {
        &self.current
    }

    /// Retain the first finite batch without dispatching a child.
    pub(super) fn open_initial_batch<'store, WriterGuard>(
        self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        now: UnixTimeV1,
        limits: ResidentConsultationTaskLimits,
    ) -> Result<ResidentProcessConsultationReady, ResidentProcessConsultationBatchFailure> {
        let coordinate = match host.open_initial_consultation_batch(
            &self.opened.started.selector,
            &self.opened.started.store,
            &self.opened.started.parent,
            &self.current,
            now,
            limits,
        ) {
            Ok(coordinate) => coordinate,
            Err(reason) => {
                return Err(ResidentProcessConsultationBatchFailure {
                    recovered: Box::new(self),
                    reason,
                });
            }
        };
        Ok(ResidentProcessConsultationReady {
            recovered: self,
            coordinate,
        })
    }
}

/// Durable parent, durable children, recovered collection, and retained first batch.
#[must_use = "the retained batch still needs child-specific recovery and dispatch"]
pub(super) struct ResidentProcessConsultationReady {
    recovered: ResidentProcessConsultationRecovered,
    coordinate: ResidentConsultationBatchCoordinate,
}

impl ResidentProcessConsultationReady {
    /// Borrow the retained finite batch coordinate.
    pub(super) const fn coordinate(&self) -> &ResidentConsultationBatchCoordinate {
        &self.coordinate
    }

    /// Borrow the durable parent operation.
    pub(super) const fn parent(&self) -> &super::super::IssuedLocalOperation {
        self.recovered.opened.started.parent()
    }

    /// Borrow the durable child operations in roster order.
    pub(super) fn children(&self) -> &[super::super::IssuedLocalOperation] {
        self.recovered.opened.started.children()
    }

    /// Borrow the re-admitted current collection.
    pub(super) const fn current(&self) -> &RecoveredConsultationCollection {
        self.recovered.current()
    }

    /// Borrow the deployment selected for this exact retained batch.
    pub(super) const fn selector(&self) -> &ResidentDeploymentAuthoritySelector {
        self.recovered.opened.started.selector()
    }

    /// Borrow the authority store selected for this exact retained batch.
    pub(super) const fn store(&self) -> &AuthorityStoreCoordinate {
        self.recovered.opened.started.store()
    }

    /// Move the retained start and current collection into the Consult-turn owner.
    ///
    /// The process task owner already retains the finite batch identified by
    /// `coordinate`. After this transfer, that task owner and the returned
    /// values are the semantic continuation; the response cache owns no copy of
    /// this state.
    pub(super) fn into_pre_policy_inputs(
        self,
    ) -> (
        ResidentProcessConsultationStarted,
        RecoveredConsultationCollection,
    ) {
        let Self {
            recovered:
                ResidentProcessConsultationRecovered {
                    opened:
                        ResidentProcessConsultationOpened {
                            started,
                            completed: _,
                        },
                    current,
                },
            coordinate: _,
        } = self;
        (started, current)
    }
}

/// Failure before the parent Consult reached complete custody.
pub(super) enum ResidentProcessConsultationBeginError {
    /// The selected child bound could not be reserved before port I/O.
    Capacity,
    /// Existing durable-start semantics rejected or retained the parent phase.
    Start(ResidentProcessConsultStartError),
}

/// Failure while rebuilding a bounded phase from supplied recovered operations.
pub(super) enum ResidentProcessConsultationResumeError {
    /// The exact deployment, store, or Consult parent did not match.
    Selection(ResidentProcessHostSelectionError),
    /// The bounded child vector could not be reserved before later port use.
    Capacity,
    /// One recovered child was not an exact narrower child of the parent.
    Child(ResidentProcessConsultationChildStartError),
    /// Exact point recovery failed before the phase could be rebuilt.
    Recovery(super::super::LocalOperationRecoveryError),
}

#[cfg(test)]
#[path = "../../../tests/support/process_consultation_started.rs"]
mod test_support;

/// Failure while adding one already-selected durable child.
pub(super) enum ResidentProcessConsultationChildStartError {
    /// The preallocated child ceiling was already full.
    BoundExceeded,
    /// Existing durable-start semantics rejected or retained the child phase.
    Start(ResidentProcessConsultStartError),
}

/// Why initial collection opening did not complete.
pub(super) enum ResidentProcessConsultationOpenJourneyError {
    /// The collection's fresh children differ from this phase's durable children.
    ChildAssociation,
    /// Existing collection admission or custody did not complete.
    Open(ResidentProcessConsultationOpenError),
    /// A retained custody retry did not complete.
    Continuation(ResidentProcessConsultationOpenContinuationFailure),
}

/// Started operations retained when initial collection opening fails.
#[must_use = "preserve the started operations and any lower custody continuation"]
pub(super) struct ResidentProcessConsultationOpenJourneyFailure {
    pub(super) started: Box<ResidentProcessConsultationStarted>,
    pub(super) reason: ResidentProcessConsultationOpenJourneyError,
}

impl ResidentProcessConsultationOpenJourneyFailure {
    /// Continue only the exact retained collection-custody phase.
    ///
    /// Admission failures and child-association failures are not retryable
    /// custody phases. A failed retry returns the same started operations and
    /// the lower continuation without repeating collection construction,
    /// signing, database commitment, or child work.
    pub(super) fn continue_custody<'store, WriterGuard, Authority>(
        self,
        host: &mut ResidentProcessHost<'store, WriterGuard>,
        access: ResidentDeploymentOperationAccess<'_>,
        authority: &mut Authority,
    ) -> Result<ResidentProcessConsultationOpened, Self> {
        let Self { started, reason } = self;
        let pending = match reason {
            ResidentProcessConsultationOpenJourneyError::Open(
                ResidentProcessConsultationOpenError::Custody(pending),
            ) => pending,
            ResidentProcessConsultationOpenJourneyError::Continuation(failure) => failure.pending,
            reason => return Err(Self { started, reason }),
        };
        match host.continue_consultation_collection_custody(access, authority, pending) {
            Ok(completed) => Ok(ResidentProcessConsultationOpened {
                started: *started,
                completed,
            }),
            Err(failure) => Err(Self {
                started,
                reason: ResidentProcessConsultationOpenJourneyError::Continuation(failure),
            }),
        }
    }
}

/// Why a durably opened collection could not be recovered.
pub(super) enum ResidentProcessConsultationJourneyRecoveryError {
    /// No collection exists at the protected head after opening completed.
    Absent,
    /// Existing protected recovery rejected the stored or admitted meaning.
    Recovery(ResidentProcessConsultationCollectionRecoveryError),
}

/// Durable opening retained when current collection recovery fails.
#[must_use = "preserve completed custody while recovery remains unresolved"]
pub(super) struct ResidentProcessConsultationRecoveryFailure {
    pub(super) opened: Box<ResidentProcessConsultationOpened>,
    pub(super) reason: ResidentProcessConsultationJourneyRecoveryError,
}

impl ResidentProcessConsultationRecoveryFailure {
    /// Retry read-only recovery of the same durably opened collection.
    pub(super) fn into_opened(self) -> ResidentProcessConsultationOpened {
        *self.opened
    }
}

/// Current collection retained when its finite batch cannot be opened.
#[must_use = "preserve the recovered collection for an exact bounded retry"]
pub(super) struct ResidentProcessConsultationBatchFailure {
    pub(super) recovered: Box<ResidentProcessConsultationRecovered>,
    pub(super) reason: ResidentProcessConsultationBatchOpenError,
}

impl ResidentProcessConsultationBatchFailure {
    /// Retry task-owner admission of the same recovered collection.
    pub(super) fn into_recovered(self) -> ResidentProcessConsultationRecovered {
        *self.recovered
    }
}
