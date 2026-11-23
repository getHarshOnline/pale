//! Structurally validated Package occurrence claims and narrowing-only derivation.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    CapabilityIdV1, CompiledGenerationDigest, EffectClass, ImmutableRecordReferenceV1,
    PackageReleaseV1, PackageResourceRefV1, WorkspaceResourceId,
};

use super::AdmittedNormativePackReference;

/// Independently narrowed axes inherited by a child Package occurrence.
///
/// A child ceiling `X` is lawful exactly when every set is a subset of its
/// parent and every named resource budget is no greater than the parent's.
/// Axis symbols are operations `O`, preconditions `Q`, capabilities `C`,
/// effects `E`, evidence `V`, rights `R`, authority `A`, disclosure `D`,
/// model use `M`, propagation `G`, fallbacks `F`, and budgets `B`.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/package-occurrence-narrowing.display.mathml"
))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OccurrenceCeilings {
    /// Operations available to the occurrence.
    pub operations: BTreeSet<PackageResourceRefV1>,
    /// Preconditions the occurrence may rely upon.
    pub preconditions: BTreeSet<WorkspaceResourceId>,
    /// Capabilities the occurrence may request.
    pub capabilities: BTreeSet<CapabilityIdV1>,
    /// Effects the occurrence may request.
    pub effects: BTreeSet<EffectClass>,
    /// Evidence contracts the occurrence may consume.
    pub evidence: BTreeSet<WorkspaceResourceId>,
    /// Rights independently granted for this occurrence.
    pub rights: BTreeSet<WorkspaceResourceId>,
    /// Authority claims independently admitted for this occurrence.
    pub authority: BTreeSet<WorkspaceResourceId>,
    /// Information-disclosure classes available to this occurrence.
    pub disclosure: BTreeSet<WorkspaceResourceId>,
    /// Model-use classes available to this occurrence.
    pub model_use: BTreeSet<WorkspaceResourceId>,
    /// Propagation modes available to derived occurrences.
    pub propagation: BTreeSet<WorkspaceResourceId>,
    /// Resource-specific finite budgets.
    pub budgets: BTreeMap<WorkspaceResourceId, u64>,
    /// Package-authored fallback procedures retained by the occurrence.
    pub fallbacks: BTreeSet<PackageResourceRefV1>,
}

impl OccurrenceCeilings {
    /// Test whether this ceiling is a narrowing of `parent` on every axis.
    #[must_use]
    pub fn is_narrowing_of(&self, parent: &Self) -> bool {
        self.operations.is_subset(&parent.operations)
            && self.preconditions.is_subset(&parent.preconditions)
            && self.capabilities.is_subset(&parent.capabilities)
            && self.effects.is_subset(&parent.effects)
            && self.evidence.is_subset(&parent.evidence)
            && self.rights.is_subset(&parent.rights)
            && self.authority.is_subset(&parent.authority)
            && self.disclosure.is_subset(&parent.disclosure)
            && self.model_use.is_subset(&parent.model_use)
            && self.propagation.is_subset(&parent.propagation)
            && self.fallbacks.is_subset(&parent.fallbacks)
            && self.budgets.iter().all(|(resource, child)| {
                parent
                    .budgets
                    .get(resource)
                    .is_some_and(|parent| child <= parent)
            })
    }
}

