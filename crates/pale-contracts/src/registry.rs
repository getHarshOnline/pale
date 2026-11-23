//! Generated JSON Schema bundle enforcement.
//!
//! Canonical schemas are embedded into the binary at compile time, so runtime
//! validation never resolves build-machine paths. Validators for embedded ids
//! compile once per process and cache either success or failure.

use std::collections::{BTreeMap, BinaryHeap};
use std::fmt;
use std::sync::{Arc, OnceLock};

use pale_core::canon;
use pale_core::records::{
    schema_set_digest, RecordSchemaIdentity, SchemaDigest, SchemaId, SchemaManifestDigest,
    SchemaRegistryIdentity, SchemaSetDigest, SchemaSetMember,
};

/// Maximum schema violations retained after deterministic lexical selection.
///
/// The validation collector may observe more failures, but its heap keeps only
/// the lexicographically earliest violations so discovery order cannot change
/// the bounded diagnostic result.
const MAX_SCHEMA_VIOLATIONS: usize = 32;
/// Maximum Unicode scalar values retained in each schema-diagnostic field.
///
/// Paths and schema-authored expectations are evidence, not a second copy of
/// the rejected input, so every field is bounded independently.
const MAX_SCHEMA_DIAGNOSTIC_CHARS: usize = 1_024;
/// Maximum exact instance-path depth retained for source navigation.
///
/// A deeper validator coordinate remains available through
/// [`SchemaViolation::instance_path`], but no partial typed path is exposed:
/// consumers must never mistake a truncated coordinate for the rejected value.
const MAX_SCHEMA_INSTANCE_PATH_SEGMENTS: usize = 64;

/// One exact step through the JSON instance rejected by a schema.
///
/// The owning validator resolves each RFC 6901 token against the original
/// instance. A numeric object key is therefore a [`Property`](Self::Property),
/// while a token selecting an array member is an [`Index`](Self::Index).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaInstancePathSegment {
    /// Descend through one exact JSON object property.
    Property(String),
    /// Select one exact JSON array member.
    Index(usize),
}

/// One safe, owned reason that a value failed an embedded JSON Schema.
///
/// The violation retains JSON instance and schema paths plus the governing
/// keyword and expected constraint. It deliberately does not retain or render
/// the rejected value, which may contain private configuration or credentials.
#[derive(Debug, Clone)]
pub struct SchemaViolation {
    instance_path: String,
    instance_path_segments: Option<Vec<SchemaInstancePathSegment>>,
    schema_path: String,
    keyword: String,
    instance_path_identity: String,
    schema_path_identity: String,
    keyword_identity: String,
    semantic_discriminator: String,
    expectation: String,
}

impl SchemaViolation {
    /// JSON Pointer locating the rejected value; an empty string means the root.
    pub fn instance_path(&self) -> &str {
        &self.instance_path
    }

    /// Exact property/index path resolved against the rejected instance.
    ///
    /// `None` means the coordinate could not be retained exactly within the
    /// fixed depth bound. The string [`Self::instance_path`] remains available
    /// for portable display, but must not be reparsed to recover type.
    #[must_use]
    pub fn instance_path_segments(&self) -> Option<&[SchemaInstancePathSegment]> {
        self.instance_path_segments.as_deref()
    }

    /// JSON Pointer locating the governing keyword in the embedded schema.
    pub fn schema_path(&self) -> &str {
        &self.schema_path
    }

    /// JSON Schema keyword whose constraint was not satisfied.
    pub fn keyword(&self) -> &str {
        &self.keyword
    }

    /// Bounded schema-derived expectation that excludes the rejected value.
    pub fn expectation(&self) -> &str {
        &self.expectation
    }

    /// Return the presentation-independent identity of this violation.
    ///
    /// The first three members are SHA-256 identities of the complete instance
    /// path, schema path, and keyword. Display strings remain independently
    /// bounded, so two long coordinates with a shared visible prefix cannot
    /// alias. The fourth member distinguishes validator variants that share a
    /// keyword and individual obligations emitted by one `required` rule.
    /// Schema-authored expectation prose remains human evidence only and never
    /// drives bounded aggregate retention.
    #[must_use]
    pub fn semantic_identity(&self) -> (&str, &str, &str, &str) {
        (
            &self.instance_path_identity,
            &self.schema_path_identity,
            &self.keyword_identity,
            &self.semantic_discriminator,
        )
    }
}

impl PartialEq for SchemaViolation {
    fn eq(&self, other: &Self) -> bool {
        self.semantic_identity() == other.semantic_identity()
    }
}

impl Eq for SchemaViolation {}

impl PartialOrd for SchemaViolation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SchemaViolation {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.semantic_identity().cmp(&other.semantic_identity())
    }
}

impl fmt::Display for SchemaViolation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let instance = if self.instance_path.is_empty() {
            "the document root"
        } else {
            self.instance_path.as_str()
        };
        let schema = if self.schema_path.is_empty() {
            "the schema root"
        } else {
            self.schema_path.as_str()
        };
        write!(
            formatter,
            "{instance} violates `{}` at {schema}: {}",
            self.keyword, self.expectation
        )
    }
}

/// Bounded deterministic violations for one rejected schema instance.
///
/// [`Self::violations`] is lexicographically ordered and contains at most 32
/// entries. [`Self::truncated_count`] records every additional violation found
/// without retaining unbounded diagnostic state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaViolations {
    schema_id: String,
    violations: Vec<SchemaViolation>,
    truncated_count: usize,
}

impl SchemaViolations {
    /// Registry identity of the embedded schema that rejected the value.
    pub fn schema_id(&self) -> &str {
        &self.schema_id
    }

    /// Retained violations in deterministic order.
    pub fn violations(&self) -> &[SchemaViolation] {
        &self.violations
    }

    /// Number of additional violations omitted after the fixed bound.
    pub fn truncated_count(&self) -> usize {
        self.truncated_count
    }
}

