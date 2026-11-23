//! Initial-v1 resident identity and durable public-contract floor.
//!
//! These records describe immutable identities, ordered control coordinates,
//! admission decisions, and recovery evidence. They intentionally implement no
//! storage engine, transport, activation, provider call, or native effect.
//!
//! Mathematical invariants stay with the record family that admits them:
//! lifecycle heads, principal bootstrap, producer and writer succession,
//! reconciliation revisions, and typed content digests each own their rendered
//! law in the corresponding module below.

mod a2a;
mod action;
mod admission;
mod capability;
mod continuity;
mod control;
mod effect;
mod event;
mod genealogy;
mod identity;
mod lifecycle;
mod operation;
mod reconciliation;
mod trust;

pub use a2a::*;
pub use action::*;
pub use admission::*;
pub use capability::*;
pub use continuity::*;
pub use control::*;
pub use effect::*;
pub use event::*;
pub use genealogy::*;
pub use identity::*;
pub use lifecycle::*;
pub use operation::*;
pub use reconciliation::*;
pub use trust::*;

/// Complete public initial-v1 resident JSON-Schema root inventory.
pub const RESIDENT_PUBLIC_ROOT_SCHEMA_IDS_V1: [&str; 61] = [
    ADAPTER_FAILURE_V1_SCHEMA_ID,
    CAPABILITY_RESULT_V1_SCHEMA_ID,
    COMPILED_GENERATION_V1_SCHEMA_ID,
    COMMON_REQUEST_ENVELOPE_V1_SCHEMA_ID,
    ACTION_ENVELOPE_V1_SCHEMA_ID,
    EFFECT_CONFLICT_CLAIM_V1_SCHEMA_ID,
    EFFECT_ADMISSION_V1_SCHEMA_ID,
    EFFECT_DISPATCH_INTENT_V1_SCHEMA_ID,
    EFFECT_RECEIPT_V1_SCHEMA_ID,
    RECONCILIATION_CASE_V1_SCHEMA_ID,
    RECONCILIATION_CUSTODY_V1_SCHEMA_ID,
    RECONCILIATION_REQUEST_V1_SCHEMA_ID,
    RECONCILIATION_RESULT_V1_SCHEMA_ID,
    RESOURCE_CONTINUITY_PROFILE_V1_SCHEMA_ID,
    CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID,
    PRINCIPAL_KEY_TRANSITION_V1_SCHEMA_ID,
    PRINCIPAL_RECORD_V1_SCHEMA_ID,
    PRINCIPAL_RECOVERY_POLICY_V1_SCHEMA_ID,
    PRINCIPAL_RECOVERY_PROOF_V1_SCHEMA_ID,
    LOCAL_ADMIN_AUTHORITY_V1_SCHEMA_ID,
    LOCAL_ADMIN_COMMAND_V1_SCHEMA_ID,
    LOCAL_ADMIN_RECEIPT_V1_SCHEMA_ID,
    PRODUCER_INCARNATION_AUTHORITY_V1_SCHEMA_ID,
    CHECKPOINT_SIGNING_AUTHORITY_V1_SCHEMA_ID,
    PRINCIPAL_TRUST_CHECKPOINT_V1_SCHEMA_ID,
    PRINCIPAL_BOOTSTRAP_RECEIPT_V1_SCHEMA_ID,
    DEPLOYMENT_CONTROL_STORE_GENESIS_RECEIPT_V1_SCHEMA_ID,
    TYPED_DIGEST_V1_SCHEMA_ID,
    OPERATION_LEASE_V1_SCHEMA_ID,
    OPERATION_HANDLE_V1_SCHEMA_ID,
    OPERATION_OBSERVATION_V1_SCHEMA_ID,
    PUBLISHED_GENERATION_HEAD_V1_SCHEMA_ID,
    ADVERTISED_GENERATION_HEAD_V1_SCHEMA_ID,
    ACTIVE_GENERATION_HEAD_V1_SCHEMA_ID,
    ACTIVATION_FENCE_V1_SCHEMA_ID,
    SESSION_ADMISSION_LEASE_V1_SCHEMA_ID,
    CAPABILITY_SESSION_GRANT_V1_SCHEMA_ID,
    CAPABILITY_SESSION_GRANT_REVOCATION_V1_SCHEMA_ID,
    CAPABILITY_REVOCATION_CHECKPOINT_V1_SCHEMA_ID,
    LOCAL_ROUTE_BINDING_V1_SCHEMA_ID,
    EXECUTION_OWNER_FENCE_V1_SCHEMA_ID,
    EVENT_REFERENCE_V1_SCHEMA_ID,
    EVENT_ACKNOWLEDGEMENT_V1_SCHEMA_ID,
    INTERACTION_OCCURRENCE_V1_SCHEMA_ID,
    GENEALOGY_EDGE_V1_SCHEMA_ID,
    RECONCILIATION_FRONTIER_V1_SCHEMA_ID,
    A2A_ROUTE_ADMISSION_V1_SCHEMA_ID,
    A2A_TASK_BINDING_V1_SCHEMA_ID,
    A2A_TASK_OBSERVATION_V1_SCHEMA_ID,
    PRINCIPAL_RESIDENT_WRITER_FENCE_V1_SCHEMA_ID,
    PRINCIPAL_RESIDENT_WRITER_TERMINAL_RECEIPT_V1_SCHEMA_ID,
    PRINCIPAL_RESIDENT_WRITER_TAKEOVER_RECEIPT_V1_SCHEMA_ID,
    PRODUCER_INCARNATION_V1_SCHEMA_ID,
    PRODUCER_INCARNATION_RETIREMENT_V1_SCHEMA_ID,
    CONTRACT_SET_V1_SCHEMA_ID,
    DEPLOYMENT_TERMS_V1_SCHEMA_ID,
    DEPLOYMENT_BINDING_PROPOSAL_V1_SCHEMA_ID,
    DEPLOYMENT_ACCEPTANCE_V1_SCHEMA_ID,
    DEPLOYMENT_ACCEPTANCE_REVOCATION_V1_SCHEMA_ID,
    DEPLOYMENT_BINDING_V1_SCHEMA_ID,
    DEPLOYMENT_RUNTIME_AUTHORITY_V1_SCHEMA_ID,
];

/// Maximum number of members in a resident initial-v1 bounded set.
pub const MAX_RESIDENT_SET_MEMBERS: usize = 1_024;
/// Maximum byte length of one opaque resident identity or reference.
pub const MAX_RESIDENT_ID_BYTES: usize = 512;
/// Maximum canonical bytes carried inline or resolved for one resident content binding.
pub(crate) const MAX_RESIDENT_CONTENT_BYTES: usize = 65_536;
/// Maximum nesting depth admitted for a resident inline JSON value.
const MAX_RESIDENT_JSON_DEPTH: usize = 32;
/// Maximum aggregate array elements and object members in inline resident JSON.
const MAX_RESIDENT_JSON_MEMBERS: usize = 4_096;

