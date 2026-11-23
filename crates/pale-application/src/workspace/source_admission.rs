//! Single admission boundary for bounded private workspace TOML records.

use std::borrow::Cow;
use std::path::Path;

use pale_core::records::WorkspaceSourceContentDigest;
use pale_ports::{SourceFilesystemPort, SourceReadLimit};
use serde::de::{DeserializeOwned, IntoDeserializer};

use super::{AuthoringTomlGrammar, WorkspaceError, WorkspaceSourceCoordinate};

/// Maximum byte length admitted for one private workspace TOML record.
///
/// The source port enforces this bound before syntax and schema admission.
pub(super) const WORKSPACE_FILE_LIMIT: SourceReadLimit = SourceReadLimit::new(4 * 1024 * 1024);

/// Minimal read capability shared by filesystem capture and captured documents.
///
/// This private seam deliberately omits recursive traversal: workspace-language
/// admission reads only the exact members named by its root manifest. Package
/// distribution capture continues to use [`SourceFilesystemPort`] directly.
pub(super) trait WorkspaceDocumentSource {
    /// Borrow or own one complete bounded source member.
    fn read_regular<'source>(
        &'source self,
        path: &Path,
        limit: SourceReadLimit,
    ) -> Result<Cow<'source, [u8]>, WorkspaceError>;
}

impl<T> WorkspaceDocumentSource for T
where
    T: SourceFilesystemPort + ?Sized,
{
    fn read_regular<'source>(
        &'source self,
        path: &Path,
        limit: SourceReadLimit,
    ) -> Result<Cow<'source, [u8]>, WorkspaceError> {
        SourceFilesystemPort::read_regular(self, path, limit)
            .map_err(WorkspaceError::Input)
            .map(Cow::Owned)
    }
}

/// Root-confined view of one source-backed private workspace.
///
/// Callers continue to name already-resolved workspace paths, while this view
/// reduces every observation to a confined relative member before delegating to
/// [`SourceFilesystemPort::read_confined_regular`]. It is the single adapter
/// between workspace-language loading and host path traversal.
pub(super) struct SourceBackedWorkspaceDocuments<'source> {
    sources: &'source dyn SourceFilesystemPort,
    root: &'source Path,
}

impl<'source> SourceBackedWorkspaceDocuments<'source> {
    /// Bind source observation to one lexical workspace root.
    pub(super) fn new(sources: &'source dyn SourceFilesystemPort, root: &'source Path) -> Self {
        Self { sources, root }
    }
}

impl WorkspaceDocumentSource for SourceBackedWorkspaceDocuments<'_> {
    fn read_regular<'source>(
        &'source self,
        path: &Path,
        limit: SourceReadLimit,
    ) -> Result<Cow<'source, [u8]>, WorkspaceError> {
        let relative = path
            .strip_prefix(self.root)
            .map_err(|_| WorkspaceError::EscapedReference(path.display().to_string()))?;
        self.sources
            .read_confined_regular(self.root, relative, limit)
            .map_err(WorkspaceError::Input)
            .map(Cow::Owned)
    }
}

/// Read one bounded UTF-8 workspace-owned content member.
pub(super) fn read_utf8_source(
    sources: &(impl WorkspaceDocumentSource + ?Sized),
    path: &Path,
) -> Result<String, WorkspaceError> {
    let bytes = sources.read_regular(path, WORKSPACE_FILE_LIMIT)?;
    std::str::from_utf8(bytes.as_ref())
        .map(str::to_owned)
        .map_err(|_| WorkspaceError::InputNotUtf8(path.to_path_buf()))
}

/// Typed record paired with exact source-byte evidence retained for derivation.
pub(super) struct AdmittedWorkspaceRecord<T> {
    pub(super) value: T,
    pub(super) source_digest: WorkspaceSourceContentDigest,
    pub(super) byte_len: u32,
}

/// Read, parse, schema-admit, and typed-decode one private workspace record.
///
/// Syntax positions come only from the original TOML parser. JSON Schema and
/// typed decoding retain the source path and a nested value path, respectively;
/// neither fabricates a TOML line or column after representation conversion.
pub(super) fn admit_toml<T: DeserializeOwned>(
    sources: &(impl WorkspaceDocumentSource + ?Sized),
    path: &Path,
    schema_id: &str,
    record: &'static str,
) -> Result<T, WorkspaceError> {
    Ok(admit_toml_with_evidence(sources, path, schema_id, record)?.value)
}

