//! Mechanical Package obligations and evidence-bound settlement.
//!
//! The compiler closes provider-neutral Package procedure before a renderer can
//! expose an operation. Evidence settlement then consumes four independent
//! coordinates; no average, acknowledgement, or provider-local success can
//! stand in for an owner-native consequence.

use std::collections::BTreeMap;

use pale_core::records::{
    CapabilityBinding, CapabilityIdV1, CapabilityRequirementIdV1, EffectClass, NonRootJsonPointer,
    OperationSelection, PackageResourceRefV1, ProofCeilingIdV1, ResolvedCapabilityFallbackV1,
    ResolvedCapabilityRequirementPolicyV1, ToolContractIdV1, ToolEvidenceContract,
    ToolOperationIdV1,
};

use super::{
    ProviderUnavailability, RepositorySemanticPlan, ResolvedOperationMechanics,
    ResolvedProviderSelection,
};

/// Exact Package subject of one compiled capability obligation.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ObligationSubject {
    /// Owner-qualified operation.
    pub operation: PackageResourceRefV1,
    /// Operation-local requirement identity.
    pub requirement: CapabilityRequirementIdV1,
}

/// Provider-neutral procedure required to discharge one capability obligation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProviderProcedureObligation {
    /// The Package requires a capability/effect pair without selecting a call recipe.
    CapabilityOnly,
    /// The Package selected one exact admitted recipe and its evidence ceiling.
    Recipe {
        /// Owner-qualified recipe identity.
        recipe: PackageResourceRefV1,
        /// Provider-neutral tool-contract identity.
        tool_contract: ToolContractIdV1,
        /// Operation within the tool contract.
        tool_operation: ToolOperationIdV1,
        /// Required response fields.
        required_fields: Vec<NonRootJsonPointer>,
        /// Strongest conclusion those fields may support.
        proof_ceiling: ProofCeilingIdV1,
        /// Whether exact evidence may refresh repository verification.
        repository_verification: RepositoryVerificationDisposition,
    },
}

/// Closed interpretation of a recipe's repository-verification flag.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepositoryVerificationDisposition {
    /// Evidence cannot refresh repository verification.
    NotEligible,
    /// Exact successful evidence may refresh repository verification.
    Eligible,
}

impl From<bool> for RepositoryVerificationDisposition {
    fn from(value: bool) -> Self {
        if value {
            Self::Eligible
        } else {
            Self::NotEligible
        }
    }
}

/// Compile-time result of joining Package policy to private provider selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CapabilityObligationDisposition {
    /// Private configuration selected a provider; invocation evidence remains separate.
    Bound(CapabilityBinding),
    /// A required capability has no usable provider and closes the affected operation.
    Blocked {
        /// Exact reason the provider join failed.
        reason: ProviderUnavailability,
    },
    /// A required capability follows its declared reduced-outcome path.
    Degraded {
        /// Exact reason the primary provider join failed.
        reason: ProviderUnavailability,
        /// Qualified Package fallback.
        fallback: ResolvedCapabilityFallbackV1,
        /// Stable reduced-outcome identity.
        degraded_outcome: pale_core::records::DegradedOutcomeIdV1,
    },
    /// An optional capability follows its declared omission path.
    Omitted {
        /// Exact reason the optional provider join failed.
        reason: ProviderUnavailability,
        /// Qualified Package fallback.
        fallback: ResolvedCapabilityFallbackV1,
    },
}

/// One capability obligation after Package/private configuration closure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityObligation {
    /// Exact operation and requirement.
    pub subject: ObligationSubject,
    /// Provider-neutral capability.
    pub capability: CapabilityIdV1,
    /// Maximum effect permitted by Package meaning.
    pub effect: EffectClass,
    /// Exact provider-neutral procedure, when Package-authored.
    pub procedure: ProviderProcedureObligation,
    /// Private provider join and Package-authored unavailable path.
    pub disposition: CapabilityObligationDisposition,
    /// Post-invocation failure posture retained without interpretation.
    pub failure_posture: pale_core::records::FailurePosture,
}

