//! Retained start intent for opening the same consultation after restart.
//!
//! The intent freezes the ordered roster meaning. It does not prove child
//! custody, collection custody, dispatch, transport, or execution.

use std::collections::BTreeSet;

use pale_core::{canon, records::*};
use pale_ports::AuthorityStoreCoordinate;
use serde::{Deserialize, Serialize};

use super::{
    ConsultationCollection, ConsultationCollectionAdmission, ConsultationCollectionError,
    ConsultationCollectionLimits, ConsultationContributionSource, ConsultationRequirement,
    ConsultationRosterSlot, ConsultationSlotId,
};
use crate::resident::local_operation_coordinator::ResidentLocalOperationStartInput;

const FORMAT: &str = "pale/private-consultation-start-intent.v1";

/// One child identity frozen before the consultation parent starts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::resident) struct ConsultationFreshChildPlan {
    request_ref: ResidentRecordRefV1,
    request_sha256: CommonRequestEnvelopeDigest,
    handle_ref: ResidentRecordRefV1,
    handle_id: OperationHandleIdV1,
    handle_sha256: OperationHandleDigest,
    operation_class: OperationClassV1,
    parent_operation: ParentOperationReferenceV1,
    compiled_generation_sha256: CompiledGenerationDigest,
    activation_fence_sha256: ActivationFenceDigest,
    instance_sha256: InstanceDigest,
    observation_ref: ResidentRecordRefV1,
    observation_schema: SchemaDigest,
    checkpoint_id: ControlCheckpointIdV1,
    checkpoint_ref: ResidentRecordRefV1,
    checkpoint_schema: SchemaDigest,
    checkpoint_profile_ref: ResidentRecordRefV1,
    checkpoint_profile_sha256: CheckpointProfileDigest,
}

impl ConsultationFreshChildPlan {
    /// Capture exact immutable identities from an already-selected child start.
    /// This does not sign, retain, authorize, or dispatch the child.
    pub(in crate::resident) fn capture(
        input: &ResidentLocalOperationStartInput<'_>,
    ) -> Result<Self, ConsultationStartIntentError> {
        let class = input.semantic.handle.operation_class;
        if !matches!(class, OperationClassV1::Invoke | OperationClassV1::Consult) {
            return Err(ConsultationStartIntentError::Child);
        }
        let (checkpoint_profile_ref, checkpoint_profile_sha256) =
            input.semantic.checkpoint_profile.coordinates();
        let parent_operation = input
            .semantic
            .handle
            .parent_operation
            .clone()
            .ok_or(ConsultationStartIntentError::Child)?;
        Ok(Self {
            request_ref: input.semantic.request_ref.clone(),
            request_sha256: input.semantic.request.canonical_request_sha256().clone(),
            handle_ref: input.semantic.handle_ref.clone(),
            handle_id: input.semantic.handle.operation_handle_id.clone(),
            handle_sha256: OperationHandleDigest::hash_canonical(&input.semantic.handle)?,
            operation_class: class,
            parent_operation,
            compiled_generation_sha256: input.semantic.handle.compiled_generation_sha256.clone(),
            activation_fence_sha256: input.semantic.handle.activation_fence_sha256.clone(),
            instance_sha256: input.semantic.handle.instance_sha256.clone(),
            observation_ref: input.semantic.observation_ref.clone(),
            observation_schema: input.semantic.observation_schema.clone(),
            checkpoint_id: input.checkpoint.id.clone(),
            checkpoint_ref: input.checkpoint.reference.clone(),
            checkpoint_schema: input.checkpoint.schema.clone(),
            checkpoint_profile_ref: checkpoint_profile_ref.clone(),
            checkpoint_profile_sha256: checkpoint_profile_sha256.clone(),
        })
    }

    pub(in crate::resident) fn child_reference(&self) -> ImmutableRecordReferenceV1 {
        ImmutableRecordReferenceV1 {
            record_ref: self.handle_ref.clone(),
            record_sha256: ArtifactDigest::parse(self.handle_sha256.as_str().to_owned())
                .expect("typed operation-handle digest is an artifact digest"),
        }
    }

