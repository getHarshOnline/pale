//! Canonical binding projection for portable workspace derivation results.
//!
//! The projection lives beside the API-owned result algebra so protocol leaves
//! cannot acquire a second spelling of subjects, facts, provenance, rules, or
//! causal proof structure.

use serde::Serialize;
use serde_json::{json, Map, Value};

use super::derivation::{
    PortableWorkspaceCausalExplanation, PortableWorkspaceCausalStep, PortableWorkspaceChangeKind,
    PortableWorkspaceChangeRoot, PortableWorkspaceChangeRootPage, PortableWorkspaceDerivationRule,
    PortableWorkspaceExplanation, PortableWorkspaceExplanationFrontier,
    PortableWorkspaceExplanationState, PortableWorkspaceFact, PortableWorkspaceImpactPage,
    PortableWorkspaceImpactState, PortableWorkspaceProvenance, PortableWorkspaceSubject,
    PortableWorkspaceSubjectComparison, PortableWorkspaceSubjectSelector,
    PortableWorkspaceSubjectValues, PortableWorkspaceValueComparison,
};
use crate::{AuthoringInstancePathSegment, AuthoringOccurrenceRole, AuthoringSourceLocation};

#[derive(Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(super) enum SubjectProjection {
    Source {
        source_id: String,
    },
    Component {
        component_id: String,
    },
    Module {
        module_id: String,
    },
    Declaration {
        owner_module_id: String,
        namespace_id: String,
        declaration_id: String,
    },
    Import {
        importer_module_id: String,
        upstream_module_id: String,
        upstream_release_id: String,
        export_id: String,
        source_namespace_id: String,
        local_namespace_id: String,
    },
    Extension {
        owner_module_id: String,
        extension_namespace_id: String,
        target_namespace_id: String,
        target_declaration_id: String,
    },
    Policy {
        owner_module_id: String,
        namespace_id: String,
        declaration_id: String,
    },
    ResponsibilityClosure,
    OrganizationalTranslationClosure,
    ExperienceComposition,
    Resource {
        owner_module_id: String,
        resource_id: String,
    },
    ResolutionLock,
    UniverseLock,
    Workspace {
        workspace_id: String,
    },
}

