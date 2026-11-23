//! Admitted generation lifecycle and cold-recovery inputs.

use std::collections::BTreeSet;

use pale_core::records::*;
use serde::{de::DeserializeOwned, Serialize};

use crate::resident::{
    GenerationLifecycleRecoveryResolver, ResidentGenerationInputSource,
    ResidentGenerationInputSourceError, ResidentGenerationReconstructionInput,
};

fn unsigned_wire<P, D, W>(preimage: &P, digest_field: &str, digest: &D) -> W
where
    P: Serialize,
    D: Serialize,
    W: DeserializeOwned,
{
    let mut value = serde_json::to_value(preimage).expect("preimage serializes");
    value
        .as_object_mut()
        .expect("preimage is an object")
        .insert(
            digest_field.into(),
            serde_json::to_value(digest).expect("digest serializes"),
        );
    serde_json::from_value(value).expect("wire decodes")
}

fn schema(id: &str) -> SchemaDigest {
    pale_contracts::record_schema_identity(id)
        .expect("registered lifecycle schema")
        .schema_sha256
}

/// Resolver for an empty first cold-open. No historical value is admissible.
pub(in super::super) struct EmptyHistorical;

impl GenerationLifecycleRecoveryResolver for EmptyHistorical {
    fn session_signing_authority(
        &self,
        _: &ResidentRecordRefV1,
        _: &ArtifactDigest,
        _: &ResidentPrincipalIdV1,
        _: &ResidentKeyIdV1,
        _: SigningKeyEpochV1,
    ) -> Option<SigningAuthorityBindingV1> {
        None
    }

    fn verify_session_admission_lease_signature(
        &self,
        _: &SigningAuthorityBindingV1,
        _: &[u8],
        _: &SessionAdmissionLeasePreimageV1,
        _: &SessionAdmissionLeaseDigest,
        _: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "empty conformance resident",
            invariant: "no retained session authority exists",
        })
    }

    fn compiled_generation(&self, _: &CompiledGenerationDigest) -> Option<&CompiledGenerationV1> {
        None
    }

    fn published_head(
        &self,
        _: &PublishedGenerationHeadDigest,
    ) -> Option<&PublishedGenerationHeadV1> {
        None
    }

    fn advertised_head(
        &self,
        _: &AdvertisedGenerationHeadDigest,
    ) -> Option<&AdvertisedGenerationHeadV1> {
        None
    }

    fn active_head(&self, _: &ActiveGenerationHeadDigest) -> Option<&ActiveGenerationHeadV1> {
        None
    }

    fn activation_fence(&self, _: &ActivationFenceDigest) -> Option<&ActivationFenceV1> {
        None
    }

    fn effective_profile(&self, _: &ProfileDigest) -> Option<&EffectiveProjectProfileV1> {
        None
    }
}

/// One admitted first lifecycle for a captured generation.
#[derive(Clone)]
pub(in super::super) struct ConformanceLifecycle {
    pub(in super::super) published: PublishedGenerationHeadV1,
    pub(in super::super) advertised: AdvertisedGenerationHeadV1,
    pub(in super::super) fence: ActivationFenceV1,
    pub(in super::super) active: ActiveGenerationHeadV1,
}