    pub(in crate::resident) fn handle_coordinate(&self) -> pale_ports::ControlRecordCoordinate {
        pale_ports::ControlRecordCoordinate::new(
            self.handle_ref.clone(),
            pale_ports::ControlRecordDigest::parse(self.handle_sha256.as_str().to_owned())
                .expect("typed operation-handle digest is a control-record digest"),
        )
    }

    pub(in crate::resident) fn matches_input(
        &self,
        input: &ResidentLocalOperationStartInput<'_>,
    ) -> Result<bool, canon::CanonError> {
        let (profile_ref, profile_digest) = input.semantic.checkpoint_profile.coordinates();
        Ok(self.request_ref == input.semantic.request_ref
            && self.request_sha256 == *input.semantic.request.canonical_request_sha256()
            && self.handle_ref == input.semantic.handle_ref
            && self.handle_id == input.semantic.handle.operation_handle_id
            && self.handle_sha256 == OperationHandleDigest::hash_canonical(&input.semantic.handle)?
            && self.operation_class == input.semantic.handle.operation_class
            && Some(&self.parent_operation) == input.semantic.handle.parent_operation.as_ref()
            && self.compiled_generation_sha256 == input.semantic.handle.compiled_generation_sha256
            && self.activation_fence_sha256 == input.semantic.handle.activation_fence_sha256
            && self.instance_sha256 == input.semantic.handle.instance_sha256
            && self.observation_ref == input.semantic.observation_ref
            && self.observation_schema == input.semantic.observation_schema
            && self.checkpoint_id == input.checkpoint.id
            && self.checkpoint_ref == input.checkpoint.reference
            && self.checkpoint_schema == *input.checkpoint.schema
            && self.checkpoint_profile_ref == *profile_ref
            && self.checkpoint_profile_sha256 == *profile_digest)
    }

    pub(in crate::resident) fn matches_operation(
        &self,
        operation: &crate::resident::IssuedLocalOperation,
    ) -> bool {
        let (reference, handle) = operation.handle();
        self.handle_ref == *reference
            && self.handle_id == *handle.operation_handle_id()
            && self.handle_sha256 == *handle.canonical_operation_handle_sha256()
            && self.operation_class == *handle.operation_class()
            && self.request_ref == *operation.request_coordinate().record_ref()
            && self.request_sha256.as_str()
                == operation.request_coordinate().record_sha256().as_str()
    }
}

/// The source that satisfies one frozen roster slot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub(in crate::resident) enum ConsultationStartIntentSource {
    /// Existing evidence already satisfied the selected policy.
    EvidenceResolved {
        evidence: ImmutableRecordReferenceV1,
    },
    /// One exact child must be present before collection opens.
    FreshChild {
        child: Box<ConsultationFreshChildPlan>,
    },
}

/// Declarative source selected before a Consult parent owns a signed identity.
///
/// Evidence can be retained immediately. A fresh child keeps only the
/// application-selected input needed by the operation-allocation owner; no
/// operation identity is accepted through this type.
pub(in crate::resident) enum ConsultationRosterTemplateSource<FreshChild> {
    EvidenceResolved {
        evidence: ImmutableRecordReferenceV1,
    },
    FreshChild {
        child: FreshChild,
    },
}

/// One ordered consultation slot before fresh child identities are allocated.
pub(in crate::resident) struct ConsultationRosterTemplateSlot<FreshChild> {
    pub(in crate::resident) slot_id: ConsultationSlotId,
    pub(in crate::resident) requirement: ConsultationRequirement,
    pub(in crate::resident) requirement_evidence: ImmutableRecordReferenceV1,
    pub(in crate::resident) participant: ParticipantRefV1,
    pub(in crate::resident) provider_binding: Option<ProviderBindingRefV1>,
    pub(in crate::resident) context: Option<ImmutableRecordReferenceV1>,
    pub(in crate::resident) source: ConsultationRosterTemplateSource<FreshChild>,
}

impl<FreshChild> ConsultationRosterTemplateSlot<FreshChild> {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn new(
        slot_id: ConsultationSlotId,
        requirement: ConsultationRequirement,
        requirement_evidence: ImmutableRecordReferenceV1,
        participant: ParticipantRefV1,
        provider_binding: Option<ProviderBindingRefV1>,
        context: Option<ImmutableRecordReferenceV1>,
        source: ConsultationRosterTemplateSource<FreshChild>,
    ) -> Self {
        Self {
            slot_id,
            requirement,
            requirement_evidence,
            participant,
            provider_binding,
            context,
            source,
        }
    }

