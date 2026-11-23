//! Resident generation, session, obligation, evidence, and reconciliation contracts.
//!
//! Generation-head lineages are sparse projections of one store-global journal.
//! For family heads `H_i`, with `s` projecting their journal sequence, a
//! successor may skip unrelated entries but must strictly advance:

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/generation-head-lineage.inline.mathml"
))]
//!
//! The first record may occupy any positive position and has no same-family
//! predecessor. Every later record names its exact same-family predecessor. The
//! journal append protocol, not an individual head record, proves adjacency to
//! the global store head.
use serde::{Deserialize, Serialize};

use super::*;
use crate::records::*;

/// Public schema root for durable published-generation head wires.
pub const PUBLISHED_GENERATION_HEAD_V1_SCHEMA_ID: &str = "published-generation-head.v1";
/// Wire schema carried by every published-generation head.
pub const PUBLISHED_GENERATION_HEAD_V1_SCHEMA: &str = "pale/published-generation-head.v1";
/// Wire version carried by every published-generation head.
pub const PUBLISHED_GENERATION_HEAD_V1_SCHEMA_VERSION: u32 = 1;
/// Public schema root for advertised-generation head wires.
pub const ADVERTISED_GENERATION_HEAD_V1_SCHEMA_ID: &str = "advertised-generation-head.v1";
/// Wire schema carried by every advertised-generation head.
pub const ADVERTISED_GENERATION_HEAD_V1_SCHEMA: &str = "pale/advertised-generation-head.v1";
/// Wire version carried by every advertised-generation head.
pub const ADVERTISED_GENERATION_HEAD_V1_SCHEMA_VERSION: u32 = 1;
/// Public schema root for active-generation head wires.
pub const ACTIVE_GENERATION_HEAD_V1_SCHEMA_ID: &str = "active-generation-head.v1";
/// Wire schema carried by every active-generation head.
pub const ACTIVE_GENERATION_HEAD_V1_SCHEMA: &str = "pale/active-generation-head.v1";
/// Wire version carried by every active-generation head.
pub const ACTIVE_GENERATION_HEAD_V1_SCHEMA_VERSION: u32 = 1;
/// Public schema root for activation-fence wires.
pub const ACTIVATION_FENCE_V1_SCHEMA_ID: &str = "activation-fence.v1";
/// Wire schema carried by every activation fence.
pub const ACTIVATION_FENCE_V1_SCHEMA: &str = "pale/activation-fence.v1";
/// Wire version carried by every activation fence.
pub const ACTIVATION_FENCE_V1_SCHEMA_VERSION: u32 = 1;
/// Public schema root for ambient session-admission lease wires.
pub const SESSION_ADMISSION_LEASE_V1_SCHEMA_ID: &str = "session-admission-lease.v1";
/// Wire schema carried by every session-admission lease.
pub const SESSION_ADMISSION_LEASE_V1_SCHEMA: &str = "pale/session-admission-lease.v1";
/// Wire version carried by every session-admission lease.
pub const SESSION_ADMISSION_LEASE_V1_SCHEMA_VERSION: u32 = 1;
/// Signature domain for [`SessionAdmissionLeaseV1`].
pub const SESSION_ADMISSION_LEASE_V1_SIGNATURE_DOMAIN: &[u8] = b"PALE_SESSION_ADMISSION_LEASE_V1\0";
/// Public schema root for nested operation-lease wires.
pub const OPERATION_LEASE_V1_SCHEMA_ID: &str = "operation-lease.v1";
/// Wire schema carried by every operation lease.
pub const OPERATION_LEASE_V1_SCHEMA: &str = "pale/operation-lease.v1";
/// Wire version carried by every operation lease.
pub const OPERATION_LEASE_V1_SCHEMA_VERSION: u32 = 1;

/// Public schema root for immutable compiled-generation wires.
pub const COMPILED_GENERATION_V1_SCHEMA_ID: &str = "compiled-generation.v1";
/// Wire schema carried by every compiled generation.
pub const COMPILED_GENERATION_V1_SCHEMA: &str = "pale/compiled-generation.v1";
/// Wire version carried by every compiled generation.
pub const COMPILED_GENERATION_V1_SCHEMA_VERSION: u32 = 1;

resident_record!(
    /// Immutable, reusable compilation admitted before publication or activation.
    ///
    /// The canonical identity covers the complete admitted compilation and
    /// source frontier. Deployment activation remains a separate monotone
    /// lineage owned by [`ActivationFenceV1`] and [`ActiveGenerationHeadV1`].
    pub CompiledGenerationV1, CompiledGenerationWireV1, CompiledGenerationPreimageV1,
    compiled_generation_digest: CompiledGenerationDigest,
    COMPILED_GENERATION_V1_SCHEMA, COMPILED_GENERATION_V1_SCHEMA_VERSION, "compiled generation",
    {
        /// Canonical identity of the exact distribution lock admitted for compilation.
        distribution_lock_digest: DistributionLockDigest,
        /// Canonical identity of the selected portable profile.
        profile_digest: ProfileDigest,
        /// Canonical identity of the profile after machine-specific resolution.
        instance_digest: InstanceDigest,
        /// Canonical identity of the compiled Package semantic intermediate representation.
        package_semantic_ir_digest: PackageSemanticIrDigest,
        /// Exact profile-selected Package releases consumed by compilation.
        profile_package_release_closure: Vec<PackageReleaseV1>,
        /// Exact profile-selected normative Pack releases consumed by compilation.
        profile_normative_pack_release_closure: Vec<NormativePackReleaseV1>,
        /// Digest of the schema manifest governing every compiled record shape.
        schema_manifest_digest: SchemaManifestDigest,
        /// Digest of the complete selected schema set.
        schema_set_digest: SchemaSetDigest,
        /// Canonical identity of the compiled principal-workspace semantics.
        workspace_semantic_ir_digest: WorkspaceSemanticIrDigest,
        /// Identity of the owner-proof closure consumed by workspace admission.
        workspace_admission_evidence_digest: WorkspaceAdmissionEvidenceDigest,
        /// Canonical identity of the principal-relative admitted workspace universe.
        principal_admitted_universe_digest: PrincipalAdmittedUniverseDigest,
        /// Exact canonically ordered workspace module releases admitted for compilation.
        workspace_module_release_closure: Vec<WorkspaceAdoptedReleaseV1>,
        /// Package releases selected through workspace resources.
        workspace_package_release_closure: Vec<PackageReleaseV1>,
        /// Normative Pack releases selected through workspace resources.
        workspace_normative_pack_release_closure: Vec<NormativePackReleaseV1>,
        /// Canonical identity of exact workspace dependency resolution.
        workspace_resolution_lock_digest: WorkspaceResolutionLockDigest,
        /// Canonical identity of the resolved workspace universe.
        workspace_universe_lock_digest: WorkspaceUniverseLockDigest,
        /// Digest of the responsibility derivations consumed by composition.
        responsibility_derivation_closure_digest: ResponsibilityDerivationClosureDigest,
        /// Digest of the optional organizational translations consumed by composition.
        organizational_translation_closure_digest: OrganizationalTranslationClosureDigest,
        /// Digest of the explicit neutral experience-composition sources.
        experience_composition_source_digest: ExperienceCompositionSourceDigest,
        /// Digest of the admitted provider-binding closure.
        provider_binding_closure_digest: ProviderBindingClosureDigest,
        /// Canonical identity of the complete experience-composition plan.
        experience_composition_plan_digest: ExperienceCompositionPlanDigest,
        /// Canonically ordered neutral execution paths selected by the profile.
        execution_paths: Vec<ExecutionPathSelectionV1>,
        /// Digest of every materialization selected for this generation.
        materialization_closure_digest: MaterializationClosureDigest,
        /// Digest of the machine-only overlay bound after portable compilation.
        machine_overlay_digest: MachineOverlayDigest,
        /// Digest of the surface executable identity admitted for execution.
        executable_identity_digest: ExecutableIdentityDigest,
        /// Identity of the exact pre-render compiler inputs.
        compilation_input_digest: CompilationInputDigest,
        /// Identity of the complete post-render one-shot compilation.
        one_shot_compilation_digest: OneShotCompilationDigest,
        /// Exact repository and dependency frontiers from which compilation read.
        repository_source_frontiers: Vec<RepositorySourceFrontierV1>
    },
    |value: &CompiledGenerationPreimageV1| {
        validate_release_closures(
            &value.profile_package_release_closure,
            &value.profile_normative_pack_release_closure,
        )?;
        validate_release_closures(
            &value.workspace_package_release_closure,
            &value.workspace_normative_pack_release_closure,
        )?;
        validate_workspace_module_release_closure(&value.workspace_module_release_closure)?;
        bounded_len("compiled execution paths", value.execution_paths.len())?;
        if !value
            .execution_paths
            .windows(2)
            .all(|pair| pair[0].id < pair[1].id)
        {
            return Err(InvalidResidentRecordV1::NonCanonical {
                relation: "compiled execution paths",
            });
        }
        validate_repository_source_frontiers(&value.repository_source_frontiers)
    }
);

