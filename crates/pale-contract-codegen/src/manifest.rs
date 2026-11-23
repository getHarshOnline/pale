//! Typed descriptor-manifest admission and all-or-nothing bundle generation.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};

use pale_core::canon;
use pale_core::records::{
    schema_set_digest, SchemaDigest, SchemaId, SchemaManifestDigest, SchemaSetDigest,
    SchemaSetMember,
};
use serde::{Deserialize, Serialize};
use serde_json::{ser::PrettyFormatter, Serializer, Value};

use crate::schema_catalog::{
    contract_producers, project_authoring_contract_set, project_contract_set,
};
use crate::schema_projection::{validate_schema_base_uri, AdmissionRoute};

/// Filename of the human-readable descriptor manifest inside a generated bundle.
pub const BUNDLE_MANIFEST_NAME: &str = "agent-surface-schemas.v1.json";
/// Maximum descriptor-manifest bytes admitted before allocation and parsing.
const MAX_MANIFEST_BYTES: u64 = 2 * 1024 * 1024;
/// Maximum bytes admitted for one schema resource before parsing.
const MAX_SCHEMA_BYTES: usize = 1024 * 1024;

/// A descriptor manifest or generated bundle violated its closed contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerationError {
    /// A bounded manifest, projection, identity, path, encoding, or I/O
    /// invariant failed.
    Contract {
        /// Human-readable cause that excludes schema instance values.
        message: String,
    },
    /// Generated cross-resource references disagree with manifest authority.
    ReferenceGraphMismatch {
        /// Exact `(referrer, referenced)` edges declared by the manifest.
        declared: BTreeSet<(String, String)>,
        /// Exact `(referrer, referenced)` edges emitted from Rust producers.
        generated: BTreeSet<(String, String)>,
    },
}

impl GenerationError {
    fn new(message: impl Into<String>) -> Self {
        Self::Contract {
            message: message.into(),
        }
    }
}

impl fmt::Display for GenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Contract { message } => formatter.write_str(message),
            Self::ReferenceGraphMismatch {
                declared,
                generated,
            } => write!(
                formatter,
                "generated cross-resource edges disagree with descriptor identity carriage: declared={declared:?}, generated={generated:?}"
            ),
        }
    }
}

impl std::error::Error for GenerationError {}

/// Typed descriptor manifest consumed by the generator.
///
/// Its `schemas` map is the active registry: every key must have exactly one
/// Rust producer, and no producer may exist outside the map.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DescriptorManifest {
    registry: String,
    canonicalization: String,
    release_state: String,
    schema_base_uri: String,
    schema_manifest_sha256: SchemaManifestDigest,
    schema_set_sha256: SchemaSetDigest,
    schemas: BTreeMap<String, ManifestEntry>,
}

impl DescriptorManifest {
    /// Return the manifest-owned canonical schema resource directory.
    #[must_use]
    pub fn schema_base_uri(&self) -> &str {
        &self.schema_base_uri
    }

    /// Return active descriptor identities in deterministic order.
    pub fn active_schema_ids(&self) -> impl ExactSizeIterator<Item = &str> {
        self.schemas.keys().map(String::as_str)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ManifestEntry {
    path: String,
    schema_sha256: SchemaDigest,
    family: String,
    schema_owner: String,
    record_authority: RecordAuthority,
    registration: Registration,
    lifecycle: Lifecycle,
    producers: Vec<Actor>,
    consumers: Vec<Actor>,
    persistence: Persistence,
    compatibility: Compatibility,
    migration: Migration,
    identity_carriage: IdentityCarriage,
}

macro_rules! string_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        enum $name {
            $($variant),+
        }
    };
}

string_enum!(RecordAuthority {
    Distribution,
    PaleSdk,
    PaleCompiler,
    PaleMaterializer,
    PaleRenderer,
    PaleRuntime,
    NativeAdapter,
    NativeOwner,
    PrincipalWorkspace,
    SharedDefinition,
});

string_enum!(LifecycleRole {
    AuthoredInput,
    DerivedRecord,
    DurableEvidence,
    DurableState,
    GeneratedProjection,
    NestedPayload,
    RuntimeRegistry,
    SharedDefinition,
    TransportWire,
});

string_enum!(CandidateDisposition {
    RetainInitialV1Candidate,
});

string_enum!(ActorType {
    AuthoritySurface,
    ProtocolParticipant,
    RustLogicalSymbol,
});

string_enum!(ImplementationState {
    ExternalNative,
    Observed,
    PlannedUnproved,
});