impl fmt::Display for SchemaViolations {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "record does not satisfy schema `{}`",
            self.schema_id
        )?;
        if let Some(first) = self.violations.first() {
            write!(formatter, ": {first}")?;
            if self.violations.len() > 1 {
                write!(
                    formatter,
                    " ({} additional violation(s) retained)",
                    self.violations.len() - 1
                )?;
            }
        }
        if self.truncated_count != 0 {
            write!(
                formatter,
                " ({} additional violation(s) truncated)",
                self.truncated_count
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for SchemaViolations {}

/// Typed cause of an embedded JSON Schema compilation failure.
///
/// The shared source lets the per-process validator cache reproduce the same
/// causal error chain without reducing the validator's typed failure to text.
#[derive(Debug, Clone)]
pub struct SchemaCompilationError {
    source: Arc<jsonschema::ValidationError<'static>>,
}

impl fmt::Display for SchemaCompilationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.source.as_ref(), formatter)
    }
}

impl std::error::Error for SchemaCompilationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

/// Failures from embedded-registry lookup, integrity, and structural validation.
///
/// [`SchemaError::Parse`], [`SchemaError::Compile`], and [`SchemaError::Integrity`]
/// contradict the embedded contract boundary. [`SchemaError::Invalid`] and
/// [`SchemaError::IdentityMismatch`] describe a supplied value that disagrees with
/// the selected descriptor; the caller owns that value's trust classification.
/// Lookup and record-encoding failures remain separately typed.
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    /// The requested contract id is not compiled into this binary.
    #[error("unknown schema id: {0}")]
    UnknownId(String),
    /// The selected definition-set schema does not contain the requested name.
    #[error("unknown definition `{definition}` in schema `{schema_id}`")]
    UnknownDefinition {
        /// Registry identity of the selected definition-set schema.
        schema_id: String,
        /// Exact key requested beneath the schema's `$defs` object.
        definition: String,
    },
    /// A typed record could not be represented as JSON for validation.
    #[error("record could not be serialized to JSON: {0}")]
    Encode(#[source] serde_json::Error),
    /// Embedded schema source is not valid strict JSON.
    #[error("schema parse failed: {0}")]
    Parse(#[from] canon::CanonError),
    /// The JSON Schema validator rejected the embedded schema itself.
    #[error("schema compile failed: {0}")]
    Compile(#[source] SchemaCompilationError),
    /// The generated registry does not match its embedded manifest or sources.
    #[error("schema registry integrity failed: {0}")]
    Integrity(String),
    /// A carried record identity does not match the exact expected descriptor.
    #[error("record schema identity does not match descriptor `{0}`")]
    IdentityMismatch(String),
    /// The supplied record does not satisfy the named embedded contract.
    #[error("{0}")]
    Invalid(SchemaViolations),
}

/// Serialize a computed or consumed record and validate it against an embedded schema.
///
/// `schema_id` is the descriptor-manifest registry identity. For descriptors
/// with self-carried identity, this also verifies the exact wire `schema`
/// constant and raw-schema SHA-256; callers must not repeat that comparison.
/// This function is effect-free. An effectful caller remains responsible for
/// placing successful admission before its own effect boundary.
///
/// # Errors
///
/// Returns [`SchemaError::Encode`] when `record` cannot be represented as JSON.
/// Otherwise returns the same contract-resolution, schema-compilation, or
/// instance-rejection errors as [`validate_value`].
pub fn validate_record<T: serde::Serialize>(
    record: &T,
    schema_id: &str,
) -> Result<(), SchemaError> {
    let value = serde_json::to_value(record).map_err(SchemaError::Encode)?;
    validate_value(&value, schema_id)
}

/// Validate an already parsed external JSON value against an embedded schema.
///
/// This validates the value and any descriptor-declared self-carried identity.
/// It cannot prove that original input bytes obeyed strict JSON, because a
/// [`serde_json::Value`] no longer retains duplicate keys. Parse trust-boundary
/// bytes with [`pale_core::canon::parse_strict`] before calling this function.
///
/// # Errors
///
/// Returns [`SchemaError::UnknownId`] when `schema_id` is not part of this
/// binary's active embedded contract set; [`SchemaError::Parse`],
/// [`SchemaError::Compile`], or [`SchemaError::Integrity`] when the embedded
/// contract boundary is incoherent; [`SchemaError::Invalid`] when `value` does
/// not satisfy the selected shape; and [`SchemaError::IdentityMismatch`] when a
/// descriptor-declared carried identity disagrees with that descriptor.
pub fn validate_value(value: &serde_json::Value, schema_id: &str) -> Result<(), SchemaError> {
    validate_value_with_root_branch(value, schema_id, None)
}

/// Validate a value while explaining an already-qualified root union branch.
///
/// `branch` is a diagnostic-only index into the embedded root `oneOf`. The
/// caller must derive it from the actual input discriminator and the same
/// immutable schema, never from a desired action. Validation and carried
/// identity admission are identical to [`validate_value`]; this selects only
/// evidence from an existing failed validation, without validating again.
/// Absent or out-of-range indices, non-root failures, and unions with multiple
/// matches retain their aggregate evidence.
///
/// # Errors
///
/// Returns the same failures as [`validate_value`], with bounded field-level
/// evidence when the qualified branch failed.
pub fn validate_value_with_root_branch(
    value: &serde_json::Value,
    schema_id: &str,
    branch: Option<usize>,
) -> Result<(), SchemaError> {
    let schema = CompiledSchema::for_id(schema_id)?;
    if let Some(violations) = collect_branch_violations(
        schema.schema_id,
        value,
        schema.inner.iter_errors(value),
        branch,
    ) {
        return Err(SchemaError::Invalid(violations));
    }
    let descriptor = schema_descriptor(schema_id)?;
    if let Some(expected_wire_id) = descriptor.wire_schema_id {
        let carried_wire_id =
            first_string_property(value, &["schema", "schema_id", "envelope_schema_id"]);
        if carried_wire_id != Some(expected_wire_id) {
            return Err(SchemaError::IdentityMismatch(schema_id.to_owned()));
        }
        if descriptor.raw_schema_sha256_carriage == "self_carried" {
            let carried_raw_sha =
                first_string_property(value, &["schema_sha256", "envelope_schema_sha256"]);
            if carried_raw_sha != Some(descriptor.schema_sha256) {
                return Err(SchemaError::IdentityMismatch(schema_id.to_owned()));
            }
        }
    }
    Ok(())
}

/// Return the first supported root identity member carrying a string value.
fn first_string_property<'a>(value: &'a serde_json::Value, fields: &[&str]) -> Option<&'a str> {
    fields
        .iter()
        .find_map(|field| value.get(*field).and_then(serde_json::Value::as_str))
}