    pub(in crate::resident) fn with_source(
        self,
        source: ConsultationStartIntentSource,
    ) -> ConsultationStartIntentSlot {
        ConsultationStartIntentSlot::new(
            self.slot_id,
            self.requirement,
            self.requirement_evidence,
            self.participant,
            self.provider_binding,
            self.context,
            source,
        )
    }
}

/// Bounded consultation meaning selected before parent or child allocation.
///
/// This value contains no operation identities. Its fresh-child payloads stay
/// private to the application owner until the parent handle fixes their exact
/// ancestry.
pub(in crate::resident) struct ConsultationRosterTemplate<FreshChild> {
    pub(in crate::resident) completion_policy: ImmutableRecordReferenceV1,
    pub(in crate::resident) opened_at: UnixTimeV1,
    pub(in crate::resident) deadline: UnixTimeV1,
    pub(in crate::resident) slots: Vec<ConsultationRosterTemplateSlot<FreshChild>>,
    pub(in crate::resident) limits: ConsultationCollectionLimits,
}

impl<FreshChild> ConsultationRosterTemplate<FreshChild> {
    /// Admit only the bounds and ordering knowable before identity allocation.
    pub(in crate::resident) fn new(
        completion_policy: ImmutableRecordReferenceV1,
        opened_at: UnixTimeV1,
        deadline: UnixTimeV1,
        slots: Vec<ConsultationRosterTemplateSlot<FreshChild>>,
        limits: ConsultationCollectionLimits,
    ) -> Result<Self, ConsultationStartIntentError> {
        if opened_at > deadline || slots.is_empty() || slots.len() > limits.maximum_slots {
            return Err(ConsultationStartIntentError::Association);
        }
        let mut slot_ids = BTreeSet::new();
        if slots
            .iter()
            .any(|slot| !slot_ids.insert(slot.slot_id.clone()))
        {
            return Err(ConsultationStartIntentError::DuplicateSlot);
        }
        Ok(Self {
            completion_policy,
            opened_at,
            deadline,
            slots,
            limits,
        })
    }

    pub(in crate::resident) fn fresh_child_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| {
                matches!(
                    slot.source,
                    ConsultationRosterTemplateSource::FreshChild { .. }
                )
            })
            .count()
    }
}

/// One ordered roster slot frozen before any child custody mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::resident) struct ConsultationStartIntentSlot {
    slot_id: ConsultationSlotId,
    requirement: ConsultationRequirement,
    requirement_evidence: ImmutableRecordReferenceV1,
    participant: ParticipantRefV1,
    provider_binding: Option<ProviderBindingRefV1>,
    context: Option<ImmutableRecordReferenceV1>,
    source: ConsultationStartIntentSource,
}

impl ConsultationStartIntentSlot {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::resident) fn new(
        slot_id: ConsultationSlotId,
        requirement: ConsultationRequirement,
        requirement_evidence: ImmutableRecordReferenceV1,
        participant: ParticipantRefV1,
        provider_binding: Option<ProviderBindingRefV1>,
        context: Option<ImmutableRecordReferenceV1>,
        source: ConsultationStartIntentSource,
    ) -> Self {
        Self {
            slot_id,
            requirement,
            requirement_evidence,
            participant,
            provider_binding,
            context,
            source,
        }
    }

    fn roster(&self) -> ConsultationRosterSlot {
        let source = match &self.source {
            ConsultationStartIntentSource::EvidenceResolved { evidence } => {
                ConsultationContributionSource::EvidenceResolved {
                    evidence: evidence.clone(),
                }
            }
            ConsultationStartIntentSource::FreshChild { child } => {
                ConsultationContributionSource::FreshChild {
                    child: child.child_reference(),
                }
            }
        };
        ConsultationRosterSlot::new(
            self.slot_id.clone(),
            self.requirement,
            self.requirement_evidence.clone(),
            self.participant.clone(),
            self.provider_binding.clone(),
            self.context.clone(),
            source,
        )
    }
}