impl From<&PortableWorkspaceSubject> for SubjectProjection {
    fn from(value: &PortableWorkspaceSubject) -> Self {
        match value.selector() {
            PortableWorkspaceSubjectSelector::Source { source_id } => Self::Source { source_id },
            PortableWorkspaceSubjectSelector::Component { component_id } => {
                Self::Component { component_id }
            }
            PortableWorkspaceSubjectSelector::Module { module_id } => Self::Module { module_id },
            PortableWorkspaceSubjectSelector::Declaration {
                owner_module_id,
                namespace_id,
                declaration_id,
            } => Self::Declaration {
                owner_module_id,
                namespace_id,
                declaration_id,
            },
            PortableWorkspaceSubjectSelector::Import {
                importer_module_id,
                upstream_module_id,
                upstream_release_id,
                export_id,
                source_namespace_id,
                local_namespace_id,
            } => Self::Import {
                importer_module_id,
                upstream_module_id,
                upstream_release_id,
                export_id,
                source_namespace_id,
                local_namespace_id,
            },
            PortableWorkspaceSubjectSelector::Extension {
                owner_module_id,
                extension_namespace_id,
                target_namespace_id,
                target_declaration_id,
            } => Self::Extension {
                owner_module_id,
                extension_namespace_id,
                target_namespace_id,
                target_declaration_id,
            },
            PortableWorkspaceSubjectSelector::Policy {
                owner_module_id,
                namespace_id,
                declaration_id,
            } => Self::Policy {
                owner_module_id,
                namespace_id,
                declaration_id,
            },
            PortableWorkspaceSubjectSelector::ResponsibilityClosure => Self::ResponsibilityClosure,
            PortableWorkspaceSubjectSelector::OrganizationalTranslationClosure => {
                Self::OrganizationalTranslationClosure
            }
            PortableWorkspaceSubjectSelector::ExperienceComposition => Self::ExperienceComposition,
            PortableWorkspaceSubjectSelector::Resource {
                owner_module_id,
                resource_id,
            } => Self::Resource {
                owner_module_id,
                resource_id,
            },
            PortableWorkspaceSubjectSelector::ResolutionLock => Self::ResolutionLock,
            PortableWorkspaceSubjectSelector::UniverseLock => Self::UniverseLock,
            PortableWorkspaceSubjectSelector::Workspace { workspace_id } => {
                Self::Workspace { workspace_id }
            }
        }
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(super) enum FactProjection<'a> {
    SourceContent {
        source_path: &'a str,
        source_digest: &'a str,
        byte_len: u32,
    },
    Component {
        component_digest: &'a str,
    },
    Module {
        module_digest: &'a str,
    },
    Declaration {
        value_digest: &'a str,
    },
    ImportSurface {
        surface_digest: &'a str,
    },
    Extension {
        value_digest: &'a str,
    },
    EffectivePolicy {
        value_digest: &'a str,
    },
    ResponsibilityClosure {
        closure_digest: &'a str,
    },
    OrganizationalTranslationClosure {
        closure_digest: &'a str,
    },
    ExperienceComposition {
        source_digest: &'a str,
    },
    Resource {
        value_digest: &'a str,
    },
    ResolutionLock {
        resolution_digest: &'a str,
    },
    UniverseLock {
        universe_digest: &'a str,
    },
    SemanticIr {
        workspace_semantic_ir_digest: &'a str,
    },
}

impl<'a> From<&'a PortableWorkspaceFact> for FactProjection<'a> {
    fn from(value: &'a PortableWorkspaceFact) -> Self {
        match value {
            PortableWorkspaceFact::SourceContent {
                source_path,
                source_digest,
                byte_len,
            } => Self::SourceContent {
                source_path,
                source_digest,
                byte_len: *byte_len,
            },
            PortableWorkspaceFact::Component { component_digest } => {
                Self::Component { component_digest }
            }
            PortableWorkspaceFact::Module { module_digest } => Self::Module { module_digest },
            PortableWorkspaceFact::Declaration { value_digest } => {
                Self::Declaration { value_digest }
            }
            PortableWorkspaceFact::ImportSurface { surface_digest } => {
                Self::ImportSurface { surface_digest }
            }
            PortableWorkspaceFact::Extension { value_digest } => Self::Extension { value_digest },
            PortableWorkspaceFact::EffectivePolicy { value_digest } => {
                Self::EffectivePolicy { value_digest }
            }
            PortableWorkspaceFact::ResponsibilityClosure { closure_digest } => {
                Self::ResponsibilityClosure { closure_digest }
            }
            PortableWorkspaceFact::OrganizationalTranslationClosure { closure_digest } => {
                Self::OrganizationalTranslationClosure { closure_digest }
            }
            PortableWorkspaceFact::ExperienceComposition { source_digest } => {
                Self::ExperienceComposition { source_digest }
            }
            PortableWorkspaceFact::Resource { value_digest } => Self::Resource { value_digest },
            PortableWorkspaceFact::ResolutionLock { resolution_digest } => {
                Self::ResolutionLock { resolution_digest }
            }
            PortableWorkspaceFact::UniverseLock { universe_digest } => {
                Self::UniverseLock { universe_digest }
            }
            PortableWorkspaceFact::SemanticIr {
                workspace_semantic_ir_digest,
            } => Self::SemanticIr {
                workspace_semantic_ir_digest,
            },
        }
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(super) enum ProvenanceProjection<'a> {
    Source {
        source_id: &'a str,
    },
    ModuleRelease {
        module_id: &'a str,
        release_id: &'a str,
        module_digest: &'a str,
    },
    Import {
        importer_module_id: &'a str,
        upstream_module_id: &'a str,
        upstream_release_id: &'a str,
        export_id: &'a str,
        source_namespace_id: &'a str,
        local_namespace_id: &'a str,
    },
    Extension {
        owner_module_id: &'a str,
        extension_namespace_id: &'a str,
        target_namespace_id: &'a str,
        target_declaration_id: &'a str,
    },
    Compilation,
}

impl<'a> From<&'a PortableWorkspaceProvenance> for ProvenanceProjection<'a> {
    fn from(value: &'a PortableWorkspaceProvenance) -> Self {
        match value {
            PortableWorkspaceProvenance::Source { source_id } => Self::Source { source_id },
            PortableWorkspaceProvenance::ModuleRelease {
                module_id,
                release_id,
                module_digest,
            } => Self::ModuleRelease {
                module_id,
                release_id,
                module_digest,
            },
            PortableWorkspaceProvenance::Import {
                importer_module_id,
                upstream_module_id,
                upstream_release_id,
                export_id,
                source_namespace_id,
                local_namespace_id,
            } => Self::Import {
                importer_module_id,
                upstream_module_id,
                upstream_release_id,
                export_id,
                source_namespace_id,
                local_namespace_id,
            },
            PortableWorkspaceProvenance::Extension {
                owner_module_id,
                extension_namespace_id,
                target_namespace_id,
                target_declaration_id,
            } => Self::Extension {
                owner_module_id,
                extension_namespace_id,
                target_namespace_id,
                target_declaration_id,
            },
            PortableWorkspaceProvenance::Compilation => Self::Compilation,
        }
    }
}