/// Exact owner-admitted inputs required to admit a compiled-generation wire.
///
/// Callers derive materialization identity from verified applied receipts,
/// executable identity from the probe-bound executable producer, and every other
/// member from its named owner before constructing this context.
pub struct CompiledGenerationAdmissionContextV1<'a> {
    /// Complete owner-resolved preimage expected on the untrusted wire.
    pub admitted_preimage: &'a CompiledGenerationPreimageV1,
}

impl CompiledGenerationV1 {
    /// Consume owner-admitted compilation inputs and seal the immutable aggregate.
    ///
    /// This function validates canonical closure ordering, bounds, schema
    /// coordinates, and the aggregate digest. The caller remains responsible
    /// for obtaining each typed coordinate from the owner named by its field.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when the preimage is structurally
    /// invalid or cannot satisfy the canonical record contract.
    pub fn admit_owner_preimage(
        preimage: CompiledGenerationPreimageV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        Self::seal(preimage)
    }

    /// Admit an untrusted compiled generation against all owner-resolved inputs.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when any admitted coordinate differs,
    /// a closure is not canonical, or the claimed canonical identity is false.
    pub fn admit_contextual_wire(
        wire: CompiledGenerationWireV1,
        context: &CompiledGenerationAdmissionContextV1<'_>,
    ) -> Result<Self, InvalidResidentRecordV1> {
        let claimed_digest = wire.compiled_generation_digest;
        let preimage = CompiledGenerationPreimageV1 {
            schema: wire.schema,
            schema_sha256: wire.schema_sha256,
            schema_version: wire.schema_version,
            distribution_lock_digest: wire.distribution_lock_digest,
            profile_digest: wire.profile_digest,
            instance_digest: wire.instance_digest,
            package_semantic_ir_digest: wire.package_semantic_ir_digest,
            profile_package_release_closure: wire.profile_package_release_closure,
            profile_normative_pack_release_closure: wire.profile_normative_pack_release_closure,
            schema_manifest_digest: wire.schema_manifest_digest,
            schema_set_digest: wire.schema_set_digest,
            workspace_semantic_ir_digest: wire.workspace_semantic_ir_digest,
            workspace_admission_evidence_digest: wire.workspace_admission_evidence_digest,
            principal_admitted_universe_digest: wire.principal_admitted_universe_digest,
            workspace_module_release_closure: wire.workspace_module_release_closure,
            workspace_package_release_closure: wire.workspace_package_release_closure,
            workspace_normative_pack_release_closure: wire.workspace_normative_pack_release_closure,
            workspace_resolution_lock_digest: wire.workspace_resolution_lock_digest,
            workspace_universe_lock_digest: wire.workspace_universe_lock_digest,
            responsibility_derivation_closure_digest: wire.responsibility_derivation_closure_digest,
            organizational_translation_closure_digest: wire
                .organizational_translation_closure_digest,
            experience_composition_source_digest: wire.experience_composition_source_digest,
            provider_binding_closure_digest: wire.provider_binding_closure_digest,
            experience_composition_plan_digest: wire.experience_composition_plan_digest,
            execution_paths: wire.execution_paths,
            materialization_closure_digest: wire.materialization_closure_digest,
            machine_overlay_digest: wire.machine_overlay_digest,
            executable_identity_digest: wire.executable_identity_digest,
            compilation_input_digest: wire.compilation_input_digest,
            one_shot_compilation_digest: wire.one_shot_compilation_digest,
            repository_source_frontiers: wire.repository_source_frontiers,
        };
        if preimage != *context.admitted_preimage {
            return Err(contextual(
                "compiled generation",
                "complete admitted inputs",
            ));
        }
        let admitted = Self::admit_owner_preimage(preimage)?;
        if admitted.compiled_generation_digest != claimed_digest {
            return Err(InvalidResidentRecordV1::DigestMismatch {
                family: "compiled generation",
            });
        }
        Ok(admitted)
    }
}

fn validate_workspace_module_release_closure(
    releases: &[WorkspaceAdoptedReleaseV1],
) -> Result<(), InvalidResidentRecordV1> {
    bounded_len("compiled workspace module releases", releases.len())?;
    if releases
        .windows(2)
        .all(|pair| pair[0].module_id < pair[1].module_id)
    {
        Ok(())
    } else {
        Err(InvalidResidentRecordV1::NonCanonical {
            relation: "compiled workspace module releases",
        })
    }
}

fn validate_repository_source_frontiers(
    frontiers: &[RepositorySourceFrontierV1],
) -> Result<(), InvalidResidentRecordV1> {
    bounded_len("compiled repository source frontiers", frontiers.len())?;
    if frontiers
        .windows(2)
        .all(|pair| pair[0].repository_id < pair[1].repository_id)
    {
        Ok(())
    } else {
        Err(InvalidResidentRecordV1::NonCanonical {
            relation: "compiled repository source frontiers",
        })
    }
}

fn contextual(record: &'static str, invariant: &'static str) -> InvalidResidentRecordV1 {
    InvalidResidentRecordV1::ContextualAdmission { record, invariant }
}

#[cfg(test)]
#[path = "../../../tests/internal/records/resident/lifecycle.rs"]
mod tests;

resident_record!(
    /// Durable published head; it does not advertise or activate the generation.
    pub PublishedGenerationHeadV1, PublishedGenerationHeadWireV1, PublishedGenerationHeadPreimageV1,
    published_generation_head_digest: PublishedGenerationHeadDigest,
    PUBLISHED_GENERATION_HEAD_V1_SCHEMA, PUBLISHED_GENERATION_HEAD_V1_SCHEMA_VERSION, "published generation head",
    {
        /// Immutable generation made durable by this publication record.
        compiled_generation_digest: CompiledGenerationDigest,
        /// Journal position at which publication became authoritative.
        journal_sequence: JournalSequenceV1,
        /// Prior publication head, absent only for this family's first journal event.
        predecessor_published_generation_head_digest: Option<PublishedGenerationHeadDigest>
    },
    |value: &PublishedGenerationHeadPreimageV1| {
        if value.journal_sequence.get() == 1
            && value.predecessor_published_generation_head_digest.is_some()
        {
            Err(InvalidResidentRecordV1::InvalidCoordinate {
                coordinate: "published head predecessor",
            })
        } else {
            Ok(())
        }
    }
);

/// Owner-resolved inputs for one published-generation head.
pub struct PublishedGenerationHeadAdmissionContextV1<'a> {
    /// Digest of the exact selected published-head schema bytes.
    pub expected_schema_sha256: &'a SchemaDigest,
    /// Exact immutable generation made durable by this publication.
    pub compiled_generation: &'a CompiledGenerationV1,
    /// Exact prior publication, absent only for this family's first event.
    pub predecessor: Option<&'a PublishedGenerationHeadV1>,
}