/// Inputs selected by the application before durable parent start.
#[derive(Clone)]
pub(in crate::resident) struct ConsultationStartIntentAdmission {
    pub(in crate::resident) completion_policy: ImmutableRecordReferenceV1,
    pub(in crate::resident) opened_at: UnixTimeV1,
    pub(in crate::resident) deadline: UnixTimeV1,
    pub(in crate::resident) slots: Vec<ConsultationStartIntentSlot>,
    pub(in crate::resident) limits: ConsultationCollectionLimits,
}

impl ConsultationStartIntentAdmission {
    /// Count the child identities that must be reserved before parent custody.
    pub(in crate::resident) fn fresh_child_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| {
                matches!(
                    slot.source,
                    ConsultationStartIntentSource::FreshChild { .. }
                )
            })
            .count()
    }
}

/// Immutable private roster intent retained with the parent operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::resident) struct ConsultationStartIntent {
    format: String,
    control_store_id: ControlStoreIdV1,
    control_incarnation_id: ControlIncarnationIdV1,
    parent_ref: ResidentRecordRefV1,
    parent_sha256: OperationHandleDigest,
    compiled_generation_sha256: CompiledGenerationDigest,
    activation_fence_sha256: ActivationFenceDigest,
    instance_sha256: InstanceDigest,
    completion_policy: ImmutableRecordReferenceV1,
    opened_at: UnixTimeV1,
    deadline: UnixTimeV1,
    maximum_slots: usize,
    maximum_journal_bytes: usize,
    maximum_frontier_bytes: usize,
    slots: Vec<ConsultationStartIntentSlot>,
}

impl ConsultationStartIntent {
    /// Admit a complete ordered intent before the parent or any child is retained.
    pub(in crate::resident) fn admit(
        store: &AuthorityStoreCoordinate,
        parent_ref: &ResidentRecordRefV1,
        parent: &OperationHandlePreimageV1,
        admission: ConsultationStartIntentAdmission,
    ) -> Result<Self, ConsultationStartIntentError> {
        let parent_sha256 = OperationHandleDigest::hash_canonical(parent)?;
        Self::from_admission(
            store,
            ConsultationIntentParent {
                reference: parent_ref,
                digest: &parent_sha256,
                handle_id: &parent.operation_handle_id,
                operation_class: parent.operation_class,
                compiled_generation: &parent.compiled_generation_sha256,
                activation_fence: &parent.activation_fence_sha256,
                instance: &parent.instance_sha256,
                created_at: parent.created_at,
                expires_at: parent.expires_at,
            },
            admission,
        )
    }

    /// Admit intent selected after the parent handle is signed but before custody.
    pub(in crate::resident) fn admit_signed(
        store: &AuthorityStoreCoordinate,
        parent_ref: &ResidentRecordRefV1,
        parent: &OperationHandleV1,
        admission: ConsultationStartIntentAdmission,
    ) -> Result<Self, ConsultationStartIntentError> {
        Self::from_admission(
            store,
            ConsultationIntentParent {
                reference: parent_ref,
                digest: parent.canonical_operation_handle_sha256(),
                handle_id: parent.operation_handle_id(),
                operation_class: *parent.operation_class(),
                compiled_generation: parent.compiled_generation_sha256(),
                activation_fence: parent.activation_fence_sha256(),
                instance: parent.instance_sha256(),
                created_at: *parent.created_at(),
                expires_at: *parent.expires_at(),
            },
            admission,
        )
    }

    fn from_admission(
        store: &AuthorityStoreCoordinate,
        parent: ConsultationIntentParent<'_>,
        admission: ConsultationStartIntentAdmission,
    ) -> Result<Self, ConsultationStartIntentError> {
        let intent = Self {
            format: FORMAT.to_owned(),
            control_store_id: store.store_id().clone(),
            control_incarnation_id: store.incarnation_id().clone(),
            parent_ref: parent.reference.clone(),
            parent_sha256: parent.digest.clone(),
            compiled_generation_sha256: parent.compiled_generation.clone(),
            activation_fence_sha256: parent.activation_fence.clone(),
            instance_sha256: parent.instance.clone(),
            completion_policy: admission.completion_policy,
            opened_at: admission.opened_at,
            deadline: admission.deadline,
            maximum_slots: admission.limits.maximum_slots,
            maximum_journal_bytes: admission.limits.maximum_journal_bytes,
            maximum_frontier_bytes: admission.limits.maximum_frontier_bytes,
            slots: admission.slots,
        };
        intent.validate(store, parent)?;
        Ok(intent)
    }