#[derive(Clone, Copy, Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub(super) enum ChangeKindProjection {
    Added,
    Modified,
    Removed,
}

impl From<PortableWorkspaceChangeKind> for ChangeKindProjection {
    fn from(value: PortableWorkspaceChangeKind) -> Self {
        match value {
            PortableWorkspaceChangeKind::Added => Self::Added,
            PortableWorkspaceChangeKind::Modified => Self::Modified,
            PortableWorkspaceChangeKind::Removed => Self::Removed,
        }
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(super) struct ChangeRootProjection<'a> {
    root_digest: &'a str,
    subject: SubjectProjection,
    kind: ChangeKindProjection,
    prior_fact: Option<FactProjection<'a>>,
    current_fact: Option<FactProjection<'a>>,
}

impl<'a> From<&'a PortableWorkspaceChangeRoot> for ChangeRootProjection<'a> {
    fn from(value: &'a PortableWorkspaceChangeRoot) -> Self {
        Self {
            root_digest: value.root_digest(),
            subject: SubjectProjection::from(value.subject()),
            kind: ChangeKindProjection::from(value.kind()),
            prior_fact: value.prior_fact().map(FactProjection::from),
            current_fact: value.current_fact().map(FactProjection::from),
        }
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(super) struct ChangeRootPageProjection<'a> {
    entries: Vec<ChangeRootProjection<'a>>,
    next_offset: Option<u32>,
}

impl<'a> From<&'a PortableWorkspaceChangeRootPage> for ChangeRootPageProjection<'a> {
    fn from(value: &'a PortableWorkspaceChangeRootPage) -> Self {
        Self {
            entries: value
                .entries()
                .iter()
                .map(ChangeRootProjection::from)
                .collect(),
            next_offset: value.next_offset(),
        }
    }
}

#[derive(Clone, Copy, Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub(super) enum ImpactStateProjection {
    Affected,
    Unaffected,
}

