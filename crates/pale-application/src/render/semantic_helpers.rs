//! Shared host names, compact text shaping, and bounded official-source guidance.

use std::collections::BTreeSet;

use pale_core::records::{OfficialReference, OperationSupport, PackageResourceRefV1};

use super::types::RenderError;
use crate::semantic::{
    RepositorySemanticPlan, ResolvedDoctrine, ResolvedOperation, ResolvedRole, ResolvedRunMode,
};

/// Maximum already-ordered official references rendered into one prompt.
///
/// The ceiling bounds generated context without inventing a second ranking policy.
const MAX_OFFICIAL_SOURCES: usize = 2;

/// Combine a page's own references with those supplied by included guidance.
pub(super) fn page_sources(
    resource: &PackageResourceRefV1,
    kind: pale_core::records::PackageResourceKindV1,
    semantics: &RepositorySemanticPlan,
    included: &[OfficialReference],
) -> Vec<OfficialReference> {
    let mut seen = BTreeSet::new();
    semantics
        .page_reading
        .get(&crate::packages::PackageResourceUse::new(
            resource.clone(),
            kind,
        ))
        .into_iter()
        .flat_map(|reading| reading.official_sources.iter())
        .chain(included.iter())
        .filter(|reference| seen.insert(reference.url.clone()))
        .take(MAX_OFFICIAL_SOURCES)
        .cloned()
        .collect()
}

/// Shorten a digest for display only; callers must retain the full identity elsewhere.
pub(super) fn short(digest: &str) -> &str {
    &digest[..digest.len().min(12)]
}

/// Serialize security-bearing render data without masking failures.
pub(super) fn json_pretty<T: serde::Serialize>(
    value: &T,
    what: &str,
) -> Result<String, RenderError> {
    let mut document = serde_json::to_string_pretty(value)
        .map_err(|error| RenderError::projection_error(what, error))?;
    document.push('\n');
    Ok(document)
}

/// Map a family identifier to the host-valid name shared by skills and agents.
pub(super) fn host_name(resource: &PackageResourceRefV1) -> String {
    format!("pale-{}", resource_slug(resource))
}

/// Encode an owner-qualified Package resource into a host-safe unique slug.
pub(super) fn resource_slug(resource: &PackageResourceRefV1) -> String {
    /// Maximum bytes retained from the readable prefix of a resource slug.
    ///
    /// Longer labels keep a digest suffix derived from the complete semantic
    /// label, so truncation preserves deterministic collision resistance.
    const MAX_BYTES: usize = 48;
    let label = crate::semantic::resource_label(resource);
    let slug = label.replace(['.', '_', ':', '/'], "-");
    if slug.len() <= MAX_BYTES && !label.contains('/') {
        return slug;
    }
    let digest = pale_core::records::ArtifactDigest::hash_bytes(label.as_bytes());
    format!(
        "{}-{}",
        &slug[..slug.len().min(MAX_BYTES - 13)],
        &digest.as_str()[..12]
    )
}

/// Derive the host name for a stack's own page independently of its guide use.
pub(super) fn stack_name(id: &PackageResourceRefV1) -> String {
    format!("pale-stack-{}", resource_slug(id))
}

/// Encode a scalar through JSON's compatible quoted-string grammar.
pub(super) fn yaml_string(value: &str) -> String {
    serde_json::Value::String(value.to_owned()).to_string()
}

/// Collapse incidental whitespace without changing word order.
pub(super) fn description(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Derive the stable host-facing name for one resolved operation.
pub(super) fn operation_name(operation: &ResolvedOperation) -> String {
    format!("pale-op-{}", resource_slug(&operation.record.id))
}

/// Derive the stable host-facing name for one resolved role.
pub(super) fn role_name(role: &ResolvedRole) -> String {
    format!("pale-role-{}", resource_slug(&role.record.id))
}

/// Derive the stable host-facing name for one resolved doctrine.
pub(super) fn doctrine_name(doctrine: &ResolvedDoctrine) -> String {
    doctrine_skill_name(&doctrine.record.id)
}

/// Derive a host-valid doctrine skill name from its canonical identity.
pub(super) fn doctrine_skill_name(id: &PackageResourceRefV1) -> String {
    format!("pale-doctrine-{}", resource_slug(id))
}

/// Derive the stable host-facing name for one resolved run mode.
pub(super) fn mode_name(mode: &ResolvedRunMode) -> String {
    format!("pale-mode-{}", resource_slug(&mode.record.id))
}

/// Summarize each independent mode axis with its selected default and source.
pub(super) fn mode_default_summary(semantics: &RepositorySemanticPlan) -> String {
    semantics
        .mode_defaults
        .iter()
        .map(|(axis, id)| {
            let source = semantics
                .mode_default_sources
                .get(axis)
                .map(ToString::to_string)
                .unwrap_or_else(|| "unknown".to_string());
            format!("{axis}={} ({source})", crate::semantic::resource_label(id))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Render the support of an already bound operation.
pub(super) fn operation_status(support: OperationSupport) -> &'static str {
    match support {
        OperationSupport::Full => "fully supported",
        OperationSupport::Degraded => "limited to the configured result with fewer effects",
        OperationSupport::Blocked => "blocked; a required tool is unavailable",
    }
}

/// Select at most the bounded, de-duplicated official sources in declared order.
pub(super) fn relevant_sources(
    direct: &[OfficialReference],
    stack_scope: &[PackageResourceRefV1],
    semantics: &RepositorySemanticPlan,
) -> Vec<OfficialReference> {
    let mut seen = BTreeSet::new();
    direct
        .iter()
        .chain(
            semantics
                .stacks
                .iter()
                .filter(|stack| stack_scope.contains(&stack.id))
                .flat_map(|stack| stack.official_sources.iter()),
        )
        .filter(|reference| seen.insert(reference.url.clone()))
        .take(MAX_OFFICIAL_SOURCES)
        .cloned()
        .collect()
}

/// Render version-sensitive source guidance only when selected references exist.
pub(super) fn official_source_guidance(sources: &[OfficialReference]) -> String {
    if sources.is_empty() {
        return String::new();
    }
    let mut text = String::from(
        "## Check facts that depend on the version\n\nWhen a fact may have changed or this guide leaves a question open, use the relevant official source below. Check the version in use and stop when the question is answered.\n",
    );
    for source in sources {
        let mut qualifiers = Vec::new();
        if let Some(jurisdiction) = &source.jurisdiction {
            qualifiers.push(format!("jurisdiction: {jurisdiction}"));
        }
        if let Some(checked) = &source.checked {
            qualifiers.push(format!("reviewed: {checked}"));
        }
        let suffix = if qualifiers.is_empty() {
            String::new()
        } else {
            format!(" ({})", qualifiers.join("; "))
        };
        text.push_str(&format!("- [{}]({}){}\n", source.label, source.url, suffix));
    }
    text
}
