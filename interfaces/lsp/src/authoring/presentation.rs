//! Reader-facing descriptions of admitted identities and effective profile facts.

use super::*;

pub(super) fn effective_authoring_markdown(
    inspection: &pale_sdk::EffectiveAuthoringInspection,
    markup: AuthoringMarkupKind,
) -> String {
    let mut facts = inspection
        .entries()
        .iter()
        .filter_map(|entry| effective_entry_markdown(&entry.detail, markup))
        .collect::<Vec<_>>();
    facts.sort();
    facts.dedup();
    if inspection.truncated() {
        facts
            .push("Additional profile facts were omitted by the bounded editor projection.".into());
    }
    facts.join("\n\n")
}

fn effective_entry_markdown(
    detail: &EffectiveProfileEntryDetail,
    markup: AuthoringMarkupKind,
) -> Option<String> {
    match detail {
        EffectiveProfileEntryDetail::DeclaredSelection { .. } => {
            Some("This identity is part of the admitted declared intent.".to_owned())
        }
        EffectiveProfileEntryDetail::Responsibility {
            contract_id,
            execution_path_id,
            ..
        } => Some(format!(
            "Responsibility contract {} participates in effective execution path {}.",
            inline_code(markup, contract_id),
            inline_code(markup, execution_path_id)
        )),
        EffectiveProfileEntryDetail::Selection { selection } => match selection {
            EffectiveProfileSelection::Package {
                package_id,
                version,
                ..
            } => Some(format!(
                "Package {} release {} is in the effective closure.",
                inline_code(markup, package_id),
                inline_code(markup, version)
            )),
            EffectiveProfileSelection::NormativePack { .. } => None,
            EffectiveProfileSelection::SemanticResource {
                repository_id,
                resource_id,
                ..
            } => Some(format!(
                "Semantic resource {} is selected for repository {}.",
                inline_code(markup, resource_id),
                inline_code(markup, repository_id)
            )),
            EffectiveProfileSelection::RunPolicy { run_policy_id } => Some(format!(
                "Run policy {} is the effective private selection.",
                inline_code(markup, run_policy_id)
            )),
            EffectiveProfileSelection::Session {
                execution_workspace_id,
                activation_root_count,
                additional_root_count,
                activation_path_count,
            } => Some(format!(
                "Execution workspace {} has an admitted session model: {} activation roots, {} additional roots, and {} activation paths.",
                inline_code(markup, execution_workspace_id),
                activation_root_count,
                additional_root_count,
                activation_path_count
            )),
            EffectiveProfileSelection::Lifecycle {
                lifecycle_id,
                event_id,
                action_id,
            } => Some(format!(
                "Lifecycle behavior {} carries event {} and action {} in the effective profile.",
                inline_code(markup, lifecycle_id),
                inline_code(markup, event_id),
                inline_code(markup, action_id)
            )),
            EffectiveProfileSelection::AuthorityRoot { authority_root_id } => Some(format!(
                "Authority root {} is selected as a logical ceiling; this is not native proof.",
                inline_code(markup, authority_root_id)
            )),
            EffectiveProfileSelection::DisclosureCeiling { .. } => None,
        },
        EffectiveProfileEntryDetail::CapabilityNarrowing { .. } => Some(
            "Private capability policy narrows the Package-owned capability/effect domain; it does not prove provider or native availability."
                .to_owned(),
        ),
        EffectiveProfileEntryDetail::GoverningCeiling { .. } => Some(
            "The effective governing ceiling is the compiler-owned meet of its admitted Package policy contributors."
                .to_owned(),
        ),
        EffectiveProfileEntryDetail::Repository {
            repository_id,
            materialization_order,
            access_ceiling,
        } => Some(format!(
            "Repository {} is materialized at order {} with a {} access ceiling. The ceiling permits later admission to be considered; it does not authorize a native effect.",
            inline_code(markup, repository_id),
            materialization_order,
            match access_ceiling {
                pale_sdk::EffectiveProfileRepositoryAccess::Read => "read-only",
                pale_sdk::EffectiveProfileRepositoryAccess::Mutate => "mutation-capable",
            }
        )),
        EffectiveProfileEntryDetail::ProviderRepository {
            repository_id,
            provider_binding_id,
            native_repository_coordinate,
            ..
        } => Some(format!(
            "Repository {} is projected through provider binding {} at opaque native coordinate {}. This is admitted private configuration; owner-native realization remains unknown.",
            inline_code(markup, repository_id),
            inline_code(markup, provider_binding_id),
            inline_code(markup, native_repository_coordinate)
        )),
        EffectiveProfileEntryDetail::ExecutionPath {
            selection_id,
            experience_path_id,
            terminal_provider_binding,
            ..
        } => {
            let terminal = terminal_provider_binding.as_ref().map_or_else(
                || "no terminal provider binding".to_owned(),
                |binding| {
                    format!(
                        "terminal provider binding {} with exact identity {}",
                        inline_code(markup, &binding.provider_binding_id),
                        inline_code(markup, &binding.provider_binding_identity),
                    )
                },
            );
            Some(format!(
                "Execution path {} selects experience path {} and {terminal}. Portable selection is admitted; owner-native realization remains unknown.",
                inline_code(markup, selection_id),
                inline_code(markup, experience_path_id),
            ))
        }
        EffectiveProfileEntryDetail::Operation {
            operation_id,
            exposure,
            ..
        } => Some(format!(
            "Operation {} is {}. {}",
            inline_code(markup, operation_id),
            operation_exposure_label(*exposure),
            operation_exposure_explanation(*exposure)
        )),
        EffectiveProfileEntryDetail::Capability {
            capability_id,
            requirement_disposition,
            provider_disposition,
            effect_class,
            ..
        } => Some(format!(
            "Capability {} has {} effect, is {}, and is {}. Native realization remains unknown.",
            inline_code(markup, capability_id),
            effect_label(*effect_class),
            requirement_disposition_label(*requirement_disposition),
            provider_disposition_label(*provider_disposition)
        )),
        EffectiveProfileEntryDetail::MachineRealization { machine_id, .. } => Some(format!(
            "Machine {} is selected, but owner-native realization remains unproven.",
            inline_code(markup, machine_id)
        )),
        EffectiveProfileEntryDetail::Finding { finding } => finding_markdown(finding, markup),
    }
}