impl ConformanceLifecycle {
    pub(in super::super) fn initial(generation: &CompiledGenerationV1) -> Self {
        let published_schema = schema(PUBLISHED_GENERATION_HEAD_V1_SCHEMA_ID);
        let published_preimage = PublishedGenerationHeadPreimageV1 {
            schema: SchemaId::parse(PUBLISHED_GENERATION_HEAD_V1_SCHEMA).expect("published schema"),
            schema_sha256: published_schema.clone(),
            schema_version: PUBLISHED_GENERATION_HEAD_V1_SCHEMA_VERSION,
            compiled_generation_digest: generation.compiled_generation_digest().clone(),
            journal_sequence: JournalSequenceV1::new(1).expect("published sequence"),
            predecessor_published_generation_head_digest: None,
        };
        let published_digest = PublishedGenerationHeadDigest::hash_canonical(&published_preimage)
            .expect("published digest");
        let published = PublishedGenerationHeadV1::admit_contextual_wire(
            unsigned_wire(
                &published_preimage,
                "published_generation_head_digest",
                &published_digest,
            ),
            &PublishedGenerationHeadAdmissionContextV1 {
                expected_schema_sha256: &published_schema,
                compiled_generation: generation,
                predecessor: None,
            },
        )
        .expect("publication admits");

        let advertised_schema = schema(ADVERTISED_GENERATION_HEAD_V1_SCHEMA_ID);
        let advertised_preimage = AdvertisedGenerationHeadPreimageV1 {
            schema: SchemaId::parse(ADVERTISED_GENERATION_HEAD_V1_SCHEMA)
                .expect("advertised schema"),
            schema_sha256: advertised_schema.clone(),
            schema_version: ADVERTISED_GENERATION_HEAD_V1_SCHEMA_VERSION,
            compiled_generation_digest: generation.compiled_generation_digest().clone(),
            published_generation_head_digest: published.published_generation_head_digest().clone(),
            journal_sequence: JournalSequenceV1::new(2).expect("advertised sequence"),
            predecessor_advertised_generation_head_digest: None,
        };
        let advertised_digest =
            AdvertisedGenerationHeadDigest::hash_canonical(&advertised_preimage)
                .expect("advertised digest");
        let advertised = AdvertisedGenerationHeadV1::admit_contextual_wire(
            unsigned_wire(
                &advertised_preimage,
                "advertised_generation_head_digest",
                &advertised_digest,
            ),
            &AdvertisedGenerationHeadAdmissionContextV1 {
                expected_schema_sha256: &advertised_schema,
                published_head: &published,
                predecessor: None,
            },
        )
        .expect("advertisement admits");

        let fence_schema = schema(ACTIVATION_FENCE_V1_SCHEMA_ID);
        let fence_preimage = ActivationFencePreimageV1 {
            schema: SchemaId::parse(ACTIVATION_FENCE_V1_SCHEMA).expect("fence schema"),
            schema_sha256: fence_schema.clone(),
            schema_version: ACTIVATION_FENCE_V1_SCHEMA_VERSION,
            activation_fence_id: ActivationFenceIdV1::parse("interface-conformance-fence")
                .expect("activation fence"),
            activation_fence_generation: ActivationFenceGenerationV1::new(1)
                .expect("fence generation"),
            compiled_generation_digest: generation.compiled_generation_digest().clone(),
            predecessor_activation_fence_digest: None,
        };
        let fence_digest =
            ActivationFenceDigest::hash_canonical(&fence_preimage).expect("fence digest");
        let fence = ActivationFenceV1::admit_contextual_wire(
            unsigned_wire(&fence_preimage, "activation_fence_digest", &fence_digest),
            &ActivationFenceAdmissionContextV1 {
                expected_schema_sha256: &fence_schema,
                compiled_generation: generation,
                predecessor: None,
            },
        )
        .expect("activation fence admits");

        let active_schema = schema(ACTIVE_GENERATION_HEAD_V1_SCHEMA_ID);
        let active_preimage = ActiveGenerationHeadPreimageV1 {
            schema: SchemaId::parse(ACTIVE_GENERATION_HEAD_V1_SCHEMA).expect("active schema"),
            schema_sha256: active_schema.clone(),
            schema_version: ACTIVE_GENERATION_HEAD_V1_SCHEMA_VERSION,
            compiled_generation_digest: generation.compiled_generation_digest().clone(),
            advertised_generation_head_digest: advertised
                .advertised_generation_head_digest()
                .clone(),
            activation_fence_digest: fence.activation_fence_digest().clone(),
            journal_sequence: JournalSequenceV1::new(3).expect("active sequence"),
            predecessor_active_generation_head_digest: None,
        };
        let active_digest =
            ActiveGenerationHeadDigest::hash_canonical(&active_preimage).expect("active digest");
        let active = ActiveGenerationHeadV1::admit_contextual_wire(
            unsigned_wire(
                &active_preimage,
                "active_generation_head_digest",
                &active_digest,
            ),
            &ActiveGenerationHeadAdmissionContextV1 {
                expected_schema_sha256: &active_schema,
                advertised_head: &advertised,
                activation_fence: &fence,
                predecessor: None,
            },
        )
        .expect("active head admits");
        Self {
            published,
            advertised,
            fence,
            active,
        }
    }
}

/// Exact historical values needed to re-admit one retained session.
pub(in super::super) struct ConformanceHistorical {
    generation: CompiledGenerationV1,
    profile: EffectiveProjectProfileV1,
    lifecycle: ConformanceLifecycle,
    signing: SigningAuthorityBindingV1,
}

