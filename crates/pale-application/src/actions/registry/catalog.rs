//! Construction and discovery of the sealed built-in action catalog.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use pale_core::records::{
    ActionBoundary, ActionContract, ActionContractDigest, ActionEffect, ActionError,
    ActionErrorCode, ActionInputKind, NativeActionCatalogV1,
    NATIVE_ACTION_CATALOG_V1_SCHEMA_VERSION,
};
use serde_json::{Map, Value};

use super::execution::NATIVE_ACTION_DEFINITIONS;
use super::seal::ActionContractPreimage;
use super::{ActionPayloadSchema, NativeAction, NativeActionRegistry, ACTION_BEHAVIOR_VERSION};
use crate::actions::{error::action_error, ACTION_RECORD_SCHEMA_VERSION, CONTRACT_RECORD};

impl NativeActionRegistry {
    /// Build the fixed registry and seal its contracts and payload schemas.
    ///
    /// Construction rejects duplicate action identities, schema-invalid
    /// contracts, and any non-bijective input-schema projection.
    ///
    /// # Errors
    ///
    /// Returns a structured [`ActionError`] when a compiled contract cannot be
    /// canonicalized, the built-in identity set is not unique, or the embedded
    /// action schemas do not describe the same closed registry.
    pub fn new() -> Result<Self, ActionError> {
        let mut actions = BTreeMap::new();
        for definition in NATIVE_ACTION_DEFINITIONS {
            let preimage = ActionContractPreimage {
                schema: CONTRACT_RECORD,
                schema_version: ACTION_RECORD_SCHEMA_VERSION,
                id: definition.id,
                version: ACTION_BEHAVIOR_VERSION,
                description: definition.description,
                effect: ActionEffect::Compute,
                boundary: ActionBoundary::Pure,
                input_kind: definition.input_kind,
                output_kind: definition.output_kind,
            };
            let digest = ActionContractDigest::hash_canonical(&preimage).map_err(|error| {
                action_error(
                    Some(definition.id),
                    ActionErrorCode::CanonicalizationFailed,
                    error.to_string(),
                )
            })?;
            let contract = preimage.into_sealed(digest);
            let id = contract.id.clone();
            if actions
                .insert(
                    id.clone(),
                    NativeAction {
                        contract,
                        execute: definition.execute,
                    },
                )
                .is_some()
            {
                return Err(action_error(
                    Some(&id),
                    ActionErrorCode::DuplicateIdentifier,
                    "duplicate native action ID",
                ));
            }
        }
        let schema_identity = pale_contracts::record_schema_identity("native-action-catalog.v1")
            .map_err(|error| {
                registry_invariant(format!("native action schema identity is invalid: {error}"))
            })?;
        let catalog = NativeActionCatalogV1 {
            schema: schema_identity.schema,
            schema_sha256: schema_identity.schema_sha256,
            schema_version: NATIVE_ACTION_CATALOG_V1_SCHEMA_VERSION,
            contracts: actions
                .values()
                .map(|action| action.contract.clone())
                .collect(),
        };
        pale_contracts::validate_record(&catalog, "native-action-catalog.v1").map_err(|error| {
            registry_invariant(format!(
                "native action contracts violate their schema: {error}"
            ))
        })?;
        let catalog_bytes = pale_core::canon::to_canonical_string(&catalog)
            .map(String::into_bytes)
            .map_err(|error| {
                action_error(
                    None,
                    ActionErrorCode::CanonicalizationFailed,
                    format!("native action catalog could not be canonicalized: {error}"),
                )
            })?;
        let payload_schemas = seal_payload_schemas(&actions)?;
        Ok(Self {
            actions,
            payload_schemas,
            catalog_bytes,
        })
    }