    pub(in crate::resident) fn validate_parent(
        &self,
        store: &AuthorityStoreCoordinate,
        parent: &crate::resident::IssuedLocalOperation,
    ) -> Result<ConsultationCollectionLimits, ConsultationStartIntentError> {
        let (reference, handle) = parent.handle();
        self.validate_signed_parent(store, reference, handle)
    }

    pub(in crate::resident) fn validate_signed_parent(
        &self,
        store: &AuthorityStoreCoordinate,
        reference: &ResidentRecordRefV1,
        handle: &OperationHandleV1,
    ) -> Result<ConsultationCollectionLimits, ConsultationStartIntentError> {
        self.validate(
            store,
            ConsultationIntentParent {
                reference,
                digest: handle.canonical_operation_handle_sha256(),
                handle_id: handle.operation_handle_id(),
                operation_class: *handle.operation_class(),
                compiled_generation: handle.compiled_generation_sha256(),
                activation_fence: handle.activation_fence_sha256(),
                instance: handle.instance_sha256(),
                created_at: *handle.created_at(),
                expires_at: *handle.expires_at(),
            },
        )
    }

    pub(in crate::resident) fn fresh_children(
        &self,
    ) -> impl Iterator<Item = &ConsultationFreshChildPlan> {
        self.slots.iter().filter_map(|slot| match &slot.source {
            ConsultationStartIntentSource::EvidenceResolved { .. } => None,
            ConsultationStartIntentSource::FreshChild { child } => Some(child.as_ref()),
        })
    }

    pub(in crate::resident) fn fresh_child_count(&self) -> usize {
        self.fresh_children().count()
    }

    pub(in crate::resident) fn slot_count(&self) -> usize {
        self.slots.len()
    }