/// Complete mechanical state of one Package operation before rendering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OperationObligationState {
    /// No private operation binding exists, so no surface may be emitted.
    Unbound,
    /// A bound operation has no Package-owned capability requirements.
    SemanticOnly {
        /// Exact private exposure selection.
        selection: OperationSelection,
    },
    /// A bound operation carries one or more exact capability obligations.
    CapabilityBearing {
        /// Exact private exposure selection.
        selection: OperationSelection,
        /// Canonically ordered non-empty requirement closure.
        capabilities: Vec<CapabilityObligation>,
    },
}

impl OperationObligationState {
    /// Borrow the private selection carried only by bound states.
    #[must_use]
    pub fn selection(&self) -> Option<&OperationSelection> {
        match self {
            Self::Unbound => None,
            Self::SemanticOnly { selection } | Self::CapabilityBearing { selection, .. } => {
                Some(selection)
            }
        }
    }

    /// Borrow the exact capability closure carried by a capability-bearing state.
    #[must_use]
    pub fn capabilities(&self) -> &[CapabilityObligation] {
        match self {
            Self::CapabilityBearing { capabilities, .. } => capabilities,
            Self::Unbound | Self::SemanticOnly { .. } => &[],
        }
    }

    /// Return whether the bound operation is eligible for model discovery.
    #[must_use]
    pub fn is_model_facing(&self) -> bool {
        self.selection()
            .is_some_and(OperationSelection::user_selectable)
    }
}

/// One operation and the exact mechanical state a renderer must consume.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationObligation {
    /// Owner-qualified operation identity.
    pub operation: PackageResourceRefV1,
    /// Closed unbound, semantic-only, or capability-bearing state.
    pub state: OperationObligationState,
}

/// Mechanical obligations for one repository, bound by profile order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryObligationClosure {
    /// Zero-based index into the effective profile's canonical repository list.
    pub repository_index: usize,
    /// Canonically ordered complete operation obligations.
    pub operations: Vec<OperationObligation>,
}

impl RepositoryObligationClosure {
    /// Borrow one operation's compiled obligation by exact owner-qualified identity.
    #[must_use]
    pub fn operation(&self, operation: &PackageResourceRefV1) -> Option<&OperationObligation> {
        self.operations
            .binary_search_by(|candidate| candidate.operation.cmp(operation))
            .ok()
            .map(|index| &self.operations[index])
    }
}

/// Complete pre-render mechanical obligation closure.
///
/// The law below maps the ordered semantic-plan sequence to one subject-sorted
/// closure at each effective-profile repository index. It reads typed semantic
/// records only; Package Markdown is not an input to the mechanical result. In
/// that law, `P` is the ordered plan sequence and `p_i` is plan `i`. The
/// `close` operator returns that plan's complete obligation set. The indexed
/// result supplies the matching canonical closure required by `render`.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/package-obligation-closure.display.mathml"
))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MechanicalObligationClosure {
    /// One closure per effective-profile repository, in canonical profile order.
    pub repositories: Vec<RepositoryObligationClosure>,
}