impl PublishedGenerationHeadV1 {
    /// Admit a published head against the exact generation and prior head.
    ///
    /// Journal order is global while publication lineage is family-local. The
    /// strict sequence relation is rendered at module level; the
    /// durable journal owns exact global adjacency, while this record binds the
    /// latest prior publication across interleaved events.
    /// Publication does not advertise or activate the compiled generation.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when schema, generation, lineage, or
    /// canonical identity differs from the resolved context.
    pub fn admit_contextual_wire(
        wire: PublishedGenerationHeadWireV1,
        context: &PublishedGenerationHeadAdmissionContextV1<'_>,
    ) -> Result<Self, InvalidResidentRecordV1> {
        let claimed_digest = wire.published_generation_head_digest;
        let preimage = PublishedGenerationHeadPreimageV1 {
            schema: wire.schema,
            schema_sha256: wire.schema_sha256,
            schema_version: wire.schema_version,
            compiled_generation_digest: wire.compiled_generation_digest,
            journal_sequence: wire.journal_sequence,
            predecessor_published_generation_head_digest: wire
                .predecessor_published_generation_head_digest,
        };
        context.compiled_generation.verify()?;
        if preimage.schema_sha256 != *context.expected_schema_sha256
            || preimage.compiled_generation_digest
                != *context.compiled_generation.compiled_generation_digest()
        {
            return Err(contextual(
                "published generation head",
                "schema and compiled generation",
            ));
        }
        let predecessor = context.predecessor.map(|value| {
            (
                *value.journal_sequence(),
                value.published_generation_head_digest(),
            )
        });
        if let Some(value) = context.predecessor {
            value.verify()?;
        }
        validate_head_lineage(
            "published generation head",
            preimage.journal_sequence,
            preimage
                .predecessor_published_generation_head_digest
                .as_ref(),
            predecessor,
        )?;
        let admitted = Self::seal(preimage)?;
        ensure_claimed_digest(
            admitted.published_generation_head_digest(),
            &claimed_digest,
            "published generation head",
        )?;
        Ok(admitted)
    }
}

resident_record!(
    /// Advertised head eligible for admission; it is not the active head.
    pub AdvertisedGenerationHeadV1, AdvertisedGenerationHeadWireV1, AdvertisedGenerationHeadPreimageV1,
    advertised_generation_head_digest: AdvertisedGenerationHeadDigest,
    ADVERTISED_GENERATION_HEAD_V1_SCHEMA, ADVERTISED_GENERATION_HEAD_V1_SCHEMA_VERSION, "advertised generation head",
    {
        /// Immutable generation made eligible for new admission.
        compiled_generation_digest: CompiledGenerationDigest,
        /// Exact durable publication on which advertisement depends.
        published_generation_head_digest: PublishedGenerationHeadDigest,
        /// Journal position at which advertisement became authoritative.
        journal_sequence: JournalSequenceV1,
        /// Prior advertisement head, absent only for this family's first journal event.
        predecessor_advertised_generation_head_digest: Option<AdvertisedGenerationHeadDigest>
    },
    |value: &AdvertisedGenerationHeadPreimageV1| {
        if value.journal_sequence.get() == 1
            && value.predecessor_advertised_generation_head_digest.is_some()
        {
            Err(InvalidResidentRecordV1::InvalidCoordinate {
                coordinate: "advertised head predecessor",
            })
        } else {
            Ok(())
        }
    }
);

/// Owner-resolved inputs for one advertised-generation head.
pub struct AdvertisedGenerationHeadAdmissionContextV1<'a> {
    /// Digest of the exact selected advertised-head schema bytes.
    pub expected_schema_sha256: &'a SchemaDigest,
    /// Exact durable publication made eligible for admission.
    pub published_head: &'a PublishedGenerationHeadV1,
    /// Exact prior advertisement, absent only for this family's first event.
    pub predecessor: Option<&'a AdvertisedGenerationHeadV1>,
}

impl AdvertisedGenerationHeadV1 {
    /// Admit an advertised head against its publication and prior advertisement.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when the advertised generation is
    /// not the published generation or its family-local lineage does not move
    /// forward in the store-global journal.
    pub fn admit_contextual_wire(
        wire: AdvertisedGenerationHeadWireV1,
        context: &AdvertisedGenerationHeadAdmissionContextV1<'_>,
    ) -> Result<Self, InvalidResidentRecordV1> {
        let claimed_digest = wire.advertised_generation_head_digest;
        let preimage = AdvertisedGenerationHeadPreimageV1 {
            schema: wire.schema,
            schema_sha256: wire.schema_sha256,
            schema_version: wire.schema_version,
            compiled_generation_digest: wire.compiled_generation_digest,
            published_generation_head_digest: wire.published_generation_head_digest,
            journal_sequence: wire.journal_sequence,
            predecessor_advertised_generation_head_digest: wire
                .predecessor_advertised_generation_head_digest,
        };
        context.published_head.verify()?;
        if preimage.schema_sha256 != *context.expected_schema_sha256
            || preimage.published_generation_head_digest
                != *context.published_head.published_generation_head_digest()
            || preimage.compiled_generation_digest
                != *context.published_head.compiled_generation_digest()
        {
            return Err(contextual(
                "advertised generation head",
                "exact published generation",
            ));
        }
        let predecessor = context.predecessor.map(|value| {
            (
                *value.journal_sequence(),
                value.advertised_generation_head_digest(),
            )
        });
        if let Some(value) = context.predecessor {
            value.verify()?;
        }
        validate_head_lineage(
            "advertised generation head",
            preimage.journal_sequence,
            preimage
                .predecessor_advertised_generation_head_digest
                .as_ref(),
            predecessor,
        )?;
        let admitted = Self::seal(preimage)?;
        ensure_claimed_digest(
            admitted.advertised_generation_head_digest(),
            &claimed_digest,
            "advertised generation head",
        )?;
        Ok(admitted)
    }
}

resident_record!(
    /// Active head selected for new sessions; existing sessions remain pinned.
    pub ActiveGenerationHeadV1, ActiveGenerationHeadWireV1, ActiveGenerationHeadPreimageV1,
    active_generation_head_digest: ActiveGenerationHeadDigest,
    ACTIVE_GENERATION_HEAD_V1_SCHEMA, ACTIVE_GENERATION_HEAD_V1_SCHEMA_VERSION, "active generation head",
    {
        /// Immutable generation selected for new sessions.
        compiled_generation_digest: CompiledGenerationDigest,
        /// Exact advertisement admitted before activation.
        advertised_generation_head_digest: AdvertisedGenerationHeadDigest,
        /// Deployment-adoption fence that distinguishes this activation lineage.
        activation_fence_digest: ActivationFenceDigest,
        /// Journal position at which the head became active.
        journal_sequence: JournalSequenceV1,
        /// Prior active head, absent only for this family's first journal event.
        predecessor_active_generation_head_digest: Option<ActiveGenerationHeadDigest>
    },
    |value: &ActiveGenerationHeadPreimageV1| {
        if value.journal_sequence.get() == 1
            && value.predecessor_active_generation_head_digest.is_some()
        {
            Err(InvalidResidentRecordV1::InvalidCoordinate {
                coordinate: "active head predecessor",
            })
        } else {
            Ok(())
        }
    }
);

/// Owner-resolved inputs for one active-generation head.
pub struct ActiveGenerationHeadAdmissionContextV1<'a> {
    /// Digest of the exact selected active-head schema bytes.
    pub expected_schema_sha256: &'a SchemaDigest,
    /// Exact advertisement selected for new sessions.
    pub advertised_head: &'a AdvertisedGenerationHeadV1,
    /// Exact activation fence governing the selected deployment adoption.
    pub activation_fence: &'a ActivationFenceV1,
    /// Exact prior active head, absent only for this family's first event.
    pub predecessor: Option<&'a ActiveGenerationHeadV1>,
}