impl ConformanceHistorical {
    pub(in super::super) fn new(
        generation: CompiledGenerationV1,
        profile: EffectiveProjectProfileV1,
        lifecycle: ConformanceLifecycle,
        signing: SigningAuthorityBindingV1,
    ) -> Self {
        Self {
            generation,
            profile,
            lifecycle,
            signing,
        }
    }

    pub(in super::super) const fn lifecycle(&self) -> &ConformanceLifecycle {
        &self.lifecycle
    }
}

impl GenerationLifecycleRecoveryResolver for ConformanceHistorical {
    fn session_signing_authority(
        &self,
        authority_ref: &ResidentRecordRefV1,
        authority_sha256: &ArtifactDigest,
        issuer_principal_id: &ResidentPrincipalIdV1,
        issuer_key_id: &ResidentKeyIdV1,
        issuer_key_epoch: SigningKeyEpochV1,
    ) -> Option<SigningAuthorityBindingV1> {
        (self.signing.principal_id() == issuer_principal_id
            && self.signing.signer_key_id() == issuer_key_id.as_str()
            && self.signing.signer_key_epoch() == &issuer_key_epoch
            && self
                .signing
                .integrated_runtime_authority()
                .is_some_and(|authority| {
                    &authority.deployment_runtime_authority_ref == authority_ref
                        && &authority.deployment_runtime_authority_sha256 == authority_sha256
                }))
        .then(|| self.signing.clone())
    }

    fn verify_session_admission_lease_signature(
        &self,
        _: &SigningAuthorityBindingV1,
        _: &[u8],
        _: &SessionAdmissionLeasePreimageV1,
        _: &SessionAdmissionLeaseDigest,
        signature: &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1> {
        (signature == &super::principal::signature())
            .then_some(())
            .ok_or(InvalidResidentRecordV1::ContextualAdmission {
                record: "interface conformance retained session",
                invariant: "deterministic session signature",
            })
    }

    fn compiled_generation(
        &self,
        digest: &CompiledGenerationDigest,
    ) -> Option<&CompiledGenerationV1> {
        (self.generation.compiled_generation_digest() == digest).then_some(&self.generation)
    }

    fn published_head(
        &self,
        digest: &PublishedGenerationHeadDigest,
    ) -> Option<&PublishedGenerationHeadV1> {
        (self.lifecycle.published.published_generation_head_digest() == digest)
            .then_some(&self.lifecycle.published)
    }

    fn advertised_head(
        &self,
        digest: &AdvertisedGenerationHeadDigest,
    ) -> Option<&AdvertisedGenerationHeadV1> {
        (self
            .lifecycle
            .advertised
            .advertised_generation_head_digest()
            == digest)
            .then_some(&self.lifecycle.advertised)
    }

    fn active_head(&self, digest: &ActiveGenerationHeadDigest) -> Option<&ActiveGenerationHeadV1> {
        (self.lifecycle.active.active_generation_head_digest() == digest)
            .then_some(&self.lifecycle.active)
    }

    fn activation_fence(&self, digest: &ActivationFenceDigest) -> Option<&ActivationFenceV1> {
        (self.lifecycle.fence.activation_fence_digest() == digest).then_some(&self.lifecycle.fence)
    }

    fn effective_profile(&self, digest: &ProfileDigest) -> Option<&EffectiveProjectProfileV1> {
        (&self.profile.profile_digest == digest).then_some(&self.profile)
    }
}

/// One-use owner for exact generation reconstruction inputs.
pub(in super::super) struct ConformanceGenerationSource {
    inputs: Vec<ResidentGenerationReconstructionInput>,
    requested: Option<BTreeSet<CompiledGenerationDigest>>,
}

impl ConformanceGenerationSource {
    pub(in super::super) fn empty() -> Self {
        Self {
            inputs: Vec::new(),
            requested: None,
        }
    }

    pub(in super::super) fn new(input: ResidentGenerationReconstructionInput) -> Self {
        Self {
            inputs: vec![input],
            requested: None,
        }
    }
}

impl ResidentGenerationInputSource for ConformanceGenerationSource {
    fn load_generation_inputs(
        &mut self,
        required_generations: &BTreeSet<CompiledGenerationDigest>,
    ) -> Result<Vec<ResidentGenerationReconstructionInput>, ResidentGenerationInputSourceError>
    {
        self.requested = Some(required_generations.clone());
        Ok(std::mem::take(&mut self.inputs))
    }
}
