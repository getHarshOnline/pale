//! Private construction of Consult and Delegate start-owner inputs.
//!
//! These factories compose the existing request-first allocation and retained-
//! lease owners. They add no identity allocator, signer, dispatcher, or provider
//! selection policy of their own.

use pale_core::records::{
    CanonicalSignatureV1, CommonRequestEnvelopeV1,
    ControlCheckpointSigningAuthorityAdmissionContextV1, InvalidResidentRecordV1, OperationClassV1,
    OperationHandleAdmissionContextV1, OperationHandlePreimageV1, OperationHandleV1,
    OperationLeaseV1, ParentOperationAdmissionContextV1, ParentOperationReferenceV1,
    ResidentKeyIdV1, ResidentRecordRefV1, SignedResidentAdmissionContextV1,
    SigningAuthorityBindingV1, CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID,
    OPERATION_HANDLE_V1_SCHEMA_ID, OPERATION_OBSERVATION_V1_SCHEMA_ID,
};
use pale_ports::{AccumulatorPreparationLimits, AuthorityStoreCoordinate};
use std::collections::VecDeque;

use super::{
    ResidentDeploymentAuthoritySelector, ResidentProcessRetainedDelegateOperationLease,
    ResidentProcessRetainedOperationLease,
};
use crate::resident::consultation::{
    ConsultationFreshChildPlan, ConsultationRosterTemplate, ConsultationRosterTemplateSlot,
    ConsultationRosterTemplateSource, ConsultationStartIntent, ConsultationStartIntentAdmission,
    ConsultationStartIntentError, ConsultationStartIntentSource,
};
use crate::resident::local_operation_coordinator::{
    ResidentLocalOperationCheckpointInput, ResidentLocalOperationStartInput,
};
use crate::resident::local_operation_start::{
    RetainedLocalOperationStartFailure, SignedConsultationParentStart,
};
use crate::resident::{
    AdmittedAmbientSession, AdmittedCompiledGeneration, AmbientSessionAdmissionContext,
    LocalOperationPublicationPolicy, LocalOperationStartInput, PreparedLocalChildParent,
    SupportedOperationCheckpointProfile,
};

/// One allocated fresh-child phase joined to the exact private roster plan.
///
/// The constructor derives the plan from the existing child-start input. A
/// caller cannot supply child identity fields separately.
pub(super) struct ResidentConsultationAllocatedChild<Child> {
    child: Child,
    plan: ConsultationFreshChildPlan,
}

impl<Child> ResidentConsultationAllocatedChild<Child> {
    pub(super) fn capture(
        child: Child,
        input: &ResidentLocalOperationStartInput<'_>,
    ) -> Result<Self, ResidentConsultationChildCaptureFailure<Child>> {
        match ConsultationFreshChildPlan::capture(input) {
            Ok(plan) => Ok(Self { child, plan }),
            Err(reason) => Err(ResidentConsultationChildCaptureFailure { child, reason }),
        }
    }

    /// Join owned child state to a plan captured from that exact start input.
    ///
    /// The program owner uses this only after the input factory returns. Keeping
    /// the join separate avoids moving an owned lease phase while that same
    /// phase is borrowed for the factory's association checks.
    pub(super) const fn from_captured(child: Child, plan: ConsultationFreshChildPlan) -> Self {
        Self { child, plan }
    }
}

/// A child identity exists, but it could not become the exact roster plan.
#[must_use = "preserve the allocated child phase on capture failure"]
pub(super) struct ResidentConsultationChildCaptureFailure<Child> {
    pub(super) child: Child,
    pub(super) reason: ConsultationStartIntentError,
}

/// Child allocation or input construction failed without losing owned state.
pub(super) enum ResidentConsultationChildAllocationFailure<Child, Failure> {
    /// No child phase exists; the owner-specific reason is retained.
    Owner(Failure),
    /// A child phase exists and remains available for exact diagnosis/recovery.
    Capture(ResidentConsultationChildCaptureFailure<Child>),
}

/// Partial roster materialization retained across an incomplete child step.
pub(super) struct ResidentConsultationRosterAssembly<FreshChild> {
    completion_policy: pale_core::records::ImmutableRecordReferenceV1,
    opened_at: pale_core::records::UnixTimeV1,
    deadline: pale_core::records::UnixTimeV1,
    limits: crate::resident::ConsultationCollectionLimits,
    materialized: Vec<crate::resident::consultation::ConsultationStartIntentSlot>,
    remaining: VecDeque<ConsultationRosterTemplateSlot<FreshChild>>,
}

/// A signed Consult parent waiting for its ordered roster to be materialized.
///
/// The parent handle already fixes the ancestry of every future child. The
/// template still contains no child identity, private intent, or custody
/// state. A later application-owned admission step must consume this exact
/// phase rather than allocate another parent.
#[must_use = "materialize this exact roster beneath its signed parent"]
pub(super) struct ResidentConsultationRosterPending<FreshChild> {
    parent: SignedConsultationParentStart,
    template: ConsultationRosterTemplate<FreshChild>,
}

impl<FreshChild> ResidentConsultationRosterPending<FreshChild> {
    /// Join the declarative roster to the parent identity created before it.
    pub(super) const fn new(
        parent: SignedConsultationParentStart,
        template: ConsultationRosterTemplate<FreshChild>,
    ) -> Self {
        Self { parent, template }
    }

    /// Borrow the signed parent while child admission remains unavailable.
    pub(super) fn parent(&self) -> &SignedConsultationParentStart {
        &self.parent
    }

    /// Return the number of fresh child requests still needing identities.
    pub(super) fn fresh_child_count(&self) -> usize {
        self.template.fresh_child_count()
    }