/// Package mechanical closure cannot be constructed without exact admitted data.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum MechanicalObligationError {
    /// Two selected recipe records claim the same owner-qualified identity.
    #[error("duplicate selected recipe `{recipe}` in repository {repository_index}")]
    DuplicateRecipe {
        /// Effective-profile repository index.
        repository_index: usize,
        /// Repeated recipe identity.
        recipe: PackageResourceRefV1,
    },
    /// Two semantic-plan operations claim the same owner-qualified identity.
    #[error("duplicate operation `{operation}` in repository {repository_index}")]
    DuplicateOperation {
        /// Effective-profile repository index.
        repository_index: usize,
        /// Repeated operation identity.
        operation: PackageResourceRefV1,
    },
    /// A capability requirement names a recipe absent from the semantic plan.
    #[error(
        "operation `{operation}` requirement `{requirement}` references missing recipe `{recipe}`"
    )]
    MissingRecipe {
        /// Owning operation.
        operation: PackageResourceRefV1,
        /// Operation-local requirement.
        requirement: CapabilityRequirementIdV1,
        /// Missing recipe.
        recipe: PackageResourceRefV1,
    },
    /// Recipe and requirement capability/effect contracts disagree.
    #[error(
        "operation `{operation}` requirement `{requirement}` disagrees with recipe `{recipe}`"
    )]
    RecipeMismatch {
        /// Owning operation.
        operation: PackageResourceRefV1,
        /// Operation-local requirement.
        requirement: CapabilityRequirementIdV1,
        /// Conflicting recipe.
        recipe: PackageResourceRefV1,
    },
    /// A capability-bearing mechanics state contains no requirements.
    #[error("operation `{operation}` is capability-bearing but contains no requirements")]
    EmptyCapabilityBearingOperation {
        /// Owning operation.
        operation: PackageResourceRefV1,
    },
}

/// Compile repository semantic plans into a deterministic pre-render closure.
///
/// Only the affected requirement follows a blocked, degraded, or omitted path;
/// provider absence never invalidates an unrelated operation or repository.
///
/// # Errors
///
/// Returns [`MechanicalObligationError`] for missing/duplicate recipes,
/// capability/effect disagreement, or an impossible empty capability-bearing
/// operation.
pub fn compile_package_obligations(
    plans: &[RepositorySemanticPlan],
) -> Result<MechanicalObligationClosure, MechanicalObligationError> {
    let repositories = plans
        .iter()
        .enumerate()
        .map(|(repository_index, plan)| compile_repository_obligations(repository_index, plan))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(MechanicalObligationClosure { repositories })
}

fn compile_repository_obligations(
    repository_index: usize,
    plan: &RepositorySemanticPlan,
) -> Result<RepositoryObligationClosure, MechanicalObligationError> {
    let recipes = plan.recipes.iter().try_fold(
        BTreeMap::new(),
        |mut index, recipe| -> Result<_, MechanicalObligationError> {
            if index.insert(&recipe.resource, &recipe.procedure).is_some() {
                return Err(MechanicalObligationError::DuplicateRecipe {
                    repository_index,
                    recipe: recipe.resource.clone(),
                });
            }
            Ok(index)
        },
    )?;

    let mut operations = Vec::with_capacity(plan.operations.len());
    for operation in &plan.operations {
        let state = match &operation.mechanics {
            ResolvedOperationMechanics::Unbound => OperationObligationState::Unbound,
            ResolvedOperationMechanics::SemanticOnly { selection } => {
                OperationObligationState::SemanticOnly {
                    selection: selection.clone(),
                }
            }
            ResolvedOperationMechanics::CapabilityBearing {
                selection,
                capabilities: requirements,
            } => {
                if requirements.is_empty() {
                    return Err(MechanicalObligationError::EmptyCapabilityBearingOperation {
                        operation: operation.record.id.clone(),
                    });
                }
                let capabilities = requirements
                    .iter()
                    .map(|requirement| {
                        let subject = ObligationSubject {
                            operation: operation.record.id.clone(),
                            requirement: requirement.requirement_id.clone(),
                        };
                        let procedure = match &requirement.recipe {
                            None => ProviderProcedureObligation::CapabilityOnly,
                            Some(recipe) => {
                                let procedure = recipes.get(recipe).ok_or_else(|| {
                                    MechanicalObligationError::MissingRecipe {
                                        operation: subject.operation.clone(),
                                        requirement: subject.requirement.clone(),
                                        recipe: recipe.clone(),
                                    }
                                })?;
                                if procedure.capability != requirement.capability_id
                                    || procedure.effect_class != requirement.effect_class
                                {
                                    return Err(MechanicalObligationError::RecipeMismatch {
                                        operation: subject.operation.clone(),
                                        requirement: subject.requirement.clone(),
                                        recipe: recipe.clone(),
                                    });
                                }
                                let ToolEvidenceContract {
                                    required_fields,
                                    proof_ceiling,
                                    repository_verifier,
                                } = &procedure.evidence;
                                ProviderProcedureObligation::Recipe {
                                    recipe: recipe.clone(),
                                    tool_contract: procedure.tool_contract.clone(),
                                    tool_operation: procedure.tool_operation.clone(),
                                    required_fields: required_fields.clone(),
                                    proof_ceiling: proof_ceiling.clone(),
                                    repository_verification: (*repository_verifier).into(),
                                }
                            }
                        };
                        Ok(CapabilityObligation {
                            subject,
                            capability: requirement.capability_id.clone(),
                            effect: requirement.effect_class,
                            procedure,
                            disposition: obligation_disposition(
                                &requirement.provider,
                                &requirement.policy,
                            ),
                            failure_posture: requirement.failure_posture,
                        })
                    })
                    .collect::<Result<Vec<_>, MechanicalObligationError>>()?;
                let mut capabilities = capabilities;
                capabilities.sort_by(|left, right| left.subject.cmp(&right.subject));
                OperationObligationState::CapabilityBearing {
                    selection: selection.clone(),
                    capabilities,
                }
            }
        };
        operations.push(OperationObligation {
            operation: operation.record.id.clone(),
            state,
        });
    }
    operations.sort_by(|left, right| left.operation.cmp(&right.operation));
    if let Some(duplicate) = operations
        .windows(2)
        .find(|pair| pair[0].operation == pair[1].operation)
    {
        return Err(MechanicalObligationError::DuplicateOperation {
            repository_index,
            operation: duplicate[0].operation.clone(),
        });
    }
    Ok(RepositoryObligationClosure {
        repository_index,
        operations,
    })
}