/// Exact, body-free context of one Package occurrence.
///
/// Optional organization/service/relationship coordinates remain absent unless
/// their owners supplied exact references. Other owner facts are likewise
/// retained by immutable reference rather than copied into Package custody.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageOccurrenceContext {
    /// Principal fact governing the occurrence.
    pub principal: ImmutableRecordReferenceV1,
    /// Controller fact governing the occurrence.
    pub controller: ImmutableRecordReferenceV1,
    /// Optional organization fact.
    pub organization: Option<ImmutableRecordReferenceV1>,
    /// Optional directional service-contract fact.
    pub service_contract: Option<ImmutableRecordReferenceV1>,
    /// Optional relationship fact.
    pub relationship: Option<ImmutableRecordReferenceV1>,
    /// Project/repository/resource scope.
    pub resource_scope: ImmutableRecordReferenceV1,
    /// Stable responsibility/role coordinate.
    pub responsibility_role: ImmutableRecordReferenceV1,
    /// Occupant/participant/software-agent coordinate.
    pub participant: ImmutableRecordReferenceV1,
    /// Origin/interface coordinate.
    pub origin_interface: ImmutableRecordReferenceV1,
    /// Execution-path/terminal-leaf coordinate.
    pub execution_path: ImmutableRecordReferenceV1,
    /// Optional provider/session coordinate for capability-bearing use.
    pub provider_session: Option<ImmutableRecordReferenceV1>,
    /// Purpose and maximum effect coordinate.
    pub purpose: WorkspaceResourceId,
    /// Exact authority frontier.
    pub authority: ImmutableRecordReferenceV1,
    /// Exact maximum effect for this occurrence.
    pub effect: EffectClass,
    /// Exact source/evidence frontier.
    pub evidence_frontier: ImmutableRecordReferenceV1,
    /// Exact disclosure ceiling.
    pub disclosure: ImmutableRecordReferenceV1,
    /// Exact model-use ceiling.
    pub model_use: ImmutableRecordReferenceV1,
    /// Exact propagation ceiling.
    pub propagation: ImmutableRecordReferenceV1,
    /// Exact budget frontier.
    pub budget: ImmutableRecordReferenceV1,
    /// Exact fallback frontier.
    pub fallback: ImmutableRecordReferenceV1,
    /// Exact settlement frontier.
    pub settlement: ImmutableRecordReferenceV1,
}

/// How a child occurrence obtained its exact context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OccurrenceContextDerivation {
    /// Root occurrence has no parent context.
    Root,
    /// Child retains its parent's exact context.
    Preserved,
    /// Child proposes a changed context under an owner-supplied delegation reference.
    ///
    /// The reference is opaque until an owner-native verifier authenticates it,
    /// so [`derive_child_occurrence`] currently rejects this disposition.
    Delegated {
        /// Body-free, as-yet-unverified delegation reference.
        evidence: ImmutableRecordReferenceV1,
    },
}

/// One occurrence claim selecting a Package release in a compiled generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageOccurrence {
    /// Immutable identity of this occurrence record.
    pub occurrence: ImmutableRecordReferenceV1,
    /// Exact immutable Package release.
    pub release: PackageReleaseV1,
    /// Compiled generation containing this occurrence.
    pub generation: CompiledGenerationDigest,
    /// Supplied body-free owner-context coordinates.
    pub context: PackageOccurrenceContext,
    /// Explicit root, preserved, or delegated context relation.
    pub context_derivation: OccurrenceContextDerivation,
    /// Structurally admitted normative-Pack selections and use rights.
    pub normative_packs: Vec<AdmittedNormativePackReference>,
    /// Effective narrowing axes.
    pub ceilings: OccurrenceCeilings,
    /// Exact parent occurrence for derived children.
    pub parent: Option<ImmutableRecordReferenceV1>,
}

/// Package occurrence claim admitted under local root or child derivation law.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedPackageOccurrence(PackageOccurrence);

impl AdmittedPackageOccurrence {
    /// Borrow the structurally admitted occurrence claim.
    #[must_use]
    pub fn record(&self) -> &PackageOccurrence {
        &self.0
    }
}

/// A child occurrence violates inherited release or ceiling law.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ChildOccurrenceError {
    /// A child silently replaced the governing Package release.
    #[error("child Package occurrence must retain the parent's exact release")]
    ReleaseSubstitution,
    /// One or more child axes widened the parent ceiling.
    #[error("child Package occurrence widens an inherited ceiling")]
    CeilingWidening,
    /// Claimed parent reference is not the exact parent occurrence.
    #[error("child Package occurrence carries a different parent reference")]
    ParentMismatch,
    /// Child silently crossed a compiled-generation boundary.
    #[error("child Package occurrence must retain the parent's compiled generation")]
    GenerationSubstitution,
    /// Preserved context was claimed while exact context coordinates changed.
    #[error("preserved child context must equal the parent's exact context")]
    PreservedContextMismatch,
    /// A body-free delegation reference cannot prove a changed context.
    #[error("delegated child context requires independently verified owner-native evidence")]
    UnverifiedDelegation,
    /// A child occurrence cannot claim root context derivation.
    #[error("child Package occurrence must name preserved or delegated context")]
    RootContextOnChild,
    /// Child selected a normative-Pack admission absent from the parent.
    #[error("child Package occurrence widens the parent's normative-Pack admissions")]
    NormativePackWidening,
    /// Child Pack admissions are not strictly ordered and duplicate-free.
    #[error("child normative-Pack admissions are not in canonical strict order")]
    NonCanonicalNormativePacks,
    /// Child context names an effect outside its narrowed ceiling.
    #[error("child Package occurrence context effect lies outside its ceiling")]
    EffectOutsideCeiling,
}