/// Validate a value against one named member of an embedded `$defs` contract.
///
/// Definition-set schemas such as `capability-procedure.v1` intentionally have
/// no standalone record root. Package admission selects the exact definition
/// by name, while this function compiles and caches a wrapper that retains the
/// owning schema's complete `$defs` closure. The definition is therefore not
/// copied into another schema family and local references remain authoritative.
///
/// This function validates shape only. The caller remains responsible for
/// choosing the definition that matches the Package resource kind before typed
/// decoding and for applying later semantic admission.
///
/// # Errors
///
/// Returns [`SchemaError::UnknownId`] for an absent schema,
/// [`SchemaError::UnknownDefinition`] for an absent `$defs` member, the normal
/// embedded-schema parse/compile/integrity failures, or [`SchemaError::Invalid`]
/// when `value` violates the selected definition.
pub fn validate_definition(
    value: &serde_json::Value,
    schema_id: &str,
    definition: &str,
) -> Result<(), SchemaError> {
    ensure_registry_integrity()?;
    let schema = EMBEDDED_SCHEMAS
        .iter()
        .find(|schema| schema.descriptor.schema_id == schema_id)
        .ok_or_else(|| SchemaError::UnknownId(schema_id.to_string()))?;
    let validator = schema.definition_validator(definition)?;
    let identity = format!("{schema_id}#/$defs/{}", escape_pointer_token(definition));
    match collect_schema_violations(&identity, value, validator.iter_errors(value)) {
        Some(violations) => Err(SchemaError::Invalid(violations)),
        None => Ok(()),
    }
}

/// Serialize a typed record and validate it against one named `$defs` member.
///
/// # Errors
///
/// Returns [`SchemaError::Encode`] when `record` cannot be represented as JSON;
/// otherwise returns the same errors as [`validate_definition`].
pub fn validate_definition_record<T: serde::Serialize>(
    record: &T,
    schema_id: &str,
    definition: &str,
) -> Result<(), SchemaError> {
    let value = serde_json::to_value(record).map_err(SchemaError::Encode)?;
    validate_definition(&value, schema_id, definition)
}

/// Generated immutable descriptor for one embedded schema.
#[derive(Debug, Clone, Copy)]
pub struct SchemaDescriptor {
    /// Registry ID used by Pale schema lookup.
    pub schema_id: &'static str,
    /// Manifest-owned canonical `$id` URI, verified against the generated schema.
    pub schema_uri: &'static str,
    /// Exact top-level wire `schema` constant, when the source declares one.
    pub wire_schema_id: Option<&'static str>,
    /// Path relative to the self-contained generated bundle.
    pub source_path: &'static str,
    /// SHA-256 of the exact captured UTF-8 source bytes.
    pub schema_sha256: &'static str,
    /// Manifest-declared raw-schema identity carriage used by record admission.
    pub raw_schema_sha256_carriage: &'static str,
    /// RFC 8785 canonical manifest entry used for full metadata verification.
    pub manifest_entry_canonical: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/pale_schema_descriptors.rs"));

/// Annotated structural schema used only for configuration-authoring help.
///
/// Its source is generated from the same Rust producer as the corresponding
/// sealed schema, but retains Rustdoc descriptions. It has no digest,
/// lifecycle, validation authority, or runtime-admission meaning.
#[derive(Debug)]
pub struct AuthoringSchema {
    schema_id: &'static str,
    source: &'static str,
    parsed: OnceLock<Result<serde_json::Value, CachedSchemaError>>,
}

impl AuthoringSchema {
    /// Descriptor-registry identity of the corresponding sealed grammar.
    #[must_use]
    pub const fn schema_id(&self) -> &'static str {
        self.schema_id
    }

    /// Borrow annotated Draft 2020-12 JSON Schema text.
    ///
    /// Consumers may use these bytes for editor presentation only. Contract
    /// admission must continue through [`CompiledSchema`] or [`validate_value`].
    #[must_use]
    pub const fn source(&self) -> &'static str {
        self.source
    }

    /// Borrow the strict parsed annotation tree cached for this process.
    ///
    /// The value remains editor-only metadata. Parsing it does not compile a
    /// validator or give the annotated resource contract-admission authority.
    ///
    /// # Errors
    ///
    /// Returns [`SchemaError::Parse`] when generated annotation bytes are not
    /// strict JSON, or [`SchemaError::Integrity`] if a cached failure cannot be
    /// reproduced. Either result contradicts the build-time generator invariant.
    pub fn value(&self) -> Result<&serde_json::Value, SchemaError> {
        self.parsed
            .get_or_init(|| {
                canon::parse_strict(self.source.as_bytes()).map_err(|_| CachedSchemaError::Parse)
            })
            .as_ref()
            .map_err(|error| error.to_public(self.source))
    }
}

include!(concat!(env!("OUT_DIR"), "/pale_authoring_schemas.rs"));

struct EmbeddedSchema {
    descriptor: SchemaDescriptor,
    source: &'static str,
    compiled: OnceLock<Result<jsonschema::Validator, CachedSchemaError>>,
    compiled_definitions:
        OnceLock<Result<BTreeMap<String, jsonschema::Validator>, CachedSchemaError>>,
}

#[derive(Debug)]
enum CachedSchemaError {
    Parse,
    Compile(SchemaCompilationError),
    Integrity(String),
}

impl CachedSchemaError {
    fn to_public(&self, source: &str) -> SchemaError {
        match self {
            // CanonError is not cloneable; immutable embedded bytes reproduce the
            // same public error while the cached failure prevents recompilation.
            Self::Parse => match canon::parse_strict(source.as_bytes()) {
                Err(error) => SchemaError::Parse(error),
                Ok(_) => SchemaError::Integrity(
                    "cached schema parse failure could not be reproduced".to_string(),
                ),
            },
            Self::Compile(source) => SchemaError::Compile(source.clone()),
            Self::Integrity(message) => SchemaError::Integrity(message.clone()),
        }
    }
}