string_enum!(PersistenceClass {
    CrossProcessSecretWire,
    CrossProcessWire,
    DistributionSource,
    DurableEvidenceStore,
    DurableMachineState,
    GeneratedSurface,
    Memory,
    Nested,
    NestedAndSchemaProjection,
    NestedHostProtocol,
    None,
    PrincipalWorkspaceSource,
    SavedPlanArtifact,
});

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    mode: String,
    observed_embedding: String,
    observed_reachability: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Lifecycle {
    state: String,
    candidate_disposition: CandidateDisposition,
    target_family: String,
    role: LifecycleRole,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Actor {
    #[serde(rename = "type")]
    actor_type: ActorType,
    id: String,
    implementation_state: ImplementationState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Persistence {
    class: PersistenceClass,
    standalone: bool,
    classification_scope: String,
    observed_record_instance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Compatibility {
    obligation: String,
    external_adoption: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Migration {
    mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentityCarriage {
    raw_schema_sha256: String,
    outer_schema_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    wire_schema_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    wire_schema_version: Option<u32>,
}

/// Exact identities of one generated descriptor manifest and active schema set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleCoordinate {
    /// RFC 8785 digest of the manifest with its own digest field omitted.
    pub schema_manifest_sha256: SchemaManifestDigest,
    /// Domain-separated digest of every active generated schema byte identity.
    pub schema_set_sha256: SchemaSetDigest,
}

/// Complete generated bundle held in memory before any filesystem mutation.
#[derive(Debug)]
pub struct ContractBundle {
    manifest_bytes: Vec<u8>,
    manifest_canonical_preimage: String,
    schemas: BTreeMap<String, GeneratedSchema>,
    coordinate: BundleCoordinate,
}

impl ContractBundle {
    /// Borrow deterministic, two-space-indented manifest bytes.
    ///
    /// The checked-in form ends with one newline for human review. Its
    /// [`BundleCoordinate::schema_manifest_sha256`] is derived separately from
    /// the RFC 8785 preimage returned by [`Self::manifest_canonical_preimage`].
    #[must_use]
    pub fn manifest_bytes(&self) -> &[u8] {
        &self.manifest_bytes
    }

    /// Borrow the generated bundle coordinate.
    #[must_use]
    pub fn coordinate(&self) -> &BundleCoordinate {
        &self.coordinate
    }

    /// Borrow the RFC 8785 manifest preimage with only its self-digest omitted.
    ///
    /// Runtime registries can embed this value to re-establish the manifest
    /// identity without re-parsing a second build-time descriptor model.
    #[must_use]
    pub fn manifest_canonical_preimage(&self) -> &str {
        &self.manifest_canonical_preimage
    }

    /// Return generated schemas as `(registry identity, bytes)` pairs.
    pub fn schemas(&self) -> impl ExactSizeIterator<Item = (&str, &[u8])> {
        self.schemas
            .iter()
            .map(|(id, schema)| (id.as_str(), schema.bytes.as_slice()))
    }

    /// Return the generated schema descriptors needed by an embedding crate.
    pub fn schema_descriptors(
        &self,
    ) -> impl ExactSizeIterator<Item = GeneratedSchemaDescriptor<'_>> {
        self.schemas
            .iter()
            .map(|(schema_id, schema)| GeneratedSchemaDescriptor {
                schema_id,
                source_path: &schema.path,
                schema_uri: &schema.schema_uri,
                wire_schema_id: schema.wire_schema_id.as_deref(),
                wire_schema_version: schema.wire_schema_version,
                schema_sha256: &schema.schema_sha256,
                raw_schema_sha256_carriage: &schema.raw_schema_sha256_carriage,
                manifest_entry_canonical: &schema.manifest_entry_canonical,
                bytes: &schema.bytes,
            })
    }
}

#[derive(Debug)]
struct GeneratedSchema {
    path: PathBuf,
    bytes: Vec<u8>,
    schema_uri: String,
    wire_schema_id: Option<String>,
    wire_schema_version: Option<u32>,
    schema_sha256: SchemaDigest,
    raw_schema_sha256_carriage: String,
    manifest_entry_canonical: String,
}

/// Borrowed descriptor for one generated bundle member.
#[derive(Debug, Clone, Copy)]
pub struct GeneratedSchemaDescriptor<'a> {
    schema_id: &'a str,
    source_path: &'a Path,
    schema_uri: &'a str,
    wire_schema_id: Option<&'a str>,
    wire_schema_version: Option<u32>,
    schema_sha256: &'a SchemaDigest,
    raw_schema_sha256_carriage: &'a str,
    manifest_entry_canonical: &'a str,
    bytes: &'a [u8],
}

impl<'a> GeneratedSchemaDescriptor<'a> {
    /// Return the manifest registry identity.
    #[must_use]
    pub fn schema_id(self) -> &'a str {
        self.schema_id
    }

    /// Return the path relative to the generated bundle root.
    #[must_use]
    pub fn source_path(self) -> &'a Path {
        self.source_path
    }

    /// Return the manifest-owned canonical `$id` URI.
    #[must_use]
    pub fn schema_uri(self) -> &'a str {
        self.schema_uri
    }

    /// Return the exact root wire identity, when the record carries one.
    #[must_use]
    pub fn wire_schema_id(self) -> Option<&'a str> {
        self.wire_schema_id
    }

    /// Return the exact root wire version, when the record carries one.
    #[must_use]
    pub fn wire_schema_version(self) -> Option<u32> {
        self.wire_schema_version
    }

    /// Return the SHA-256 identity of the exact generated schema bytes.
    #[must_use]
    pub fn schema_sha256(self) -> &'a SchemaDigest {
        self.schema_sha256
    }

    /// Return whether the record itself carries its raw schema digest.
    #[must_use]
    pub fn raw_schema_sha256_carriage(self) -> &'a str {
        self.raw_schema_sha256_carriage
    }

    /// Return the RFC 8785 descriptor-manifest entry.
    #[must_use]
    pub fn manifest_entry_canonical(self) -> &'a str {
        self.manifest_entry_canonical
    }

    /// Borrow the exact generated UTF-8 JSON Schema bytes.
    #[must_use]
    pub fn bytes(self) -> &'a [u8] {
        self.bytes
    }
}