    /// Materialize the roster once the application supplies child admission.
    pub(super) fn prepare<Child, Failure, AllocateChild, ObservationSign>(
        self,
        mut allocate_child: AllocateChild,
        sign_observation: ObservationSign,
    ) -> Result<
        ResidentPreparedConsultationStart<Child>,
        ResidentConsultationRosterPreparationFailure<FreshChild, Child, Failure>,
    >
    where
        AllocateChild: FnMut(
            (
                &AuthorityStoreCoordinate,
                &ResidentRecordRefV1,
                &OperationHandleV1,
                &OperationLeaseV1,
            ),
            &FreshChild,
        ) -> Result<
            ResidentConsultationAllocatedChild<Child>,
            ResidentConsultationChildAllocationFailure<Child, Failure>,
        >,
        ObservationSign: FnOnce(
            &[u8],
            &pale_core::records::OperationObservationPreimageV1,
            &pale_core::records::OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        let Self { parent, template } = self;
        let roster = match ResidentConsultationRosterAssembly::from_template(template) {
            Ok(roster) => roster,
            Err(template) => {
                return Err(ResidentConsultationRosterPreparationFailure::Capacity {
                    parent,
                    template,
                });
            }
        };
        prepare_consultation_roster_from(
            parent,
            Vec::new(),
            roster,
            None,
            |parent, child, prior| {
                debug_assert!(prior.is_none());
                allocate_child(parent, child)
            },
            sign_observation,
        )
    }
}

impl<FreshChild> ResidentConsultationRosterAssembly<FreshChild> {
    fn from_template(
        template: ConsultationRosterTemplate<FreshChild>,
    ) -> Result<Self, ConsultationRosterTemplate<FreshChild>> {
        let mut materialized = Vec::new();
        if materialized
            .try_reserve_exact(template.slots.len())
            .is_err()
        {
            return Err(template);
        }
        Ok(Self {
            completion_policy: template.completion_policy,
            opened_at: template.opened_at,
            deadline: template.deadline,
            limits: template.limits,
            materialized,
            remaining: template.slots.into(),
        })
    }

    fn finish(self) -> ConsultationStartIntentAdmission {
        debug_assert!(self.remaining.is_empty());
        ConsultationStartIntentAdmission {
            completion_policy: self.completion_policy,
            opened_at: self.opened_at,
            deadline: self.deadline,
            slots: self.materialized,
            limits: self.limits,
        }
    }
}

/// Parent preparation and ordered allocated child phases ready for custody.
#[must_use = "commit the prepared parent before starting its exact child prefix"]
pub(super) struct ResidentPreparedConsultationStart<Child> {
    pub(super) parent: Box<crate::resident::PreparedLocalOperationStart>,
    pub(super) children: Vec<Child>,
}

/// A complete prepared Consult waiting for parent custody.
///
/// The ordered child values already name their allocated identities. They stay
/// unavailable to the child starter until the parent reaches durable custody.
#[must_use = "commit the prepared parent or retain this exact schedule"]
pub(super) struct ResidentConsultationParentCommitPending<Child> {
    parent: Box<crate::resident::PreparedLocalOperationStart>,
    remaining: VecDeque<Child>,
    child_count: usize,
}

/// A parent commit failed while every allocated child remained scheduled.
#[must_use = "retain the parent continuation and exact child schedule"]
pub(super) enum ResidentConsultationParentCommitFailure<Child, Failure> {
    /// Prefix capacity was unavailable before any parent custody effect.
    Capacity(ResidentConsultationParentCommitPending<Child>),
    /// Parent custody failed and its owner-specific continuation is retained.
    Owner {
        remaining: VecDeque<Child>,
        child_count: usize,
        reason: Failure,
    },
}

/// The exact lower parent phase selected for one retry attempt.
pub(super) enum ResidentConsultationParentCommitInput<Failure> {
    /// No physical owner ran because bounded prefix capacity was unavailable.
    Prepared(Box<crate::resident::PreparedLocalOperationStart>),
    /// The physical owner ran and returned its only safe continuation.
    Continue(Failure),
}

/// A durable Consult parent with an exact started-child prefix.
///
/// `remaining` keeps the allocated suffix in roster order. Advancing a child
/// borrows the front value, so a refusal cannot consume it or allocate another
/// identity in its place.
#[must_use = "advance the exact child prefix or retain this schedule"]
pub(super) struct ResidentConsultationChildSchedule<Parent, Child, StartedChild> {
    parent: Parent,
    started: Vec<StartedChild>,
    remaining: VecDeque<Child>,
}

/// The next boundary after one bounded child-start attempt.
#[must_use = "continue the child schedule or initialize its collection"]
pub(super) enum ResidentConsultationChildAdvance<Parent, Child, StartedChild> {
    /// At least one allocated child still awaits durable start.
    Next(ResidentConsultationChildSchedule<Parent, Child, StartedChild>),
    /// The complete child prefix is durable and collection initialization may begin.
    InitializeCollection(ResidentConsultationCollectionInitialization<Parent, StartedChild>),
}

/// A child start failed without consuming the selected child or durable prefix.
#[must_use = "retry this exact child or preserve the schedule"]
pub(super) struct ResidentConsultationChildAdvanceFailure<Parent, Child, StartedChild, Failure> {
    pub(super) schedule: ResidentConsultationChildSchedule<Parent, Child, StartedChild>,
    pub(super) reason: Failure,
}

/// One child attempt either advances or retains the complete ordered schedule.
type ResidentConsultationChildAdvanceResult<Parent, Child, StartedChild, Failure> = Result<
    ResidentConsultationChildAdvance<Parent, Child, StartedChild>,
    ResidentConsultationChildAdvanceFailure<Parent, Child, StartedChild, Failure>,
>;