impl From<PortableWorkspaceImpactState> for ImpactStateProjection {
    fn from(value: PortableWorkspaceImpactState) -> Self {
        match value {
            PortableWorkspaceImpactState::Affected => Self::Affected,
            PortableWorkspaceImpactState::Unaffected => Self::Unaffected,
        }
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(super) struct ImpactProjection {
    subject: SubjectProjection,
    state: ImpactStateProjection,
    contributing_root_count: u32,
}

#[derive(Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(super) struct ImpactPageProjection {
    entries: Vec<ImpactProjection>,
    next_offset: Option<u32>,
}

impl From<&PortableWorkspaceImpactPage> for ImpactPageProjection {
    fn from(value: &PortableWorkspaceImpactPage) -> Self {
        Self {
            entries: value
                .entries()
                .iter()
                .map(|impact| ImpactProjection {
                    subject: SubjectProjection::from(impact.subject()),
                    state: ImpactStateProjection::from(impact.state()),
                    contributing_root_count: impact.contributing_root_count(),
                })
                .collect(),
            next_offset: value.next_offset(),
        }
    }
}

#[derive(Clone, Copy, Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub(super) enum DerivationRuleProjection {
    SourceAdmission,
    ComponentAdmission,
    ModuleSealing,
    DeclarationLinking,
    ImportLinking,
    ExtensionApplication,
    PolicyComposition,
    ResponsibilityLowering,
    OrganizationalTranslation,
    ExperienceComposition,
    ResourceSelection,
    ResolutionLocking,
    UniverseLocking,
    SemanticIrSealing,
}

impl From<PortableWorkspaceDerivationRule> for DerivationRuleProjection {
    fn from(value: PortableWorkspaceDerivationRule) -> Self {
        match value {
            PortableWorkspaceDerivationRule::SourceAdmission => Self::SourceAdmission,
            PortableWorkspaceDerivationRule::ComponentAdmission => Self::ComponentAdmission,
            PortableWorkspaceDerivationRule::ModuleSealing => Self::ModuleSealing,
            PortableWorkspaceDerivationRule::DeclarationLinking => Self::DeclarationLinking,
            PortableWorkspaceDerivationRule::ImportLinking => Self::ImportLinking,
            PortableWorkspaceDerivationRule::ExtensionApplication => Self::ExtensionApplication,
            PortableWorkspaceDerivationRule::PolicyComposition => Self::PolicyComposition,
            PortableWorkspaceDerivationRule::ResponsibilityLowering => Self::ResponsibilityLowering,
            PortableWorkspaceDerivationRule::OrganizationalTranslation => {
                Self::OrganizationalTranslation
            }
            PortableWorkspaceDerivationRule::ExperienceComposition => Self::ExperienceComposition,
            PortableWorkspaceDerivationRule::ResourceSelection => Self::ResourceSelection,
            PortableWorkspaceDerivationRule::ResolutionLocking => Self::ResolutionLocking,
            PortableWorkspaceDerivationRule::UniverseLocking => Self::UniverseLocking,
            PortableWorkspaceDerivationRule::SemanticIrSealing => Self::SemanticIrSealing,
        }
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(super) struct CausalStepProjection<'a> {
    node_digest: &'a str,
    subject: SubjectProjection,
    provenance: ProvenanceProjection<'a>,
    rule: DerivationRuleProjection,
    fact: FactProjection<'a>,
}

impl<'a> From<&'a PortableWorkspaceCausalStep> for CausalStepProjection<'a> {
    fn from(value: &'a PortableWorkspaceCausalStep) -> Self {
        Self {
            node_digest: value.node_digest(),
            subject: SubjectProjection::from(value.subject()),
            provenance: ProvenanceProjection::from(value.provenance()),
            rule: DerivationRuleProjection::from(value.rule()),
            fact: FactProjection::from(value.fact()),
        }
    }
}

#[derive(Clone, Copy, Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub(super) enum ExplanationFrontierProjection {
    Predecessor,
    Current,
}

impl From<PortableWorkspaceExplanationFrontier> for ExplanationFrontierProjection {
    fn from(value: PortableWorkspaceExplanationFrontier) -> Self {
        match value {
            PortableWorkspaceExplanationFrontier::Predecessor => Self::Predecessor,
            PortableWorkspaceExplanationFrontier::Current => Self::Current,
        }
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(super) struct CausalExplanationProjection<'a> {
    root: ChangeRootProjection<'a>,
    frontier: ExplanationFrontierProjection,
    steps: Vec<CausalStepProjection<'a>>,
}

impl<'a> From<&'a PortableWorkspaceCausalExplanation> for CausalExplanationProjection<'a> {
    fn from(value: &'a PortableWorkspaceCausalExplanation) -> Self {
        Self {
            root: ChangeRootProjection::from(value.root()),
            frontier: ExplanationFrontierProjection::from(value.frontier()),
            steps: value
                .steps()
                .iter()
                .map(CausalStepProjection::from)
                .collect(),
        }
    }
}