impl EmbeddedSchema {
    fn validator(&'static self) -> Result<&'static jsonschema::Validator, SchemaError> {
        match self.compiled.get_or_init(|| {
            let value = canon::parse_strict(self.source.as_bytes())
                .map_err(|_| CachedSchemaError::Parse)?;
            compile_validator(&value).map_err(CachedSchemaError::Compile)
        }) {
            Ok(validator) => Ok(validator),
            Err(error) => Err(error.to_public(self.source)),
        }
    }

    fn definition_validator(
        &'static self,
        definition: &str,
    ) -> Result<&'static jsonschema::Validator, SchemaError> {
        let validators = match self
            .compiled_definitions
            .get_or_init(|| compile_definition_validators(self.source))
        {
            Ok(validators) => validators,
            Err(error) => return Err(error.to_public(self.source)),
        };
        validators
            .get(definition)
            .ok_or_else(|| SchemaError::UnknownDefinition {
                schema_id: self.descriptor.schema_id.to_string(),
                definition: definition.to_string(),
            })
    }
}

fn compile_definition_validators(
    source: &str,
) -> Result<BTreeMap<String, jsonschema::Validator>, CachedSchemaError> {
    let schema = canon::parse_strict(source.as_bytes()).map_err(|_| CachedSchemaError::Parse)?;
    let definitions = schema
        .get("$defs")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| {
            CachedSchemaError::Integrity("embedded schema has no `$defs` object".into())
        })?;
    let mut validators = BTreeMap::new();
    for definition in definitions.keys() {
        let wrapper = serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$defs": definitions,
            "$ref": format!("#/$defs/{}", escape_pointer_token(definition)),
        });
        let validator = compile_validator(&wrapper).map_err(CachedSchemaError::Compile)?;
        validators.insert(definition.clone(), validator);
    }
    Ok(validators)
}

fn escape_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

fn compile_validator(
    schema: &serde_json::Value,
) -> Result<jsonschema::Validator, SchemaCompilationError> {
    jsonschema::options()
        .with_retriever(EmbeddedSchemaRetriever)
        .build(schema)
        .map_err(|source| SchemaCompilationError {
            source: Arc::new(source),
        })
}

fn collect_schema_violations<'instance>(
    schema_id: &str,
    instance: &serde_json::Value,
    errors: impl Iterator<Item = jsonschema::ValidationError<'instance>>,
) -> Option<SchemaViolations> {
    collect_branch_violations(schema_id, instance, errors, None)
}

fn collect_branch_violations<'instance>(
    schema_id: &str,
    instance: &serde_json::Value,
    errors: impl Iterator<Item = jsonschema::ValidationError<'instance>>,
    branch: Option<usize>,
) -> Option<SchemaViolations> {
    let mut retained = BinaryHeap::with_capacity(MAX_SCHEMA_VIOLATIONS + 1);
    let mut total = 0usize;
    let mut retain = |error: &jsonschema::ValidationError<'_>, refine: bool| {
        total = total.saturating_add(1);
        let mut violation = schema_violation(error, instance);
        if refine {
            if let Some(expectation) = scalar_union_expectation(error) {
                violation.expectation = bounded_schema_text(expectation);
            }
        }
        retained.push(violation);
        if retained.len() > MAX_SCHEMA_VIOLATIONS {
            retained.pop();
        }
    };
    for error in errors {
        let selected = branch.and_then(|index| {
            if !error.instance_path().as_str().is_empty()
                || error.schema_path().as_str() != "/oneOf"
            {
                return None;
            }
            match error.kind() {
                jsonschema::error::ValidationErrorKind::OneOfNotValid { context } => {
                    context.get(index).filter(|errors| !errors.is_empty())
                }
                _ => None,
            }
        });
        if let Some(selected) = selected {
            for child in selected {
                retain(child, true);
            }
        } else {
            retain(&error, false);
        }
    }
    (total != 0).then(|| {
        let violations = retained.into_sorted_vec();
        SchemaViolations {
            schema_id: bounded_schema_text(schema_id),
            truncated_count: total.saturating_sub(violations.len()),
            violations,
        }
    })
}

/// Report necessary string-constant requirements from a failed scalar union.
/// Observed failures do not prove that these are the branches' only constraints:
/// the list does not assert that substituting a listed string makes input valid.
/// Other branch obligations remain conditional and cannot be flattened.
fn scalar_union_expectation(error: &jsonschema::ValidationError<'_>) -> Option<String> {
    use jsonschema::error::ValidationErrorKind;

    let ValidationErrorKind::OneOfNotValid { context } = error.kind() else {
        return None;
    };
    if context.is_empty() || context.len() > MAX_SCHEMA_VIOLATIONS {
        return None;
    }
    let mut values = Vec::with_capacity(context.len());
    let mut characters = 0usize;
    for branch in context {
        let [failure] = branch.as_slice() else {
            return None;
        };
        if failure.instance_path() != error.instance_path() {
            return None;
        }
        let ValidationErrorKind::Constant {
            expected_value: serde_json::Value::String(value),
        } = failure.kind()
        else {
            return None;
        };
        characters = characters.saturating_add(value.chars().count());
        if characters > MAX_SCHEMA_DIAGNOSTIC_CHARS {
            return None;
        }
        values.push(value);
    }
    values.sort_unstable();
    values.dedup();
    let values = values
        .into_iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    Some(format!(
        "one of the schema-declared strings: {}",
        values.join(", ")
    ))
}

fn schema_violation(
    error: &jsonschema::ValidationError<'_>,
    instance: &serde_json::Value,
) -> SchemaViolation {
    let instance_path = error.instance_path().to_string();
    let schema_path = error.schema_path().to_string();
    let keyword = error.kind().keyword();
    SchemaViolation {
        instance_path_identity: pale_core::sha256_hex(instance_path.as_bytes()),
        schema_path_identity: pale_core::sha256_hex(schema_path.as_bytes()),
        keyword_identity: pale_core::sha256_hex(keyword.as_bytes()),
        instance_path_segments: resolve_instance_path(instance, &instance_path),
        instance_path: bounded_schema_text(instance_path),
        schema_path: bounded_schema_text(schema_path),
        keyword: bounded_schema_text(keyword),
        semantic_discriminator: schema_semantic_discriminator(error.kind()),
        expectation: bounded_schema_text(schema_expectation(error.kind())),
    }
}