impl<FreshChild, Child, Failure>
    ResidentConsultationRosterPreparationFailure<FreshChild, Child, Failure>
{
    /// Retry the exact failed roster position without rebuilding its parent or prefix.
    ///
    /// The first callback receives the retained failure from that same slot.
    /// Later slots receive `None`. A caller that cannot safely resume the
    /// retained phase must return it unchanged.
    pub(super) fn continue_prepare<AllocateChild, ObservationSign>(
        self,
        allocate_child: AllocateChild,
        sign_observation: ObservationSign,
    ) -> Result<
        ResidentPreparedConsultationStart<Child>,
        ResidentConsultationRosterPreparationFailure<FreshChild, Child, Failure>,
    >
    where
        AllocateChild: FnMut(
            (
                &AuthorityStoreCoordinate,
                &ResidentRecordRefV1,
                &OperationHandleV1,
                &OperationLeaseV1,
            ),
            &FreshChild,
            Option<ResidentConsultationChildAllocationFailure<Child, Failure>>,
        ) -> Result<
            ResidentConsultationAllocatedChild<Child>,
            ResidentConsultationChildAllocationFailure<Child, Failure>,
        >,
        ObservationSign: FnOnce(
            &[u8],
            &pale_core::records::OperationObservationPreimageV1,
            &pale_core::records::OperationObservationDigest,
        ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
    {
        match self {
            Self::Capacity { parent, template } => {
                let roster = match ResidentConsultationRosterAssembly::from_template(template) {
                    Ok(roster) => roster,
                    Err(template) => return Err(Self::Capacity { parent, template }),
                };
                prepare_consultation_roster_from(
                    parent,
                    Vec::new(),
                    roster,
                    None,
                    allocate_child,
                    sign_observation,
                )
            }
            Self::Child {
                parent,
                children,
                roster,
                reason,
            } => prepare_consultation_roster_from(
                parent,
                children,
                *roster,
                Some(reason),
                allocate_child,
                sign_observation,
            ),
            failure @ (Self::Intent { .. } | Self::Parent { .. }) => Err(failure),
        }
    }
}

/// A durable parent and complete durable child prefix ready for collection work.
///
/// This type has no constructor outside the parent-commit and child-prefix
/// schedule. Collection initialization therefore cannot precede parent custody.
#[must_use = "initialize or recover the exact consultation collection"]
pub(super) struct ResidentConsultationCollectionInitialization<Parent, StartedChild> {
    pub(super) parent: Parent,
    pub(super) children: Vec<StartedChild>,
}

impl<Child> ResidentPreparedConsultationStart<Child> {
    /// Change only the private child phase representation while preserving order.
    pub(super) fn try_map_children<Mapped>(
        self,
        mut map: impl FnMut(Child) -> Mapped,
    ) -> Result<ResidentPreparedConsultationStart<Mapped>, Self> {
        let mut children = Vec::new();
        if children.try_reserve_exact(self.children.len()).is_err() {
            return Err(self);
        }
        children.extend(self.children.into_iter().map(&mut map));
        Ok(ResidentPreparedConsultationStart {
            parent: self.parent,
            children,
        })
    }

    /// Validate the prepared roster and retain its exact child order for custody.
    pub(super) fn into_parent_commit(
        self,
    ) -> Result<
        ResidentConsultationParentCommitPending<Child>,
        ResidentPreparedConsultationStart<Child>,
    > {
        let expected = self
            .parent
            .consultation_start_intent()
            .map(crate::resident::consultation::ConsultationStartIntent::fresh_child_count);
        if expected != Some(self.children.len()) {
            return Err(self);
        }
        let child_count = self.children.len();
        Ok(ResidentConsultationParentCommitPending {
            parent: self.parent,
            remaining: self.children.into(),
            child_count,
        })
    }
}

impl<Child> ResidentConsultationParentCommitPending<Child> {
    /// Commit the parent once, then expose only its already allocated child suffix.
    pub(super) fn commit<Parent, StartedChild, Failure, Commit>(
        self,
        commit: Commit,
    ) -> Result<
        ResidentConsultationChildAdvance<Parent, Child, StartedChild>,
        ResidentConsultationParentCommitFailure<Child, Failure>,
    >
    where
        Commit: FnOnce(crate::resident::PreparedLocalOperationStart) -> Result<Parent, Failure>,
    {
        let mut started = Vec::new();
        if started.try_reserve_exact(self.child_count).is_err() {
            return Err(ResidentConsultationParentCommitFailure::Capacity(self));
        }
        let Self {
            parent,
            remaining,
            child_count,
        } = self;
        let parent = match commit(*parent) {
            Ok(parent) => parent,
            Err(reason) => {
                return Err(ResidentConsultationParentCommitFailure::Owner {
                    remaining,
                    child_count,
                    reason,
                });
            }
        };
        let schedule = ResidentConsultationChildSchedule {
            parent,
            started,
            remaining,
        };
        Ok(schedule.classify())
    }
}

impl<Child, Failure> ResidentConsultationParentCommitFailure<Child, Failure> {
    /// Retry the exact parent boundary without changing the child schedule.
    ///
    /// Capacity failure still owns the prepared parent. An owner failure owns
    /// its lower continuation instead. The tagged callback input keeps that
    /// distinction explicit and prevents a retry from reconstructing parent
    /// meaning or reallocating any child.
    pub(super) fn continue_commit<Parent, StartedChild, Continue>(
        self,
        continue_owner: Continue,
    ) -> Result<
        ResidentConsultationChildAdvance<Parent, Child, StartedChild>,
        ResidentConsultationParentCommitFailure<Child, Failure>,
    >
    where
        Continue: FnOnce(ResidentConsultationParentCommitInput<Failure>) -> Result<Parent, Failure>,
    {
        match self {
            Self::Capacity(pending) => pending.commit(|prepared| {
                continue_owner(ResidentConsultationParentCommitInput::Prepared(Box::new(
                    prepared,
                )))
            }),
            Self::Owner {
                remaining,
                child_count,
                reason,
            } => {
                let mut started = Vec::new();
                if started.try_reserve_exact(child_count).is_err() {
                    return Err(Self::Owner {
                        remaining,
                        child_count,
                        reason,
                    });
                }
                let parent =
                    match continue_owner(ResidentConsultationParentCommitInput::Continue(reason)) {
                        Ok(parent) => parent,
                        Err(reason) => {
                            return Err(Self::Owner {
                                remaining,
                                child_count,
                                reason,
                            });
                        }
                    };
                Ok(ResidentConsultationChildSchedule {
                    parent,
                    started,
                    remaining,
                }
                .classify())
            }
        }
    }
}

impl<Parent, Child, StartedChild> ResidentConsultationChildSchedule<Parent, Child, StartedChild> {
    /// Borrow the durable parent without granting collection authority.
    pub(super) const fn parent(&self) -> &Parent {
        &self.parent
    }

    /// Borrow the durable child prefix in roster order.
    pub(super) fn started(&self) -> &[StartedChild] {
        &self.started
    }

    /// Borrow the exact next allocated child without removing it.
    pub(super) fn next_child(&self) -> Option<&Child> {
        self.remaining.front()
    }

    /// Advance one exact child. Failure retains the same front value.
    pub(super) fn advance<Failure, Start>(
        mut self,
        start: Start,
    ) -> ResidentConsultationChildAdvanceResult<Parent, Child, StartedChild, Failure>
    where
        Start: FnOnce(&Parent, &Child) -> Result<StartedChild, Failure>,
    {
        let Some(child) = self.remaining.front() else {
            return Ok(self.classify());
        };
        let started = match start(&self.parent, child) {
            Ok(started) => started,
            Err(reason) => {
                return Err(ResidentConsultationChildAdvanceFailure {
                    schedule: self,
                    reason,
                });
            }
        };
        let _ = self
            .remaining
            .pop_front()
            .expect("borrowed child remains at the schedule front");
        self.started.push(started);
        Ok(self.classify())
    }

    /// Advance one child while allowing a failed attempt to return its exact phase.
    ///
    /// This is used when a lower owner consumes an inert child phase and must
    /// replace it with a signed or custody continuation. The returned child is
    /// restored at the same roster position before the failure escapes.
    pub(super) fn advance_owned<Failure, Start>(
        mut self,
        start: Start,
    ) -> ResidentConsultationChildAdvanceResult<Parent, Child, StartedChild, Failure>
    where
        Start: FnOnce(&Parent, Child) -> Result<StartedChild, (Child, Failure)>,
    {
        let Some(child) = self.remaining.pop_front() else {
            return Ok(self.classify());
        };
        match start(&self.parent, child) {
            Ok(started) => self.started.push(started),
            Err((child, reason)) => {
                self.remaining.push_front(child);
                return Err(ResidentConsultationChildAdvanceFailure {
                    schedule: self,
                    reason,
                });
            }
        }
        Ok(self.classify())
    }

    fn classify(self) -> ResidentConsultationChildAdvance<Parent, Child, StartedChild> {
        if self.remaining.is_empty() {
            ResidentConsultationChildAdvance::InitializeCollection(
                ResidentConsultationCollectionInitialization {
                    parent: self.parent,
                    children: self.started,
                },
            )
        } else {
            ResidentConsultationChildAdvance::Next(self)
        }
    }
}

/// Why a signed Consult parent did not reach complete semantic preparation.
#[must_use = "preserve the exact parent and any allocated child prefix"]
pub(super) enum ResidentConsultationRosterPreparationFailure<FreshChild, Child, Failure> {
    /// Capacity for the bounded child prefix or roster projection was unavailable.
    Capacity {
        parent: SignedConsultationParentStart,
        template: ConsultationRosterTemplate<FreshChild>,
    },
    /// One child step failed; the parent, prefix, and remaining roster survive.
    Child {
        parent: SignedConsultationParentStart,
        children: Vec<Child>,
        roster: Box<ResidentConsultationRosterAssembly<FreshChild>>,
        reason: ResidentConsultationChildAllocationFailure<Child, Failure>,
    },
    /// The materialized roster did not match the signed parent.
    Intent {
        parent: SignedConsultationParentStart,
        children: Vec<Child>,
        admission: Box<ConsultationStartIntentAdmission>,
        reason: ConsultationStartIntentError,
    },
    /// Parent observation or deterministic preparation did not finish.
    Parent {
        children: Vec<Child>,
        intent: Box<ConsultationStartIntent>,
        reason: Box<RetainedLocalOperationStartFailure>,
    },
}

/// Failure before the parent and complete bounded roster were prepared.
#[must_use = "preserve the exact allocation-owner or roster continuation"]
pub(super) enum ResidentConsultationStartPreparationFailure<
    FreshChild,
    Child,
    ParentFailure,
    ChildFailure,
> {
    /// The existing parent allocation/input owner did not produce a signed phase.
    Parent {
        template: ConsultationRosterTemplate<FreshChild>,
        reason: ParentFailure,
    },
    /// The parent exists; roster preparation retains every completed child phase.
    Roster(ResidentConsultationRosterPreparationFailure<FreshChild, Child, ChildFailure>),
}

/// Allocate the parent first, then freeze fresh children beneath that identity.
///
/// The parent callback owns request lookup, identity allocation, lease custody,
/// contextual admission, and handle signing. It must return its own exact
/// continuation on failure. This function never retries either callback.
pub(super) fn prepare_consultation_start<
    FreshChild,
    Child,
    ParentFailure,
    ChildFailure,
    AllocateParent,
    AllocateChild,
    ObservationSign,
>(
    template: ConsultationRosterTemplate<FreshChild>,
    allocate_parent: AllocateParent,
    allocate_child: AllocateChild,
    sign_observation: ObservationSign,
) -> Result<
    ResidentPreparedConsultationStart<Child>,
    ResidentConsultationStartPreparationFailure<FreshChild, Child, ParentFailure, ChildFailure>,
>
where
    AllocateParent: FnOnce() -> Result<SignedConsultationParentStart, ParentFailure>,
    AllocateChild: FnMut(
        (
            &AuthorityStoreCoordinate,
            &ResidentRecordRefV1,
            &OperationHandleV1,
            &OperationLeaseV1,
        ),
        &FreshChild,
    ) -> Result<
        ResidentConsultationAllocatedChild<Child>,
        ResidentConsultationChildAllocationFailure<Child, ChildFailure>,
    >,
    ObservationSign: FnOnce(
        &[u8],
        &pale_core::records::OperationObservationPreimageV1,
        &pale_core::records::OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let parent = match allocate_parent() {
        Ok(parent) => parent,
        Err(reason) => {
            return Err(ResidentConsultationStartPreparationFailure::Parent { template, reason });
        }
    };
    ResidentConsultationRosterPending::new(parent, template)
        .prepare(allocate_child, sign_observation)
        .map_err(ResidentConsultationStartPreparationFailure::Roster)
}

/// Freeze a bounded roster against one already-signed Consult parent.
///
/// Evidence-resolved slots pass through unchanged. Fresh slots call the
/// existing child allocation/input owner in roster order. Each successful call
/// must return a plan captured from that exact child input. Only after every
/// fresh identity is frozen does this function attach the intent and continue
/// parent preparation. No database, anchor, dispatch, or collection port is
/// available here.
pub(super) fn prepare_consultation_roster<
    FreshChild,
    Child,
    Failure,
    AllocateChild,
    ObservationSign,
>(
    parent: SignedConsultationParentStart,
    template: ConsultationRosterTemplate<FreshChild>,
    mut allocate_child: AllocateChild,
    sign_observation: ObservationSign,
) -> Result<
    ResidentPreparedConsultationStart<Child>,
    ResidentConsultationRosterPreparationFailure<FreshChild, Child, Failure>,
>
where
    AllocateChild: FnMut(
        (
            &AuthorityStoreCoordinate,
            &ResidentRecordRefV1,
            &OperationHandleV1,
            &OperationLeaseV1,
        ),
        &FreshChild,
    ) -> Result<
        ResidentConsultationAllocatedChild<Child>,
        ResidentConsultationChildAllocationFailure<Child, Failure>,
    >,
    ObservationSign: FnOnce(
        &[u8],
        &pale_core::records::OperationObservationPreimageV1,
        &pale_core::records::OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let fresh_children = template.fresh_child_count();
    let mut children = Vec::new();
    if children.try_reserve_exact(fresh_children).is_err() {
        return Err(ResidentConsultationRosterPreparationFailure::Capacity { parent, template });
    }
    let roster = match ResidentConsultationRosterAssembly::from_template(template) {
        Ok(roster) => roster,
        Err(template) => {
            return Err(ResidentConsultationRosterPreparationFailure::Capacity {
                parent,
                template,
            });
        }
    };
    prepare_consultation_roster_from(
        parent,
        children,
        roster,
        None,
        |parent, child, prior| {
            debug_assert!(prior.is_none());
            allocate_child(parent, child)
        },
        sign_observation,
    )
}

fn prepare_consultation_roster_from<FreshChild, Child, Failure, AllocateChild, ObservationSign>(
    parent: SignedConsultationParentStart,
    mut children: Vec<Child>,
    mut roster: ResidentConsultationRosterAssembly<FreshChild>,
    mut prior_failure: Option<ResidentConsultationChildAllocationFailure<Child, Failure>>,
    mut allocate_child: AllocateChild,
    sign_observation: ObservationSign,
) -> Result<
    ResidentPreparedConsultationStart<Child>,
    ResidentConsultationRosterPreparationFailure<FreshChild, Child, Failure>,
>
where
    AllocateChild: FnMut(
        (
            &AuthorityStoreCoordinate,
            &ResidentRecordRefV1,
            &OperationHandleV1,
            &OperationLeaseV1,
        ),
        &FreshChild,
        Option<ResidentConsultationChildAllocationFailure<Child, Failure>>,
    ) -> Result<
        ResidentConsultationAllocatedChild<Child>,
        ResidentConsultationChildAllocationFailure<Child, Failure>,
    >,
    ObservationSign: FnOnce(
        &[u8],
        &pale_core::records::OperationObservationPreimageV1,
        &pale_core::records::OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    while let Some(slot) = roster.remaining.pop_front() {
        let source = match &slot.source {
            ConsultationRosterTemplateSource::EvidenceResolved { evidence } => {
                ConsultationStartIntentSource::EvidenceResolved {
                    evidence: evidence.clone(),
                }
            }
            ConsultationRosterTemplateSource::FreshChild { child } => {
                let allocated = match allocate_child(parent.parent(), child, prior_failure.take()) {
                    Ok(allocated) => allocated,
                    Err(reason) => {
                        roster.remaining.push_front(slot);
                        return Err(ResidentConsultationRosterPreparationFailure::Child {
                            parent,
                            children,
                            roster: Box::new(roster),
                            reason,
                        });
                    }
                };
                children.push(allocated.child);
                ConsultationStartIntentSource::FreshChild {
                    child: Box::new(allocated.plan),
                }
            }
        };
        roster.materialized.push(slot.with_source(source));
    }
    let admission = roster.finish();
    let intent = match ConsultationStartIntent::admit_signed(
        parent.parent().0,
        parent.parent().1,
        parent.parent().2,
        admission.clone(),
    ) {
        Ok(intent) => intent,
        Err(reason) => {
            return Err(ResidentConsultationRosterPreparationFailure::Intent {
                parent,
                children,
                admission: Box::new(admission),
                reason,
            });
        }
    };
    match parent.continue_with_intent(intent.clone(), sign_observation) {
        Ok(parent) => Ok(ResidentPreparedConsultationStart {
            parent: Box::new(parent),
            children,
        }),
        Err(reason) => Err(ResidentConsultationRosterPreparationFailure::Parent {
            children,
            intent: Box::new(intent),
            reason: Box::new(reason),
        }),
    }
}

/// Non-identity start meaning retained from one compiled application policy.
///
/// The fields cannot be populated from a request frame. The application owner
/// must first select the checkpoint profile, publication policy, lifetimes, and
/// physical bounds from its retained compiled policy.
#[derive(Clone)]
pub(super) struct ResidentCompiledOperationStartPolicy {
    checkpoint_profile: SupportedOperationCheckpointProfile,
    publication_policy: LocalOperationPublicationPolicy,
    maximum_record_bytes: usize,
    handle_expires_at: pale_core::records::UnixTimeV1,
    handle_retain_until: pale_core::records::UnixTimeV1,
    accumulator_limits: AccumulatorPreparationLimits,
}

impl ResidentCompiledOperationStartPolicy {
    /// Retain an application-selected compiled projection.
    ///
    /// This constructor accepts no operation identity, parent, route, provider,
    /// endpoint, key material, or request-supplied limit.
    pub(super) const fn from_compiled_projection(
        checkpoint_profile: SupportedOperationCheckpointProfile,
        publication_policy: LocalOperationPublicationPolicy,
        maximum_record_bytes: usize,
        handle_expires_at: pale_core::records::UnixTimeV1,
        handle_retain_until: pale_core::records::UnixTimeV1,
        accumulator_limits: AccumulatorPreparationLimits,
    ) -> Self {
        Self {
            checkpoint_profile,
            publication_policy,
            maximum_record_bytes,
            handle_expires_at,
            handle_retain_until,
            accumulator_limits,
        }
    }

    /// Return the application-selected per-record bound used before custody access.
    pub(super) const fn maximum_record_bytes(&self) -> usize {
        self.maximum_record_bytes
    }
}

/// Complete retained policy for starting one root Consult.
pub(super) struct ResidentStartConsultPolicy<FreshChild> {
    operation: ResidentCompiledOperationStartPolicy,
    roster: ConsultationRosterTemplate<FreshChild>,
}

impl<FreshChild> ResidentStartConsultPolicy<FreshChild> {
    /// Join compiled parent bounds to an ordered identity-free roster.
    pub(super) const fn from_compiled_projection(
        operation: ResidentCompiledOperationStartPolicy,
        roster: ConsultationRosterTemplate<FreshChild>,
    ) -> Self {
        Self { operation, roster }
    }

    /// Return the application-selected per-record bound.
    pub(super) const fn maximum_record_bytes(&self) -> usize {
        self.operation.maximum_record_bytes()
    }
}

/// Complete retained policy for starting one Delegate child.
#[derive(Clone)]
pub(super) struct ResidentStartDelegatePolicy {
    operation: ResidentCompiledOperationStartPolicy,
}

impl ResidentStartDelegatePolicy {
    /// Retain the compiled child-operation bounds.
    pub(super) const fn from_compiled_projection(
        operation: ResidentCompiledOperationStartPolicy,
    ) -> Self {
        Self { operation }
    }

    /// Return the application-selected per-record bound.
    pub(super) const fn maximum_record_bytes(&self) -> usize {
        self.operation.maximum_record_bytes()
    }
}

/// Exact admitted state from which a start input can be constructed.
///
/// The generation is supplied explicitly so the factory never resolves an
/// operation from ambient global state. Construction rechecks that it is the
/// session generation and that it owns the request's Package operation.
pub(super) struct ResidentOperationStartFactoryBasis<'a> {
    selector: &'a ResidentDeploymentAuthoritySelector,
    request: &'a CommonRequestEnvelopeV1,
    session: &'a AdmittedAmbientSession,
    current: &'a AmbientSessionAdmissionContext<'a>,
    generation: &'a AdmittedCompiledGeneration,
    handle_signing_authority: &'a SigningAuthorityBindingV1,
    checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
}