/// Read one strict, bounded descriptor manifest without generating or writing.
///
/// # Errors
///
/// Returns an error for a non-regular path, oversized or non-strict JSON, an
/// unknown field, or malformed descriptor metadata.
pub fn read_descriptor_manifest(path: &Path) -> Result<DescriptorManifest, GenerationError> {
    let bytes = read_regular_bounded(path, MAX_MANIFEST_BYTES)?;
    let value = canon::parse_strict(&bytes)
        .map_err(|error| GenerationError::new(format!("strict manifest parse failed: {error}")))?;
    if value.get("schema_base_uri").is_none() {
        return Err(GenerationError::new(
            "manifest is missing required manifest-owned schema_base_uri",
        ));
    }
    let manifest: DescriptorManifest = serde_json::from_value(value)
        .map_err(|error| GenerationError::new(format!("manifest metadata is invalid: {error}")))?;
    validate_manifest(&manifest)?;
    Ok(manifest)
}

/// Render and seal all active schemas in memory.
///
/// This function asserts producer bijection, identity-carriage constants,
/// generated cross-resource edges, and closed resource references. It never
/// reads predecessor schema bytes and never repairs Rust or manifest meaning.
///
/// # Errors
///
/// Returns an error on any catalog, manifest, projection, identity, reference,
/// path, canonicalization, or digest failure.
pub fn generate_bundle(
    mut manifest: DescriptorManifest,
) -> Result<ContractBundle, GenerationError> {
    validate_manifest(&manifest)?;
    let producer_ids = contract_producers()
        .into_iter()
        .map(|producer| producer.schema_id().to_string())
        .collect::<BTreeSet<_>>();
    let active_ids = manifest.schemas.keys().cloned().collect::<BTreeSet<_>>();
    if producer_ids != active_ids {
        return Err(GenerationError::new(format!(
            "producer/active-map mismatch: producers={producer_ids:?}, active={active_ids:?}"
        )));
    }
    validate_admission_qualification(&manifest)?;

    let projected = project_contract_set(&manifest.schema_base_uri)
        .map_err(|error| GenerationError::new(format!("project contract set: {error}")))?;
    let mut parsed = BTreeMap::new();
    let mut generated = BTreeMap::new();
    for (schema_id, value) in projected {
        let entry = manifest
            .schemas
            .get(&schema_id)
            .ok_or_else(|| GenerationError::new(format!("orphan producer `{schema_id}`")))?;
        validate_generated_resource(&schema_id, entry, &manifest.schema_base_uri, &value)?;
        let bytes = pretty_json_document(&value)?;
        if bytes.len() > MAX_SCHEMA_BYTES {
            return Err(GenerationError::new(format!(
                "generated schema `{schema_id}` exceeds {MAX_SCHEMA_BYTES} bytes"
            )));
        }
        let path = confined_relative_path(&entry.path)?;
        if path == Path::new(BUNDLE_MANIFEST_NAME) {
            return Err(GenerationError::new(format!(
                "schema `{schema_id}` collides with the bundle manifest path"
            )));
        }
        parsed.insert(schema_id.clone(), value);
        generated.insert(
            schema_id.clone(),
            GeneratedSchema {
                path,
                bytes,
                schema_uri: format!("{}{schema_id}.json", manifest.schema_base_uri),
                wire_schema_id: entry.identity_carriage.wire_schema_id.clone(),
                wire_schema_version: entry.identity_carriage.wire_schema_version,
                schema_sha256: entry.schema_sha256.clone(),
                raw_schema_sha256_carriage: entry.identity_carriage.raw_schema_sha256.clone(),
                manifest_entry_canonical: String::new(),
            },
        );
    }

    validate_generated_reference_graph(&manifest, &parsed)?;
    for (schema_id, schema) in &mut generated {
        let entry = manifest.schemas.get_mut(schema_id).ok_or_else(|| {
            GenerationError::new(format!(
                "active descriptor `{schema_id}` disappeared during generation"
            ))
        })?;
        entry.schema_sha256 = SchemaDigest::hash_bytes(&schema.bytes);
        schema.schema_sha256 = entry.schema_sha256.clone();
    }
    let schema_set_sha256 =
        schema_set_digest(manifest.schemas.iter().map(|(id, entry)| SchemaSetMember {
            schema_id: id.clone(),
            schema_sha256: entry.schema_sha256.clone(),
        }))
        .map_err(|error| GenerationError::new(format!("schema-set digest failed: {error}")))?;
    manifest.schema_set_sha256 = schema_set_sha256.clone();

    let mut manifest_preimage = serde_json::to_value(&manifest)
        .map_err(|error| GenerationError::new(format!("encode manifest preimage: {error}")))?;
    manifest_preimage
        .as_object_mut()
        .ok_or_else(|| GenerationError::new("typed manifest did not serialize as an object"))?
        .remove("schema_manifest_sha256")
        .ok_or_else(|| GenerationError::new("typed manifest omitted its self-digest"))?;
    let manifest_canonical_preimage = canon::to_canonical_string(&manifest_preimage)
        .map_err(|error| GenerationError::new(format!("manifest preimage failed: {error}")))?;
    let schema_manifest_sha256 = SchemaManifestDigest::hash_canonical(&manifest_preimage)
        .map_err(|error| GenerationError::new(format!("manifest digest failed: {error}")))?;
    manifest.schema_manifest_sha256 = schema_manifest_sha256.clone();
    let manifest_bytes = pretty_json_document(&manifest)?;
    for (schema_id, schema) in &mut generated {
        let entry = manifest.schemas.get(schema_id).ok_or_else(|| {
            GenerationError::new(format!(
                "active descriptor `{schema_id}` disappeared before sealing"
            ))
        })?;
        schema.manifest_entry_canonical = canon::to_canonical_string(entry).map_err(|error| {
            GenerationError::new(format!(
                "descriptor `{schema_id}` canonicalization failed: {error}"
            ))
        })?;
    }

    Ok(ContractBundle {
        manifest_bytes,
        manifest_canonical_preimage,
        schemas: generated,
        coordinate: BundleCoordinate {
            schema_manifest_sha256,
            schema_set_sha256,
        },
    })
}