/// Resolve an RFC 6901 pointer without guessing whether a numeric token is a
/// property or an array index.
fn resolve_instance_path(
    instance: &serde_json::Value,
    pointer: &str,
) -> Option<Vec<SchemaInstancePathSegment>> {
    if pointer.is_empty() {
        return Some(Vec::new());
    }
    let tokens = pointer.strip_prefix('/')?.split('/');
    let mut current = instance;
    let mut path = Vec::new();
    for encoded in tokens {
        if path.len() == MAX_SCHEMA_INSTANCE_PATH_SEGMENTS {
            return None;
        }
        let token = decode_pointer_token(encoded)?;
        match current {
            serde_json::Value::Object(properties) => {
                current = properties.get(&token)?;
                path.push(SchemaInstancePathSegment::Property(token));
            }
            serde_json::Value::Array(items) => {
                if token.len() > 1 && token.starts_with('0') {
                    return None;
                }
                let index = token.parse::<usize>().ok()?;
                current = items.get(index)?;
                path.push(SchemaInstancePathSegment::Index(index));
            }
            _ => return None,
        }
    }
    Some(path)
}

fn decode_pointer_token(encoded: &str) -> Option<String> {
    let mut token = String::with_capacity(encoded.len());
    let mut characters = encoded.chars();
    while let Some(character) = characters.next() {
        if character != '~' {
            token.push(character);
            continue;
        }
        token.push(match characters.next()? {
            '0' => '~',
            '1' => '/',
            _ => return None,
        });
    }
    Some(token)
}

fn schema_semantic_discriminator(kind: &jsonschema::error::ValidationErrorKind) -> String {
    use jsonschema::error::ValidationErrorKind;

    let tag = match kind {
        ValidationErrorKind::AdditionalItems { .. } => "additional_items",
        ValidationErrorKind::AdditionalProperties { .. } => "additional_properties",
        ValidationErrorKind::AnyOf { .. } => "any_of",
        ValidationErrorKind::BacktrackLimitExceeded { .. } => "backtrack_limit_exceeded",
        ValidationErrorKind::RegexEngineFailure { .. } => "regex_engine_failure",
        ValidationErrorKind::Constant { .. } => "constant",
        ValidationErrorKind::Contains => "contains",
        ValidationErrorKind::ContentEncoding { .. } => "content_encoding",
        ValidationErrorKind::ContentMediaType { .. } => "content_media_type",
        ValidationErrorKind::Custom { .. } => "custom",
        ValidationErrorKind::Enum { .. } => "enum",
        ValidationErrorKind::ExclusiveMaximum { .. } => "exclusive_maximum",
        ValidationErrorKind::ExclusiveMinimum { .. } => "exclusive_minimum",
        ValidationErrorKind::FalseSchema => "false_schema",
        ValidationErrorKind::Format { .. } => "format",
        ValidationErrorKind::FromUtf8 { .. } => "from_utf8",
        ValidationErrorKind::MaxItems { .. } => "max_items",
        ValidationErrorKind::Maximum { .. } => "maximum",
        ValidationErrorKind::MaxLength { .. } => "max_length",
        ValidationErrorKind::MaxProperties { .. } => "max_properties",
        ValidationErrorKind::MinItems { .. } => "min_items",
        ValidationErrorKind::Minimum { .. } => "minimum",
        ValidationErrorKind::MinLength { .. } => "min_length",
        ValidationErrorKind::MinProperties { .. } => "min_properties",
        ValidationErrorKind::MultipleOf { .. } => "multiple_of",
        ValidationErrorKind::Not { .. } => "not",
        ValidationErrorKind::OneOfMultipleValid { .. } => "one_of_multiple_valid",
        ValidationErrorKind::OneOfNotValid { .. } => "one_of_not_valid",
        ValidationErrorKind::Pattern { .. } => "pattern",
        ValidationErrorKind::PropertyNames { .. } => "property_names",
        ValidationErrorKind::Required { property } => {
            // A single `required` keyword may emit one violation per missing
            // schema-owned property at the same instance and schema paths.
            // Hash the complete canonical property rather than a bounded
            // display prefix so long names cannot alias.
            let digest = pale_core::canon::digest(property)
                .expect("a serde JSON value always has a canonical representation");
            return format!("required:{digest}");
        }
        ValidationErrorKind::Type { .. } => "type",
        ValidationErrorKind::UnevaluatedItems { .. } => "unevaluated_items",
        ValidationErrorKind::UnevaluatedProperties { .. } => "unevaluated_properties",
        ValidationErrorKind::UniqueItems => "unique_items",
        ValidationErrorKind::Referencing(_) => "referencing",
    };
    tag.to_string()
}