impl<'a> ResidentOperationStartFactoryBasis<'a> {
    /// Bind exact admitted request, session, generation, and signing witnesses.
    #[allow(clippy::too_many_arguments)]
    pub(super) const fn new(
        selector: &'a ResidentDeploymentAuthoritySelector,
        request: &'a CommonRequestEnvelopeV1,
        session: &'a AdmittedAmbientSession,
        current: &'a AmbientSessionAdmissionContext<'a>,
        generation: &'a AdmittedCompiledGeneration,
        handle_signing_authority: &'a SigningAuthorityBindingV1,
        checkpoint_authority: &'a ControlCheckpointSigningAuthorityAdmissionContextV1<'a>,
    ) -> Self {
        Self {
            selector,
            request,
            session,
            current,
            generation,
            handle_signing_authority,
            checkpoint_authority,
        }
    }
}

/// Build one root Consult input from an existing retained start allocation.
///
/// The callback is a lending boundary only: it receives the existing owner's
/// input and identity-free roster while all borrowed admission evidence is
/// alive. It cannot replace any identity or retained policy field.
pub(super) fn with_start_consult_owner_input<FreshChild, Output, UseOwner>(
    pending: &ResidentProcessRetainedOperationLease,
    basis: ResidentOperationStartFactoryBasis<'_>,
    policy: ResidentStartConsultPolicy<FreshChild>,
    use_owner: UseOwner,
) -> Result<Output, ResidentOperationStartFactoryError>
where
    UseOwner: for<'start> FnOnce(
        ResidentLocalOperationStartInput<'start>,
        ConsultationRosterTemplate<FreshChild>,
    ) -> Output,
{
    let ResidentStartConsultPolicy { operation, roster } = policy;
    with_operation_start_owner_input(
        pending,
        basis,
        operation,
        OperationClassV1::Consult,
        None,
        |_, _, _| Ok(()),
        |input, ()| use_owner(input, roster),
    )
}