const fn operation_exposure_label(value: EffectiveProfileOperationExposure) -> &'static str {
    match value {
        EffectiveProfileOperationExposure::Unbound => "unbound",
        EffectiveProfileOperationExposure::SemanticOnly => "semantic-only",
        EffectiveProfileOperationExposure::CapabilityBearing => "capability-bearing",
    }
}

const fn operation_exposure_explanation(value: EffectiveProfileOperationExposure) -> &'static str {
    match value {
        EffectiveProfileOperationExposure::Unbound => "No private operation binding exists.",
        EffectiveProfileOperationExposure::SemanticOnly => {
            "A private binding exists and Package meaning declares zero capability requirements."
        }
        EffectiveProfileOperationExposure::CapabilityBearing => {
            "A private binding exists and Package-owned capability requirements are present."
        }
    }
}

const fn requirement_disposition_label(
    value: EffectiveProfileRequirementDisposition,
) -> &'static str {
    match value {
        EffectiveProfileRequirementDisposition::Satisfied => "satisfied by portable configuration",
        EffectiveProfileRequirementDisposition::ProviderBindingAbsent => {
            "missing a private provider binding"
        }
        EffectiveProfileRequirementDisposition::Narrowed { .. } => "narrowed by private policy",
        EffectiveProfileRequirementDisposition::Blocked { .. } => "blocked by admitted policy",
    }
}

