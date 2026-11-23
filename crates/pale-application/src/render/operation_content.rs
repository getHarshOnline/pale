//! Human-facing skill projection for one selected on-demand operation.
//!
//! Rendering composes the admitted Markdown lineage and adds only mechanical
//! invocation facts. It does not reinterpret Package procedure or claim that a
//! configured provider is currently usable.

use crate::terminal_adapters::TerminalAdapterDescriptor;

use super::semantic_content::{
    capability_contract, description, external_boundary, official_source_guidance, operation_name,
    operation_requires_manual_invocation, operation_status, operation_tool_guidance,
    operation_tools_frontmatter, relevant_sources, role_name, yaml_string,
};
use super::types::HostOperationProjection;
use super::RenderError;
use crate::semantic::{RepositorySemanticPlan, ResolvedOperation};

/// Render one model-facing operation without exposing unavailable mechanics as usable.
pub(super) fn operation_skill(
    operation: &ResolvedOperation,
    semantics: &RepositorySemanticPlan,
    host: TerminalAdapterDescriptor,
    projection: &HostOperationProjection,
) -> Result<String, RenderError> {
    let name = operation_name(operation);
    let aliases = operation
        .selection()
        .map(pale_core::records::OperationSelection::aliases)
        .unwrap_or_default();
    let aliases = if aliases.is_empty() {
        "(none)".to_string()
    } else {
        aliases.join(", ")
    };
    let outputs = if operation.record.outputs.is_empty() {
        "- No saved result is declared for a fully supported run.\n".to_string()
    } else {
        operation
            .record
            .outputs
            .iter()
            .map(|output| format!("- {output}\n"))
            .collect()
    };
    let role_names: Vec<String> = operation
        .record
        .roles
        .iter()
        .map(|id| {
            semantics
                .roles
                .iter()
                .find(|role| role.record.id == *id)
                .map(role_name)
                .ok_or_else(|| {
                    RenderError::invalid_input(
                        "operation roles",
                        format!(
                            "operation `{}` references missing role `{id}`",
                            crate::semantic::resource_label(&operation.record.id)
                        ),
                    )
                })
        })
        .collect::<Result<_, _>>()?;
    let roles = if role_names.is_empty() {
        String::new()
    } else {
        format!(
            "## Specialist agents\n\nWhen an independent specialist would help, delegate using these generated agent names: {}. Pass the question, relevant sources or diff, limits, and check results. Specialists return findings with supporting evidence; this task remains responsible for any action.\n\n",
            role_names
                .iter()
                .map(|name| format!("`{name}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    let required_doctrine = super::body_links::skill_body(&operation.required_doctrine, semantics)?;
    let doctrine = if required_doctrine.is_empty() {
        String::new()
    } else {
        format!(
            "## Required guidance\n\nThe required guidance is included below. Use this copy.\n\n{}\n\n",
            required_doctrine.trim()
        )
    };
    let blocked = projection
        .blocked_reasons
        .iter()
        .map(|reason| format!("- Blocked: {reason}\n"))
        .collect::<String>();
    let degraded = projection
        .degraded_reasons
        .iter()
        .map(|reason| format!("- Reduced result: {reason}\n"))
        .collect::<String>();
    let optional = projection
        .optional_reasons
        .iter()
        .map(|reason| format!("- Optional tool unavailable: {reason}\n"))
        .collect::<String>();
    let unavailable = if blocked.is_empty() && degraded.is_empty() && optional.is_empty() {
        String::new()
    } else {
        format!("\nCurrent tool limits:\n{blocked}{degraded}{optional}")
    };
    let capabilities = capability_contract(operation.capabilities(), projection)?;
    let tool_guidance = operation_tool_guidance(operation, semantics, &projection.capabilities)?;
    // Accepted Package operations do not own repository-selection patterns.
    // Repository/profile facts remain available through the surrounding
    // rendered context rather than being reconstructed from Package source.
    let sources = relevant_sources(&[], &[], semantics);
    let stack_navigation = if semantics.stacks.is_empty() {
        ""
    } else {
        "When the procedure calls for stack guidance, use the Stacks section of `.pale/semantic-index.md` to find the skills selected for this repository. Open the guidance needed for this task.\n\n"
    };
    let body = super::body_links::skill_body(&operation.body, semantics)?;
    let navigation = super::body_links::page_navigation(
        &operation.record.id,
        pale_core::records::PackageResourceKindV1::Operation,
        semantics,
        "../",
    )?;
    super::closing_questions::finish(&operation.record.id, pale_core::records::PackageResourceKindV1::Operation, semantics, "../", format!(
        "---\nname: {name}\ndescription: {description}\n{manual}{allowed}---\n\n# {title}\n\nInvoke as `{name}`. Other search terms for this task: {aliases}. These are search labels, not extra commands.\nStatus: {status}\n\n{external}## Tools and permissions\n\n{capabilities}{unavailable}{tool_guidance}\n\n## Expected result\n\nWhen fully supported, return:\n{outputs}If the task is blocked or can produce only a reduced result, follow the provider instructions above and report what remains unavailable.\n\n{roles}{stack_navigation}{doctrine}## Procedure\n\n{body}\n\n{official}",
        name = name,
        description = yaml_string(&description(&operation.record.description)),
        manual = if host == TerminalAdapterDescriptor::claude_code()
            && operation_requires_manual_invocation(operation)
        {
            "disable-model-invocation: true\n"
        } else {
            ""
        },
        allowed = if projection.support != pale_core::records::OperationSupport::Blocked
            && projection
                .capabilities
                .iter()
                .filter(|capability| capability.required())
                .all(|capability| {
                    capability.available()
                        || capability.policy.on_unavailable()
                            == pale_core::records::CapabilityUnavailability::Degrade
                },)
        {
            operation_tools_frontmatter(operation, semantics, host, &projection.capabilities)
        } else {
            String::new()
        },
        title = operation.record.description,
        aliases = aliases,
        status = operation_status(projection.support),
        external = external_boundary(operation.capabilities()),
        capabilities = capabilities,
        unavailable = unavailable,
        tool_guidance = tool_guidance,
        outputs = outputs,
        roles = roles,
        doctrine = format_args!("{navigation}{doctrine}"),
        body = body.trim(),
        official = official_source_guidance(&super::semantic_helpers::page_sources(&operation.record.id, pale_core::records::PackageResourceKindV1::Operation, semantics, &sources)),
    ))
}