/// Build one already-allocated Consult-child input beneath its signed parent.
///
/// The parent handle exists before this function can run. The child class is
/// restricted to Invoke or Consult, and every child identity still comes from
/// the retained request-first allocation owner.
pub(super) fn with_start_consult_child_owner_input<Output, UseOwner>(
    pending: &ResidentProcessRetainedOperationLease,
    basis: ResidentOperationStartFactoryBasis<'_>,
    policy: ResidentCompiledOperationStartPolicy,
    operation_class: OperationClassV1,
    parent: (
        &AuthorityStoreCoordinate,
        &ResidentRecordRefV1,
        &OperationHandleV1,
        &OperationLeaseV1,
    ),
    use_owner: UseOwner,
) -> Result<Output, ResidentOperationStartFactoryError>
where
    UseOwner: for<'start> FnOnce(ResidentLocalOperationStartInput<'start>) -> Output,
{
    if !matches!(
        operation_class,
        OperationClassV1::Invoke | OperationClassV1::Consult
    ) {
        return Err(ResidentOperationStartFactoryError::Association);
    }
    let (store, reference, handle, lease) = parent;
    if store != pending.fresh.allocation.store() {
        return Err(ResidentOperationStartFactoryError::Association);
    }
    pending
        .lease
        .verify_against_parent_operation(lease)
        .map_err(ResidentOperationStartFactoryError::Record)?;
    let parent = ResidentProtectedParentBasis {
        reference,
        handle,
        lease,
    };
    with_operation_start_owner_input(
        pending,
        basis,
        policy,
        operation_class,
        Some(parent),
        |_, _, _| Ok(()),
        |input, ()| use_owner(input),
    )
}