impl ActiveGenerationHeadV1 {
    /// Admit an active head against its advertisement, fence, and prior head.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] unless advertisement and fence name
    /// the same compiled generation and the active-head lineage moves forward
    /// in the store-global journal.
    pub fn admit_contextual_wire(
        wire: ActiveGenerationHeadWireV1,
        context: &ActiveGenerationHeadAdmissionContextV1<'_>,
    ) -> Result<Self, InvalidResidentRecordV1> {
        let claimed_digest = wire.active_generation_head_digest;
        let preimage = ActiveGenerationHeadPreimageV1 {
            schema: wire.schema,
            schema_sha256: wire.schema_sha256,
            schema_version: wire.schema_version,
            compiled_generation_digest: wire.compiled_generation_digest,
            advertised_generation_head_digest: wire.advertised_generation_head_digest,
            activation_fence_digest: wire.activation_fence_digest,
            journal_sequence: wire.journal_sequence,
            predecessor_active_generation_head_digest: wire
                .predecessor_active_generation_head_digest,
        };
        context.advertised_head.verify()?;
        context.activation_fence.verify()?;
        if preimage.schema_sha256 != *context.expected_schema_sha256
            || preimage.advertised_generation_head_digest
                != *context.advertised_head.advertised_generation_head_digest()
            || preimage.activation_fence_digest
                != *context.activation_fence.activation_fence_digest()
            || preimage.compiled_generation_digest
                != *context.advertised_head.compiled_generation_digest()
            || preimage.compiled_generation_digest
                != *context.activation_fence.compiled_generation_digest()
        {
            return Err(contextual(
                "active generation head",
                "advertised generation and activation fence",
            ));
        }
        let predecessor = context.predecessor.map(|value| {
            (
                *value.journal_sequence(),
                value.active_generation_head_digest(),
            )
        });
        if let Some(value) = context.predecessor {
            value.verify()?;
        }
        validate_head_lineage(
            "active generation head",
            preimage.journal_sequence,
            preimage.predecessor_active_generation_head_digest.as_ref(),
            predecessor,
        )?;
        let admitted = Self::seal(preimage)?;
        ensure_claimed_digest(
            admitted.active_generation_head_digest(),
            &claimed_digest,
            "active generation head",
        )?;
        Ok(admitted)
    }
}

resident_record!(
    /// Deployment-adoption fence, distinct from compiled content and native revision.
    pub ActivationFenceV1, ActivationFenceWireV1, ActivationFencePreimageV1,
    activation_fence_digest: ActivationFenceDigest,
    ACTIVATION_FENCE_V1_SCHEMA, ACTIVATION_FENCE_V1_SCHEMA_VERSION, "activation fence",
    {
        /// Stable identity shared by every generation of this deployment fence.
        activation_fence_id: ActivationFenceIdV1,
        /// Positive generation fencing stale deployment adopters.
        activation_fence_generation: ActivationFenceGenerationV1,
        /// Exact compiled generation adopted at this fence generation.
        compiled_generation_digest: CompiledGenerationDigest,
        /// Prior fence generation, absent only for generation one.
        predecessor_activation_fence_digest: Option<ActivationFenceDigest>
    },
    |value: &ActivationFencePreimageV1| {
        if (value.activation_fence_generation.get() == 1)
            != value.predecessor_activation_fence_digest.is_none()
        { Err(InvalidResidentRecordV1::InvalidCoordinate { coordinate: "activation fence predecessor" }) }
        else { Ok(()) }
    }
);

/// Owner-resolved inputs for one activation-fence generation.
pub struct ActivationFenceAdmissionContextV1<'a> {
    /// Digest of the exact selected activation-fence schema bytes.
    pub expected_schema_sha256: &'a SchemaDigest,
    /// Exact compiled generation adopted by this fence generation.
    pub compiled_generation: &'a CompiledGenerationV1,
    /// Exact prior fence generation, absent only for generation one.
    pub predecessor: Option<&'a ActivationFenceV1>,
}

impl ActivationFenceV1 {
    /// Admit an activation fence against its compiled generation and predecessor.
    ///
    /// A successor preserves fence identity and advances its generation exactly
    /// once under the resident-record law; reactivating identical content
    /// therefore still receives a distinct deployment-adoption coordinate.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when generation, fence identity,
    /// predecessor, or schema coordinates do not agree exactly.
    pub fn admit_contextual_wire(
        wire: ActivationFenceWireV1,
        context: &ActivationFenceAdmissionContextV1<'_>,
    ) -> Result<Self, InvalidResidentRecordV1> {
        let claimed_digest = wire.activation_fence_digest;
        let preimage = ActivationFencePreimageV1 {
            schema: wire.schema,
            schema_sha256: wire.schema_sha256,
            schema_version: wire.schema_version,
            activation_fence_id: wire.activation_fence_id,
            activation_fence_generation: wire.activation_fence_generation,
            compiled_generation_digest: wire.compiled_generation_digest,
            predecessor_activation_fence_digest: wire.predecessor_activation_fence_digest,
        };
        context.compiled_generation.verify()?;
        if preimage.schema_sha256 != *context.expected_schema_sha256
            || preimage.compiled_generation_digest
                != *context.compiled_generation.compiled_generation_digest()
        {
            return Err(contextual(
                "activation fence",
                "schema and compiled generation",
            ));
        }
        validate_fence_lineage(&preimage, context.predecessor)?;
        let admitted = Self::seal(preimage)?;
        ensure_claimed_digest(
            admitted.activation_fence_digest(),
            &claimed_digest,
            "activation fence",
        )?;
        Ok(admitted)
    }
}

fn validate_head_lineage<D: PartialEq>(
    record: &'static str,
    sequence: JournalSequenceV1,
    claimed_predecessor: Option<&D>,
    resolved_predecessor: Option<(JournalSequenceV1, &D)>,
) -> Result<(), InvalidResidentRecordV1> {
    match resolved_predecessor {
        None if claimed_predecessor.is_none() => Ok(()),
        Some((prior_sequence, prior_digest))
            if sequence.get() > prior_sequence.get()
                && claimed_predecessor == Some(prior_digest) =>
        {
            Ok(())
        }
        _ => Err(contextual(
            record,
            "exact predecessor and later global journal sequence",
        )),
    }
}

fn validate_fence_lineage(
    value: &ActivationFencePreimageV1,
    predecessor: Option<&ActivationFenceV1>,
) -> Result<(), InvalidResidentRecordV1> {
    match predecessor {
        None if value.activation_fence_generation.get() == 1
            && value.predecessor_activation_fence_digest.is_none() =>
        {
            Ok(())
        }
        Some(prior) => {
            prior.verify()?;
            let exact_successor = prior.activation_fence_generation().checked_successor()
                == Some(value.activation_fence_generation);
            if exact_successor
                && value.activation_fence_id == *prior.activation_fence_id()
                && value.predecessor_activation_fence_digest.as_ref()
                    == Some(prior.activation_fence_digest())
            {
                Ok(())
            } else {
                Err(contextual(
                    "activation fence",
                    "stable fence identity and exact successor",
                ))
            }
        }
        None => Err(contextual("activation fence", "initial fence generation")),
    }
}

fn ensure_claimed_digest<D: PartialEq>(
    observed: &D,
    claimed: &D,
    family: &'static str,
) -> Result<(), InvalidResidentRecordV1> {
    if observed == claimed {
        Ok(())
    } else {
        Err(InvalidResidentRecordV1::DigestMismatch { family })
    }
}

/// Privacy-preserving correlation to the native session identity actually exposed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "visibility", rename_all = "snake_case", deny_unknown_fields)]
pub enum NativeSessionCorrelationV1 {
    /// The harness exposed a native identity that can be correlated.
    Observed {
        /// Exact privacy-preserving native session identity.
        native_session_identity: HarnessActivityIdentity,
    },
    /// The native harness does not expose a safe correlation identity.
    Unobserved {
        /// Stable bounded explanation of the visibility limitation.
        reason: ResidentReasonV1,
    },
}