/// Admit one TOML record while retaining its exact bounded source-byte identity.
pub(super) fn admit_toml_with_evidence<T: DeserializeOwned>(
    sources: &(impl WorkspaceDocumentSource + ?Sized),
    path: &Path,
    schema_id: &str,
    record: &'static str,
) -> Result<AdmittedWorkspaceRecord<T>, WorkspaceError> {
    let bytes = sources.read_regular(path, WORKSPACE_FILE_LIMIT)?;
    admit_toml_bytes_with_evidence(bytes.as_ref(), path, schema_id, record)
}

/// Admit exact already-retained TOML bytes through the workspace's sole syntax,
/// schema, and typed-decoding pipeline.
///
/// Host capture uses this after reading a capability record so the lifecycle
/// inventory is derived from the same bytes returned to path-free consumers.
pub(super) fn admit_toml_bytes<T: DeserializeOwned>(
    bytes: &[u8],
    path: &Path,
    schema_id: &str,
    record: &'static str,
) -> Result<T, WorkspaceError> {
    Ok(admit_toml_bytes_with_evidence(bytes, path, schema_id, record)?.value)
}

/// Decode exact retained bytes while preserving their derivation identity.
fn admit_toml_bytes_with_evidence<T: DeserializeOwned>(
    bytes: &[u8],
    path: &Path,
    schema_id: &str,
    record: &'static str,
) -> Result<AdmittedWorkspaceRecord<T>, WorkspaceError> {
    let source_digest = WorkspaceSourceContentDigest::hash_bytes(bytes);
    let byte_len =
        u32::try_from(bytes.len()).expect("the four-megabyte workspace source bound fits in u32");
    let json = toml_json(bytes, path)?;
    pale_contracts::validate_value(&json, schema_id).map_err(|source| {
        WorkspaceError::SchemaCheck {
            coordinate: WorkspaceSourceCoordinate::path_only(path),
            source: Box::new(source),
        }
    })?;
    Ok(AdmittedWorkspaceRecord {
        value: decode_json(json, path, record)?,
        source_digest,
        byte_len,
    })
}

/// Validate exact TOML bytes against one compiler-owned authoring grammar.
///
/// This is context-free structural admission only. It deliberately returns no
/// decoded value and cannot establish cross-record meaning, owner proof, or an
/// effective profile.
pub(super) fn validate_authoring_toml(
    bytes: &[u8],
    path: &Path,
    grammar: AuthoringTomlGrammar,
) -> Result<(), WorkspaceError> {
    let json = toml_json(bytes, path)?;
    let result = grammar.definition_name().map_or_else(
        || pale_contracts::validate_value(&json, grammar.schema_id()),
        |definition| pale_contracts::validate_definition(&json, grammar.schema_id(), definition),
    );
    result.map_err(|source| WorkspaceError::SchemaCheck {
        coordinate: WorkspaceSourceCoordinate::path_only(path),
        source: Box::new(source),
    })
}

fn toml_json(bytes: &[u8], path: &Path) -> Result<serde_json::Value, WorkspaceError> {
    let text =
        std::str::from_utf8(bytes).map_err(|_| WorkspaceError::InputNotUtf8(path.to_path_buf()))?;
    let value: toml::Value = toml::from_str(text).map_err(|mut source| {
        let coordinate = source_coordinate(path, text, source.span());
        // The parser otherwise retains and may render the complete private file.
        source.set_input(None);
        WorkspaceError::TomlSyntax {
            coordinate,
            source: Box::new(source),
        }
    })?;
    serde_json::to_value(value).map_err(|source| WorkspaceError::TomlToJson {
        coordinate: WorkspaceSourceCoordinate::path_only(path),
        source,
    })
}

fn source_coordinate(
    path: &Path,
    text: &str,
    span: Option<std::ops::Range<usize>>,
) -> WorkspaceSourceCoordinate {
    let Some(span) = span else {
        return WorkspaceSourceCoordinate::path_only(path);
    };
    let Some(prefix) = text.get(..span.start) else {
        return WorkspaceSourceCoordinate::path_only(path);
    };
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rsplit('\n')
        .next()
        .map_or(1, |tail| tail.chars().count() + 1);
    WorkspaceSourceCoordinate::with_span(path, span, line, column)
}

fn decode_json<T: DeserializeOwned>(
    value: serde_json::Value,
    path: &Path,
    record: &'static str,
) -> Result<T, WorkspaceError> {
    serde_path_to_error::deserialize(value.into_deserializer()).map_err(|source| {
        let value_path = source.path().to_string();
        WorkspaceError::Decode {
            coordinate: Box::new(WorkspaceSourceCoordinate::path_only(path)),
            record,
            value_path,
            source: Box::new(source),
        }
    })
}

/// Read and validate one bounded workspace record through the source port.
#[cfg(test)]
#[path = "../../tests/internal/workspace/source_admission.rs"]
mod tests;