const fn provider_disposition_label(value: EffectiveProfileProviderDisposition) -> &'static str {
    match value {
        EffectiveProfileProviderDisposition::Bound { .. } => "bound to a private provider",
        EffectiveProfileProviderDisposition::MissingBinding => "provider-unbound",
        EffectiveProfileProviderDisposition::ExcludedByNarrowing => "excluded by narrowing",
        EffectiveProfileProviderDisposition::EffectMismatch => "bound with an effect mismatch",
        EffectiveProfileProviderDisposition::RecipeContractMismatch => {
            "bound with a recipe-contract mismatch"
        }
    }
}

const fn effect_label(value: EffectiveProfileEffectClass) -> &'static str {
    match value {
        EffectiveProfileEffectClass::Compute => "compute",
        EffectiveProfileEffectClass::Read => "read",
        EffectiveProfileEffectClass::Execute => "execute",
        EffectiveProfileEffectClass::Write => "write",
        EffectiveProfileEffectClass::Network => "network",
        EffectiveProfileEffectClass::Publish => "publish",
    }
}

fn finding_markdown(
    finding: &EffectiveProfileFinding,
    markup: AuthoringMarkupKind,
) -> Option<String> {
    match finding {
        EffectiveProfileFinding::NativeRealizationUnknown { machine_id } => Some(format!(
            "Native realization on machine {} is unknown.",
            inline_code(markup, machine_id)
        )),
        EffectiveProfileFinding::OperationUnbound { operation_id, .. } => Some(format!(
            "Operation {} is selected but unbound; only that operation closes.",
            inline_code(markup, operation_id)
        )),
        EffectiveProfileFinding::Requirement {
            capability_id,
            disposition,
            ..
        } => Some(format!(
            "Capability {} is {}.",
            inline_code(markup, capability_id),
            requirement_disposition_label(*disposition)
        )),
    }
}

fn inline_code(markup: AuthoringMarkupKind, value: &str) -> String {
    match markup {
        AuthoringMarkupKind::Markdown => format!("`{value}`"),
        AuthoringMarkupKind::PlainText => value.to_owned(),
    }
}