    /// RFC 8785 bytes of the sealed [`NativeActionCatalogV1`].
    ///
    /// This is an owned-registry projection for the narrow API boundary, not a
    /// raw-contract iterator. Catalog membership proves compiled mechanics only;
    /// it establishes no project selection, route admission, authority, or effect.
    #[must_use]
    pub fn canonical_catalog_bytes(&self) -> &[u8] {
        &self.catalog_bytes
    }

    /// Borrow the closed contract set inside the application crate.
    ///
    /// Renderers use this iterator while constructing application-owned discovery
    /// artifacts. External consumers receive the sealed catalog projection or
    /// the narrower façade descriptors instead of this internal view.
    pub(crate) fn contracts(&self) -> impl ExactSizeIterator<Item = &ActionContract> {
        self.actions.values().map(|action| &action.contract)
    }

    /// Return the mechanical contract for an exact built-in identity.
    ///
    /// Presence in this catalog does not establish project selection or route
    /// admission; those checks belong to the caller boundary.
    pub fn contract(&self, action_id: &str) -> Option<&ActionContract> {
        self.actions.get(action_id).map(|action| &action.contract)
    }

    /// Return the schema fragment paired with one registered input kind.
    pub(in crate::actions) fn payload_schema_for(
        &self,
        kind: ActionInputKind,
    ) -> Option<&Map<String, Value>> {
        self.payload_schemas
            .get(&kind)
            .map(|payload| &payload.schema)
    }

    /// Explain only the branch selected by the supplied tag, not the requested action.
    pub(in crate::actions) fn input_branch(&self, value: &Value) -> Option<usize> {
        let supplied = value.get("kind")?.as_str()?;
        self.payload_schemas
            .iter()
            .find_map(|(kind, payload)| (kind.as_str() == supplied).then_some(payload.branch))
    }
}

fn seal_payload_schemas(
    actions: &BTreeMap<String, NativeAction>,
) -> Result<BTreeMap<ActionInputKind, ActionPayloadSchema>, ActionError> {
    let source = pale_contracts::embedded_schema("action-input.v1")
        .map_err(|error| registry_invariant(error.to_string()))?;
    let schema = pale_core::canon::parse_strict(source.as_bytes())
        .map_err(|error| registry_invariant(format!("action-input.v1 is invalid: {error}")))?;
    let branches = schema
        .get("oneOf")
        .and_then(Value::as_array)
        .ok_or_else(|| registry_invariant("action-input.v1 has no oneOf payload union"))?;

    let mut expected = BTreeMap::new();
    for action in actions.values() {
        if expected
            .insert(
                action.contract.input_kind.as_str(),
                action.contract.input_kind,
            )
            .is_some()
        {
            return Err(registry_invariant(format!(
                "multiple actions claim input kind `{}`",
                action.contract.input_kind
            )));
        }
    }

    let mut payloads = BTreeMap::new();
    for (index, branch) in branches.iter().enumerate() {
        let kind = branch
            .pointer("/properties/kind/const")
            .and_then(Value::as_str)
            .ok_or_else(|| registry_invariant("action-input.v1 branch has no kind constant"))?;
        let typed_kind = expected.get(kind).copied().ok_or_else(|| {
            registry_invariant(format!(
                "action-input.v1 declares unregistered input kind `{kind}`"
            ))
        })?;
        let payload = branch
            .pointer("/properties/payload")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                registry_invariant(format!(
                    "action-input.v1 kind `{kind}` has no object payload schema"
                ))
            })?;
        let payload = project_payload_schema(&schema, payload).map_err(registry_invariant)?;
        if payloads
            .insert(
                typed_kind,
                ActionPayloadSchema {
                    branch: index,
                    schema: payload,
                },
            )
            .is_some()
        {
            return Err(registry_invariant(format!(
                "action-input.v1 repeats input kind `{kind}`"
            )));
        }
    }

    if payloads.len() != expected.len() {
        let missing = expected
            .into_iter()
            .filter_map(|(name, kind)| (!payloads.contains_key(&kind)).then_some(name))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(registry_invariant(format!(
            "action-input.v1 is missing registered input kinds: {missing}"
        )));
    }
    Ok(payloads)
}