#[derive(Clone, Copy, Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub(super) enum ExplanationStateProjection {
    NotPresent,
    Unaffected,
    Affected,
}

impl From<PortableWorkspaceExplanationState> for ExplanationStateProjection {
    fn from(value: PortableWorkspaceExplanationState) -> Self {
        match value {
            PortableWorkspaceExplanationState::NotPresent => Self::NotPresent,
            PortableWorkspaceExplanationState::Unaffected => Self::Unaffected,
            PortableWorkspaceExplanationState::Affected => Self::Affected,
        }
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(super) struct ExplanationProjection<'a> {
    subject: SubjectProjection,
    state: ExplanationStateProjection,
    causal: Option<CausalExplanationProjection<'a>>,
}

impl<'a> From<&'a PortableWorkspaceExplanation> for ExplanationProjection<'a> {
    fn from(value: &'a PortableWorkspaceExplanation) -> Self {
        Self {
            subject: SubjectProjection::from(value.subject()),
            state: ExplanationStateProjection::from(value.state()),
            causal: value.causal().map(CausalExplanationProjection::from),
        }
    }
}

pub(super) fn comparison_subject_page(
    page: &super::PortableWorkspaceComparisonSubjectPage,
) -> Vec<u8> {
    let comparison = page.comparison();
    canonical(Value::Object(Map::from_iter([
        ("receiptDigest".into(), json!(comparison.receipt_digest())),
        (
            "priorPrincipalUniverseDigest".into(),
            json!(comparison.prior_principal_universe_digest()),
        ),
        (
            "currentPrincipalUniverseDigest".into(),
            json!(comparison.current_principal_universe_digest()),
        ),
        (
            "priorSemanticIrDigest".into(),
            json!(comparison.prior_semantic_ir_digest()),
        ),
        (
            "currentSemanticIrDigest".into(),
            json!(comparison.current_semantic_ir_digest()),
        ),
        (
            "subjects".into(),
            Value::Array(page.subjects().iter().map(subject).collect()),
        ),
        (
            "subjectIdentityBytes".into(),
            json!(page.subject_identity_bytes()),
        ),
        ("inspected".into(), json!(page.inspected())),
        ("keyComparisons".into(), json!(page.key_comparisons())),
        ("isComplete".into(), json!(page.continuation().is_none())),
    ])))
}

pub(super) fn subject_comparison(value: &PortableWorkspaceSubjectComparison) -> Vec<u8> {
    let values = match value.values() {
        PortableWorkspaceSubjectValues::Absent => json!({"membership": "absent"}),
        PortableWorkspaceSubjectValues::PriorOnly { prior } => {
            json!({"membership": "prior_only", "priorFact": fact(prior)})
        }
        PortableWorkspaceSubjectValues::CurrentOnly { current } => {
            json!({"membership": "current_only", "currentFact": fact(current)})
        }
        PortableWorkspaceSubjectValues::Shared {
            prior,
            current,
            comparison,
        } => json!({
            "membership": "shared",
            "priorFact": fact(prior),
            "currentFact": fact(current),
            "comparison": match comparison {
                PortableWorkspaceValueComparison::Equal => "equal",
                PortableWorkspaceValueComparison::Changed => "changed",
            },
        }),
    };
    canonical(json!({
        "receiptDigest": value.receipt_digest(),
        "subject": subject(value.subject()),
        "values": values,
    }))
}

pub(super) fn dependency_page(page: &super::PortableWorkspaceDependencyPage) -> Vec<u8> {
    canonical(json!({
        "principalUniverseDigest": page.principal_universe_digest(),
        "semanticIrDigest": page.semantic_ir_digest(),
        "subject": subject(page.subject()),
        "root": page.root().map(causal_step),
        "dependencies": page.dependencies().iter().map(causal_step).collect::<Vec<_>>(),
        "copiedStringBytes": page.copied_string_bytes(),
        "isComplete": page.continuation().is_none(),
    }))
}