/// Generate annotated editor schemas for an explicit subset of Rust producers.
///
/// These bytes retain Rustdoc-derived descriptions but are deliberately
/// excluded from [`ContractBundle`], descriptor manifests, and schema-set
/// identities. They are build-time assistance for configuration editors, not
/// portable validation authority.
///
/// # Errors
///
/// Returns an error when the schema base URI is invalid, a requested producer
/// is absent, projection fails, or an annotated schema exceeds the bounded
/// editor-resource ceiling.
pub fn generate_authoring_schemas<'a>(
    schema_base_uri: &str,
    schema_ids: impl IntoIterator<Item = &'a str>,
) -> Result<BTreeMap<String, Vec<u8>>, GenerationError> {
    const MAX_AUTHORING_SCHEMA_BYTES: usize = MAX_SCHEMA_BYTES * 4;

    let roots = schema_ids
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    let projected = project_authoring_contract_set(schema_base_uri)
        .map_err(|error| GenerationError::new(format!("project authoring schemas: {error}")))?;
    let available = projected.keys().cloned().collect::<BTreeSet<_>>();
    let missing = roots.difference(&available).cloned().collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(GenerationError::new(format!(
            "unknown authoring schema producer(s): {missing:?}"
        )));
    }
    let mut requested = roots.clone();
    let mut pending = roots.into_iter().collect::<Vec<_>>();
    while let Some(schema_id) = pending.pop() {
        let mut references = BTreeSet::new();
        collect_authoring_schema_references(
            &projected[&schema_id],
            schema_base_uri,
            &mut references,
        );
        for referenced in references {
            if !available.contains(&referenced) {
                return Err(GenerationError::new(format!(
                    "authoring schema `{schema_id}` references unknown producer `{referenced}`"
                )));
            }
            if requested.insert(referenced.clone()) {
                pending.push(referenced);
            }
        }
    }
    requested
        .into_iter()
        .map(|schema_id| {
            let value = &projected[&schema_id];
            let bytes = pretty_json_document(value)?;
            if bytes.len() > MAX_AUTHORING_SCHEMA_BYTES {
                return Err(GenerationError::new(format!(
                    "authoring schema `{schema_id}` exceeds {MAX_AUTHORING_SCHEMA_BYTES} bytes"
                )));
            }
            Ok((schema_id, bytes))
        })
        .collect()
}

/// Collect Pale-owned external references from one projected schema tree.
fn collect_authoring_schema_references(
    value: &Value,
    schema_base_uri: &str,
    references: &mut BTreeSet<String>,
) {
    match value {
        Value::Array(values) => {
            for value in values {
                collect_authoring_schema_references(value, schema_base_uri, references);
            }
        }
        Value::Object(object) => {
            if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
                if let Some(resource) = reference.strip_prefix(schema_base_uri) {
                    let resource_name = resource
                        .split_once('#')
                        .map_or(resource, |(resource, _)| resource);
                    let schema_id = resource_name.strip_suffix(".json").unwrap_or(resource_name);
                    references.insert(schema_id.to_owned());
                }
            }
            for child in object.values() {
                collect_authoring_schema_references(child, schema_base_uri, references);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

/// Join generator qualification to manifest-owned authority and lifecycle.
///
/// This check runs only while constructing the contract bundle. It keeps the
/// catalog's trust-transition classification mechanically attached to the
/// descriptor that names the record authority and producer; it does not make
/// the generator an admission dispatcher.
fn validate_admission_qualification(manifest: &DescriptorManifest) -> Result<(), GenerationError> {
    for producer in contract_producers() {
        let schema_id = producer.schema_id();
        let entry = manifest.schemas.get(schema_id).ok_or_else(|| {
            GenerationError::new(format!(
                "qualification producer `{schema_id}` has no manifest authority"
            ))
        })?;
        if entry.producers.is_empty() {
            return Err(GenerationError::new(format!(
                "qualification producer `{schema_id}` has no named construction/admission owner"
            )));
        }
        let owner_is_external = matches!(
            entry.record_authority,
            RecordAuthority::NativeAdapter | RecordAuthority::NativeOwner
        );
        let has_observed_owner = entry
            .producers
            .iter()
            .any(|actor| actor.implementation_state == ImplementationState::Observed);
        if !owner_is_external && !has_observed_owner {
            return Err(GenerationError::new(format!(
                "Pale-owned qualification `{schema_id}` has no observed owner witness: record_authority={:?}",
                entry.record_authority
            )));
        }

        let lifecycle_is_generated = entry.lifecycle.role == LifecycleRole::GeneratedProjection;
        let route_is_generated = producer.admission_route() == AdmissionRoute::Generated;
        if lifecycle_is_generated != route_is_generated {
            return Err(GenerationError::new(format!(
                "qualification route for `{schema_id}` disagrees with manifest lifecycle: route={:?}, record_authority={:?}, lifecycle={:?}",
                producer.admission_route(),
                entry.record_authority,
                entry.lifecycle.role
            )));
        }
    }
    Ok(())
}

/// Compare a generated bundle with an existing directory without mutation.
///
/// # Errors
///
/// Returns an error for a missing, extra, symlinked, non-regular, or byte-stale
/// member.
pub fn verify_bundle(bundle: &ContractBundle, directory: &Path) -> Result<(), GenerationError> {
    let actual = regular_file_inventory(directory)?;
    let mut expected = bundle
        .schemas
        .values()
        .map(|schema| schema.path.clone())
        .collect::<BTreeSet<_>>();
    expected.insert(PathBuf::from(BUNDLE_MANIFEST_NAME));
    if actual != expected {
        return Err(GenerationError::new(format!(
            "bundle membership drift: expected={expected:?}, actual={actual:?}"
        )));
    }
    compare_file(
        &directory.join(BUNDLE_MANIFEST_NAME),
        &bundle.manifest_bytes,
    )?;
    for schema in bundle.schemas.values() {
        compare_file(&directory.join(&schema.path), &schema.bytes)?;
    }
    Ok(())
}

/// Write a complete bundle through a private sibling directory and one rename.
///
/// The caller-selected destination must not already exist. Updating an existing
/// bundle therefore requires generating a new sibling coordinate and selecting
/// it separately; this function never exposes a partially written directory.
///
/// # Errors
///
/// Returns an error when the destination is invalid or exists, or any staged
/// write, durability sync, or final rename fails.
pub fn write_bundle_atomically(
    bundle: &ContractBundle,
    destination: &Path,
) -> Result<(), GenerationError> {
    if destination.exists() {
        return Err(GenerationError::new(format!(
            "bundle destination already exists: {}",
            destination.display()
        )));
    }
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or_else(|| GenerationError::new("bundle destination has no parent directory"))?;
    let metadata = fs::symlink_metadata(parent).map_err(|error| {
        GenerationError::new(format!(
            "inspect bundle parent {}: {error}",
            parent.display()
        ))
    })?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err(GenerationError::new(format!(
            "bundle parent is not a regular directory: {}",
            parent.display()
        )));
    }
    let name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| GenerationError::new("bundle destination name must be UTF-8"))?;
    let staging = allocate_staging_directory(parent, name)?;
    let mut guard = StagingGuard::new(staging.clone());

    write_synced(&staging.join(BUNDLE_MANIFEST_NAME), &bundle.manifest_bytes)?;
    for schema in bundle.schemas.values() {
        let path = staging.join(&schema.path);
        if let Some(schema_parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(schema_parent).map_err(|error| {
                GenerationError::new(format!("create {}: {error}", schema_parent.display()))
            })?;
        }
        write_synced(&path, &schema.bytes)?;
    }
    sync_tree_directories(&staging)?;
    fs::rename(&staging, destination).map_err(|error| {
        GenerationError::new(format!(
            "publish bundle {} -> {}: {error}",
            staging.display(),
            destination.display()
        ))
    })?;
    guard.disarm();
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            GenerationError::new(format!("sync bundle parent {}: {error}", parent.display()))
        })?;
    Ok(())
}

