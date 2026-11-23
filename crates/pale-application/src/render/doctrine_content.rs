//! On-demand doctrine, run-mode, and bounded-tool skills.

use pale_core::records::OperationCapabilityContractV1;

use super::semantic_content::{
    description, doctrine_name, doctrine_skill_name, mode_name, official_source_guidance,
    operation_is_model_facing, operation_name, relevant_sources, yaml_string,
};
use crate::semantic::{
    RepositorySemanticPlan, ResolvedDoctrine, ResolvedOperation, ResolvedRunMode,
};

/// Render one doctrine body with its selected sources and native mechanics.
pub(super) fn doctrine_skill(
    doctrine: &ResolvedDoctrine,
    semantics: &RepositorySemanticPlan,
) -> Result<String, super::RenderError> {
    let sources = relevant_sources(&doctrine.record.official_sources, &[], semantics);
    let required = if doctrine.record.requires.is_empty() {
        String::new()
    } else {
        let names = doctrine
            .record
            .requires
            .iter()
            .map(|id| {
                semantics
                    .doctrine
                    .iter()
                    .any(|item| item.record.id == *id)
                    .then(|| format!("`{}`", doctrine_skill_name(id)))
                    .ok_or_else(|| {
                        super::RenderError::invalid_input(
                            "doctrine dependencies",
                            format!(
                                "doctrine `{}` requires missing doctrine `{id}`",
                                crate::semantic::resource_label(&doctrine.record.id)
                            ),
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        format!(
            "Required guidance is included in this skill: {}.\n\n",
            names.join(", ")
        )
    };
    let is_page = semantics
        .page_reading
        .contains_key(&crate::packages::PackageResourceUse::new(
            doctrine.record.id.clone(),
            pale_core::records::PackageResourceKindV1::Doctrine,
        ));
    let related = if is_page || doctrine.related_reading.is_empty() {
        String::new()
    } else {
        let related = doctrine
            .related_reading
            .iter()
            .map(|reference| super::body_links::related_reading(reference, semantics))
            .collect::<Vec<_>>();
        format!(
            "Further reading, when this subject is needed: {}.\n\n",
            related.join("; ")
        )
    };
    let operations = relevant_selected_operations(doctrine, semantics);
    let body = super::body_links::skill_body(&doctrine.body, semantics)?;
    let navigation = super::body_links::page_navigation(
        &doctrine.record.id,
        pale_core::records::PackageResourceKindV1::Doctrine,
        semantics,
        "../",
    )?;
    super::closing_questions::finish(&doctrine.record.id, pale_core::records::PackageResourceKindV1::Doctrine, semantics, "../", format!(
        "---\nname: {name}\ndescription: {description}\n---\n\n# {title}\n\nUse this guide for its stated task. Tools and actions still require the project’s configuration and permissions.\n\n{required}{related}{body}{operations}\n\n{official}",
        name = doctrine_name(doctrine),
        description = yaml_string(&description(&doctrine.record.description)),
        title = doctrine.title,
        required = format_args!("{navigation}{required}"),
        related = related,
        body = body.trim(),
        operations = operations,
        official = official_source_guidance(&super::semantic_helpers::page_sources(&doctrine.record.id, pale_core::records::PackageResourceKindV1::Doctrine, semantics, &sources)),
    ))
}

/// List model-facing selected operations that consume one doctrine entry.
///
/// The relationship is explanatory only and grants neither selection nor effect.
fn relevant_selected_operations(
    doctrine: &ResolvedDoctrine,
    semantics: &RepositorySemanticPlan,
) -> String {
    let operations = semantics
        .operations
        .iter()
        .filter(|operation| operation_is_model_facing(operation))
        .filter(|operation| {
            operation
                .doctrine_ids
                .iter()
                .any(|id| id == &doctrine.record.id)
        })
        .map(|operation| {
            format!(
                "- `{}` (`{}`): {}",
                operation_name(operation),
                crate::semantic::resource_label(&operation.record.id),
                description(&operation.record.description)
            )
        })
        .collect::<Vec<_>>();
    if operations.is_empty() {
        return String::new();
    }
    format!(
        "\n\n## Tasks that use this guide\n\nThese configured tasks use this guide. Follow the chosen task’s invocation rules and permissions.\n\n{}",
        operations.join("\n")
    )
}

/// Render one selected run mode as bounded agent guidance.
pub(super) fn run_mode_skill(mode: &ResolvedRunMode) -> String {
    let body = mode.body.as_ref().map(|body| body.materialize());
    let body = body.as_ref().map(|body| body.trim()).unwrap_or(
        "Hide optional Pale guidance for this session. Required permissions and safety rules still apply.",
    );
    format!(
        "---\nname: {name}\ndescription: {description}\n---\n\n# {title}\n\nThis command changes the session’s `{axis}` guidance setting. Ownership, confidentiality, read-only access, required approvals and execution limits still apply. Change the mode through the Pale runtime so the session state is updated.\n\n{body}\n",
        name = mode_name(mode),
        description = yaml_string(&description(&mode.record.description)),
        title = mode.record.description,
        axis = mode.record.axis,
        body = body,
    )
}

/// The compact payload injected while a mode is active. Suppressing modes have
/// no payload; their absence is the behavior rather than an empty placeholder.
pub(super) fn runtime_mode_content(mode: &ResolvedRunMode) -> Option<String> {
    let body = mode.runtime_body.as_ref()?.materialize();
    Some(format!(
        "## Active Pale mode: {id} ({axis})\n\n{body}\n",
        id = crate::semantic::resource_label(&mode.record.id),
        axis = mode.record.axis,
        body = body.trim(),
    ))
}

/// Compose each bounded tool's mechanical guidance into the operation contract
/// that owns it. The rendered operation skill, contract, and lease then share
/// one digest-bound invocation boundary.
pub(super) fn operation_tool_guidance(
    operation: &ResolvedOperation,
    semantics: &RepositorySemanticPlan,
    capabilities: &[OperationCapabilityContractV1],
) -> Result<String, super::RenderError> {
    let selected = operation
        .capabilities()
        .iter()
        .zip(capabilities)
        .filter_map(|(semantic, route)| semantic.recipe.as_ref().map(|recipe| (recipe, route)))
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Ok(String::new());
    }

    let mut text = String::from(
        "## How to use the configured tools\n\nThis task uses the Package recipes below. A recipe describes the procedure; the configured route identifies which provider can run it on this host.\n",
    );
    for (recipe_id, route) in selected {
        let recipe = semantics
            .recipes
            .iter()
            .find(|recipe| &recipe.resource == recipe_id)
            .ok_or_else(|| {
                super::RenderError::invalid_input(
                    "operation capabilities",
                    format!(
                        "operation `{}` references missing recipe `{}`",
                        crate::semantic::resource_label(&operation.record.id),
                        crate::semantic::resource_label(recipe_id)
                    ),
                )
            })?;
        let provider = match route.provider_state.provider() {
            None => "No compatible private provider is selected. Follow the configured response to an unavailable provider; do not substitute another.".to_string(),
            Some(kind) if !route.available() => format!(
                "Private provider `{kind}` is selected, but this route has not been checked as usable. Do not invoke it."
            ),
            Some(kind) => {
                let target = route
                    .provider_state
                    .target()
                    .map(|target| format!(" target `{target}`"))
                    .unwrap_or_default();
                let tools = route
                    .provider_state
                    .host_tools()
                    .iter()
                    .map(|tool| format!("`{}`", tool.model_tool_name()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "Provider `{kind}`{target} is usable through these permitted host tools: {tools}."
                )
            }
        };
        text.push_str(&format!(
            "\n### {} (`{}`)\n\nTool contract `{}` operation `{}`; largest permitted change `{}`; strongest supported claim `{}`. {provider}\n",
            recipe.procedure.description,
            crate::semantic::resource_label(&recipe.resource),
            recipe.procedure.tool_contract,
            recipe.procedure.tool_operation,
            recipe.procedure.effect_class,
            recipe.procedure.evidence.proof_ceiling,
        ));
    }
    Ok(text)
}
