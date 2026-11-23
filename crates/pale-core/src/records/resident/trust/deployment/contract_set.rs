//! Exact immutable contract selection for one compiled deployment.
//!
//! A contract set is an ordered value, not a mutable-registry query. Each
//! member binds the canonical contract identity to one immutable schema
//! reference and the digest of those schema bytes. Compatibility remains an
//! owner-resolved admission obligation; carrying its reference and digest does
//! not make the profile true.

use super::*;
use serde::Serialize;

/// Public schema root for immutable deployment contract sets.
pub const CONTRACT_SET_V1_SCHEMA_ID: &str = "contract-set.v1";
/// Wire schema carried by every immutable deployment contract set.
pub const CONTRACT_SET_V1_SCHEMA: &str = "pale/contract-set.v1";

/// One exact owner-published schema selected into a deployment contract set.
///
/// Ordering is lexicographic over all three members. The enclosing
/// [`ContractSetV1`] additionally requires `contract_id` to be unique, so two
/// different schema references cannot claim the same canonical contract.
#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct ContractSetEntryV1 {
    /// Canonical identity declared by the selected owner-published schema.
    contract_id: SchemaId,
    /// Immutable reference resolving the exact schema bytes.
    contract_schema_ref: ResidentRecordRefV1,
    /// SHA-256 of the raw schema bytes resolved by `contract_schema_ref`.
    contract_schema_sha256: SchemaDigest,
}

impl ContractSetEntryV1 {
    /// Construct one structural contract-set entry.
    ///
    /// This constructor binds claims only. [`ContractSetV1::admit_contextual_wire`]
    /// requires the contract owner to resolve and validate them before the set
    /// becomes admitted.
    #[must_use]
    pub const fn new(
        contract_id: SchemaId,
        contract_schema_ref: ResidentRecordRefV1,
        contract_schema_sha256: SchemaDigest,
    ) -> Self {
        Self {
            contract_id,
            contract_schema_ref,
            contract_schema_sha256,
        }
    }

    /// Borrow the selected canonical contract identity.
    #[must_use]
    pub const fn contract_id(&self) -> &SchemaId {
        &self.contract_id
    }

    /// Borrow the immutable schema reference.
    #[must_use]
    pub const fn contract_schema_ref(&self) -> &ResidentRecordRefV1 {
        &self.contract_schema_ref
    }

    /// Borrow the digest of the resolved schema bytes.
    #[must_use]
    pub const fn contract_schema_sha256(&self) -> &SchemaDigest {
        &self.contract_schema_sha256
    }
}

resident_integrity_record!(
    /// Immutable, compatibility-qualified schema closure selected for one deployment.
    ContractSetV1,
    ContractSetWireV1,
    ContractSetPreimageV1,
    canonical_contract_set_sha256: ContractSetDigest,
    CONTRACT_SET_V1_SCHEMA,
    "contract set",
    {
        /// Stable identity of this contract-set value.
        contract_set_id: ContractSetIdV1,
        /// Strictly ordered, duplicate-free selected contract schemas.
        entries: Vec<ContractSetEntryV1>,
        /// Immutable reference to owner-resolved compatibility evidence.
        compatibility_profile_ref: ResidentRecordRefV1,
        /// Canonical digest of the resolved compatibility profile.
        compatibility_profile_sha256: CompatibilityProfileDigest
    },
    validate_contract_set_shape,
    wire_transform = contract_set_wire_schema
);

impl ContractSetV1 {
    /// Admit an exact contract set after resolving every selected schema and
    /// its compatibility profile.
    ///
    /// `resolve_contract` must resolve the entry's immutable reference, compare
    /// the raw schema bytes with its digest, and establish that those bytes
    /// declare the same canonical contract identity. `resolve_compatibility`
    /// must establish that the exact profile applies to the complete selected
    /// set. Neither callback may treat an opaque reference as proof.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when the schema coordinate,
    /// canonical set identity, order, uniqueness, selected contract, or
    /// compatibility evidence is invalid.
    pub fn admit_contextual_wire<C, P>(
        wire: ContractSetWireV1,
        expected_schema_sha256: &SchemaDigest,
        mut resolve_contract: C,
        resolve_compatibility: P,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        C: FnMut(&ContractSetEntryV1) -> Result<(), InvalidResidentRecordV1>,
        P: FnOnce(
            &ResidentRecordRefV1,
            &CompatibilityProfileDigest,
            &[ContractSetEntryV1],
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, _digest| {
            if preimage.schema_sha256 != *expected_schema_sha256 {
                return Err(InvalidResidentRecordV1::SchemaCoordinate {
                    family: "contract set",
                });
            }
            for entry in &preimage.entries {
                resolve_contract(entry)?;
            }
            resolve_compatibility(
                &preimage.compatibility_profile_ref,
                &preimage.compatibility_profile_sha256,
                &preimage.entries,
            )
        })
    }
}

fn validate_contract_set_shape(
    value: &ContractSetPreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    strictly_sorted("contract set entries", &value.entries)?;
    if value
        .entries
        .windows(2)
        .any(|pair| pair[0].contract_id == pair[1].contract_id)
    {
        return Err(InvalidResidentRecordV1::NonCanonical {
            relation: "contract set contract ids",
        });
    }
    Ok(())
}

fn contract_set_wire_schema(schema: &mut schemars::Schema) {
    project_resident_set_property_v1(schema, "entries", None);
}