fn schema_expectation(kind: &jsonschema::error::ValidationErrorKind) -> String {
    use jsonschema::error::{TypeKind, ValidationErrorKind};

    match kind {
        ValidationErrorKind::AdditionalItems { limit } => format!("at most {limit} item(s)"),
        ValidationErrorKind::AdditionalProperties { unexpected } => {
            format!("no undeclared properties ({} found)", unexpected.len())
        }
        ValidationErrorKind::AnyOf { .. } => "at least one declared alternative".into(),
        ValidationErrorKind::BacktrackLimitExceeded { .. } => {
            "pattern evaluation within the validator limit".into()
        }
        ValidationErrorKind::RegexEngineFailure { .. } => {
            "successful evaluation of the declared pattern".into()
        }
        ValidationErrorKind::Constant { .. } => "the schema-declared constant".into(),
        ValidationErrorKind::Contains => "an item satisfying the declared subschema".into(),
        ValidationErrorKind::ContentEncoding { content_encoding } => {
            format!("content encoded as `{content_encoding}`")
        }
        ValidationErrorKind::ContentMediaType { content_media_type } => {
            format!("content with media type `{content_media_type}`")
        }
        ValidationErrorKind::Custom { keyword, .. } => {
            format!("custom `{keyword}` constraint")
        }
        ValidationErrorKind::Enum { .. } => "one of the schema-declared values".into(),
        ValidationErrorKind::ExclusiveMaximum { limit } => format!("a value below {limit}"),
        ValidationErrorKind::ExclusiveMinimum { limit } => format!("a value above {limit}"),
        ValidationErrorKind::FalseSchema => "no value is admitted by this schema".into(),
        ValidationErrorKind::Format { format } => format!("format `{format}`"),
        ValidationErrorKind::FromUtf8 { .. } => "valid UTF-8 content".into(),
        ValidationErrorKind::MaxItems { limit } => format!("at most {limit} item(s)"),
        ValidationErrorKind::Maximum { limit } => format!("a value no greater than {limit}"),
        ValidationErrorKind::MaxLength { limit } => {
            format!("a string of at most {limit} character(s)")
        }
        ValidationErrorKind::MaxProperties { limit } => {
            format!("at most {limit} properties")
        }
        ValidationErrorKind::MinItems { limit } => format!("at least {limit} item(s)"),
        ValidationErrorKind::Minimum { limit } => format!("a value no less than {limit}"),
        ValidationErrorKind::MinLength { limit } => {
            format!("a string of at least {limit} character(s)")
        }
        ValidationErrorKind::MinProperties { limit } => {
            format!("at least {limit} properties")
        }
        ValidationErrorKind::MultipleOf { multiple_of } => {
            format!("a multiple of {multiple_of}")
        }
        ValidationErrorKind::Not { .. } => "a value excluded by the negated subschema".into(),
        ValidationErrorKind::OneOfMultipleValid { .. } => {
            "exactly one declared alternative (more than one matched)".into()
        }
        ValidationErrorKind::OneOfNotValid { .. } => {
            "exactly one declared alternative (none matched)".into()
        }
        ValidationErrorKind::Pattern { pattern } => {
            format!("a string matching pattern `{pattern}`")
        }
        ValidationErrorKind::PropertyNames { .. } => {
            "property names satisfying the declared subschema".into()
        }
        ValidationErrorKind::Required { property } => {
            format!("required property {property}")
        }
        ValidationErrorKind::Type { kind } => match kind {
            TypeKind::Single(kind) => format!("JSON type `{kind}`"),
            TypeKind::Multiple(kinds) => format!(
                "one of JSON types {}",
                kinds
                    .iter()
                    .map(|kind| format!("`{kind}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        },
        ValidationErrorKind::UnevaluatedItems { unexpected } => {
            format!("no unevaluated items ({} found)", unexpected.len())
        }
        ValidationErrorKind::UnevaluatedProperties { unexpected } => {
            format!("no unevaluated properties ({} found)", unexpected.len())
        }
        ValidationErrorKind::UniqueItems => "unique array items".into(),
        ValidationErrorKind::Referencing(_) => "a resolvable embedded schema reference".into(),
    }
}

fn bounded_schema_text(value: impl fmt::Display) -> String {
    let mut text = value.to_string();
    if let Some((boundary, _)) = text.char_indices().nth(MAX_SCHEMA_DIAGNOSTIC_CHARS) {
        text.truncate(boundary);
        text.push('…');
    }
    text
}

#[derive(Clone, Copy)]
struct EmbeddedSchemaRetriever;

impl jsonschema::Retrieve for EmbeddedSchemaRetriever {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        ensure_registry_integrity().map_err(|error| error.to_string())?;
        for embedded in EMBEDDED_SCHEMAS.iter() {
            if embedded.descriptor.schema_uri == uri.as_str() {
                return Ok(canon::parse_strict(embedded.source.as_bytes())?);
            }
        }
        Err(format!("embedded schema resource not found: {uri}").into())
    }
}

/// Resolve one generated schema descriptor by registry ID.
///
/// # Errors
///
/// Returns a registry-integrity failure before exposing metadata, or
/// [`SchemaError::UnknownId`] for an ID outside the compiled set.
pub fn schema_descriptor(id: &str) -> Result<&'static SchemaDescriptor, SchemaError> {
    ensure_registry_integrity()?;
    EMBEDDED_SCHEMAS
        .iter()
        .find(|schema| schema.descriptor.schema_id == id)
        .map(|schema| &schema.descriptor)
        .ok_or_else(|| SchemaError::UnknownId(id.to_string()))
}

/// Resolve one editor-only annotated schema by descriptor-registry identity.
///
/// This lookup verifies the sealed registry first so editor help cannot outlive
/// its corresponding contract producer. The returned annotation source is not
/// accepted as a contract or authority coordinate.
///
/// # Errors
///
/// Returns a registry-integrity failure, or [`SchemaError::UnknownId`] when the
/// identity is not one of the explicit authoring grammars.
pub fn authoring_schema(id: &str) -> Result<&'static AuthoringSchema, SchemaError> {
    ensure_registry_integrity()?;
    AUTHORING_SCHEMAS
        .iter()
        .find(|schema| schema.schema_id == id)
        .ok_or_else(|| SchemaError::UnknownId(id.to_owned()))
}

/// Derive an exact self-carried record identity from one generated descriptor.
///
/// The caller supplies the manifest registry ID. This function never scans or
/// infers a descriptor from a record's wire spelling.
///
/// # Errors
///
/// Returns the ordinary registry-integrity/unknown-ID errors, or an integrity
/// error when the descriptor has no exact top-level wire identity or declares
/// that its raw-schema identity is inherited from an outer record.
pub fn record_schema_identity(registry_id: &str) -> Result<RecordSchemaIdentity, SchemaError> {
    let descriptor = schema_descriptor(registry_id)?;
    if descriptor.raw_schema_sha256_carriage != "self_carried" {
        return Err(SchemaError::Integrity(format!(
            "descriptor `{registry_id}` does not self-carry raw schema identity"
        )));
    }
    let wire_schema_id = descriptor.wire_schema_id.ok_or_else(|| {
        SchemaError::Integrity(format!(
            "descriptor `{registry_id}` has no exact wire schema identity"
        ))
    })?;
    let schema = SchemaId::parse(wire_schema_id).map_err(|error| {
        SchemaError::Integrity(format!(
            "descriptor `{registry_id}` has an invalid wire schema identity: {error}"
        ))
    })?;
    let schema_sha256 = SchemaDigest::parse(descriptor.schema_sha256).map_err(|error| {
        SchemaError::Integrity(format!(
            "descriptor `{registry_id}` has an invalid raw schema digest: {error}"
        ))
    })?;
    Ok(RecordSchemaIdentity {
        schema,
        schema_sha256,
    })
}

/// Resolve one self-carried record contract by its exact wire schema identity.
///
/// Use this lookup when an already decoded record supplies its wire `schema`
/// value. Manifest registry IDs remain the input to [`record_schema_identity`].
///
/// # Errors
///
/// Returns a registry-integrity failure, [`SchemaError::UnknownId`] when the
/// wire identity is absent, or the same record-identity errors as
/// [`record_schema_identity`].
pub fn record_schema_identity_by_wire_id(
    wire_schema_id: &str,
) -> Result<RecordSchemaIdentity, SchemaError> {
    ensure_registry_integrity()?;
    let mut matches = EMBEDDED_SCHEMAS
        .iter()
        .filter(|schema| schema.descriptor.wire_schema_id == Some(wire_schema_id));
    let descriptor = matches
        .next()
        .map(|schema| &schema.descriptor)
        .ok_or_else(|| SchemaError::UnknownId(wire_schema_id.to_owned()))?;
    if matches.next().is_some() {
        return Err(SchemaError::Integrity(format!(
            "wire schema identity `{wire_schema_id}` is ambiguous"
        )));
    }
    record_schema_identity(descriptor.schema_id)
}

/// Admit a carried standalone-record identity against an explicitly selected descriptor.
///
/// # Errors
///
/// Returns the same errors as [`record_schema_identity`], or
/// [`SchemaError::IdentityMismatch`] when either typed member differs.
pub fn admit_record_schema_identity(
    expected_registry_id: &str,
    claimed: &RecordSchemaIdentity,
) -> Result<(), SchemaError> {
    let expected = record_schema_identity(expected_registry_id)?;
    if &expected == claimed {
        Ok(())
    } else {
        Err(SchemaError::IdentityMismatch(
            expected_registry_id.to_string(),
        ))
    }
}

/// Resolve the exact compiled manifest and schema-set identities.
///
/// This returns the binary's captured build-time authority. It performs no
/// runtime filesystem discovery or freshness check against source files.
///
/// # Errors
///
/// Returns [`SchemaError::Integrity`] unless every generated descriptor,
/// captured schema byte string, manifest classification, and set member agrees.
pub fn schema_registry_identity() -> Result<SchemaRegistryIdentity, SchemaError> {
    ensure_registry_integrity()
}

/// Return the number of active members in the verified embedded registry.
///
/// The count is derived from the same generated descriptor table as
/// [`schema_registry_identity`]; it is not a separately maintained registry
/// constant.
///
/// # Errors
///
/// Returns [`SchemaError::Integrity`] unless the complete embedded registry is
/// internally coherent.
pub fn schema_count() -> Result<usize, SchemaError> {
    ensure_registry_integrity()?;
    Ok(EMBEDDED_SCHEMAS.len())
}

/// Borrow the exact UTF-8 source captured for a canonical schema at build time.
///
/// # Errors
///
/// Returns [`SchemaError::Integrity`] when the compiled registry is incoherent
/// or [`SchemaError::UnknownId`] when `id` is not an active registry identity.
pub fn embedded_schema(id: &str) -> Result<&'static str, SchemaError> {
    ensure_registry_integrity()?;
    EMBEDDED_SCHEMAS
        .iter()
        .find(|schema| schema.descriptor.schema_id == id)
        .map(|schema| schema.source)
        .ok_or_else(|| SchemaError::UnknownId(id.to_string()))
}