/// Enforce the shared cardinality ceiling for one resident collection.
pub(crate) fn bounded_len(
    relation: &'static str,
    len: usize,
) -> Result<(), InvalidResidentRecordV1> {
    if len <= MAX_RESIDENT_SET_MEMBERS {
        Ok(())
    } else {
        Err(InvalidResidentRecordV1::BoundExceeded {
            relation,
            observed: len,
            maximum: MAX_RESIDENT_SET_MEMBERS,
        })
    }
}

/// Enforce the shared byte ceiling for one inline or owner-resolved content binding.
pub(crate) fn validate_bounded_content_bytes(
    relation: &'static str,
    bytes: &[u8],
) -> Result<(), InvalidResidentRecordV1> {
    if bytes.len() <= MAX_RESIDENT_CONTENT_BYTES {
        Ok(())
    } else {
        Err(InvalidResidentRecordV1::BoundExceeded {
            relation,
            observed: bytes.len(),
            maximum: MAX_RESIDENT_CONTENT_BYTES,
        })
    }
}

/// Enforce bounded, strictly increasing wire order for a set-like collection.
pub(crate) fn strictly_sorted<T: Ord>(
    relation: &'static str,
    values: &[T],
) -> Result<(), InvalidResidentRecordV1> {
    bounded_len(relation, values.len())?;
    if values.windows(2).all(|pair| pair[0] < pair[1]) {
        Ok(())
    } else {
        Err(InvalidResidentRecordV1::NonCanonical { relation })
    }
}

/// Validate one canonical inline JSON value against the shared resident bounds.
pub(crate) fn validate_bounded_json(
    relation: &'static str,
    value: &serde_json::Value,
) -> Result<String, InvalidResidentRecordV1> {
    let canonical = crate::canon::to_canonical_string(value)?;
    validate_bounded_content_bytes(relation, canonical.as_bytes())?;
    let (depth, members) = resident_json_shape(value);
    if depth > MAX_RESIDENT_JSON_DEPTH {
        return Err(InvalidResidentRecordV1::BoundExceeded {
            relation,
            observed: depth,
            maximum: MAX_RESIDENT_JSON_DEPTH,
        });
    }
    if members > MAX_RESIDENT_JSON_MEMBERS {
        return Err(InvalidResidentRecordV1::BoundExceeded {
            relation,
            observed: members,
            maximum: MAX_RESIDENT_JSON_MEMBERS,
        });
    }
    Ok(canonical)
}

fn resident_json_shape(value: &serde_json::Value) -> (usize, usize) {
    match value {
        serde_json::Value::Array(values) => {
            values
                .iter()
                .fold((1, values.len()), |(depth, count), value| {
                    let (child_depth, child_count) = resident_json_shape(value);
                    (depth.max(child_depth + 1), count + child_count)
                })
        }
        serde_json::Value::Object(values) => {
            values
                .values()
                .fold((1, values.len()), |(depth, count), value| {
                    let (child_depth, child_count) = resident_json_shape(value);
                    (depth.max(child_depth + 1), count + child_count)
                })
        }
        _ => (1, 0),
    }
}