resident_signed_record!(
    /// Bounded ambient admission lease for one native-harness session.
    SessionAdmissionLeaseV1, SessionAdmissionLeaseWireV1, SessionAdmissionLeasePreimageV1,
    session_admission_lease_digest: SessionAdmissionLeaseDigest,
    SESSION_ADMISSION_LEASE_V1_SCHEMA, "session admission lease",
    {
        /// Stable identity of this ambient session admission.
        session_admission_id: SessionAdmissionIdV1,
        /// Immutable generation pinned for the lifetime of the session.
        compiled_generation_digest: CompiledGenerationDigest,
        /// Exact active head from which this session admission was pinned.
        active_generation_head_digest: ActiveGenerationHeadDigest,
        /// Exact activation fence observed when the session was admitted.
        activation_fence_digest: ActivationFenceDigest,
        /// Immutable deployment binding used for admission.
        deployment_binding_ref: ResidentRecordRefV1,
        /// Digest of the referenced deployment-binding bytes.
        deployment_binding_sha256: ArtifactDigest,
        /// Exact deployment-runtime authority that signed this lease.
        deployment_runtime_authority_ref: ResidentRecordRefV1,
        /// Digest of the exact deployment-runtime-authority bytes.
        deployment_runtime_authority_sha256: ArtifactDigest,
        /// Principal whose authority bounds the session.
        principal_id: ResidentPrincipalIdV1,
        /// Controller boundary within the principal scope.
        controller_id: ResidentControllerIdV1,
        /// Optional relationship boundary; absence does not imply a relationship.
        relationship_scope: Option<RelationshipScopeV1>,
        /// Repository identity admitted for the native session.
        repository_id: LogicalRepositoryIdV1,
        /// Canonical project identity admitted for the session.
        project_id: LogicalProjectIdV1,
        /// Canonical execution workspace admitted for the session.
        execution_workspace_id: LogicalExecutionWorkspaceIdV1,
        /// Portable profile pinned by the session.
        profile_digest: ProfileDigest,
        /// Machine-resolved instance pinned by the session.
        instance_digest: InstanceDigest,
        /// Exact neutral execution path and terminal harness.
        execution_path: ExecutionPathSelectionV1,
        /// Digest of the exact harness-adapter source.
        harness_adapter_source_sha256: ArtifactDigest,
        /// Digest of the built harness adapter.
        harness_adapter_build_sha256: ArtifactDigest,
        /// Digest of the invoked harness executable.
        harness_executable_sha256: ArtifactDigest,
        /// Digest of the admitted harness configuration.
        harness_configuration_sha256: ArtifactDigest,
        /// Canonically ordered capability ceiling available to child leases.
        capability_ceiling: Vec<CapabilityIdV1>,
        /// Canonically ordered readable-record ceiling available to child leases.
        read_ceiling: Vec<ResidentRecordRefV1>,
        /// Canonically ordered effect-class ceiling available to child leases.
        effect_ceiling: Vec<EffectClass>,
        /// Positive aggregate budget ceiling for one child operation.
        budget_ceiling_units: LeaseBudgetUnitsV1,
        /// Positive execution-time ceiling for one child operation.
        timeout_ceiling_millis: LeaseTimeoutMillisV1,
        /// Canonically ordered evidence obligations every child lease must retain.
        evidence_obligations: Vec<ResidentRecordRefV1>,
        /// Immutable destination to which required evidence is committed.
        evidence_destination: ResidentRecordRefV1,
        /// Lease issue time in Unix seconds.
        issued_at: UnixTimeV1,
        /// Hard session expiry in Unix seconds.
        expires_at: UnixTimeV1,
        /// Positive revocation epoch checked at admission.
        revocation_epoch: PrincipalRevocationEpochV1,
        /// Optional immutable revocation record checked at admission.
        checked_revocation_ref: Option<ResidentRecordRefV1>,
        /// Digest paired with [`Self::checked_revocation_ref`].
        checked_revocation_sha256: Option<ArtifactDigest>,
        /// Native-session correlation actually available from the harness.
        native_session_correlation: NativeSessionCorrelationV1,
        /// Principal under whose runtime authority this lease was issued.
        issuer_principal_id: ResidentPrincipalIdV1,
        /// Exact runtime-authority key used for the lease signature.
        issuer_key_id: ResidentKeyIdV1,
        /// Positive runtime-authority key epoch checked for the lease.
        issuer_key_epoch: SigningKeyEpochV1
    },
    |value: &SessionAdmissionLeasePreimageV1| {
        strictly_sorted("session capability ceiling", &value.capability_ceiling)?;
        strictly_sorted("session read ceiling", &value.read_ceiling)?;
        strictly_sorted("session effect ceiling", &value.effect_ceiling)?;
        strictly_sorted("session evidence obligations", &value.evidence_obligations)?;
        if value.issued_at >= value.expires_at
            || value.checked_revocation_ref.is_some()
                != value.checked_revocation_sha256.is_some()
        {
            Err(InvalidResidentRecordV1::InvalidCoordinate { coordinate: "session lifetime/revocation" })
        } else { Ok(()) }
    }
);

/// Owner-resolved generation state for one ambient session admission.
pub struct SessionAdmissionLeaseContextV1<'a> {
    /// Exact active head observed by admission.
    pub active_head: &'a ActiveGenerationHeadV1,
    /// Exact immutable generation resolved from the active head.
    pub compiled_generation: &'a CompiledGenerationV1,
    /// Exact portable profile whose digest is bound by the compiled generation.
    pub effective_profile: &'a EffectiveProjectProfileV1,
    /// Exact deployment-adoption fence resolved from the active head.
    pub activation_fence: &'a ActivationFenceV1,
    /// Exact schema and deployment-runtime signing authority.
    pub signed: SignedResidentAdmissionContextV1<'a>,
}

impl SessionAdmissionLeaseV1 {
    /// Admit a session only when head, generation, profile, fence, route, and source agree.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidResidentRecordV1`] when any resolved generation-state
    /// coordinate differs, the session route/repository is outside the compiled
    /// closure, or the claimed canonical identity is false.
    pub fn admit_contextual_wire<F>(
        wire: SessionAdmissionLeaseWireV1,
        context: &SessionAdmissionLeaseContextV1<'_>,
        verify_signature: F,
    ) -> Result<Self, InvalidResidentRecordV1>
    where
        F: FnOnce(
            &[u8],
            &SessionAdmissionLeasePreimageV1,
            &SessionAdmissionLeaseDigest,
            &CanonicalSignatureV1,
        ) -> Result<(), InvalidResidentRecordV1>,
    {
        Self::admit_wire(wire, |preimage, digest, signature| {
            validate_signed_coordinates(
                "session admission lease",
                &preimage.schema_sha256,
                context.signed.expected_schema_sha256,
                &preimage.issuer_principal_id,
                &preimage.issuer_key_id,
                preimage.issuer_key_epoch,
                context.signed.signing_authority,
            )?;
            validate_integrated_runtime_authority_coordinates(
                "session admission lease",
                &preimage.deployment_runtime_authority_ref,
                &preimage.deployment_runtime_authority_sha256,
                context.signed.signing_authority,
            )?;
            validate_session_generation_context(preimage, context)?;
            verify_signature(
                SESSION_ADMISSION_LEASE_V1_SIGNATURE_DOMAIN,
                preimage,
                digest,
                signature,
            )
        })
    }
}

impl SessionAdmissionLeaseWireV1 {
    /// Borrow the claimed schema identity for exact retained-body lookup.
    #[must_use]
    pub const fn schema_id(&self) -> &SchemaId {
        &self.schema_id
    }

    /// Borrow the claimed schema digest for exact retained-body lookup.
    #[must_use]
    pub const fn schema_sha256(&self) -> &SchemaDigest {
        &self.schema_sha256
    }

    /// Borrow the claimed canonical lease digest.
    #[must_use]
    pub const fn session_admission_lease_digest(&self) -> &SessionAdmissionLeaseDigest {
        &self.session_admission_lease_digest
    }

    /// Borrow the claimed compiled-generation identity.
    #[must_use]
    pub const fn compiled_generation_digest(&self) -> &CompiledGenerationDigest {
        &self.compiled_generation_digest
    }

    /// Borrow the claimed active-head identity.
    #[must_use]
    pub const fn active_generation_head_digest(&self) -> &ActiveGenerationHeadDigest {
        &self.active_generation_head_digest
    }

    /// Borrow the claimed activation-fence identity.
    #[must_use]
    pub const fn activation_fence_digest(&self) -> &ActivationFenceDigest {
        &self.activation_fence_digest
    }

    /// Borrow the claimed effective-profile identity.
    #[must_use]
    pub const fn profile_digest(&self) -> &ProfileDigest {
        &self.profile_digest
    }

    /// Borrow the claimed project identity.
    #[must_use]
    pub const fn project_id(&self) -> &LogicalProjectIdV1 {
        &self.project_id
    }

    /// Borrow the claimed execution-workspace identity.
    #[must_use]
    pub const fn execution_workspace_id(&self) -> &LogicalExecutionWorkspaceIdV1 {
        &self.execution_workspace_id
    }

    /// Borrow the claimed deployment-runtime-authority reference.
    #[must_use]
    pub const fn deployment_runtime_authority_ref(&self) -> &ResidentRecordRefV1 {
        &self.deployment_runtime_authority_ref
    }

    /// Borrow the claimed deployment-runtime-authority digest.
    #[must_use]
    pub const fn deployment_runtime_authority_sha256(&self) -> &ArtifactDigest {
        &self.deployment_runtime_authority_sha256
    }