/// Project one tagged-union payload into a self-contained tool-input schema.
///
/// The generated action schema represents each payload as one local `$ref`.
/// An MCP descriptor cannot publish that reference by itself because the
/// enclosing schema's definitions are no longer present. This projection
/// inlines the exact referenced payload definition and carries only the local
/// definitions reachable from it. The source schema remains the single shape
/// owner; no field or validation law is reconstructed here.
fn project_payload_schema(
    source_schema: &Value,
    payload: &Map<String, Value>,
) -> Result<Map<String, Value>, String> {
    if payload.len() != 1 {
        return Err("action-input.v1 payload projection is not one exact local reference".into());
    }
    let root_reference = payload
        .get("$ref")
        .ok_or_else(|| "action-input.v1 payload projection has no $ref".to_owned())?
        .as_str()
        .ok_or_else(|| "action-input.v1 payload projection has a non-string $ref".to_owned())?;
    if !root_reference.starts_with("#/$defs/") {
        return Err(format!(
            "action-input.v1 payload depends on non-local reference `{root_reference}`"
        ));
    }
    let definitions = source_schema
        .get("$defs")
        .and_then(Value::as_object)
        .ok_or_else(|| "action-input.v1 has no object $defs registry".to_owned())?;
    let definition_index = definitions
        .iter()
        .map(|(name, definition)| (definition_pointer(name), (name, definition)))
        .collect::<BTreeMap<_, _>>();
    let (_, root) = definition_index.get(root_reference).ok_or_else(|| {
        format!("action-input.v1 payload has unresolved local reference `{root_reference}`")
    })?;
    let mut projected = root
        .as_object()
        .cloned()
        .ok_or_else(|| format!("action-input.v1 `{root_reference}` is not an object schema"))?;

    let mut pending = local_definition_references(root)?;
    let mut visited = BTreeSet::new();
    let mut retained = Map::new();
    while let Some(reference) = pending.pop_front() {
        if !visited.insert(reference.clone()) {
            continue;
        }
        let (name, definition) = definition_index.get(&reference).ok_or_else(|| {
            format!("action-input.v1 payload has unresolved local reference `{reference}`")
        })?;
        pending.extend(local_definition_references(definition)?);
        retained.insert((*name).clone(), (*definition).clone());
    }
    if !retained.is_empty() {
        projected.insert("$defs".into(), Value::Object(retained));
    }
    Ok(projected)
}

/// Collect local definition references without accepting a remote schema host.
fn local_definition_references(value: &Value) -> Result<VecDeque<String>, String> {
    fn visit(value: &Value, references: &mut VecDeque<String>) -> Result<(), String> {
        match value {
            Value::Object(object) => {
                for (key, value) in object {
                    if key == "$ref" {
                        let reference = value.as_str().ok_or_else(|| {
                            "action-input.v1 contains a non-string $ref".to_owned()
                        })?;
                        if !reference.starts_with("#/$defs/") {
                            return Err(format!(
                                "action-input.v1 payload depends on non-local reference `{reference}`"
                            ));
                        }
                        references.push_back(reference.to_owned());
                    } else {
                        visit(value, references)?;
                    }
                }
            }
            Value::Array(values) => {
                for value in values {
                    visit(value, references)?;
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
        Ok(())
    }

    let mut references = VecDeque::new();
    visit(value, &mut references)?;
    Ok(references)
}

/// Encode one definition name as its exact RFC 6901 local-schema pointer.
fn definition_pointer(name: &str) -> String {
    format!("#/$defs/{}", name.replace('~', "~0").replace('/', "~1"))
}

fn registry_invariant(message: impl Into<String>) -> ActionError {
    action_error(None, ActionErrorCode::InternalInvariant, message)
}