pub(super) fn subject_inspection(page: &super::PortableWorkspaceSubjectInspection) -> Vec<u8> {
    canonical(
        serde_json::to_value(SubjectInspectionProjection::from(page))
            .expect("subject inspection projection contains only serializable values"),
    )
}

#[derive(Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
#[serde(rename_all = "camelCase")]
pub(super) struct SubjectInspectionProjection<'a> {
    workspace_id: &'a str,
    principal_universe_digest: &'a str,
    semantic_ir_digest: &'a str,
    admission_evidence_digest: &'a str,
    subject: SubjectProjection,
    root: Option<CausalStepProjection<'a>>,
    dependencies: Vec<CausalStepProjection<'a>>,
    occurrences: Vec<OccurrenceProjection<'a>>,
    inspected: u16,
    record_anchor: Option<SourceLocationProjection<'a>>,
    dependency_copied_string_bytes: u32,
    source_copied_string_bytes: u32,
    inputs_complete: bool,
    sources_complete: bool,
}

impl<'a> From<&'a super::PortableWorkspaceSubjectInspection> for SubjectInspectionProjection<'a> {
    fn from(page: &'a super::PortableWorkspaceSubjectInspection) -> Self {
        Self {
            workspace_id: page.workspace_id(),
            principal_universe_digest: page.principal_universe_digest(),
            semantic_ir_digest: page.semantic_ir_digest(),
            admission_evidence_digest: page.admission_evidence_digest(),
            subject: SubjectProjection::from(page.subject()),
            root: page.root().map(CausalStepProjection::from),
            dependencies: page
                .dependencies()
                .iter()
                .map(CausalStepProjection::from)
                .collect(),
            occurrences: page
                .occurrences()
                .iter()
                .map(|occurrence| OccurrenceProjection {
                    location: SourceLocationProjection::from(occurrence.location()),
                    role: OccurrenceRoleProjection::from(occurrence.role()),
                })
                .collect(),
            inspected: page.inspected(),
            record_anchor: page.record_anchor().map(SourceLocationProjection::from),
            dependency_copied_string_bytes: page.dependency_copied_string_bytes(),
            source_copied_string_bytes: page.source_copied_string_bytes(),
            inputs_complete: page.inputs_complete(),
            sources_complete: page.sources_complete(),
        }
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
struct OccurrenceProjection<'a> {
    location: SourceLocationProjection<'a>,
    role: OccurrenceRoleProjection,
}

#[derive(Clone, Copy, Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
enum OccurrenceRoleProjection {
    Definition,
    Reference,
    Projection,
}

impl From<AuthoringOccurrenceRole> for OccurrenceRoleProjection {
    fn from(role: AuthoringOccurrenceRole) -> Self {
        match role {
            AuthoringOccurrenceRole::Definition => Self::Definition,
            AuthoringOccurrenceRole::Reference => Self::Reference,
            AuthoringOccurrenceRole::Projection => Self::Projection,
        }
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[cfg_attr(feature = "wire-schema", schemars(deny_unknown_fields))]
struct SourceLocationProjection<'a> {
    source_label: &'a str,
    path: Vec<InstancePathSegmentProjection<'a>>,
}

impl<'a> From<&'a AuthoringSourceLocation> for SourceLocationProjection<'a> {
    fn from(location: &'a AuthoringSourceLocation) -> Self {
        Self {
            source_label: location.source_label(),
            path: location
                .path()
                .iter()
                .map(InstancePathSegmentProjection::from)
                .collect(),
        }
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "wire-schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", content = "value", rename_all = "kebab-case")]
enum InstancePathSegmentProjection<'a> {
    Field(&'a str),
    Index(usize),
}