/// Build one Delegate child input from a parent protected before allocation.
///
/// The retained phase can exist only after the process host protected the exact
/// parent, allocated under that parent, admitted the narrowed child lease, and
/// retained the lease body. This factory cannot reorder those steps.
pub(super) fn with_start_delegate_owner_input<Output, UseOwner>(
    pending: &ResidentProcessRetainedDelegateOperationLease,
    basis: ResidentOperationStartFactoryBasis<'_>,
    policy: ResidentStartDelegatePolicy,
    use_owner: UseOwner,
) -> Result<Output, ResidentOperationStartFactoryError>
where
    UseOwner: for<'start> FnOnce(
        ResidentLocalOperationStartInput<'start>,
        PreparedLocalChildParent,
    ) -> Output,
{
    let ResidentProcessRetainedDelegateOperationLease { retained, parent } = pending;
    validate_factory_basis(retained, &basis, &policy.operation)?;
    if parent.store != *retained.fresh.allocation.store() {
        return Err(ResidentOperationStartFactoryError::Association);
    }
    retained
        .lease
        .verify_against_parent_operation(&parent.lease)
        .map_err(ResidentOperationStartFactoryError::Record)?;

    let (parent_ref, parent_handle) = parent.parent.operation();
    let parent_ref = parent_ref.clone();
    let parent_handle = parent_handle.clone();
    let parent_lease = parent.lease.clone();
    let protected_parent = parent.parent.duplicate_for_start_attempt();
    let parent_basis = ResidentProtectedParentBasis {
        reference: &parent_ref,
        handle: &parent_handle,
        lease: &parent_lease,
    };
    with_operation_start_owner_input(
        retained,
        basis,
        policy.operation,
        OperationClassV1::Delegate,
        Some(parent_basis),
        |_, _, _| Ok(()),
        |input, ()| use_owner(input, protected_parent),
    )
}