/// Root occurrence violates its parent/context relation.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum RootOccurrenceError {
    /// A root cannot name a parent occurrence.
    #[error("root Package occurrence cannot name a parent")]
    ParentPresent,
    /// A root must carry the root context disposition.
    #[error("root Package occurrence must use root context derivation")]
    NonRootContextDerivation,
    /// Root Pack admissions are not strictly ordered and duplicate-free.
    #[error("root normative-Pack admissions are not in canonical strict order")]
    NonCanonicalNormativePacks,
    /// Root context names an effect outside its ceiling.
    #[error("root Package occurrence context effect lies outside its ceiling")]
    EffectOutsideCeiling,
}

/// Structurally admit one root occurrence claim.
///
/// This validates local shape, Pack ordering, and the effect ceiling. It does
/// not authenticate the referenced release, generation, or owner coordinates,
/// and therefore implies neither materialization nor activation.
///
/// # Errors
///
/// Returns [`RootOccurrenceError`] when the record names a parent or a
/// non-root context derivation.
pub fn admit_root_occurrence(
    occurrence: PackageOccurrence,
) -> Result<AdmittedPackageOccurrence, RootOccurrenceError> {
    if occurrence.parent.is_some() {
        return Err(RootOccurrenceError::ParentPresent);
    }
    if occurrence.context_derivation != OccurrenceContextDerivation::Root {
        return Err(RootOccurrenceError::NonRootContextDerivation);
    }
    if !normative_packs_are_canonical(&occurrence.normative_packs) {
        return Err(RootOccurrenceError::NonCanonicalNormativePacks);
    }
    if !occurrence
        .ceilings
        .effects
        .contains(&occurrence.context.effect)
    {
        return Err(RootOccurrenceError::EffectOutsideCeiling);
    }
    Ok(AdmittedPackageOccurrence(occurrence))
}

/// Structurally admit a child occurrence only when every inherited axis narrows.
///
/// # Errors
///
/// Returns [`ChildOccurrenceError`] for release substitution, an incorrect
/// parent reference, generation substitution, dishonest context derivation, or
/// any widened operation, precondition, capability, effect, evidence, right,
/// authority, disclosure, model-use, propagation, budget, or fallback axis.
pub fn derive_child_occurrence(
    parent: &AdmittedPackageOccurrence,
    child: PackageOccurrence,
) -> Result<AdmittedPackageOccurrence, ChildOccurrenceError> {
    let parent = parent.record();
    if child.release != parent.release {
        return Err(ChildOccurrenceError::ReleaseSubstitution);
    }
    if child.parent.as_ref() != Some(&parent.occurrence) {
        return Err(ChildOccurrenceError::ParentMismatch);
    }
    if child.generation != parent.generation {
        return Err(ChildOccurrenceError::GenerationSubstitution);
    }
    match &child.context_derivation {
        OccurrenceContextDerivation::Root => return Err(ChildOccurrenceError::RootContextOnChild),
        OccurrenceContextDerivation::Preserved if child.context != parent.context => {
            return Err(ChildOccurrenceError::PreservedContextMismatch)
        }
        OccurrenceContextDerivation::Delegated { .. } => {
            return Err(ChildOccurrenceError::UnverifiedDelegation)
        }
        OccurrenceContextDerivation::Preserved => {}
    }
    if !child.ceilings.is_narrowing_of(&parent.ceilings) {
        return Err(ChildOccurrenceError::CeilingWidening);
    }
    if !normative_packs_are_canonical(&child.normative_packs) {
        return Err(ChildOccurrenceError::NonCanonicalNormativePacks);
    }
    if !child.ceilings.effects.contains(&child.context.effect) {
        return Err(ChildOccurrenceError::EffectOutsideCeiling);
    }
    if !child.normative_packs.iter().all(|child_pack| {
        parent
            .normative_packs
            .iter()
            .any(|parent_pack| child_pack == parent_pack)
    }) {
        return Err(ChildOccurrenceError::NormativePackWidening);
    }
    Ok(AdmittedPackageOccurrence(child))
}

fn normative_packs_are_canonical(packs: &[AdmittedNormativePackReference]) -> bool {
    packs
        .windows(2)
        .all(|pair| pair[0].record().admission < pair[1].record().admission)
}
