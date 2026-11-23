//! Safe composition details shared by compilation and configuration-review failures.

use pale_application::compile::CompositionError;
use pale_core::records::{DiagnosticFactV1Wire, DiagnosticSubjectV1Wire};

/// Project declared identities and closed violations, never private reason strings.
pub(crate) fn composition_failure_details(
    error: &CompositionError,
) -> (Option<DiagnosticSubjectV1Wire>, Vec<DiagnosticFactV1Wire>) {
    let fact = |key: &str, value: String| DiagnosticFactV1Wire {
        key: key.to_owned(),
        value,
    };
    let (kind, id, facts) = match error {
        CompositionError::ProviderBinding { binding, violation } => (
            "provider_binding",
            binding,
            vec![fact("violation", violation.to_string())],
        ),
        CompositionError::DeclaredSchema { schema_id, .. } => ("schema", schema_id, Vec::new()),
        CompositionError::DuplicateId { kind, id } => (
            "composition_declaration",
            id,
            vec![fact("record_kind", (*kind).to_owned())],
        ),
        CompositionError::MissingReference {
            kind,
            owner,
            target_kind,
            target,
        } => (
            "composition_declaration",
            owner,
            vec![
                fact("record_kind", (*kind).to_owned()),
                fact("target_kind", (*target_kind).to_owned()),
                fact("target", target.clone()),
            ],
        ),
        CompositionError::ProviderCompatibility { provider, .. } => {
            ("provider", provider, Vec::new())
        }
        CompositionError::Delegation { delegation, .. } => ("delegation", delegation, Vec::new()),
        CompositionError::Edge { edge, .. } => ("composition_edge", edge, Vec::new()),
        CompositionError::Path { path, .. } => ("experience_path", path, Vec::new()),
        CompositionError::PathSelection {
            selection,
            experience_path,
        } => (
            "execution_path_selection",
            selection,
            vec![fact("experience_path", experience_path.clone())],
        ),
        CompositionError::DeclarationLimit { kind } => {
            return (None, vec![fact("collection", (*kind).to_owned())]);
        }
        CompositionError::Schema(_) | CompositionError::Canonicalization(_) => {
            return (None, Vec::new());
        }
    };
    (
        Some(DiagnosticSubjectV1Wire {
            kind: kind.to_owned(),
            id: id.clone(),
        }),
        facts,
    )
}