    /// Borrow the claimed signer principal.
    #[must_use]
    pub const fn issuer_principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.issuer_principal_id
    }

    /// Borrow the claimed signer key.
    #[must_use]
    pub const fn issuer_key_id(&self) -> &ResidentKeyIdV1 {
        &self.issuer_key_id
    }

    /// Borrow the claimed signer key epoch.
    #[must_use]
    pub const fn issuer_key_epoch(&self) -> &SigningKeyEpochV1 {
        &self.issuer_key_epoch
    }
}

fn validate_session_generation_context(
    value: &SessionAdmissionLeasePreimageV1,
    context: &SessionAdmissionLeaseContextV1<'_>,
) -> Result<(), InvalidResidentRecordV1> {
    context.active_head.verify()?;
    context.compiled_generation.verify()?;
    context
        .effective_profile
        .verify()
        .map_err(|_| contextual("session admission lease", "admitted effective profile"))?;
    context.activation_fence.verify()?;
    let compiled_digest = context.compiled_generation.compiled_generation_digest();
    let fence_digest = context.activation_fence.activation_fence_digest();
    if value.schema_sha256 != *context.signed.expected_schema_sha256
        || value.active_generation_head_digest
            != *context.active_head.active_generation_head_digest()
        || value.compiled_generation_digest != *compiled_digest
        || value.compiled_generation_digest != *context.active_head.compiled_generation_digest()
        || value.compiled_generation_digest
            != *context.activation_fence.compiled_generation_digest()
        || value.activation_fence_digest != *fence_digest
        || value.activation_fence_digest != *context.active_head.activation_fence_digest()
        || value.instance_digest != *context.compiled_generation.instance_digest()
    {
        return Err(contextual(
            "session admission lease",
            "active head generation and fence agreement",
        ));
    }
    if context.compiled_generation.profile_digest() != &context.effective_profile.profile_digest
        || value.profile_digest != context.effective_profile.profile_digest
        || value.project_id != context.effective_profile.project_id
        || value.execution_workspace_id != context.effective_profile.execution_workspace_id
    {
        return Err(contextual(
            "session admission lease",
            "compiled effective profile and project/execution-workspace coordinates",
        ));
    }
    if context
        .compiled_generation
        .execution_paths()
        .binary_search_by(|path| path.id.cmp(&value.execution_path.id))
        .ok()
        .is_none_or(|index| {
            context.compiled_generation.execution_paths()[index] != value.execution_path
        })
    {
        return Err(contextual(
            "session admission lease",
            "compiled execution path",
        ));
    }
    if context
        .compiled_generation
        .repository_source_frontiers()
        .binary_search_by(|frontier| frontier.repository_id.cmp(&value.repository_id))
        .is_err()
    {
        return Err(contextual(
            "session admission lease",
            "compiled repository source frontier",
        ));
    }
    Ok(())
}

/// Correlation for one observable or explicitly unobservable invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "visibility", rename_all = "snake_case", deny_unknown_fields)]
pub enum InvocationCorrelationV1 {
    /// The operation invocation has an observable native correlation.
    Observed {
        /// Pale-owned invocation correlation identity.
        invocation_correlation_id: InvocationCorrelationIdV1,
        /// Owner-native invocation identity when safely exposed.
        native_invocation_id: NativeInvocationIdV1,
    },
    /// The native owner cannot expose an invocation identity.
    Unobserved {
        /// Stable bounded explanation of the visibility limitation.
        reason: ResidentReasonV1,
    },
}

/// Requested child axes used to construct an [`OperationLeaseV1`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationLeaseRequestV1 {
    /// Stable child-lease identity.
    pub operation_lease_id: OperationLeaseIdV1,
    /// Exact Package-owned operation selected for this invocation.
    pub operation: PackageResourceRefV1,
    /// Capability subset requested from the parent ceiling.
    pub capabilities: Vec<CapabilityIdV1>,
    /// Effect-class subset requested from the parent ceiling.
    pub effects: Vec<EffectClass>,
    /// Read subset requested from the parent ceiling.
    pub read_set: Vec<ResidentRecordRefV1>,
    /// Evidence obligations retained from the parent requirement set.
    pub evidence_obligations: Vec<ResidentRecordRefV1>,
    /// Positive budget no greater than the parent ceiling.
    pub budget_units: LeaseBudgetUnitsV1,
    /// Positive timeout no greater than the parent ceiling.
    pub timeout_millis: LeaseTimeoutMillisV1,
    /// Hard expiry no later than the parent session expiry.
    pub expires_at: UnixTimeV1,
    /// Native invocation correlation or explicit visibility limitation.
    pub invocation_correlation: InvocationCorrelationV1,
}

/// Borrowed child axes used by the operation-narrowing law.
///
/// Sealing and re-admission share this view so verification never reconstructs
/// or clones an owned [`OperationLeaseRequestV1`]. The view carries no parent
/// identity and cannot establish liveness; it only proves the subset and
/// lifetime inequalities documented by [`OperationLeaseV1`].
struct OperationNarrowingAxesV1<'record> {
    capabilities: &'record [CapabilityIdV1],
    effects: &'record [EffectClass],
    read_set: &'record [ResidentRecordRefV1],
    evidence_obligations: &'record [ResidentRecordRefV1],
    budget_units: LeaseBudgetUnitsV1,
    timeout_millis: LeaseTimeoutMillisV1,
    expires_at: UnixTimeV1,
}

impl<'record> OperationNarrowingAxesV1<'record> {
    /// Borrow the governed ceilings from an admitted parent session.
    fn from_session(parent: &'record SessionAdmissionLeaseV1) -> Self {
        Self {
            capabilities: parent.capability_ceiling(),
            effects: parent.effect_ceiling(),
            read_set: parent.read_ceiling(),
            evidence_obligations: parent.evidence_obligations(),
            budget_units: *parent.budget_ceiling_units(),
            timeout_millis: *parent.timeout_ceiling_millis(),
            expires_at: *parent.expires_at(),
        }
    }

    /// Borrow the governed axes from an unsealed child request.
    fn from_request(request: &'record OperationLeaseRequestV1) -> Self {
        Self {
            capabilities: &request.capabilities,
            effects: &request.effects,
            read_set: &request.read_set,
            evidence_obligations: &request.evidence_obligations,
            budget_units: request.budget_units,
            timeout_millis: request.timeout_millis,
            expires_at: request.expires_at,
        }
    }