/// Process-local handle to one cached embedded-schema validator.
///
/// This lower-level handle checks JSON Schema shape only. Use [`validate_value`]
/// or [`validate_record`] when descriptor-declared self-carried identity must
/// also be admitted.
pub struct CompiledSchema {
    schema_id: &'static str,
    inner: &'static jsonschema::Validator,
}

impl CompiledSchema {
    /// Reuse the process-local validator for a canonical embedded schema.
    ///
    /// # Errors
    ///
    /// Returns an unknown-contract or embedded-schema failure before exposing
    /// a validator handle.
    pub fn for_id(id: &str) -> Result<Self, SchemaError> {
        ensure_registry_integrity()?;
        let schema = EMBEDDED_SCHEMAS
            .iter()
            .find(|schema| schema.descriptor.schema_id == id)
            .ok_or_else(|| SchemaError::UnknownId(id.to_string()))?;
        Ok(Self {
            schema_id: schema.descriptor.schema_id,
            inner: schema.validator()?,
        })
    }

    /// Validate `instance` against the compiled JSON Schema shape.
    ///
    /// Unlike [`validate_value`], this does not compare the record's `schema` or
    /// `schema_sha256` members with descriptor identity. Rejections retain a
    /// bounded, deterministic [`SchemaViolations`] value rather than collapsing
    /// the validator result to a Boolean.
    ///
    /// # Errors
    ///
    /// Returns the safe structured violations reported for `instance`.
    pub fn validate(&self, instance: &serde_json::Value) -> Result<(), SchemaViolations> {
        match collect_schema_violations(self.schema_id, instance, self.inner.iter_errors(instance))
        {
            Some(violations) => Err(violations),
            None => Ok(()),
        }
    }
}

// Cache success and failure alike so corrupt embedded authority cannot produce
// repeated compilation work or inconsistent answers within one process.
static REGISTRY_INTEGRITY: OnceLock<Result<SchemaRegistryIdentity, CachedSchemaError>> =
    OnceLock::new();

fn ensure_registry_integrity() -> Result<SchemaRegistryIdentity, SchemaError> {
    match REGISTRY_INTEGRITY.get_or_init(verify_registry_integrity) {
        Ok(identity) => Ok(identity.clone()),
        Err(error) => Err(error.to_public("")),
    }
}