fn validate_manifest(manifest: &DescriptorManifest) -> Result<(), GenerationError> {
    if manifest.registry != "pale.agent-surface-schemas.v1"
        || manifest.canonicalization != "RFC8785-JCS"
        || manifest.release_state != "prepublication_candidate"
    {
        return Err(GenerationError::new(
            "manifest header does not match the initial-v1 descriptor contract",
        ));
    }
    validate_schema_base_uri(&manifest.schema_base_uri).map_err(GenerationError::new)?;
    reject_candidate_only_identifier(&manifest.registry, "registry id")?;
    let mut paths = BTreeSet::new();
    for (schema_id, entry) in &manifest.schemas {
        validate_logical_id(schema_id, "schema id")?;
        validate_manifest_entry(schema_id, entry)?;
        let path = confined_relative_path(&entry.path)?;
        if path.file_name().and_then(|name| name.to_str())
            != Some(format!("{schema_id}.json").as_str())
        {
            return Err(GenerationError::new(format!(
                "schema `{schema_id}` path basename must be `{schema_id}.json`"
            )));
        }
        if !paths.insert(path) {
            return Err(GenerationError::new(
                "two active descriptors claim the same bundle path",
            ));
        }
    }
    Ok(())
}

fn validate_manifest_entry(schema_id: &str, entry: &ManifestEntry) -> Result<(), GenerationError> {
    let _ = (
        entry.record_authority,
        entry.lifecycle.role,
        entry.persistence.class,
        entry.persistence.standalone,
    );
    validate_logical_id(&entry.family, "family")?;
    validate_logical_id(&entry.lifecycle.target_family, "target family")?;
    if entry.schema_owner != "pale"
        || entry.registration.mode != "runtime_embedded"
        || entry.registration.observed_embedding != "build_embedded"
        || entry.registration.observed_reachability
            != ["build_embedded".to_string(), "loader_active".to_string()]
        || entry.lifecycle.state != "current_dirty_worktree"
        || entry.lifecycle.candidate_disposition != CandidateDisposition::RetainInitialV1Candidate
        || entry.persistence.classification_scope != "design_boundary"
        || entry.persistence.observed_record_instance != "not_asserted"
        || entry.compatibility.obligation != "none_proved"
        || entry.compatibility.external_adoption != "no_evidence_observed"
        || entry.migration.mode != "none"
    {
        return Err(GenerationError::new(format!(
            "schema `{schema_id}` descriptor claims an unadmitted state"
        )));
    }
    validate_actors(schema_id, "producer", &entry.producers)?;
    validate_actors(schema_id, "consumer", &entry.consumers)?;
    if entry
        .identity_carriage
        .outer_schema_ids
        .iter()
        .collect::<BTreeSet<_>>()
        .len()
        != entry.identity_carriage.outer_schema_ids.len()
    {
        return Err(GenerationError::new(format!(
            "schema `{schema_id}` has malformed or duplicate outer schema ids"
        )));
    }
    for outer in &entry.identity_carriage.outer_schema_ids {
        validate_logical_id(outer, "outer schema id")?;
    }
    if let Some(wire_schema_id) = &entry.identity_carriage.wire_schema_id {
        reject_candidate_only_identifier(wire_schema_id, "wire schema id")?;
        SchemaId::parse(wire_schema_id.as_str()).map_err(|error| {
            GenerationError::new(format!(
                "schema `{schema_id}` has invalid expected wire_schema_id: {error}"
            ))
        })?;
    }
    if entry.identity_carriage.wire_schema_version.is_some()
        && entry.identity_carriage.wire_schema_id.is_none()
    {
        return Err(GenerationError::new(format!(
            "schema `{schema_id}` declares a wire version without a wire schema identity"
        )));
    }
    if !matches!(
        entry.identity_carriage.raw_schema_sha256.as_str(),
        "absent" | "self_carried"
    ) {
        return Err(GenerationError::new(format!(
            "schema `{schema_id}` raw schema carriage must be `absent` or `self_carried`"
        )));
    }
    Ok(())
}