impl<'a> From<&'a AuthoringInstancePathSegment> for InstancePathSegmentProjection<'a> {
    fn from(segment: &'a AuthoringInstancePathSegment) -> Self {
        match segment {
            AuthoringInstancePathSegment::Field(field) => Self::Field(field),
            AuthoringInstancePathSegment::Index(index) => Self::Index(*index),
        }
    }
}

pub(super) fn dependent_page(page: &super::PortableWorkspaceDependentPage) -> Vec<u8> {
    canonical(Value::Object(Map::from_iter([
        ("workspaceId".into(), json!(page.workspace_id())),
        (
            "principalUniverseDigest".into(),
            json!(page.principal_universe_digest()),
        ),
        ("semanticIrDigest".into(), json!(page.semantic_ir_digest())),
        (
            "admissionEvidenceDigest".into(),
            json!(page.admission_evidence_digest()),
        ),
        ("subject".into(), subject(page.subject())),
        ("root".into(), page.root().map_or(Value::Null, causal_step)),
        (
            "dependents".into(),
            Value::Array(page.dependents().iter().map(causal_step).collect()),
        ),
        ("inspected".into(), json!(page.inspected())),
        (
            "inputDigestComparisons".into(),
            json!(page.input_digest_comparisons()),
        ),
        ("isComplete".into(), json!(page.is_complete())),
    ])))
}

pub(super) fn change_root_page(page: &PortableWorkspaceChangeRootPage) -> Vec<u8> {
    canonical(ChangeRootPageProjection::from(page))
}

pub(super) fn contributing_roots(page: &super::PortableWorkspaceContributingRootsPage) -> Vec<u8> {
    use super::PortableWorkspaceContributingRootsState;
    let comparison = page.comparison();
    canonical(Value::Object(Map::from_iter([
        ("receiptDigest".into(), json!(comparison.receipt_digest())),
        (
            "priorPrincipalUniverseDigest".into(),
            json!(comparison.prior_principal_universe_digest()),
        ),
        (
            "currentPrincipalUniverseDigest".into(),
            json!(comparison.current_principal_universe_digest()),
        ),
        (
            "priorSemanticIrDigest".into(),
            json!(comparison.prior_semantic_ir_digest()),
        ),
        (
            "currentSemanticIrDigest".into(),
            json!(comparison.current_semantic_ir_digest()),
        ),
        ("subject".into(), subject(page.subject())),
        (
            "state".into(),
            json!(match page.state() {
                PortableWorkspaceContributingRootsState::NotPresent => "not_present",
                PortableWorkspaceContributingRootsState::Unaffected => "unaffected",
                PortableWorkspaceContributingRootsState::Affected => "affected",
            }),
        ),
        (
            "roots".into(),
            Value::Array(page.roots().iter().map(change_root).collect()),
        ),
        ("isComplete".into(), json!(page.continuation().is_none())),
    ])))
}

pub(super) fn impact_page(page: &PortableWorkspaceImpactPage) -> Vec<u8> {
    canonical(ImpactPageProjection::from(page))
}

pub(super) fn explanation(value: &PortableWorkspaceExplanation) -> Vec<u8> {
    canonical(ExplanationProjection::from(value))
}

fn canonical(value: impl Serialize) -> Vec<u8> {
    pale_core::canon::to_canonical_string(&value)
        .expect("API-owned portable workspace projection is canonical JSON")
        .into_bytes()
}

fn subject(value: &PortableWorkspaceSubject) -> Value {
    serde_json::to_value(SubjectProjection::from(value))
        .expect("SDK-owned subject projection is JSON")
}

fn fact(value: &PortableWorkspaceFact) -> Value {
    serde_json::to_value(FactProjection::from(value)).expect("SDK-owned fact projection is JSON")
}

fn change_root(root: &PortableWorkspaceChangeRoot) -> Value {
    serde_json::to_value(ChangeRootProjection::from(root))
        .expect("SDK-owned change-root projection is JSON")
}

fn causal_step(step: &PortableWorkspaceCausalStep) -> Value {
    serde_json::to_value(CausalStepProjection::from(step))
        .expect("SDK-owned causal-step projection is JSON")
}