fn verify_registry_integrity() -> Result<SchemaRegistryIdentity, CachedSchemaError> {
    let manifest_sha256 = SchemaManifestDigest::parse(SCHEMA_MANIFEST_SHA256)
        .map_err(|error| CachedSchemaError::Integrity(error.to_string()))?;
    let set_sha256 = SchemaSetDigest::parse(SCHEMA_SET_SHA256)
        .map_err(|error| CachedSchemaError::Integrity(error.to_string()))?;
    let manifest_preimage = canon::parse_strict(SCHEMA_MANIFEST_CANONICAL_PREIMAGE.as_bytes())
        .map_err(|error| CachedSchemaError::Integrity(error.to_string()))?;
    let computed_manifest = SchemaManifestDigest::hash_canonical(&manifest_preimage)
        .map_err(|error| CachedSchemaError::Integrity(error.to_string()))?;
    if computed_manifest != manifest_sha256 {
        return Err(CachedSchemaError::Integrity(
            "embedded canonical manifest preimage has the wrong digest".into(),
        ));
    }
    if manifest_preimage.get("schema_manifest_sha256").is_some()
        || manifest_preimage
            .get("schema_set_sha256")
            .and_then(serde_json::Value::as_str)
            != Some(set_sha256.as_str())
    {
        return Err(CachedSchemaError::Integrity(
            "embedded manifest preimage has invalid self/set identity".into(),
        ));
    }
    let manifest_schemas = manifest_preimage
        .get("schemas")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| CachedSchemaError::Integrity("manifest schemas object is absent".into()))?;
    if manifest_schemas.len() != EMBEDDED_SCHEMAS.len() {
        return Err(CachedSchemaError::Integrity(
            "manifest/generated descriptor membership differs".into(),
        ));
    }

    let mut members = Vec::with_capacity(EMBEDDED_SCHEMAS.len());
    for embedded in EMBEDDED_SCHEMAS.iter() {
        let descriptor = &embedded.descriptor;
        verify_generated_source_identity(descriptor, embedded.source)?;
        let declared = SchemaDigest::parse(descriptor.schema_sha256)
            .map_err(|error| CachedSchemaError::Integrity(error.to_string()))?;
        let actual = SchemaDigest::hash_bytes(embedded.source.as_bytes());
        if actual != declared {
            return Err(CachedSchemaError::Integrity(format!(
                "captured source digest differs for `{}`",
                descriptor.schema_id
            )));
        }
        let canonical_entry = canon::parse_strict(descriptor.manifest_entry_canonical.as_bytes())
            .map_err(|error| CachedSchemaError::Integrity(error.to_string()))?;
        if manifest_schemas.get(descriptor.schema_id) != Some(&canonical_entry) {
            return Err(CachedSchemaError::Integrity(format!(
                "generated metadata differs for `{}`",
                descriptor.schema_id
            )));
        }
        if canonical_entry
            .pointer("/identity_carriage/raw_schema_sha256")
            .and_then(serde_json::Value::as_str)
            != Some(descriptor.raw_schema_sha256_carriage)
        {
            return Err(CachedSchemaError::Integrity(format!(
                "generated identity carriage differs for `{}`",
                descriptor.schema_id
            )));
        }
        members.push(SchemaSetMember {
            schema_id: descriptor.schema_id.to_string(),
            schema_sha256: declared,
        });
    }
    let computed_set = schema_set_digest(members)
        .map_err(|error| CachedSchemaError::Integrity(error.to_string()))?;
    if computed_set != set_sha256 {
        return Err(CachedSchemaError::Integrity(
            "captured embedded sources have the wrong schema-set digest".into(),
        ));
    }
    Ok(SchemaRegistryIdentity {
        schema_manifest_sha256: manifest_sha256,
        schema_set_sha256: set_sha256,
    })
}

fn verify_generated_source_identity(
    descriptor: &SchemaDescriptor,
    source: &str,
) -> Result<(), CachedSchemaError> {
    let parsed = canon::parse_strict(source.as_bytes())
        .map_err(|error| CachedSchemaError::Integrity(error.to_string()))?;
    let source_uri = parsed.get("$id").and_then(serde_json::Value::as_str);
    if source_uri != Some(descriptor.schema_uri) {
        return Err(CachedSchemaError::Integrity(format!(
            "generated schema URI differs from captured source for `{}`",
            descriptor.schema_id
        )));
    }
    let source_wire_schema_id = ["schema", "schema_id", "envelope_schema_id"]
        .into_iter()
        .filter_map(|field| {
            parsed
                .pointer(&format!("/properties/{field}/const"))
                .and_then(serde_json::Value::as_str)
        })
        .next();
    if source_wire_schema_id != descriptor.wire_schema_id {
        return Err(CachedSchemaError::Integrity(format!(
            "generated wire schema ID differs from captured source for `{}`",
            descriptor.schema_id
        )));
    }
    if let Some(wire_schema_id) = source_wire_schema_id {
        SchemaId::parse(wire_schema_id).map_err(|error| {
            CachedSchemaError::Integrity(format!(
                "generated wire schema ID is invalid for `{}`: {error}",
                descriptor.schema_id
            ))
        })?;
    }
    let declares_raw_schema_sha = parsed
        .get("properties")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|properties| {
            ["schema_sha256", "envelope_schema_sha256"]
                .iter()
                .any(|field| properties.contains_key(*field))
        });
    let requires_raw_schema_sha = parsed
        .get("required")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|required| {
            required.iter().any(|member| {
                matches!(
                    member.as_str(),
                    Some("schema_sha256" | "envelope_schema_sha256")
                )
            })
        });
    let self_carries_raw_schema = descriptor.raw_schema_sha256_carriage == "self_carried";
    if declares_raw_schema_sha != self_carries_raw_schema
        || requires_raw_schema_sha != self_carries_raw_schema
    {
        return Err(CachedSchemaError::Integrity(format!(
            "generated raw-schema carriage differs from captured source for `{}`",
            descriptor.schema_id
        )));
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/internal/registry.rs"]
mod tests;