    /// Borrow the governed axes from an already sealed child lease.
    fn from_lease(lease: &'record OperationLeaseV1) -> Self {
        Self {
            capabilities: &lease.capabilities,
            effects: &lease.effects,
            read_set: &lease.read_set,
            evidence_obligations: &lease.evidence_obligations,
            budget_units: lease.budget_units,
            timeout_millis: lease.timeout_millis,
            expires_at: lease.expires_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
/// Canonical digest preimage for an [`OperationLeaseV1`].
struct OperationLeasePreimageV1 {
    /// Stable operation-lease schema identity.
    schema: SchemaId,
    /// Digest of the exact selected operation-lease schema bytes.
    schema_sha256: SchemaDigest,
    /// Initial public-contract major.
    schema_version: u32,
    /// Stable identity of the child lease.
    operation_lease_id: OperationLeaseIdV1,
    /// Exact parent session identity copied from its sealed record.
    session_admission_id: SessionAdmissionIdV1,
    /// Canonical digest of the exact parent session lease.
    session_admission_lease_digest: SessionAdmissionLeaseDigest,
    /// Exact Package-owned operation admitted for invocation.
    operation: PackageResourceRefV1,
    /// Canonically ordered capability subset of the parent ceiling.
    capabilities: Vec<CapabilityIdV1>,
    /// Canonically ordered effect-class subset of the parent ceiling.
    effects: Vec<EffectClass>,
    /// Canonically ordered read subset of the parent ceiling.
    read_set: Vec<ResidentRecordRefV1>,
    /// Canonically ordered obligations retained from the parent session.
    evidence_obligations: Vec<ResidentRecordRefV1>,
    /// Positive child budget no greater than the parent ceiling.
    budget_units: LeaseBudgetUnitsV1,
    /// Positive child timeout no greater than the parent ceiling.
    timeout_millis: LeaseTimeoutMillisV1,
    /// Hard child expiry no later than the parent expiry.
    expires_at: UnixTimeV1,
    /// Native invocation correlation actually available to the child operation.
    invocation_correlation: InvocationCorrelationV1,
}

/// Nested operation lease whose child axes are proven against an exact parent session.
///
/// Let `S` be the parent session and `L` this child lease. In the rendered law,
/// `C`, `E`, and `R` are the capability, effect, and read sets; `Ω` is the
/// evidence-obligation set; and `b`, `t`, and `x` are the budget, timeout, and
/// expiry coordinates. A child may add evidence obligations but cannot discard
/// obligations inherited from its parent.
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/operation-lease-narrowing.display.mathml"
))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationLeaseV1 {
    /// Stable operation-lease schema identity.
    #[schemars(with = "String", extend("const" = OPERATION_LEASE_V1_SCHEMA))]
    schema: SchemaId,
    /// Digest of the exact selected operation-lease schema bytes.
    schema_sha256: SchemaDigest,
    /// Initial public-contract major.
    #[schemars(extend("const" = OPERATION_LEASE_V1_SCHEMA_VERSION))]
    schema_version: u32,
    /// Stable identity of this child lease.
    operation_lease_id: OperationLeaseIdV1,
    /// Exact parent session identity copied at sealing.
    session_admission_id: SessionAdmissionIdV1,
    /// Canonical digest of the exact parent session lease.
    session_admission_lease_digest: SessionAdmissionLeaseDigest,
    /// Exact Package-owned operation admitted for invocation.
    operation: PackageResourceRefV1,
    /// Canonically ordered capability subset of the parent ceiling.
    capabilities: Vec<CapabilityIdV1>,
    /// Canonically ordered effect-class subset of the parent ceiling.
    effects: Vec<EffectClass>,
    /// Canonically ordered read subset of the parent ceiling.
    read_set: Vec<ResidentRecordRefV1>,
    /// Canonically ordered obligations retained from the parent session.
    evidence_obligations: Vec<ResidentRecordRefV1>,
    /// Positive child budget no greater than the parent ceiling.
    budget_units: LeaseBudgetUnitsV1,
    /// Positive child timeout no greater than the parent ceiling.
    timeout_millis: LeaseTimeoutMillisV1,
    /// Hard child expiry no later than the parent expiry.
    expires_at: UnixTimeV1,
    /// Native invocation correlation actually available to this operation.
    invocation_correlation: InvocationCorrelationV1,
    /// Canonical identity of this exact operation-lease preimage.
    operation_lease_digest: OperationLeaseDigest,
}

/// Untrusted structural wire shape for [`OperationLeaseV1`].
///
/// Deserialization proves only the field shape. Use
/// [`OperationLeaseV1::admit_wire`] with the exact resolved parent session to
/// establish semantic admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperationLeaseWireV1 {
    /// Claimed initial-v1 schema identity.
    #[schemars(with = "String", extend("const" = OPERATION_LEASE_V1_SCHEMA))]
    schema: SchemaId,
    /// Claimed selected schema digest.
    schema_sha256: SchemaDigest,
    /// Claimed public-contract major.
    #[schemars(extend("const" = OPERATION_LEASE_V1_SCHEMA_VERSION))]
    schema_version: u32,
    /// Claimed child-lease identity.
    operation_lease_id: OperationLeaseIdV1,
    /// Claimed parent session identity.
    session_admission_id: SessionAdmissionIdV1,
    /// Claimed canonical parent-session digest.
    session_admission_lease_digest: SessionAdmissionLeaseDigest,
    /// Claimed Package-owned operation.
    operation: PackageResourceRefV1,
    /// Claimed capability subset.
    capabilities: Vec<CapabilityIdV1>,
    /// Claimed effect-class subset.
    effects: Vec<EffectClass>,
    /// Claimed read subset.
    read_set: Vec<ResidentRecordRefV1>,
    /// Claimed retained evidence obligations.
    evidence_obligations: Vec<ResidentRecordRefV1>,
    /// Claimed positive child budget.
    budget_units: LeaseBudgetUnitsV1,
    /// Claimed positive child timeout.
    timeout_millis: LeaseTimeoutMillisV1,
    /// Claimed hard child expiry.
    expires_at: UnixTimeV1,
    /// Claimed invocation correlation.
    invocation_correlation: InvocationCorrelationV1,
    /// Claimed canonical self-digest.
    operation_lease_digest: OperationLeaseDigest,
}

impl OperationLeaseWireV1 {
    /// Borrow the claimed parent-session digest for historical resolution.
    ///
    /// This accessor exposes an untrusted wire coordinate only. The referenced
    /// session and this operation lease still require contextual admission.
    #[must_use]
    pub const fn session_admission_lease_digest(&self) -> &SessionAdmissionLeaseDigest {
        &self.session_admission_lease_digest
    }
}

impl OperationLeaseV1 {
    /// Seal a child lease only after proving it narrows the exact parent session.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] when any child axis widens the
    /// parent, canonical ordering fails, or the lease preimage cannot be sealed.
    pub fn seal(
        schema_sha256: SchemaDigest,
        parent: &SessionAdmissionLeaseV1,
        request: OperationLeaseRequestV1,
    ) -> Result<Self, InvalidResidentRecordV1> {
        validate_operation_narrowing(
            OperationNarrowingAxesV1::from_session(parent),
            OperationNarrowingAxesV1::from_request(&request),
        )?;
        let preimage = OperationLeasePreimageV1 {
            schema: SchemaId::parse(OPERATION_LEASE_V1_SCHEMA).map_err(|_| {
                InvalidResidentRecordV1::SchemaCoordinate {
                    family: "operation lease",
                }
            })?,
            schema_sha256,
            schema_version: OPERATION_LEASE_V1_SCHEMA_VERSION,
            operation_lease_id: request.operation_lease_id,
            session_admission_id: parent.session_admission_id().clone(),
            session_admission_lease_digest: parent.session_admission_lease_digest().clone(),
            operation: request.operation,
            capabilities: request.capabilities,
            effects: request.effects,
            read_set: request.read_set,
            evidence_obligations: request.evidence_obligations,
            budget_units: request.budget_units,
            timeout_millis: request.timeout_millis,
            expires_at: request.expires_at,
            invocation_correlation: request.invocation_correlation,
        };
        Self::from_preimage(preimage)
    }

    /// Admit an untrusted wire lease against its exact resolved parent session.
    ///
    /// Admission binds the claimed parent identity and digest and reproves every
    /// capability, effect, read, evidence, budget, timeout, and expiry axis.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] for a wrong parent, invalid
    /// self-digest, noncanonical collection, or widening on any governed axis.
    pub fn admit_wire(
        wire: OperationLeaseWireV1,
        parent: &SessionAdmissionLeaseV1,
        expected_schema_sha256: &SchemaDigest,
    ) -> Result<Self, InvalidResidentRecordV1> {
        let value = Self {
            schema: wire.schema,
            schema_sha256: wire.schema_sha256,
            schema_version: wire.schema_version,
            operation_lease_id: wire.operation_lease_id,
            session_admission_id: wire.session_admission_id,
            session_admission_lease_digest: wire.session_admission_lease_digest,
            operation: wire.operation,
            capabilities: wire.capabilities,
            effects: wire.effects,
            read_set: wire.read_set,
            evidence_obligations: wire.evidence_obligations,
            budget_units: wire.budget_units,
            timeout_millis: wire.timeout_millis,
            expires_at: wire.expires_at,
            invocation_correlation: wire.invocation_correlation,
            operation_lease_digest: wire.operation_lease_digest,
        };
        if value.schema_sha256 != *expected_schema_sha256 {
            return Err(InvalidResidentRecordV1::SchemaCoordinate {
                family: "operation lease",
            });
        }
        value.verify_against_parent(parent)?;
        Ok(value)
    }

    /// Reprove this lease against the exact parent admission resolved by its digest.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] for a parent identity/digest mismatch,
    /// invalid self-digest, or any widening across the seven governed axes.
    pub fn verify_against_parent(
        &self,
        parent: &SessionAdmissionLeaseV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        self.verify_identity()?;
        if self.session_admission_id != *parent.session_admission_id()
            || self.session_admission_lease_digest != *parent.session_admission_lease_digest()
        {
            return Err(InvalidResidentRecordV1::LeaseWidening);
        }
        validate_operation_narrowing(
            OperationNarrowingAxesV1::from_session(parent),
            OperationNarrowingAxesV1::from_lease(self),
        )
    }