fn validate_generated_resource(
    schema_id: &str,
    entry: &ManifestEntry,
    schema_base_uri: &str,
    schema: &Value,
) -> Result<(), GenerationError> {
    let root = schema.as_object().ok_or_else(|| {
        GenerationError::new(format!("schema `{schema_id}` generated a boolean root"))
    })?;
    let expected_uri = format!("{schema_base_uri}{schema_id}.json");
    if root.get("$id").and_then(Value::as_str) != Some(expected_uri.as_str()) {
        return Err(GenerationError::new(format!(
            "schema `{schema_id}` did not retain manifest-owned `$id` `{expected_uri}`"
        )));
    }
    validate_forbidden_schema_keywords(schema_id, schema, true)?;
    let actual_wire_id = root_wire_schema_id(schema_id, root)?;
    let actual_version = match schema.pointer("/properties/schema_version/const") {
        Some(value) => Some(
            value
                .as_u64()
                .and_then(|version| u32::try_from(version).ok())
                .ok_or_else(|| {
                    GenerationError::new(format!(
                        "schema `{schema_id}` generated an invalid wire version const"
                    ))
                })?,
        ),
        None => None,
    };
    if actual_wire_id != entry.identity_carriage.wire_schema_id.as_deref() {
        return Err(GenerationError::new(format!(
            "schema `{schema_id}` generated wire schema const {actual_wire_id:?}, expected {:?}",
            entry.identity_carriage.wire_schema_id
        )));
    }
    if actual_version != entry.identity_carriage.wire_schema_version {
        return Err(GenerationError::new(format!(
            "schema `{schema_id}` generated wire version const {actual_version:?}, expected {:?}",
            entry.identity_carriage.wire_schema_version
        )));
    }
    let declares_raw_digest = root
        .get("properties")
        .and_then(Value::as_object)
        .is_some_and(|properties| {
            ["schema_sha256", "envelope_schema_sha256"]
                .iter()
                .any(|field| properties.contains_key(*field))
        });
    let requires_raw_digest =
        root.get("required")
            .and_then(Value::as_array)
            .is_some_and(|required| {
                required.iter().any(|member| {
                    matches!(
                        member.as_str(),
                        Some("schema_sha256" | "envelope_schema_sha256")
                    )
                })
            });
    if declares_raw_digest != requires_raw_digest {
        return Err(GenerationError::new(format!(
            "schema `{schema_id}` generated partial raw-schema carriage"
        )));
    }
    let carries_raw_digest = declares_raw_digest && requires_raw_digest;
    if carries_raw_digest != (entry.identity_carriage.raw_schema_sha256 == "self_carried") {
        return Err(GenerationError::new(format!(
            "schema `{schema_id}` generated raw-schema carriage disagrees with its descriptor"
        )));
    }
    Ok(())
}

/// Resolve the one permitted root-level wire-schema constant.
fn root_wire_schema_id<'a>(
    schema_id: &str,
    root: &'a serde_json::Map<String, Value>,
) -> Result<Option<&'a str>, GenerationError> {
    let Some(properties) = root.get("properties").and_then(Value::as_object) else {
        return Ok(None);
    };
    let mut observed = ["schema", "schema_id", "envelope_schema_id"]
        .into_iter()
        .filter_map(|field| {
            properties
                .get(field)
                .and_then(Value::as_object)
                .and_then(|property| property.get("const"))
        });
    let first = observed
        .next()
        .map(|value| {
            value.as_str().ok_or_else(|| {
                GenerationError::new(format!(
                    "schema `{schema_id}` generated a non-string wire schema const"
                ))
            })
        })
        .transpose()?;
    if observed.next().is_some() {
        return Err(GenerationError::new(format!(
            "schema `{schema_id}` generated more than one root wire schema const"
        )));
    }
    Ok(first)
}

fn validate_forbidden_schema_keywords(
    schema_id: &str,
    value: &Value,
    is_root: bool,
) -> Result<(), GenerationError> {
    let Some(object) = value.as_object() else {
        return Ok(());
    };
    for keyword in ["$anchor", "$dynamicAnchor", "$dynamicRef"] {
        if object.contains_key(keyword) {
            return Err(GenerationError::new(format!(
                "schema `{schema_id}` generated forbidden keyword `{keyword}`"
            )));
        }
    }
    if !is_root && object.contains_key("$id") {
        return Err(GenerationError::new(format!(
            "schema `{schema_id}` generated a nested `$id`"
        )));
    }
    for_each_subschema(value, |child| {
        validate_forbidden_schema_keywords(schema_id, child, false)
    })?;
    Ok(())
}