pub(crate) fn obligation_disposition(
    provider: &ResolvedProviderSelection,
    policy: &ResolvedCapabilityRequirementPolicyV1,
) -> CapabilityObligationDisposition {
    match (provider, policy) {
        (ResolvedProviderSelection::Bound(binding), _) => {
            CapabilityObligationDisposition::Bound(binding.clone())
        }
        (
            ResolvedProviderSelection::Unbound(reason),
            ResolvedCapabilityRequirementPolicyV1::RequiredBlock,
        ) => CapabilityObligationDisposition::Blocked { reason: *reason },
        (
            ResolvedProviderSelection::Unbound(reason),
            ResolvedCapabilityRequirementPolicyV1::RequiredDegrade {
                fallback,
                degraded_outcome,
            },
        ) => CapabilityObligationDisposition::Degraded {
            reason: *reason,
            fallback: fallback.clone(),
            degraded_outcome: degraded_outcome.clone(),
        },
        (
            ResolvedProviderSelection::Unbound(reason),
            ResolvedCapabilityRequirementPolicyV1::Optional { fallback },
        ) => CapabilityObligationDisposition::Omitted {
            reason: *reason,
            fallback: fallback.clone(),
        },
    }
}

mod evidence;
mod settlement;

pub use evidence::{
    validate_evidence_coordinates, AdmittedEvidenceCoordinates, CoverageCoordinate, CoverageGap,
    EvidenceCoordinate, EvidenceCoordinateError, EvidenceCoordinateState, EvidenceCoordinates,
    FreshnessCoordinate, FreshnessLayer, FreshnessState, GroundingAssertion, GroundingCoordinate,
    GroundingKind, IndependenceCoordinate, IndependenceCoupling, IndependenceRelation,
    ProviderEvidenceCoordinate, RepeatabilityCoordinate, RepeatabilityMethod,
    MAX_EVIDENCE_AXIS_MEMBERS,
};
pub use settlement::{
    settle_obligation, EvidenceSettlementInput, EvidenceSettlementState, NativeVerification,
    OwnerPolicyRequirement, QualitativeFrontier, ResidualAction, SettlementDecision,
    SettlementTarget,
};

#[cfg(test)]
#[path = "../../tests/internal/semantic/obligation.rs"]
mod tests;
