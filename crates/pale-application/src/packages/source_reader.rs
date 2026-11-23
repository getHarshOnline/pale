//! Parse Package TOML, check its schema and decode typed records.

use std::path::Path;

use pale_contracts::CompiledSchema;
use serde::de::{DeserializeOwned, IntoDeserializer};

use super::PackageError;

/// Parse TOML into the JSON value used for schema checks and typed decoding.
pub(super) fn toml_to_json(text: &str, path: &Path) -> Result<serde_json::Value, PackageError> {
    let value: toml::Value = toml::from_str(text).map_err(|mut source| {
        // The parser otherwise retains and may render the complete Package file.
        source.set_input(None);
        PackageError::TomlSyntax {
            path: path.to_path_buf(),
            source: Box::new(source),
        }
    })?;
    serde_json::to_value(value).map_err(|source| PackageError::TomlToJson {
        path: path.to_path_buf(),
        source,
    })
}

/// Parse TOML using a named definition in the manifest's schema.
///
/// `capability-procedure.v1` supplies shared definitions rather than a schema
/// for a complete source file. As in [`parse_source_with_schema`], parse the
/// TOML once, check that value against the selected `$defs` entry, then decode
/// the same value into the requested type.
pub(in crate::packages) fn parse_named_definition<T>(
    text: &str,
    source: &Path,
    schema_id: &str,
    definition: &str,
    label: &str,
) -> Result<T, PackageError>
where
    T: DeserializeOwned,
{
    let value = toml_to_json(text, source)?;
    pale_contracts::validate_definition(&value, schema_id, definition).map_err(
        |error| match error {
            pale_contracts::SchemaError::Invalid(violations) => PackageError::SourceSchema {
                path: source.to_path_buf(),
                source: violations,
            },
            other => PackageError::SchemaCheck(other),
        },
    )?;
    serde_path_to_error::deserialize(value.into_deserializer()).map_err(|source_error| {
        let value_path = source_error.path().to_string();
        PackageError::SourceDecode {
            path: source.to_path_buf(),
            label: label.to_string(),
            value_path,
            source: Box::new(source_error),
        }
    })
}

/// Parse captured Package TOML, check its schema, then decode the same value.
pub(in crate::packages) fn parse_source_with_schema<T>(
    text: &str,
    source: &Path,
    schema: &CompiledSchema,
    label: &str,
) -> Result<T, PackageError>
where
    T: DeserializeOwned,
{
    let value = toml_to_json(text, source)?;
    schema
        .validate(&value)
        .map_err(|source_error| PackageError::SourceSchema {
            path: source.to_path_buf(),
            source: source_error,
        })?;
    serde_path_to_error::deserialize(value.into_deserializer()).map_err(|source_error| {
        let value_path = source_error.path().to_string();
        PackageError::SourceDecode {
            path: source.to_path_buf(),
            label: label.to_string(),
            value_path,
            source: Box::new(source_error),
        }
    })
}