pub(super) const fn identity_kind_label(kind: pale_sdk::AuthoringIdentityKind) -> &'static str {
    match kind {
        pale_sdk::AuthoringIdentityKind::SourceDocument => "source document",
        pale_sdk::AuthoringIdentityKind::Workspace => "workspace",
        pale_sdk::AuthoringIdentityKind::Project => "project",
        pale_sdk::AuthoringIdentityKind::ExecutionWorkspace => "execution workspace",
        pale_sdk::AuthoringIdentityKind::Repository => "repository",
        pale_sdk::AuthoringIdentityKind::ExecutionPath => "execution path",
        pale_sdk::AuthoringIdentityKind::Module => "workspace module",
        pale_sdk::AuthoringIdentityKind::ModuleRelease => "module release",
        pale_sdk::AuthoringIdentityKind::Component => "workspace component",
        pale_sdk::AuthoringIdentityKind::Export => "module export",
        pale_sdk::AuthoringIdentityKind::Namespace => "workspace namespace",
        pale_sdk::AuthoringIdentityKind::Declaration => "workspace declaration",
        pale_sdk::AuthoringIdentityKind::Resource => "logical resource",
        pale_sdk::AuthoringIdentityKind::RunPolicy => "run policy",
        pale_sdk::AuthoringIdentityKind::LifecycleBehavior => "lifecycle behavior",
        pale_sdk::AuthoringIdentityKind::Capability => "capability",
        pale_sdk::AuthoringIdentityKind::AuthorityRoot => "authority root",
        pale_sdk::AuthoringIdentityKind::ProviderBinding => "provider binding",
        pale_sdk::AuthoringIdentityKind::Machine => "machine binding",
        pale_sdk::AuthoringIdentityKind::ResponsibilityDerivation => "responsibility derivation",
        pale_sdk::AuthoringIdentityKind::ResponsibilityProjection => "responsibility projection",
        pale_sdk::AuthoringIdentityKind::ResponsibilityGenealogyLink => {
            "responsibility Genealogy link"
        }
        pale_sdk::AuthoringIdentityKind::LogicalScope => "logical scope",
        pale_sdk::AuthoringIdentityKind::LogicalOrganization => "logical organization",
        pale_sdk::AuthoringIdentityKind::OrganizationalGroup => "organizational group",
        pale_sdk::AuthoringIdentityKind::DisciplineTree => "discipline tree",
        pale_sdk::AuthoringIdentityKind::OrganizationalRole => "organizational role",
        pale_sdk::AuthoringIdentityKind::OrganizationalOccupant => "organizational occupant",
        pale_sdk::AuthoringIdentityKind::OrganizationalProvider => "organizational provider",
        pale_sdk::AuthoringIdentityKind::CommunicationActor => "communication actor",
        pale_sdk::AuthoringIdentityKind::LeadershipTopic => "leadership topic",
        pale_sdk::AuthoringIdentityKind::OrganizationalRelation => "organizational relation",
        pale_sdk::AuthoringIdentityKind::OrganizationalObject => "organizational object",
        pale_sdk::AuthoringIdentityKind::GroupLensProfile => "group-lens profile",
        pale_sdk::AuthoringIdentityKind::DirectionalService => "directional service",
        pale_sdk::AuthoringIdentityKind::OrganizationalProjection => "organizational projection",
        pale_sdk::AuthoringIdentityKind::OrganizationalInspection => "organizational inspection",
        pale_sdk::AuthoringIdentityKind::CompositionParticipant => "composition participant",
        pale_sdk::AuthoringIdentityKind::CompositionResponsibilityContract => {
            "composition responsibility contract"
        }
        pale_sdk::AuthoringIdentityKind::CompositionParticipantManifest => {
            "composition participant manifest"
        }
        pale_sdk::AuthoringIdentityKind::CompositionProvider => "composition provider",
        pale_sdk::AuthoringIdentityKind::CompositionProviderBinding => {
            "composition provider binding"
        }
        pale_sdk::AuthoringIdentityKind::CompositionEntryContract => "composition entry contract",
        pale_sdk::AuthoringIdentityKind::CompositionRequestTranslation => {
            "composition request translation"
        }
        pale_sdk::AuthoringIdentityKind::CompositionResultTranslation => {
            "composition result translation"
        }
        pale_sdk::AuthoringIdentityKind::CompositionTerminationContract => {
            "composition termination contract"
        }
        pale_sdk::AuthoringIdentityKind::CompositionSettlementContract => {
            "composition settlement contract"
        }
        pale_sdk::AuthoringIdentityKind::CompositionContinuityContract => {
            "composition continuity contract"
        }
        pale_sdk::AuthoringIdentityKind::CompositionEdge => "composition edge",
        pale_sdk::AuthoringIdentityKind::ExperiencePath => "experience path",
        pale_sdk::AuthoringIdentityKind::PackageDoctrine => "Package doctrine",
        pale_sdk::AuthoringIdentityKind::PackageOperationArchetype => "Package operation archetype",
        pale_sdk::AuthoringIdentityKind::PackageOperation => "Package operation",
        pale_sdk::AuthoringIdentityKind::PackageRoleArchetype => "Package role archetype",
        pale_sdk::AuthoringIdentityKind::PackageRole => "Package role",
        pale_sdk::AuthoringIdentityKind::PackageStack => "Package stack",
        pale_sdk::AuthoringIdentityKind::PackageRunMode => "Package run mode",
        pale_sdk::AuthoringIdentityKind::PackagePolicyFamily => "Package policy family",
        pale_sdk::AuthoringIdentityKind::PackageGoverningOrder => "Package governing order",
        pale_sdk::AuthoringIdentityKind::PackageCapabilityPolicy => "Package capability policy",
        pale_sdk::AuthoringIdentityKind::PackageToolRecipe => "Package tool recipe",
        pale_sdk::AuthoringIdentityKind::Package => "Package",
    }
}