struct ResidentProtectedParentBasis<'a> {
    reference: &'a ResidentRecordRefV1,
    handle: &'a OperationHandleV1,
    lease: &'a OperationLeaseV1,
}

#[allow(clippy::too_many_arguments)]
fn with_operation_start_owner_input<Intent, Output, BuildIntent, UseOwner>(
    pending: &ResidentProcessRetainedOperationLease,
    basis: ResidentOperationStartFactoryBasis<'_>,
    policy: ResidentCompiledOperationStartPolicy,
    operation_class: OperationClassV1,
    parent: Option<ResidentProtectedParentBasis<'_>>,
    build_intent: BuildIntent,
    use_owner: UseOwner,
) -> Result<Output, ResidentOperationStartFactoryError>
where
    BuildIntent: FnOnce(
        &AuthorityStoreCoordinate,
        &ResidentRecordRefV1,
        &OperationHandlePreimageV1,
    ) -> Result<Intent, ResidentOperationStartFactoryError>,
    UseOwner: for<'start> FnOnce(ResidentLocalOperationStartInput<'start>, Intent) -> Output,
{
    validate_factory_basis(pending, &basis, &policy)?;
    let handle_schema = pale_contracts::record_schema_identity(OPERATION_HANDLE_V1_SCHEMA_ID)
        .map_err(ResidentOperationStartFactoryError::Contract)?;
    let observation_schema =
        pale_contracts::record_schema_identity(OPERATION_OBSERVATION_V1_SCHEMA_ID)
            .map_err(ResidentOperationStartFactoryError::Contract)?;
    let checkpoint_schema =
        pale_contracts::record_schema_identity(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID)
            .map_err(ResidentOperationStartFactoryError::Contract)?;
    let request_binding = basis
        .request
        .local_request_binding()
        .map_err(ResidentOperationStartFactoryError::Record)?;
    let issuer_key_id =
        ResidentKeyIdV1::parse(basis.handle_signing_authority.signer_key_id().to_owned())
            .map_err(ResidentOperationStartFactoryError::Record)?;
    let parent_reference = parent.as_ref().map(|parent| ParentOperationReferenceV1 {
        operation_handle_id: parent.handle.operation_handle_id().clone(),
        operation_handle_ref: parent.reference.clone(),
        operation_handle_sha256: parent.handle.canonical_operation_handle_sha256().clone(),
    });
    let handle = OperationHandlePreimageV1 {
        schema_id: handle_schema.schema.clone(),
        schema_sha256: handle_schema.schema_sha256.clone(),
        operation_handle_id: pending.fresh.allocation.handle_id().clone(),
        operation_class,
        parent_operation: parent_reference,
        request_binding: request_binding.clone(),
        operation_lease_ref: pending.fresh.allocation.lease_ref().clone(),
        operation_lease_sha256: pending.lease.operation_lease_digest().clone(),
        compiled_generation_sha256: basis.session.lease().compiled_generation_digest().clone(),
        activation_fence_sha256: basis.session.lease().activation_fence_digest().clone(),
        instance_sha256: basis.session.lease().instance_digest().clone(),
        created_at: basis.current.now,
        expires_at: policy.handle_expires_at,
        retain_until: policy.handle_retain_until,
        deployment_runtime_authority_ref: basis.session.runtime_authority_ref().clone(),
        deployment_runtime_authority_sha256: basis.session.runtime_authority_sha256().clone(),
        issuer_principal_id: basis.session.lease().issuer_principal_id().clone(),
        issuer_key_id,
        issuer_key_epoch: *basis.handle_signing_authority.signer_key_epoch(),
    };
    let parent_context = parent
        .as_ref()
        .map(|parent| ParentOperationAdmissionContextV1 {
            operation_handle_ref: parent.reference,
            operation_handle: parent.handle,
            operation_lease: parent.lease,
        });
    let context = OperationHandleAdmissionContextV1 {
        signed: SignedResidentAdmissionContextV1 {
            expected_schema_sha256: &handle_schema.schema_sha256,
            signing_authority: basis.handle_signing_authority,
        },
        request_binding: &request_binding,
        operation_lease_ref: pending.fresh.allocation.lease_ref(),
        operation_lease: &pending.lease,
        parent_operation: parent_context,
        compiled_generation_sha256: basis.session.lease().compiled_generation_digest(),
        activation_fence_sha256: basis.session.lease().activation_fence_digest(),
        instance_sha256: basis.session.lease().instance_digest(),
    };
    let intent = build_intent(
        pending.fresh.allocation.store(),
        pending.fresh.allocation.handle_ref(),
        &handle,
    )?;
    let input = ResidentLocalOperationStartInput {
        semantic: LocalOperationStartInput {
            checkpoint_profile: &policy.checkpoint_profile,
            request_ref: pending.fresh.allocation.request_ref().clone(),
            request: basis.request,
            handle_ref: pending.fresh.allocation.handle_ref().clone(),
            handle,
            context: &context,
            session: basis.session,
            current: basis.current,
            generation: basis.generation,
            publication_policy: policy.publication_policy.as_input(),
            checkpoint_authority: basis.checkpoint_authority,
            observation_ref: pending.fresh.allocation.observation_ref().clone(),
            observation_schema: observation_schema.schema_sha256,
        },
        accumulator_limits: policy.accumulator_limits,
        checkpoint: ResidentLocalOperationCheckpointInput {
            id: pending.fresh.allocation.checkpoint_id().clone(),
            reference: pending.fresh.allocation.checkpoint_ref().clone(),
            schema: &checkpoint_schema.schema_sha256,
            authority: basis.checkpoint_authority,
        },
        maximum_record_bytes: policy.maximum_record_bytes,
    };
    Ok(use_owner(input, intent))
}