    pub(in crate::resident) fn evidence_resolved_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| {
                matches!(
                    slot.source,
                    ConsultationStartIntentSource::EvidenceResolved { .. }
                )
            })
            .count()
    }

    pub(in crate::resident) fn matches_collection(
        &self,
        collection: &ConsultationCollection,
    ) -> bool {
        self.completion_policy == *collection.completion_policy()
            && self.opened_at == collection.opened_at()
            && self.deadline == collection.deadline()
            && self.slots.len() == collection.slots().len()
            && self
                .slots
                .iter()
                .zip(collection.slots())
                .all(|(expected, actual)| expected.roster() == *actual.roster())
    }

    /// Rebuild the exact initial collection after the parent and fresh children
    /// have durable identities.
    ///
    /// The retained intent remains the owner of roster meaning. This method
    /// only joins that meaning to the exact issued operations selected during
    /// the parent-first start journey.
    pub(in crate::resident) fn admit_initial_collection(
        &self,
        store: &AuthorityStoreCoordinate,
        parent: &crate::resident::IssuedLocalOperation,
        children: &[crate::resident::IssuedLocalOperation],
    ) -> Result<(ConsultationCollection, ConsultationCollectionLimits), ConsultationCollectionError>
    {
        let limits = self
            .validate_parent(store, parent)
            .map_err(|error| match error {
                ConsultationStartIntentError::Bounds => ConsultationCollectionError::Bounds,
                ConsultationStartIntentError::Association
                | ConsultationStartIntentError::DuplicateSlot
                | ConsultationStartIntentError::DuplicateChild
                | ConsultationStartIntentError::Child
                | ConsultationStartIntentError::Canonical(_) => {
                    ConsultationCollectionError::Association
                }
            })?;
        if children.len() != self.fresh_child_count()
            || self
                .fresh_children()
                .zip(children)
                .any(|(expected, child)| !expected.matches_operation(child))
        {
            return Err(ConsultationCollectionError::Child);
        }
        let (parent_ref, parent_handle) = parent.handle();
        let child_handles = children
            .iter()
            .map(crate::resident::IssuedLocalOperation::handle)
            .collect::<Vec<_>>();
        let collection = ConsultationCollection::admit_initial(
            parent_ref,
            parent_handle,
            ConsultationCollectionAdmission {
                completion_policy: self.completion_policy.clone(),
                opened_at: self.opened_at,
                deadline: self.deadline,
                slots: self
                    .slots
                    .iter()
                    .map(ConsultationStartIntentSlot::roster)
                    .collect(),
            },
            &child_handles,
            limits,
        )?;
        Ok((collection, limits))
    }

    pub(in crate::resident) fn limits(
        &self,
    ) -> Result<ConsultationCollectionLimits, ConsultationStartIntentError> {
        ConsultationCollectionLimits::new(
            self.maximum_slots,
            self.maximum_journal_bytes,
            self.maximum_frontier_bytes,
        )
        .map_err(|_| ConsultationStartIntentError::Bounds)
    }

    fn validate(
        &self,
        store: &AuthorityStoreCoordinate,
        parent: ConsultationIntentParent<'_>,
    ) -> Result<ConsultationCollectionLimits, ConsultationStartIntentError> {
        let limits = self.limits()?;
        if self.format != FORMAT
            || self.control_store_id != *store.store_id()
            || self.control_incarnation_id != *store.incarnation_id()
            || self.parent_ref != *parent.reference
            || self.parent_sha256 != *parent.digest
            || self.compiled_generation_sha256 != *parent.compiled_generation
            || self.activation_fence_sha256 != *parent.activation_fence
            || self.instance_sha256 != *parent.instance
            || parent.operation_class != OperationClassV1::Consult
            || self.opened_at < parent.created_at
            || self.opened_at > self.deadline
            || self.deadline > parent.expires_at
            || self.slots.is_empty()
            || self.slots.len() > limits.maximum_slots
        {
            return Err(ConsultationStartIntentError::Association);
        }
        let expected_parent = ParentOperationReferenceV1 {
            operation_handle_id: parent.handle_id.clone(),
            operation_handle_ref: parent.reference.clone(),
            operation_handle_sha256: parent.digest.clone(),
        };
        let mut slot_ids = BTreeSet::new();
        let mut children = BTreeSet::new();
        for slot in &self.slots {
            if !slot_ids.insert(slot.slot_id.clone()) {
                return Err(ConsultationStartIntentError::DuplicateSlot);
            }
            let ConsultationStartIntentSource::FreshChild { child } = &slot.source else {
                continue;
            };
            if !children.insert((child.handle_ref.clone(), child.handle_sha256.clone())) {
                return Err(ConsultationStartIntentError::DuplicateChild);
            }
            if !matches!(
                child.operation_class,
                OperationClassV1::Invoke | OperationClassV1::Consult
            ) || child.parent_operation != expected_parent
                || child.compiled_generation_sha256 != *parent.compiled_generation
                || child.activation_fence_sha256 != *parent.activation_fence
                || child.instance_sha256 != *parent.instance
            {
                return Err(ConsultationStartIntentError::Child);
            }
        }
        Ok(limits)
    }
}

struct ConsultationIntentParent<'a> {
    reference: &'a ResidentRecordRefV1,
    digest: &'a OperationHandleDigest,
    handle_id: &'a OperationHandleIdV1,
    operation_class: OperationClassV1,
    compiled_generation: &'a CompiledGenerationDigest,
    activation_fence: &'a ActivationFenceDigest,
    instance: &'a InstanceDigest,
    created_at: UnixTimeV1,
    expires_at: UnixTimeV1,
}

/// Rejection of a private consultation start intent.
#[derive(Debug, thiserror::Error)]
pub(in crate::resident) enum ConsultationStartIntentError {
    #[error("consultation intent has an invalid bound")]
    Bounds,
    #[error("consultation intent does not belong to its parent")]
    Association,
    #[error("consultation intent repeats a slot")]
    DuplicateSlot,
    #[error("consultation intent repeats a child")]
    DuplicateChild,
    #[error("consultation intent contains an invalid child plan")]
    Child,
    #[error(transparent)]
    Canonical(#[from] canon::CanonError),
}