fn validate_generated_reference_graph(
    manifest: &DescriptorManifest,
    schemas: &BTreeMap<String, Value>,
) -> Result<(), GenerationError> {
    let resources = schemas
        .keys()
        .map(|id| {
            (
                format!("{}{id}.json", manifest.schema_base_uri),
                id.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut actual_edges = BTreeSet::new();
    for (source_id, schema) in schemas {
        let mut references = Vec::new();
        collect_refs(schema, &mut references)?;
        for reference in references {
            if let Some(fragment) = reference.strip_prefix('#') {
                resolve_fragment(source_id, schema, fragment)?;
                continue;
            }
            let (resource, fragment) = reference.split_once('#').unwrap_or((reference, ""));
            let target_id = resources.get(resource).ok_or_else(|| {
                GenerationError::new(format!(
                    "schema `{source_id}` generated foreign or unresolved `$ref` `{reference}`"
                ))
            })?;
            let target = &schemas[*target_id];
            if !fragment.is_empty() {
                resolve_fragment(target_id, target, fragment)?;
            }
            actual_edges.insert((source_id.clone(), (*target_id).to_string()));
        }
    }
    let declared_edges = manifest
        .schemas
        .iter()
        .flat_map(|(nested, entry)| {
            entry
                .identity_carriage
                .outer_schema_ids
                .iter()
                .map(|outer| (outer.clone(), nested.clone()))
        })
        .collect::<BTreeSet<_>>();
    if actual_edges != declared_edges {
        return Err(GenerationError::ReferenceGraphMismatch {
            declared: declared_edges,
            generated: actual_edges,
        });
    }
    Ok(())
}

fn collect_refs<'a>(value: &'a Value, refs: &mut Vec<&'a str>) -> Result<(), GenerationError> {
    let Some(object) = value.as_object() else {
        return Ok(());
    };
    if let Some(reference) = object.get("$ref") {
        refs.push(
            reference
                .as_str()
                .ok_or_else(|| GenerationError::new("generated `$ref` is not a string"))?,
        );
    }
    for_each_subschema(value, |child| collect_refs(child, refs))?;
    Ok(())
}

fn for_each_subschema<'a>(
    value: &'a Value,
    mut visit: impl FnMut(&'a Value) -> Result<(), GenerationError>,
) -> Result<(), GenerationError> {
    let Some(object) = value.as_object() else {
        return Ok(());
    };
    for keyword in [
        "additionalItems",
        "additionalProperties",
        "contains",
        "contentSchema",
        "else",
        "if",
        "items",
        "not",
        "propertyNames",
        "then",
        "unevaluatedItems",
        "unevaluatedProperties",
    ] {
        if let Some(child) = object.get(keyword) {
            visit(child)?;
        }
    }
    for keyword in ["allOf", "anyOf", "oneOf", "prefixItems"] {
        if let Some(children) = object.get(keyword).and_then(Value::as_array) {
            for child in children {
                visit(child)?;
            }
        }
    }
    for keyword in [
        "$defs",
        "dependentSchemas",
        "patternProperties",
        "properties",
    ] {
        if let Some(children) = object.get(keyword).and_then(Value::as_object) {
            for child in children.values() {
                visit(child)?;
            }
        }
    }
    Ok(())
}

fn resolve_fragment(schema_id: &str, value: &Value, fragment: &str) -> Result<(), GenerationError> {
    if !fragment.is_empty() && (!fragment.starts_with('/') || value.pointer(fragment).is_none()) {
        return Err(GenerationError::new(format!(
            "schema `{schema_id}` has unresolved non-RFC6901 fragment `#{fragment}`"
        )));
    }
    Ok(())
}

/// Render the sole filesystem representation of one generated JSON value.
///
/// RFC 8785 remains the semantic digest and wire preimage where a contract
/// explicitly requires it. Generated files instead use one deterministic,
/// two-space-indented representation so the exact bytes remain reviewable.
fn pretty_json_document<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, GenerationError> {
    let mut bytes = Vec::new();
    let formatter = PrettyFormatter::with_indent(b"  ");
    let mut serializer = Serializer::with_formatter(&mut bytes, formatter);
    value
        .serialize(&mut serializer)
        .map_err(|error| GenerationError::new(format!("pretty JSON encoding failed: {error}")))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn validate_actors(schema_id: &str, label: &str, actors: &[Actor]) -> Result<(), GenerationError> {
    if actors.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(GenerationError::new(format!(
            "schema `{schema_id}` {label} actors must be sorted and unique"
        )));
    }
    for actor in actors {
        validate_bounded_string(&actor.id, "actor id")?;
        let valid = matches!(
            (actor.actor_type, actor.implementation_state),
            (ActorType::AuthoritySurface, ImplementationState::Observed)
                | (ActorType::RustLogicalSymbol, ImplementationState::Observed)
                | (
                    ActorType::ProtocolParticipant,
                    ImplementationState::ExternalNative
                )
                | (
                    ActorType::ProtocolParticipant,
                    ImplementationState::Observed
                )
                | (
                    ActorType::ProtocolParticipant,
                    ImplementationState::PlannedUnproved
                )
        );
        if !valid {
            return Err(GenerationError::new(format!(
                "schema `{schema_id}` has an invalid actor/state pair"
            )));
        }
    }
    Ok(())
}

fn validate_logical_id(value: &str, label: &str) -> Result<(), GenerationError> {
    validate_bounded_string(value, label)?;
    if !manifest_logical_id(value) {
        return Err(GenerationError::new(format!(
            "{label} is not a bounded lowercase logical id: {value}"
        )));
    }
    reject_candidate_only_identifier(value, label)?;
    Ok(())
}

/// Apply the core-owned candidate-only identifier inventory at bundle and ABI boundaries.
fn reject_candidate_only_identifier(value: &str, label: &str) -> Result<(), GenerationError> {
    if let Some(token) =
        pale_core::records::candidate_only_workspace_language_identifier_token_v1(value)
    {
        Err(GenerationError::new(format!(
            "{label} contains candidate-only token `{token}`: {value}"
        )))
    } else {
        Ok(())
    }
}

fn manifest_logical_id(value: &str) -> bool {
    !value.starts_with('.')
        && !value.ends_with('.')
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-_.".contains(&byte)
        })
}