fn validate_factory_basis(
    pending: &ResidentProcessRetainedOperationLease,
    basis: &ResidentOperationStartFactoryBasis<'_>,
    policy: &ResidentCompiledOperationStartPolicy,
) -> Result<(), ResidentOperationStartFactoryError> {
    let lease = basis.session.lease();
    let request_session = basis.request.session_admission();
    let store = pending.fresh.allocation.store();
    let exact_operation = basis
        .generation
        .operation(lease.repository_id(), basis.request.operation());
    if policy.maximum_record_bytes == 0
        || pending.fresh.maximum_record_bytes != policy.maximum_record_bytes
        || pending.fresh.selector != *basis.selector
        || pending.fresh.request_ref != *pending.fresh.allocation.request_ref()
        || pending.fresh.request_sha256 != *basis.request.canonical_request_sha256()
        || pending.fresh.allocation.lease_id() != pending.lease.operation_lease_id()
        || pending.lease.operation() != basis.request.operation()
        || request_session.session_admission_id != *lease.session_admission_id()
        || request_session.session_admission_lease_digest != *lease.session_admission_lease_digest()
        || basis.generation.record().compiled_generation_digest()
            != lease.compiled_generation_digest()
        || basis
            .current
            .generation
            .compiled_generation
            .compiled_generation_digest()
            != lease.compiled_generation_digest()
        || exact_operation.is_none()
        || !basis.selector.matches_current_session(basis.current)
        || !basis.selector.matches_admitted_session(basis.session)
        || !basis
            .selector
            .matches_signing_authority(basis.handle_signing_authority)
        || !basis
            .selector
            .matches_signing_authority(&basis.request.to_preimage().signing_authority_binding)
        || basis.checkpoint_authority.principal_id != &basis.selector.principal_id
        || basis.checkpoint_authority.control_store_id != store.store_id()
        || basis.checkpoint_authority.control_incarnation_id != store.incarnation_id()
        || policy.handle_expires_at <= basis.current.now
        || policy.handle_retain_until < policy.handle_expires_at
    {
        return Err(ResidentOperationStartFactoryError::Association);
    }
    pending
        .lease
        .verify_against_parent(lease)
        .map_err(ResidentOperationStartFactoryError::Record)
}

/// Why the application could not construct an existing start owner's input.
#[derive(Debug, thiserror::Error)]
pub(super) enum ResidentOperationStartFactoryError {
    /// Retained request, allocation, session, generation, signer, or policy differs.
    #[error("retained operation-start inputs do not form one admitted start")]
    Association,
    /// An embedded initial-v1 contract identity is unavailable.
    #[error("operation-start contract identity is unavailable: {0}")]
    Contract(#[from] pale_contracts::SchemaError),
    /// A core record or association is invalid.
    #[error("operation-start record is invalid: {0}")]
    Record(#[from] InvalidResidentRecordV1),
    /// The retained Consult roster does not belong to the parent start.
    #[error("consultation start intent is invalid: {0}")]
    ConsultationIntent(#[from] ConsultationStartIntentError),
}

#[cfg(test)]
#[path = "../../../tests/internal/resident/process_host/start_factories.rs"]
mod tests;