/// A resident public record violated its exact initial-v1 contract.
#[derive(Debug, thiserror::Error)]
pub enum InvalidResidentRecordV1 {
    /// A public record carries the wrong schema identity or major.
    #[error("{family} schema coordinate is not initial-v1")]
    SchemaCoordinate {
        /// Record family whose schema coordinate was rejected.
        family: &'static str,
    },
    /// A bounded identity is empty, too long, or contains control characters.
    #[error("resident {kind} is not a bounded printable identity")]
    InvalidIdentity {
        /// Semantic identity kind whose wire spelling was rejected.
        kind: &'static str,
    },
    /// An ordered coordinate lies outside its admitted numeric domain.
    #[error("resident {coordinate} lies outside its admitted numeric domain")]
    InvalidCoordinate {
        /// Ordered coordinate whose value or predecessor law failed.
        coordinate: &'static str,
    },
    /// A bounded collection exceeded its initial-v1 ceiling.
    #[error("resident {relation} has {observed} members; maximum is {maximum}")]
    BoundExceeded {
        /// Bounded relation that exceeded its ceiling.
        relation: &'static str,
        /// Number of supplied members.
        observed: usize,
        /// Initial-v1 maximum number of members.
        maximum: usize,
    },
    /// A set-like collection is duplicated or not canonically ordered.
    #[error("resident {relation} is not strictly ordered and duplicate-free")]
    NonCanonical {
        /// Set, relation, or projection that violated canonical form.
        relation: &'static str,
    },
    /// A child lease widens or outlives its ambient session.
    #[error("operation lease is not a bounded narrowing of its ambient session")]
    LeaseWidening,
    /// An effect state carries fields forbidden or required by another state.
    #[error("effect {record} fields contradict state {state}")]
    EffectState {
        /// Effect-control record carrying the contradiction.
        record: &'static str,
        /// State or invariant contradicted by the supplied fields.
        state: &'static str,
    },
    /// A settlement or reconciliation transition is internally contradictory.
    #[error("resident settlement/reconciliation state is internally contradictory")]
    SettlementState,
    /// A wire-valid record failed a resolved parent or predecessor invariant.
    #[error("{record} contextual admission failed: {invariant}")]
    ContextualAdmission {
        /// Record family whose contextual relationship was rejected.
        record: &'static str,
        /// Stable invariant label suitable for diagnostics and tests.
        invariant: &'static str,
    },
    /// The claimed self-digest differs from the canonical preimage.
    #[error("{family} digest does not match its canonical preimage")]
    DigestMismatch {
        /// Record family whose claimed identity did not reproduce.
        family: &'static str,
    },
    /// Canonical JSON encoding failed.
    #[error(transparent)]
    Canon(#[from] crate::canon::CanonError),
    /// A selected Package or normative Pack closure is not exact and canonical.
    #[error(transparent)]
    ReleaseClosure(#[from] crate::records::ReleaseClosureErrorV1),
}

macro_rules! resident_id {
    ($(#[$meta:meta])* $name:ident, $kind:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, schemars::JsonSchema)]
        #[serde(transparent)]
        pub struct $name(#[schemars(length(min = 1, max = 512))] String);

        impl $name {
            /// Validate an externally supplied resident identity.
            ///
            /// # Errors
            ///
            /// Returns [`InvalidResidentRecordV1`] when the spelling is empty,
            /// oversized, or contains a control character.
            pub fn parse(value: impl Into<String>) -> Result<Self, InvalidResidentRecordV1> {
                let value = value.into();
                if value.is_empty()
                    || value.len() > $crate::records::MAX_RESIDENT_ID_BYTES
                    || value.chars().any(char::is_control)
                    || !unicode_normalization::is_nfc(&value)
                {
                    Err(InvalidResidentRecordV1::InvalidIdentity { kind: $kind })
                } else {
                    Ok(Self(value))
                }
            }

            /// Borrow the stable wire spelling.
            #[must_use]
            pub fn as_str(&self) -> &str { &self.0 }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where D: serde::Deserializer<'de> {
                Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
            }
        }
    };
}

/// Decide whether an integer lies in one coordinate's closed portable domain.
const fn resident_coordinate_in_domain(value: u64, minimum: u64) -> bool {
    value >= minimum && value <= crate::canon::JSON_SAFE_INTEGER_MAX
}

/// Declare one nominal portable integer coordinate without collapsing its domain type.
///
/// The macro owns only the shared RFC-8785 numeric mechanics. Callers retain
/// their semantic type, domain name, zero law, and lifecycle meaning.
macro_rules! resident_coordinate {
    ($(#[$meta:meta])* $name:ident, $kind:literal, zero_aware) => {
        resident_coordinate!(@define
            $(#[$meta])*
            #[doc = ""]
            #[doc = "Its portable wire domain is `0 <= n <= 2^53 - 1`."]
            $name,
            $kind,
            0,
            "crate::canon::json_safe_u64_schema"
        );
    };
    ($(#[$meta:meta])* $name:ident, $kind:literal, positive) => {
        resident_coordinate!(@define
            $(#[$meta])*
            #[doc = ""]
            #[doc = "Its portable wire domain is `1 <= n <= 2^53 - 1`."]
            $name,
            $kind,
            1,
            "crate::canon::positive_json_safe_u64_schema"
        );
    };
    (@define $(#[$meta:meta])* $name:ident, $kind:literal, $minimum:literal, $schema:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, schemars::JsonSchema)]
        #[serde(transparent)]
        pub struct $name(
            #[schemars(schema_with = $schema)] u64,
        );

        impl $name {
            /// Construct a portable coordinate in this type's admitted domain.
            ///
            /// # Errors
            /// Returns [`InvalidResidentRecordV1::InvalidCoordinate`] when the
            /// value violates this coordinate's minimum or the inclusive
            /// [`crate::canon::JSON_SAFE_INTEGER_MAX`] ceiling.
            pub const fn new(value: u64) -> Result<Self, InvalidResidentRecordV1> {
                if !crate::records::resident::resident_coordinate_in_domain(value, $minimum) {
                    Err(InvalidResidentRecordV1::InvalidCoordinate { coordinate: $kind })
                } else { Ok(Self(value)) }
            }
            /// Return the ordered integer coordinate.
            #[must_use]
            pub const fn get(self) -> u64 { self.0 }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where D: serde::Deserializer<'de> {
                Self::new(u64::deserialize(deserializer)?).map_err(serde::de::Error::custom)
            }
        }
    };
}

macro_rules! resident_successor_coordinate {
    ($(#[$meta:meta])* $name:ident, $kind:literal, $domain:ident) => {
        resident_coordinate!(
            $(#[$meta])*
            $name,
            $kind,
            $domain
        );

        impl $name {
            /// Advance exactly once without leaving the portable JCS domain.
            ///
            /// `None` is the explicit exhaustion state at
            /// [`crate::canon::JSON_SAFE_INTEGER_MAX`].
            #[must_use]
            pub const fn checked_successor(self) -> Option<Self> {
                if self.0 == crate::canon::JSON_SAFE_INTEGER_MAX {
                    None
                } else {
                    Some(Self(self.0 + 1))
                }
            }
        }
    };
}

macro_rules! resident_sequence {
    ($(#[$meta:meta])* $name:ident, $kind:literal) => {
        crate::records::resident_successor_coordinate!(
            $(#[$meta])*
            $name,
            $kind,
            positive
        );
    };
}

pub(crate) use resident_coordinate;
pub(crate) use resident_id;
pub(crate) use resident_sequence;
pub(crate) use resident_successor_coordinate;

/// Exact schema coordinates projected by one macro-generated resident record.
trait ResidentSchemaIdentityV1 {
    /// Root property that carries the schema identity.
    const SCHEMA_FIELD: &'static str;
    /// Exact initial-v1 wire schema identity.
    const SCHEMA: &'static str;
    /// Optional `(property, value)` pair for an explicit major-version field.
    const VERSION: Option<(&'static str, u32)>;
}

/// Close a macro-generated JSON Schema over its exact resident identity.
///
/// Schemars field extensions cannot interpolate a macro expression reliably.
/// The type-level transform retains one Rust SSOT while emitting the same
/// direct `const` constraints as hand-written wire records.
fn close_resident_schema_identity_v1<T: ResidentSchemaIdentityV1>(schema: &mut schemars::Schema) {
    let Some(properties) = schema
        .ensure_object()
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };
    if let Some(property) = properties
        .get_mut(T::SCHEMA_FIELD)
        .and_then(serde_json::Value::as_object_mut)
    {
        property.insert("const".into(), T::SCHEMA.into());
    }
    if let Some((field, version)) = T::VERSION {
        if let Some(property) = properties
            .get_mut(field)
            .and_then(serde_json::Value::as_object_mut)
        {
            property.insert("const".into(), version.into());
        }
    }
    forbid_explicit_null_properties_v1(schema);
}

/// Remove `null` from optional root members while preserving omission.
///
/// Resident initial-v1 wires use one canonical absence spelling: omit the
/// member. This transform keeps the Rust [`Option`] model while ensuring the
/// generated public schema rejects an explicitly authored JSON `null`.
fn forbid_explicit_null_properties_v1(schema: &mut schemars::Schema) {
    let Some(properties) = schema
        .ensure_object()
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };
    for property in properties.values_mut() {
        let Some(object) = property.as_object_mut() else {
            continue;
        };
        let Some(any_of) = object
            .remove("anyOf")
            .and_then(|value| value.as_array().cloned())
        else {
            continue;
        };
        let mut admitted = any_of
            .into_iter()
            .filter(|variant| {
                variant.get("type").and_then(serde_json::Value::as_str) != Some("null")
            })
            .collect::<Vec<_>>();
        match admitted.len() {
            0 => {
                object.insert("not".into(), serde_json::json!({ "type": "null" }));
            }
            1 => {
                if let Some(replacement) = admitted.pop() {
                    if let Some(replacement) = replacement.as_object() {
                        for (key, value) in replacement {
                            object.insert(key.clone(), value.clone());
                        }
                    } else {
                        object.insert("allOf".into(), serde_json::json!([replacement]));
                    }
                } else {
                    object.insert("not".into(), serde_json::json!({ "type": "null" }));
                }
            }
            _ => {
                object.insert("anyOf".into(), admitted.into());
            }
        }
    }
}

/// Project an all-or-none dependency for one group of optional wire members.
///
/// Each present member requires every other member in `properties`. Runtime
/// admission still validates the same law; this transform prevents the public
/// structural schema from accepting a partial coordinate first.
pub(crate) fn require_all_or_none_properties_v1(
    schema: &mut schemars::Schema,
    properties: &[&str],
) {
    let dependencies: serde_json::Map<String, serde_json::Value> = properties
        .iter()
        .map(|property| {
            let required = properties
                .iter()
                .filter(|candidate| candidate != &property)
                .map(|candidate| serde_json::Value::String((*candidate).into()))
                .collect();
            ((*property).into(), serde_json::Value::Array(required))
        })
        .collect();
    let root = schema.ensure_object();
    let existing = root
        .entry("dependentRequired")
        .or_insert_with(|| serde_json::json!({}));
    let Some(existing) = existing.as_object_mut() else {
        return;
    };
    existing.extend(dependencies);
}

/// Project the shared resident set bound and uniqueness for one wire member.
///
/// Canonical ordering remains a Rust admission law because JSON Schema cannot
/// express lexicographic sequence order. `minimum` is present only for sets
/// whose owning record forbids the empty identity.
pub(crate) fn project_resident_set_property_v1(
    schema: &mut schemars::Schema,
    property: &str,
    minimum: Option<usize>,
) {
    let Some(collection) = schema
        .ensure_object()
        .get_mut("properties")
        .and_then(serde_json::Value::as_object_mut)
        .and_then(|properties| properties.get_mut(property))
        .and_then(serde_json::Value::as_object_mut)
    else {
        return;
    };
    collection.insert("maxItems".into(), MAX_RESIDENT_SET_MEMBERS.into());
    collection.insert("uniqueItems".into(), true.into());
    if let Some(minimum) = minimum {
        collection.insert("minItems".into(), minimum.into());
    }
}

/// Reject authored root members whose value is explicit JSON `null`.
fn reject_explicit_resident_null_v1(value: &serde_json::Value) -> Result<(), &'static str> {
    if value
        .as_object()
        .is_some_and(|object| object.values().any(serde_json::Value::is_null))
    {
        Err("explicit JSON null is forbidden; omit an absent optional resident member")
    } else {
        Ok(())
    }
}

/// Implement strict owned deserialization for one macro-generated wire.
macro_rules! impl_resident_wire_deserialize {
    ($wire:ident { $( $field:ident : $field_ty:ty ),+ $(,)? }) => {
        impl<'de> serde::Deserialize<'de> for $wire {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = <serde_json::Value as serde::Deserialize>::deserialize(deserializer)?;
                $crate::records::resident::reject_explicit_resident_null_v1(&value)
                    .map_err(serde::de::Error::custom)?;
                #[derive(serde::Deserialize)]
                #[serde(deny_unknown_fields)]
                struct UntrustedWire {
                    $( $field: $field_ty, )+
                }
                let UntrustedWire { $( $field ),+ } =
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(Self { $( $field ),+ })
            }
        }
    };
}

pub(crate) use impl_resident_wire_deserialize;

macro_rules! resident_record {
    (
        $(#[$meta:meta])*
        $visibility:vis $name:ident, $wire:ident, $preimage:ident,
        $digest_field:ident : $digest:ty,
        $schema:expr, $version:expr, $family:literal,
        { $( $(#[$field_meta:meta])* $field:ident : $field_ty:ty ),+ $(,)? },
        $validate:expr
    ) => {
        $(#[$meta])*
        #[serde_with::skip_serializing_none]
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
        #[schemars(transform = close_resident_schema_identity_v1::<$name>)]
        #[serde(deny_unknown_fields)]
        $visibility struct $name {
            /// Stable initial-v1 schema identity.
            #[schemars(with = "String", extend("const" = $schema))]
            schema: crate::records::SchemaId,
            /// SHA-256 of the exact selected JSON Schema bytes.
            schema_sha256: crate::records::SchemaDigest,
            /// Initial public-contract major.
            #[schemars(extend("const" = $version))]
            schema_version: u32,
            $( $(#[$field_meta])* $field: $field_ty, )+
            /// Canonical identity of this exact record preimage.
            $digest_field: $digest,
        }

        #[doc = concat!("Validated construction preimage for [`", stringify!($name), "`].")]
        #[serde_with::skip_serializing_none]
        #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
        $visibility struct $preimage {
            /// Stable initial-v1 schema identity.
            pub schema: crate::records::SchemaId,
            /// SHA-256 of the exact selected JSON Schema bytes.
            pub schema_sha256: crate::records::SchemaDigest,
            /// Initial public-contract major.
            pub schema_version: u32,
            $( $(#[$field_meta])* pub $field: $field_ty, )+
        }

        #[doc = concat!("Untrusted structural wire shape for [`", stringify!($name), "`].")]
        #[doc = "\n\nDeserializing this type establishes only field shape. Resolve it through"]
        #[doc = "the record owner's contract path; this macro exposes no generic admission."]
        #[serde_with::skip_serializing_none]
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
        #[schemars(transform = close_resident_schema_identity_v1::<$wire>)]
        #[serde(deny_unknown_fields)]
        $visibility struct $wire {
            /// Claimed initial-v1 schema identity supplied on the untrusted wire.
            schema: crate::records::SchemaId,
            /// Claimed digest of the selected JSON Schema bytes.
            schema_sha256: crate::records::SchemaDigest,
            /// Claimed public-contract major.
            schema_version: u32,
            $(
                #[doc = concat!("Claimed untrusted `", stringify!($field), "` field.")]
                $field: $field_ty,
            )+
            /// Claimed canonical identity of the record preimage.
            $digest_field: $digest,
        }

        $crate::records::resident::impl_resident_wire_deserialize!($wire {
            schema: crate::records::SchemaId,
            schema_sha256: crate::records::SchemaDigest,
            schema_version: u32,
            $( $field: $field_ty, )+
            $digest_field: $digest,
        });

        impl $name {
            /// Validate and seal one canonical initial-v1 record.
            ///
            /// # Errors
            ///
            /// Returns [`InvalidResidentRecordV1`] when schema, structural,
            /// canonical-order, bound, or digest-preimage validation fails.
            pub(crate) fn seal(preimage: $preimage) -> Result<Self, InvalidResidentRecordV1> {
                if preimage.schema.as_str() != $schema || preimage.schema_version != $version {
                    return Err(InvalidResidentRecordV1::SchemaCoordinate { family: $family });
                }
                ($validate)(&preimage)?;
                let $digest_field = <$digest>::hash_canonical(&preimage)?;
                let $preimage { schema, schema_sha256, schema_version, $( $field ),+ } = preimage;
                Ok(Self { schema, schema_sha256, schema_version, $( $field ),+, $digest_field })
            }

            /// Revalidate structure and canonical identity.
            ///
            /// # Errors
            ///
            /// Returns [`InvalidResidentRecordV1`] when the sealed record no
            /// longer reproduces its exact validated canonical preimage.
            pub fn verify(&self) -> Result<(), InvalidResidentRecordV1> {
                let preimage = $preimage {
                    schema: self.schema.clone(),
                    schema_sha256: self.schema_sha256.clone(),
                    schema_version: self.schema_version,
                    $( $field: self.$field.clone() ),+
                };
                if preimage.schema.as_str() != $schema || preimage.schema_version != $version {
                    return Err(InvalidResidentRecordV1::SchemaCoordinate { family: $family });
                }
                let observed = Self::seal(preimage)?.$digest_field;
                if observed == self.$digest_field { Ok(()) } else {
                    Err(InvalidResidentRecordV1::DigestMismatch { family: $family })
                }
            }

            /// Borrow the stable initial-v1 schema identity.
            #[must_use]
            pub fn schema(&self) -> &crate::records::SchemaId { &self.schema }

            /// Borrow the exact selected JSON Schema digest.
            #[must_use]
            pub fn schema_sha256(&self) -> &crate::records::SchemaDigest { &self.schema_sha256 }

            /// Return the initial public-contract major.
            #[must_use]
            pub const fn schema_version(&self) -> u32 { self.schema_version }

            $(
                $(#[$field_meta])*
                #[must_use]
                pub fn $field(&self) -> &$field_ty { &self.$field }
            )+

            #[doc = concat!("Borrow the canonical `", stringify!($digest_field), "`.")]
            #[must_use]
            pub fn $digest_field(&self) -> &$digest { &self.$digest_field }
        }

        impl $wire {
            /// Borrow the claimed initial-v1 schema identity.
            #[must_use]
            pub fn schema(&self) -> &crate::records::SchemaId { &self.schema }

            /// Borrow the claimed selected JSON Schema digest.
            #[must_use]
            pub fn schema_sha256(&self) -> &crate::records::SchemaDigest { &self.schema_sha256 }

            /// Return the claimed initial public-contract major.
            #[must_use]
            pub const fn schema_version(&self) -> u32 { self.schema_version }

            $(
                #[doc = concat!("Borrow the claimed untrusted `", stringify!($field), "` field.")]
                #[must_use]
                pub fn $field(&self) -> &$field_ty { &self.$field }
            )+

            #[doc = concat!("Borrow the claimed untrusted `", stringify!($digest_field), "`.")]
            #[must_use]
            pub fn $digest_field(&self) -> &$digest { &self.$digest_field }
        }

        impl From<&$name> for $wire {
            /// Project an admitted record into its structurally identical wire shape.
            ///
            /// This conversion preserves the record's existing claims. It performs
            /// no admission and grants no authority to a wire received elsewhere.
            fn from(value: &$name) -> Self {
                Self {
                    schema: value.schema.clone(),
                    schema_sha256: value.schema_sha256.clone(),
                    schema_version: value.schema_version,
                    $( $field: value.$field.clone(), )+
                    $digest_field: value.$digest_field.clone(),
                }
            }
        }

        impl ResidentSchemaIdentityV1 for $name {
            const SCHEMA_FIELD: &'static str = "schema";
            const SCHEMA: &'static str = $schema;
            const VERSION: Option<(&'static str, u32)> = Some(("schema_version", $version));
        }

        impl ResidentSchemaIdentityV1 for $wire {
            const SCHEMA_FIELD: &'static str = "schema";
            const SCHEMA: &'static str = $schema;
            const VERSION: Option<(&'static str, u32)> = Some(("schema_version", $version));
        }

    };
}

pub(crate) use resident_record;

/// Declare an unsigned semantic record whose integrity is carried by one
/// canonical self-digest.
///
/// This is the unsigned counterpart to [`resident_signed_record!`]. It owns
/// the shared initial-v1 schema coordinate, strict wire shape, JCS preimage,
/// digest comparison, and accessors. Record modules remain responsible for
/// their semantic validation and public contextual admission API.
macro_rules! resident_integrity_record {
    (
        $(#[$meta:meta])*
        $name:ident, $wire:ident, $preimage:ident,
        $digest_field:ident : $digest:ty,
        $schema:expr, $family:literal,
        { $( $(#[$field_meta:meta])* $field:ident : $field_ty:ty ),+ $(,)? },
        $validate:expr
        $(, wire_transform = $wire_transform:expr)?
    ) => {
        $(#[$meta])*
        #[serde_with::skip_serializing_none]
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
        #[schemars(transform = close_resident_schema_identity_v1::<$name>)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            #[schemars(with = "String", extend("const" = $schema))]
            /// Stable initial-v1 schema identity.
            schema_id: crate::records::SchemaId,
            /// SHA-256 of the exact selected schema bytes.
            schema_sha256: crate::records::SchemaDigest,
            $( $(#[$field_meta])* $field: $field_ty, )+
            /// Canonical identity of every preceding member.
            $digest_field: $digest,
        }

        #[doc = concat!("Validated construction preimage for [`", stringify!($name), "`].")]
        #[serde_with::skip_serializing_none]
        #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
        pub struct $preimage {
            /// Stable initial-v1 schema identity.
            pub schema_id: crate::records::SchemaId,
            /// SHA-256 of the exact selected schema bytes.
            pub schema_sha256: crate::records::SchemaDigest,
            $( $(#[$field_meta])* pub $field: $field_ty, )+
        }

        #[doc = concat!("Untrusted structural wire shape for [`", stringify!($name), "`].")]
        #[doc = "\n\nDeserializing this type establishes only field shape. Use the record-specific"]
        #[doc = "public contextual admission API; generic sealing and admission stay crate-private."]
        #[serde_with::skip_serializing_none]
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
        #[schemars(transform = close_resident_schema_identity_v1::<$wire>)]
        $(#[schemars(transform = $wire_transform)])?
        #[serde(deny_unknown_fields)]
        pub struct $wire {
            #[schemars(with = "String", extend("const" = $schema))]
            /// Stable initial-v1 schema identity supplied on the wire.
            schema_id: crate::records::SchemaId,
            /// Claimed digest of the selected JSON Schema bytes.
            schema_sha256: crate::records::SchemaDigest,
            $( $(#[$field_meta])* $field: $field_ty, )+
            /// Claimed canonical identity of every preceding member.
            $digest_field: $digest,
        }

        $crate::records::resident::impl_resident_wire_deserialize!($wire {
            schema_id: crate::records::SchemaId,
            schema_sha256: crate::records::SchemaDigest,
            $( $field: $field_ty, )+
            $digest_field: $digest,
        });

        impl $name {
            /// Seal a structurally valid canonical preimage.
            ///
            /// # Errors
            ///
            /// Returns [`InvalidResidentRecordV1`] when the schema coordinate,
            /// structural law, bound, or canonical-order law fails.
            pub(crate) fn seal(preimage: $preimage) -> Result<Self, InvalidResidentRecordV1> {
                if preimage.schema_id.as_str() != $schema {
                    return Err(InvalidResidentRecordV1::SchemaCoordinate { family: $family });
                }
                ($validate)(&preimage)?;
                let $digest_field = <$digest>::hash_canonical(&preimage)?;
                let $preimage { schema_id, schema_sha256, $( $field ),+ } = preimage;
                Ok(Self { schema_id, schema_sha256, $( $field ),+, $digest_field })
            }

            /// Revalidate structure and the canonical self-digest.
            ///
            /// # Errors
            ///
            /// Returns [`InvalidResidentRecordV1`] when the record no longer
            /// reproduces its exact validated preimage.
            pub fn verify(&self) -> Result<(), InvalidResidentRecordV1> {
                let preimage = $preimage {
                    schema_id: self.schema_id.clone(),
                    schema_sha256: self.schema_sha256.clone(),
                    $( $field: self.$field.clone() ),+
                };
                if preimage.schema_id.as_str() != $schema {
                    return Err(InvalidResidentRecordV1::SchemaCoordinate { family: $family });
                }
                ($validate)(&preimage)?;
                if <$digest>::hash_canonical(&preimage)? == self.$digest_field {
                    Ok(())
                } else {
                    Err(InvalidResidentRecordV1::DigestMismatch { family: $family })
                }
            }

            /// Admit an untrusted integrity wire after resolved contextual checks.
            ///
            /// # Errors
            ///
            /// Returns [`InvalidResidentRecordV1`] when structural, digest, or
            /// contextual admission fails.
            pub(crate) fn admit_wire<F>(
                wire: $wire,
                admit: F,
            ) -> Result<Self, InvalidResidentRecordV1>
            where
                F: FnOnce(&$preimage, &$digest) -> Result<(), InvalidResidentRecordV1>,
            {
                let preimage = $preimage {
                    schema_id: wire.schema_id,
                    schema_sha256: wire.schema_sha256,
                    $( $field: wire.$field, )+
                };
                if preimage.schema_id.as_str() != $schema {
                    return Err(InvalidResidentRecordV1::SchemaCoordinate { family: $family });
                }
                ($validate)(&preimage)?;
                let observed = <$digest>::hash_canonical(&preimage)?;
                if observed != wire.$digest_field {
                    return Err(InvalidResidentRecordV1::DigestMismatch { family: $family });
                }
                admit(&preimage, &observed)?;
                Self::seal(preimage)
            }

            /// Borrow the stable initial-v1 schema identity.
            #[must_use]
            pub fn schema_id(&self) -> &crate::records::SchemaId { &self.schema_id }

            /// Borrow the exact selected JSON Schema digest.
            #[must_use]
            pub fn schema_sha256(&self) -> &crate::records::SchemaDigest { &self.schema_sha256 }

            $(
                $(#[$field_meta])*
                #[must_use]
                pub fn $field(&self) -> &$field_ty { &self.$field }
            )+

            #[doc = concat!("Borrow the canonical `", stringify!($digest_field), "`.")]
            #[must_use]
            pub fn $digest_field(&self) -> &$digest { &self.$digest_field }
        }

        impl ResidentSchemaIdentityV1 for $name {
            const SCHEMA_FIELD: &'static str = "schema_id";
            const SCHEMA: &'static str = $schema;
            const VERSION: Option<(&'static str, u32)> = None;
        }

        impl ResidentSchemaIdentityV1 for $wire {
            const SCHEMA_FIELD: &'static str = "schema_id";
            const SCHEMA: &'static str = $schema;
            const VERSION: Option<(&'static str, u32)> = None;
        }
    };
}

pub(crate) use resident_integrity_record;

macro_rules! resident_signed_record {
    (
        $(#[$meta:meta])*
        $name:ident, $wire:ident, $preimage:ident,
        $digest_field:ident : $digest:ty,
        $schema:expr, $family:literal,
        { $( $(#[$field_meta:meta])* $field:ident : $field_ty:ty ),+ $(,)? },
        $validate:expr
        $(, wire_transform = $wire_transform:expr)?
    ) => {
        $(#[$meta])*
        #[serde_with::skip_serializing_none]
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
        #[schemars(transform = close_resident_schema_identity_v1::<$name>)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            #[schemars(with = "String", extend("const" = $schema))]
            /// Stable initial-v1 schema identity.
            schema_id: crate::records::SchemaId,
            /// SHA-256 of the exact selected schema bytes.
            schema_sha256: crate::records::SchemaDigest,
            $( $(#[$field_meta])* $field: $field_ty, )+
            /// Canonical identity of the unsigned preimage.
            $digest_field: $digest,
            /// Opaque signature interpreted by the selected signing authority.
            signature: CanonicalSignatureV1,
        }

        #[doc = concat!("Unsigned construction preimage for [`", stringify!($name), "`].")]
        #[serde_with::skip_serializing_none]
        #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
        pub struct $preimage {
            /// Stable initial-v1 schema identity.
            pub schema_id: crate::records::SchemaId,
            /// SHA-256 of the exact selected schema bytes.
            pub schema_sha256: crate::records::SchemaDigest,
            $( $(#[$field_meta])* pub $field: $field_ty, )+
        }

        #[doc = concat!("Untrusted structural wire shape for [`", stringify!($name), "`].")]
        #[doc = "\n\nDeserializing this type establishes only field shape. Contextual admission must"]
        #[doc = "separately establish authority; generic sealing and admission stay crate-private."]
        #[serde_with::skip_serializing_none]
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
        #[schemars(transform = close_resident_schema_identity_v1::<$wire>)]
        $(#[schemars(transform = $wire_transform)])?
        #[serde(deny_unknown_fields)]
        pub struct $wire {
            /// Stable initial-v1 schema identity supplied on the wire.
            schema_id: crate::records::SchemaId,
            /// Claimed digest of the selected JSON Schema bytes.
            schema_sha256: crate::records::SchemaDigest,
            $( $(#[$field_meta])* $field: $field_ty, )+
            /// Claimed canonical identity of the unsigned preimage.
            $digest_field: $digest,
            /// Opaque signature supplied by the selected signing authority.
            signature: CanonicalSignatureV1,
        }

        $crate::records::resident::impl_resident_wire_deserialize!($wire {
            schema_id: crate::records::SchemaId,
            schema_sha256: crate::records::SchemaDigest,
            $( $field: $field_ty, )+
            $digest_field: $digest,
            signature: CanonicalSignatureV1,
        });

        impl $name {
            /// Seal the canonical unsigned preimage beside its opaque signature.
            /// Cryptographic verification remains the admitting authority's job.
            ///
            /// # Errors
            ///
            /// Returns [`InvalidResidentRecordV1`] when schema, structural,
            /// canonical-order, bound, or digest-preimage validation fails.
            pub(crate) fn seal(preimage: $preimage, signature: CanonicalSignatureV1) -> Result<Self, InvalidResidentRecordV1> {
                if preimage.schema_id.as_str() != $schema {
                    return Err(InvalidResidentRecordV1::SchemaCoordinate { family: $family });
                }
                ($validate)(&preimage)?;
                let $digest_field = <$digest>::hash_canonical(&preimage)?;
                let $preimage { schema_id, schema_sha256, $( $field ),+ } = preimage;
                Ok(Self { schema_id, schema_sha256, $( $field ),+, $digest_field, signature })
            }

            /// Revalidate structure and the canonical unsigned digest.
            ///
            /// # Errors
            ///
            /// Returns [`InvalidResidentRecordV1`] when the record no longer
            /// reproduces its exact unsigned canonical preimage.
            pub fn verify(&self) -> Result<(), InvalidResidentRecordV1> {
                let preimage = $preimage {
                    schema_id: self.schema_id.clone(), schema_sha256: self.schema_sha256.clone(),
                    $( $field: self.$field.clone() ),+
                };
                if preimage.schema_id.as_str() != $schema {
                    return Err(InvalidResidentRecordV1::SchemaCoordinate { family: $family });
                }
                ($validate)(&preimage)?;
                if <$digest>::hash_canonical(&preimage)? == self.$digest_field { Ok(()) } else {
                    Err(InvalidResidentRecordV1::DigestMismatch { family: $family })
                }
            }

            /// Admit an untrusted signed wire value with resolved authority and context.
            ///
            /// The callback must verify the opaque signature against the selected
            /// signing authority and enforce any predecessor or parent relationship.
            ///
            /// # Errors
            ///
            /// Returns [`InvalidResidentRecordV1`] when structural, digest,
            /// signature-authority, or contextual admission fails.
            pub(crate) fn admit_wire<F>(wire: $wire, admit: F) -> Result<Self, InvalidResidentRecordV1>
            where
                F: FnOnce(
                    &$preimage,
                    &$digest,
                    &CanonicalSignatureV1,
                ) -> Result<(), InvalidResidentRecordV1>,
            {
                let preimage = $preimage {
                    schema_id: wire.schema_id.clone(),
                    schema_sha256: wire.schema_sha256.clone(),
                    $( $field: wire.$field.clone() ),+
                };
                if preimage.schema_id.as_str() != $schema {
                    return Err(InvalidResidentRecordV1::SchemaCoordinate { family: $family });
                }
                ($validate)(&preimage)?;
                let observed = <$digest>::hash_canonical(&preimage)?;
                if observed != wire.$digest_field {
                    return Err(InvalidResidentRecordV1::DigestMismatch { family: $family });
                }
                admit(&preimage, &observed, &wire.signature)?;
                Self::seal(preimage, wire.signature)
            }

            /// Borrow the stable initial-v1 schema identity.
            #[must_use] pub fn schema_id(&self) -> &crate::records::SchemaId { &self.schema_id }
            /// Borrow the exact selected JSON Schema digest.
            #[must_use] pub fn schema_sha256(&self) -> &crate::records::SchemaDigest { &self.schema_sha256 }
            $( $(#[$field_meta])* #[must_use] pub fn $field(&self) -> &$field_ty { &self.$field } )+
            #[doc = concat!("Borrow the canonical `", stringify!($digest_field), "`.")]
            #[must_use] pub fn $digest_field(&self) -> &$digest { &self.$digest_field }
            /// Borrow the opaque authority signature.
            #[must_use] pub fn signature(&self) -> &CanonicalSignatureV1 { &self.signature }
        }

        impl From<&$name> for $wire {
            /// Project an admitted signed record into its exact untrusted wire shape.
            ///
            /// This typed conversion preserves the record's existing fields. It
            /// performs no admission and allocates no intermediate JSON value.
            fn from(value: &$name) -> Self {
                Self {
                    schema_id: value.schema_id.clone(),
                    schema_sha256: value.schema_sha256.clone(),
                    $( $field: value.$field.clone(), )+
                    $digest_field: value.$digest_field.clone(),
                    signature: value.signature.clone(),
                }
            }
        }

        impl ResidentSchemaIdentityV1 for $name {
            const SCHEMA_FIELD: &'static str = "schema_id";
            const SCHEMA: &'static str = $schema;
            const VERSION: Option<(&'static str, u32)> = None;
        }

        impl ResidentSchemaIdentityV1 for $wire {
            const SCHEMA_FIELD: &'static str = "schema_id";
            const SCHEMA: &'static str = $schema;
            const VERSION: Option<(&'static str, u32)> = None;
        }

    };
}

pub(crate) use resident_signed_record;

/// Declare a signed semantic record whose structural wire fields require
/// record-owned contextual conversion before admission.
macro_rules! resident_signed_context_record {
    (
        $(#[$meta:meta])*
        $name:ident, $wire:ident, $preimage:ident,
        $digest_field:ident : $digest:ty,
        $schema:expr, $family:literal,
        { $( $(#[$field_meta:meta])* $field:ident : $field_ty:ty ),+ $(,)? },
        wire { $( $wire_field:ident : $wire_field_ty:ty ),+ $(,)? },
        $validate:expr
    ) => {
        $(#[$meta])*
        #[serde_with::skip_serializing_none]
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
        #[schemars(transform = close_resident_schema_identity_v1::<$name>)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            #[schemars(with = "String", extend("const" = $schema))]
            /// Stable initial-v1 schema identity.
            schema_id: crate::records::SchemaId,
            /// SHA-256 of the exact selected schema bytes.
            schema_sha256: crate::records::SchemaDigest,
            $( $(#[$field_meta])* $field: $field_ty, )+
            /// Canonical identity of the unsigned preimage.
            $digest_field: $digest,
            /// Opaque signature interpreted by the selected signing authority.
            signature: CanonicalSignatureV1,
        }

        #[doc = concat!("Validated unsigned construction preimage for [`", stringify!($name), "`].")]
        #[serde_with::skip_serializing_none]
        #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
        pub struct $preimage {
            /// Stable initial-v1 schema identity.
            pub schema_id: crate::records::SchemaId,
            /// SHA-256 of the exact selected schema bytes.
            pub schema_sha256: crate::records::SchemaDigest,
            $( $(#[$field_meta])* pub $field: $field_ty, )+
        }

        #[doc = concat!("Untrusted structural wire shape for [`", stringify!($name), "`].")]
        #[doc = "\n\nDeserializing this type establishes only field shape. Use the semantic"]
        #[doc = "record's owning contextual admission function before use."]
        #[serde_with::skip_serializing_none]
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
        #[schemars(transform = close_resident_schema_identity_v1::<$wire>)]
        #[serde(deny_unknown_fields)]
        pub struct $wire {
            #[schemars(with = "String", extend("const" = $schema))]
            schema_id: crate::records::SchemaId,
            schema_sha256: crate::records::SchemaDigest,
            $( $wire_field: $wire_field_ty, )+
            $digest_field: $digest,
            signature: CanonicalSignatureV1,
        }

        $crate::records::resident::impl_resident_wire_deserialize!($wire {
            schema_id: crate::records::SchemaId,
            schema_sha256: crate::records::SchemaDigest,
            $( $wire_field: $wire_field_ty, )+
            $digest_field: $digest,
            signature: CanonicalSignatureV1,
        });

        impl ResidentSchemaIdentityV1 for $name {
            const SCHEMA_FIELD: &'static str = "schema_id";
            const SCHEMA: &'static str = $schema;
            const VERSION: Option<(&'static str, u32)> = None;
        }

        impl ResidentSchemaIdentityV1 for $wire {
            const SCHEMA_FIELD: &'static str = "schema_id";
            const SCHEMA: &'static str = $schema;
            const VERSION: Option<(&'static str, u32)> = None;
        }

        impl $name {
            pub(crate) fn seal(
                preimage: $preimage,
                signature: CanonicalSignatureV1,
            ) -> Result<Self, InvalidResidentRecordV1> {
                if preimage.schema_id.as_str() != $schema {
                    return Err(InvalidResidentRecordV1::SchemaCoordinate { family: $family });
                }
                ($validate)(&preimage)?;
                let $digest_field = <$digest>::hash_canonical(&preimage)?;
                let $preimage { schema_id, schema_sha256, $( $field ),+ } = preimage;
                Ok(Self { schema_id, schema_sha256, $( $field ),+, $digest_field, signature })
            }

            pub(crate) fn admit_parts<F>(
                preimage: $preimage,
                claimed_digest: $digest,
                signature: CanonicalSignatureV1,
                admit: F,
            ) -> Result<Self, InvalidResidentRecordV1>
            where
                F: FnOnce(
                    &$preimage,
                    &$digest,
                    &CanonicalSignatureV1,
                ) -> Result<(), InvalidResidentRecordV1>,
            {
                if preimage.schema_id.as_str() != $schema {
                    return Err(InvalidResidentRecordV1::SchemaCoordinate { family: $family });
                }
                ($validate)(&preimage)?;
                let observed = <$digest>::hash_canonical(&preimage)?;
                if observed != claimed_digest {
                    return Err(InvalidResidentRecordV1::DigestMismatch { family: $family });
                }
                admit(&preimage, &observed, &signature)?;
                Self::seal(preimage, signature)
            }

            /// Revalidate structure and canonical unsigned identity.
            ///
            /// # Errors
            /// Returns [`InvalidResidentRecordV1`] when structure or the
            /// canonical digest no longer reproduces.
            pub fn verify(&self) -> Result<(), InvalidResidentRecordV1> {
                let preimage = $preimage {
                    schema_id: self.schema_id.clone(),
                    schema_sha256: self.schema_sha256.clone(),
                    $( $field: self.$field.clone() ),+
                };
                if preimage.schema_id.as_str() != $schema {
                    return Err(InvalidResidentRecordV1::SchemaCoordinate { family: $family });
                }
                ($validate)(&preimage)?;
                if <$digest>::hash_canonical(&preimage)? == self.$digest_field {
                    Ok(())
                } else {
                    Err(InvalidResidentRecordV1::DigestMismatch { family: $family })
                }
            }

            /// Borrow the stable initial-v1 schema identity.
            #[must_use]
            pub fn schema_id(&self) -> &crate::records::SchemaId { &self.schema_id }
            /// Borrow the exact selected JSON Schema digest.
            #[must_use]
            pub fn schema_sha256(&self) -> &crate::records::SchemaDigest { &self.schema_sha256 }
            $( $(#[$field_meta])* #[must_use] pub fn $field(&self) -> &$field_ty { &self.$field } )+
            #[doc = concat!("Borrow the canonical `", stringify!($digest_field), "`.")]
            #[must_use]
            pub fn $digest_field(&self) -> &$digest { &self.$digest_field }
            /// Borrow the opaque authority signature.
            #[must_use]
            pub fn signature(&self) -> &CanonicalSignatureV1 { &self.signature }
        }
    };
}

pub(crate) use resident_signed_context_record;

pub(crate) fn validate_signed_coordinates(
    family: &'static str,
    schema_sha256: &crate::records::SchemaDigest,
    expected_schema_sha256: &crate::records::SchemaDigest,
    issuer_principal_id: &ResidentPrincipalIdV1,
    issuer_key_id: &ResidentKeyIdV1,
    issuer_key_epoch: SigningKeyEpochV1,
    authority: &SigningAuthorityBindingV1,
) -> Result<(), InvalidResidentRecordV1> {
    authority.verify()?;
    if schema_sha256 != expected_schema_sha256 {
        return Err(InvalidResidentRecordV1::SchemaCoordinate { family });
    }
    if issuer_principal_id != authority.principal_id()
        || issuer_key_id.as_str() != authority.signer_key_id()
        || issuer_key_epoch != *authority.signer_key_epoch()
    {
        return Err(InvalidResidentRecordV1::ContextualAdmission {
            record: family,
            invariant: "exact signing authority",
        });
    }
    Ok(())
}

/// Bind a Pale-native runtime record to its exact deployment authority.
pub(crate) fn validate_integrated_runtime_authority_coordinates(
    family: &'static str,
    deployment_runtime_authority_ref: &ResidentRecordRefV1,
    deployment_runtime_authority_sha256: &crate::records::ArtifactDigest,
    authority: &SigningAuthorityBindingV1,
) -> Result<(), InvalidResidentRecordV1> {
    let runtime = authority.integrated_runtime_authority().ok_or(
        InvalidResidentRecordV1::ContextualAdmission {
            record: family,
            invariant: "integrated runtime signing authority",
        },
    )?;
    if deployment_runtime_authority_ref != &runtime.deployment_runtime_authority_ref
        || deployment_runtime_authority_sha256 != &runtime.deployment_runtime_authority_sha256
    {
        return Err(InvalidResidentRecordV1::ContextualAdmission {
            record: family,
            invariant: "exact deployment runtime authority",
        });
    }
    Ok(())
}

/// Admit an observation's move across one resolved runtime-authority rotation.
///
/// An unchanged ref/digest pair needs no additional resolution. A changed pair
/// must change both coordinates and `admit_rotation` must prove an admitted
/// succession within the same deployment-binding, control-store, and control-
/// incarnation lineage. The callback may traverse more than one authority
/// epoch; local validity of the successor alone is insufficient.
pub(crate) fn admit_runtime_authority_rotation<F>(
    family: &'static str,
    predecessor_ref: &ResidentRecordRefV1,
    predecessor_sha256: &crate::records::ArtifactDigest,
    successor_ref: &ResidentRecordRefV1,
    successor_sha256: &crate::records::ArtifactDigest,
    admit_rotation: F,
) -> Result<(), InvalidResidentRecordV1>
where
    F: FnOnce(
        &ResidentRecordRefV1,
        &crate::records::ArtifactDigest,
        &ResidentRecordRefV1,
        &crate::records::ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    let ref_changed = predecessor_ref != successor_ref;
    let digest_changed = predecessor_sha256 != successor_sha256;
    match (ref_changed, digest_changed) {
        (false, false) => Ok(()),
        (true, true) => admit_rotation(
            predecessor_ref,
            predecessor_sha256,
            successor_ref,
            successor_sha256,
        ),
        _ => Err(InvalidResidentRecordV1::ContextualAdmission {
            record: family,
            invariant: "coherent deployment runtime authority rotation",
        }),
    }
}

/// Resolved schema and signing authority shared by signed resident admission.
#[derive(Clone, Copy)]
pub struct SignedResidentAdmissionContextV1<'a> {
    /// Digest of the exact selected record-schema bytes.
    pub expected_schema_sha256: &'a crate::records::SchemaDigest,
    /// Exact authority record used to verify the resident signature.
    pub signing_authority: &'a SigningAuthorityBindingV1,
}