fn validate_bounded_string(value: &str, label: &str) -> Result<(), GenerationError> {
    if value.is_empty() || value.len() > 512 || value.chars().any(char::is_control) {
        return Err(GenerationError::new(format!(
            "{label} is empty, oversized, or contains controls"
        )));
    }
    Ok(())
}

fn confined_relative_path(value: &str) -> Result<PathBuf, GenerationError> {
    let path = Path::new(value);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(GenerationError::new(format!(
            "bundle path is not confined: {value}"
        )));
    }
    Ok(path.to_path_buf())
}

fn read_regular_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, GenerationError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| GenerationError::new(format!("inspect {}: {error}", path.display())))?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return Err(GenerationError::new(format!(
            "not a regular non-symlink file: {}",
            path.display()
        )));
    }
    if metadata.len() > maximum {
        return Err(GenerationError::new(format!(
            "file exceeds {maximum} bytes: {}",
            path.display()
        )));
    }
    fs::read(path)
        .map_err(|error| GenerationError::new(format!("read {}: {error}", path.display())))
}

fn compare_file(path: &Path, expected: &[u8]) -> Result<(), GenerationError> {
    let actual = read_regular_bounded(path, expected.len().try_into().unwrap_or(u64::MAX))?;
    if actual != expected {
        return Err(GenerationError::new(format!(
            "generated bundle member is stale: {}",
            path.display()
        )));
    }
    Ok(())
}

fn regular_file_inventory(root: &Path) -> Result<BTreeSet<PathBuf>, GenerationError> {
    let root_metadata = fs::symlink_metadata(root).map_err(|error| {
        GenerationError::new(format!("inspect bundle {}: {error}", root.display()))
    })?;
    if !root_metadata.file_type().is_dir() || root_metadata.file_type().is_symlink() {
        return Err(GenerationError::new(format!(
            "bundle is not a regular directory: {}",
            root.display()
        )));
    }
    fn walk(
        root: &Path,
        directory: &Path,
        found: &mut BTreeSet<PathBuf>,
    ) -> Result<(), GenerationError> {
        for entry in fs::read_dir(directory).map_err(|error| {
            GenerationError::new(format!("read {}: {error}", directory.display()))
        })? {
            let entry = entry.map_err(|error| GenerationError::new(error.to_string()))?;
            let file_type = entry
                .file_type()
                .map_err(|error| GenerationError::new(error.to_string()))?;
            let path = entry.path();
            if file_type.is_symlink() {
                return Err(GenerationError::new(format!(
                    "bundle contains a symlink: {}",
                    path.display()
                )));
            }
            if file_type.is_dir() {
                walk(root, &path, found)?;
            } else if file_type.is_file() {
                found.insert(
                    path.strip_prefix(root)
                        .expect("walked path stays below root")
                        .to_path_buf(),
                );
            } else {
                return Err(GenerationError::new(format!(
                    "bundle contains a non-regular member: {}",
                    path.display()
                )));
            }
        }
        Ok(())
    }
    let mut found = BTreeSet::new();
    walk(root, root, &mut found)?;
    Ok(found)
}

fn allocate_staging_directory(parent: &Path, name: &str) -> Result<PathBuf, GenerationError> {
    for attempt in 0..128u32 {
        let candidate = parent.join(format!(
            ".{name}.pale-contract-codegen-{}-{attempt}",
            std::process::id()
        ));
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(GenerationError::new(format!(
                    "create staging directory {}: {error}",
                    candidate.display()
                )));
            }
        }
    }
    Err(GenerationError::new(
        "could not allocate a collision-free staging directory",
    ))
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), GenerationError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| GenerationError::new(format!("create {}: {error}", path.display())))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| GenerationError::new(format!("write {}: {error}", path.display())))
}

fn sync_tree_directories(root: &Path) -> Result<(), GenerationError> {
    let mut directories = vec![root.to_path_buf()];
    let mut index = 0;
    while index < directories.len() {
        let directory = directories[index].clone();
        index += 1;
        for entry in fs::read_dir(&directory).map_err(|error| {
            GenerationError::new(format!("read {}: {error}", directory.display()))
        })? {
            let entry = entry.map_err(|error| GenerationError::new(error.to_string()))?;
            if entry
                .file_type()
                .map_err(|error| GenerationError::new(error.to_string()))?
                .is_dir()
            {
                directories.push(entry.path());
            }
        }
    }
    for directory in directories.into_iter().rev() {
        File::open(&directory)
            .and_then(|handle| handle.sync_all())
            .map_err(|error| {
                GenerationError::new(format!("sync directory {}: {error}", directory.display()))
            })?;
    }
    Ok(())
}

struct StagingGuard {
    path: PathBuf,
    armed: bool,
}

impl StagingGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for StagingGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

#[cfg(test)]
#[path = "../tests/internal/manifest.rs"]
mod tests;