    /// Reprove that this child lease narrows an exact parent operation lease.
    ///
    /// Both leases must descend from the same admitted session. Across the
    /// operation boundary, capability, effect, read, budget, timeout, and
    /// expiry axes may only narrow, while inherited evidence obligations may
    /// only be retained or strengthened.
    ///
    /// # Errors
    /// Returns [`InvalidResidentRecordV1`] when either lease identity fails,
    /// their session parents differ, or the child widens a governed axis.
    pub fn verify_against_parent_operation(
        &self,
        parent: &Self,
    ) -> Result<(), InvalidResidentRecordV1> {
        self.verify_identity()?;
        parent.verify_identity()?;
        if self.session_admission_id != parent.session_admission_id
            || self.session_admission_lease_digest != parent.session_admission_lease_digest
        {
            return Err(InvalidResidentRecordV1::LeaseWidening);
        }
        validate_operation_narrowing(
            OperationNarrowingAxesV1::from_lease(parent),
            OperationNarrowingAxesV1::from_lease(self),
        )
    }

    /// Borrow the stable initial-v1 schema identity.
    #[must_use]
    pub const fn schema(&self) -> &SchemaId {
        &self.schema
    }

    /// Borrow the digest of the exact selected JSON Schema bytes.
    #[must_use]
    pub const fn schema_sha256(&self) -> &SchemaDigest {
        &self.schema_sha256
    }

    /// Return the initial public-contract major.
    #[must_use]
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    /// Borrow the stable child-lease identity.
    #[must_use]
    pub const fn operation_lease_id(&self) -> &OperationLeaseIdV1 {
        &self.operation_lease_id
    }

    /// Borrow the exact parent session identity copied at sealing.
    #[must_use]
    pub const fn session_admission_id(&self) -> &SessionAdmissionIdV1 {
        &self.session_admission_id
    }

    /// Borrow the canonical digest of the exact parent session lease.
    #[must_use]
    pub const fn session_admission_lease_digest(&self) -> &SessionAdmissionLeaseDigest {
        &self.session_admission_lease_digest
    }

    /// Borrow the exact Package-owned operation admitted for invocation.
    #[must_use]
    pub const fn operation(&self) -> &PackageResourceRefV1 {
        &self.operation
    }

    /// Borrow the canonically ordered capability subset.
    #[must_use]
    pub fn capabilities(&self) -> &[CapabilityIdV1] {
        &self.capabilities
    }

    /// Borrow the canonically ordered effect-class subset.
    #[must_use]
    pub fn effects(&self) -> &[EffectClass] {
        &self.effects
    }

    /// Borrow the canonically ordered read subset.
    #[must_use]
    pub fn read_set(&self) -> &[ResidentRecordRefV1] {
        &self.read_set
    }

    /// Borrow the canonically ordered evidence obligations retained from the parent.
    #[must_use]
    pub fn evidence_obligations(&self) -> &[ResidentRecordRefV1] {
        &self.evidence_obligations
    }

    /// Return the positive child budget.
    #[must_use]
    pub const fn budget_units(&self) -> &LeaseBudgetUnitsV1 {
        &self.budget_units
    }

    /// Return the positive child timeout in milliseconds.
    #[must_use]
    pub const fn timeout_millis(&self) -> &LeaseTimeoutMillisV1 {
        &self.timeout_millis
    }

    /// Return the hard child expiry in Unix seconds.
    #[must_use]
    pub const fn expires_at(&self) -> UnixTimeV1 {
        self.expires_at
    }

    /// Borrow the native invocation correlation or visibility limitation.
    #[must_use]
    pub const fn invocation_correlation(&self) -> &InvocationCorrelationV1 {
        &self.invocation_correlation
    }

    /// Borrow the canonical identity of the exact operation-lease preimage.
    #[must_use]
    pub const fn operation_lease_digest(&self) -> &OperationLeaseDigest {
        &self.operation_lease_digest
    }

    /// Validate and seal a fully parent-bound operation-lease preimage.
    fn from_preimage(preimage: OperationLeasePreimageV1) -> Result<Self, InvalidResidentRecordV1> {
        validate_operation_lease_structure(&preimage)?;
        let operation_lease_digest = OperationLeaseDigest::hash_canonical(&preimage)?;
        let OperationLeasePreimageV1 {
            schema,
            schema_sha256,
            schema_version,
            operation_lease_id,
            session_admission_id,
            session_admission_lease_digest,
            operation,
            capabilities,
            effects,
            read_set,
            evidence_obligations,
            budget_units,
            timeout_millis,
            expires_at,
            invocation_correlation,
        } = preimage;
        Ok(Self {
            schema,
            schema_sha256,
            schema_version,
            operation_lease_id,
            session_admission_id,
            session_admission_lease_digest,
            operation,
            capabilities,
            effects,
            read_set,
            evidence_obligations,
            budget_units,
            timeout_millis,
            expires_at,
            invocation_correlation,
            operation_lease_digest,
        })
    }

    /// Recompute and compare the canonical self-digest without resolving a parent.
    fn verify_identity(&self) -> Result<(), InvalidResidentRecordV1> {
        let preimage = OperationLeasePreimageV1 {
            schema: self.schema.clone(),
            schema_sha256: self.schema_sha256.clone(),
            schema_version: self.schema_version,
            operation_lease_id: self.operation_lease_id.clone(),
            session_admission_id: self.session_admission_id.clone(),
            session_admission_lease_digest: self.session_admission_lease_digest.clone(),
            operation: self.operation.clone(),
            capabilities: self.capabilities.clone(),
            effects: self.effects.clone(),
            read_set: self.read_set.clone(),
            evidence_obligations: self.evidence_obligations.clone(),
            budget_units: self.budget_units,
            timeout_millis: self.timeout_millis,
            expires_at: self.expires_at,
            invocation_correlation: self.invocation_correlation.clone(),
        };
        validate_operation_lease_structure(&preimage)?;
        if OperationLeaseDigest::hash_canonical(&preimage)? == self.operation_lease_digest {
            Ok(())
        } else {
            Err(InvalidResidentRecordV1::DigestMismatch {
                family: "operation lease",
            })
        }
    }
}

/// Validate schema, canonical set ordering, and positive scalar axes.
fn validate_operation_lease_structure(
    value: &OperationLeasePreimageV1,
) -> Result<(), InvalidResidentRecordV1> {
    if value.schema.as_str() != OPERATION_LEASE_V1_SCHEMA
        || value.schema_version != OPERATION_LEASE_V1_SCHEMA_VERSION
    {
        return Err(InvalidResidentRecordV1::SchemaCoordinate {
            family: "operation lease",
        });
    }
    strictly_sorted("operation lease capabilities", &value.capabilities)?;
    strictly_sorted("operation lease effects", &value.effects)?;
    strictly_sorted("operation lease read set", &value.read_set)?;
    strictly_sorted(
        "operation evidence obligations",
        &value.evidence_obligations,
    )?;
    Ok(())
}

/// Prove every child axis is no broader or longer-lived than its parent.
///
/// A child retains every evidence item retained by its parent.
/// A child may add bounded obligations but cannot drop an inherited one.
fn validate_operation_narrowing(
    parent: OperationNarrowingAxesV1<'_>,
    child: OperationNarrowingAxesV1<'_>,
) -> Result<(), InvalidResidentRecordV1> {
    let subset = |child: &[CapabilityIdV1], parent: &[CapabilityIdV1]| {
        child.iter().all(|item| parent.binary_search(item).is_ok())
    };
    if child.expires_at > parent.expires_at
        || child.budget_units > parent.budget_units
        || child.timeout_millis > parent.timeout_millis
        || !subset(child.capabilities, parent.capabilities)
        || !child
            .effects
            .iter()
            .all(|item| parent.effects.binary_search(item).is_ok())
        || !child
            .read_set
            .iter()
            .all(|item| parent.read_set.binary_search(item).is_ok())
        || !parent
            .evidence_obligations
            .iter()
            .all(|item| child.evidence_obligations.binary_search(item).is_ok())
    {
        Err(InvalidResidentRecordV1::LeaseWidening)
    } else {
        Ok(())
    }
}
