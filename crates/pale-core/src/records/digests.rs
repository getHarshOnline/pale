//! Digest newtypes that prevent cross-domain transposition.
//!
//! Every wrapper uses the same lowercase SHA-256 wire representation, but Rust
//! will not substitute (for example) an [`ArtifactDigest`] where a
//! [`SchemaDigest`] is required. Use [`ArtifactDigest::hash_bytes`] only for
//! exact-byte identity, [`ProfileDigest::hash_canonical`] only for an RFC 8785
//! record identity, and [`ArtifactDigest::parse`] only to validate a claimed
//! representation. Parsing or deserialization never proves that bytes were
//! observed, a self-digest was recomputed, a record was admitted, or an effect
//! occurred; the owning boundary must perform that comparison.

use serde::{Deserialize, Deserializer, Serialize};

/// Structural grammar shared by every lowercase SHA-256 wire newtype.
pub(crate) const SHA256_HEX_PATTERN: &str = r"^[0-9a-f]{64}$";

/// A wire value cannot represent the digest kind it claims.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{0} must be exactly 64 lowercase hexadecimal characters")]
pub struct InvalidDigest(&'static str);

/// A native harness supplied no bounded, usable identity to bind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error(
    "native-harness {kind} identity must be non-empty, at most 1024 bytes, \
     and contain no control characters"
)]
pub struct InvalidHostIdentity {
    kind: &'static str,
}

fn validate_host_identity(value: &str, kind: &'static str) -> Result<(), InvalidHostIdentity> {
    if value.is_empty() || value.len() > 1_024 || value.chars().any(char::is_control) {
        return Err(InvalidHostIdentity { kind });
    }
    Ok(())
}

/// Declare a transparent digest newtype. Wire encoding, ordering, and hashing
/// match the inner string, while the Rust type prevents digest-domain swaps.
/// Parsing and deserialization validate representation; domain-specific
/// constructors below perform the hash rather than accepting a claimed result.
macro_rules! digest_newtype {
    ($(#[$m:meta])* $name:ident) => {
        $(#[$m])*
        #[derive(
            Debug,
            Clone,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            Serialize,
            schemars::JsonSchema,
        )]
        #[serde(transparent)]
        #[schemars(
            rename = "Sha256Digest",
            description = "A SHA-256 digest encoded as exactly 64 lowercase hexadecimal characters."
        )]
        pub struct $name(#[schemars(regex(pattern = SHA256_HEX_PATTERN))] String);

        impl $name {
            /// Validate a digest received outside the trusted hashing primitive.
            ///
            /// # Errors
            ///
            /// Returns [`InvalidDigest`] unless the value is exactly 64
            /// lowercase hexadecimal characters.
            pub fn parse(value: impl Into<String>) -> Result<Self, InvalidDigest> {
                let value = value.into();
                if crate::is_sha256_hex(&value) {
                    Ok(Self(value))
                } else {
                    Err(InvalidDigest(stringify!($name)))
                }
            }

            /// Borrow the lowercase hexadecimal representation.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::parse(value).map_err(serde::de::Error::custom)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl TryFrom<String> for $name {
            type Error = InvalidDigest;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = InvalidDigest;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }
    };
}

/// Add the only infallible constructor earned by a byte-identity digest.
macro_rules! byte_digest {
    ($name:ident) => {
        impl $name {
            /// Compute this digest from the exact supplied bytes.
            #[must_use]
            pub fn hash_bytes(bytes: &[u8]) -> Self {
                Self(crate::sha256_hex(bytes))
            }
        }
    };
}

/// Add the only infallible constructor earned by a canonical-record digest.
macro_rules! canonical_digest {
    ($name:ident) => {
        impl $name {
            /// Compute this digest from the RFC 8785 encoding of `value`.
            ///
            /// # Errors
            ///
            /// Returns [`crate::canon::CanonError`] when `value` cannot satisfy
            /// the canonical record contract.
            pub fn hash_canonical<T: Serialize + ?Sized>(
                value: &T,
            ) -> Result<Self, crate::canon::CanonError> {
                Ok(Self(crate::canon::digest(value)?))
            }
        }
    };
}

digest_newtype!(
    /// SHA-256 of the canonical [`crate::records::EffectiveProjectProfileV1`]: the portable,
    /// path-free semantic selection.
    ProfileDigest
);
canonical_digest!(ProfileDigest);
digest_newtype!(
    /// SHA-256 of the canonical [`crate::records::ResolvedProjectInstanceV1`]: the profile
    /// bound to a machine (paths + executable identity); distinct from [`ProfileDigest`] so the
    /// orthogonal profile/instance split is enforced by the type system.
    InstanceDigest
);
canonical_digest!(InstanceDigest);
digest_newtype!(
    /// SHA-256 (hex) of the canonical generated agent-surface manifest.
    ManifestDigest
);
canonical_digest!(ManifestDigest);
digest_newtype!(
    /// SHA-256 of the exact raw UTF-8 bytes of one JSON Schema source file.
    SchemaDigest
);
byte_digest!(SchemaDigest);
digest_newtype!(
    /// SHA-256 of the RFC 8785 canonical full schema descriptor manifest with
    /// its own digest member omitted.
    SchemaManifestDigest
);
canonical_digest!(SchemaManifestDigest);
digest_newtype!(
    /// Domain-separated SHA-256 identity of one exact embedded schema set.
    SchemaSetDigest
);
digest_newtype!(
    /// SHA-256 identity of the canonical inputs selected for one compilation.
    /// It excludes rendered outputs so projections may carry it without a cycle.
    CompilationInputDigest
);
canonical_digest!(CompilationInputDigest);
digest_newtype!(
    /// Canonical identity of one exact distribution lock.
    ///
    /// Unlike [`ArtifactDigest`], this identity is earned only after the
    /// Package lock is canonical by semantic owner identity. The hashed body is
    /// the complete [`crate::records::DistributionLockFile`], including its
    /// compiler/schema metadata and exact Package releases.
    DistributionLockDigest
);

/// A distribution lock could not earn its typed canonical identity.
#[derive(Debug, thiserror::Error)]
pub enum DistributionLockDigestError {
    /// The Package-release lock was unbounded, duplicated, or non-canonical.
    #[error(transparent)]
    ReleaseClosure(#[from] super::release::ReleaseClosureErrorV1),
    /// The admitted lock could not satisfy canonical JSON.
    #[error(transparent)]
    Canon(#[from] crate::canon::CanonError),
}

impl DistributionLockDigest {
    /// Validate and hash one complete distribution lock.
    ///
    /// # Errors
    ///
    /// Returns [`DistributionLockDigestError`] when the Package lock is not in
    /// canonical owner order or the complete lock cannot be canonically encoded.
    pub fn from_lock(
        lock: &super::binding::DistributionLockFile,
    ) -> Result<Self, DistributionLockDigestError> {
        super::release::validate_release_closures(&lock.packages, &[])?;
        Ok(Self(crate::canon::digest(lock)?))
    }
}
digest_newtype!(
    /// SHA-256 identity of the canonical validated and linked Package semantic IR.
    ///
    /// This Package-corpus identity is deliberately distinct from workspace
    /// language compilation and from the final compilation identity.
    PackageSemanticIrDigest
);
canonical_digest!(PackageSemanticIrDigest);
digest_newtype!(
    /// Domain-separated canonical identity of one workspace source component.
    ///
    /// This identity is stable across carrier-only TOML whitespace and key-order
    /// changes; exact carrier bytes remain local parser provenance rather than a
    /// portable semantic identity.
    WorkspaceComponentDigest
);
canonical_digest!(WorkspaceComponentDigest);
digest_newtype!(
    /// Domain-separated identity of one effective resolved workspace import surface.
    WorkspaceImportSurfaceDigest
);
canonical_digest!(WorkspaceImportSurfaceDigest);
digest_newtype!(
    /// Exact opaque identity of publisher admission evidence.
    WorkspacePublisherProofDigest
);
byte_digest!(WorkspacePublisherProofDigest);
digest_newtype!(
    /// Exact opaque identity of protected-delivery admission evidence.
    WorkspaceProtectedDeliveryProofDigest
);
byte_digest!(WorkspaceProtectedDeliveryProofDigest);
digest_newtype!(
    /// Domain-separated identity of every proof consumed by universe admission.
    WorkspaceAdmissionEvidenceDigest
);
canonical_digest!(WorkspaceAdmissionEvidenceDigest);
digest_newtype!(
    /// Domain-separated canonical identity of one workspace module candidate.
    WorkspaceModuleDigest
);
canonical_digest!(WorkspaceModuleDigest);
digest_newtype!(
    /// Domain-separated canonical identity of an exact release-resolution lock.
    WorkspaceResolutionLockDigest
);
canonical_digest!(WorkspaceResolutionLockDigest);
digest_newtype!(
    /// Domain-separated canonical identity of one principal-relative universe lock.
    WorkspaceUniverseLockDigest
);
canonical_digest!(WorkspaceUniverseLockDigest);
digest_newtype!(
    /// Domain-separated canonical identity of one workspace derivation receipt.
    WorkspaceDerivationReceiptDigest
);
canonical_digest!(WorkspaceDerivationReceiptDigest);
digest_newtype!(
    /// Domain-separated identity of normalized workspace semantics.
    WorkspaceSemanticIrDigest
);
canonical_digest!(WorkspaceSemanticIrDigest);
digest_newtype!(
    /// Domain-separated identity of the canonically lowered responsibility set.
    ResponsibilityDerivationClosureDigest
);
canonical_digest!(ResponsibilityDerivationClosureDigest);
digest_newtype!(
    /// Domain-separated identity of the provider bindings admitted for a generation.
    ProviderBindingClosureDigest
);
canonical_digest!(ProviderBindingClosureDigest);
digest_newtype!(
    /// Domain-separated identity of every materialization selected for a generation.
    MaterializationClosureDigest
);
canonical_digest!(MaterializationClosureDigest);
digest_newtype!(
    /// Domain-separated identity of the machine-local overlay bound to a generation.
    MachineOverlayDigest
);
canonical_digest!(MachineOverlayDigest);
digest_newtype!(
    /// Domain-separated identity of the verified Pale executable and interface claim.
    ExecutableIdentityDigest
);
canonical_digest!(ExecutableIdentityDigest);
digest_newtype!(
    /// Domain-separated identity of the optional organizational translation closure.
    OrganizationalTranslationClosureDigest
);
canonical_digest!(OrganizationalTranslationClosureDigest);
digest_newtype!(
    /// Domain-separated identity of explicit neutral composition declarations.
    ExperienceCompositionSourceDigest
);
canonical_digest!(ExperienceCompositionSourceDigest);
digest_newtype!(
    /// Domain-separated identity of the principal-admitted workspace universe.
    PrincipalAdmittedUniverseDigest
);
canonical_digest!(PrincipalAdmittedUniverseDigest);
digest_newtype!(
    /// SHA-256 identity of one compiler-derived v1 experience composition plan.
    /// It does not prove installation, activation, provider occurrence, native
    /// effect, owner acceptance, or settlement.
    ExperienceCompositionPlanDigest
);
canonical_digest!(ExperienceCompositionPlanDigest);
digest_newtype!(
    /// SHA-256 identity of every consequential rendered surface and support field
    /// for one repository projection.
    RepositoryRenderDigest
);
canonical_digest!(RepositoryRenderDigest);
digest_newtype!(
    /// SHA-256 identity of one complete post-render, one-shot compilation result.
    /// This is not a resident generation or activation counter.
    OneShotCompilationDigest
);
canonical_digest!(OneShotCompilationDigest);

/// One exact member of a schema-set digest preimage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
pub struct SchemaSetMember {
    /// Registry identity of the embedded schema.
    pub schema_id: String,
    /// SHA-256 of the schema's exact raw UTF-8 source bytes.
    pub schema_sha256: SchemaDigest,
}

/// A schema set cannot be represented by one unambiguous canonical preimage.
#[derive(Debug, thiserror::Error)]
pub enum SchemaSetDigestError {
    /// Two members claimed the same registry identity.
    #[error("duplicate schema id in schema set: {0}")]
    DuplicateSchemaId(String),
    /// The sorted member set could not satisfy the canonical JSON contract.
    #[error("schema set canonicalization failed: {0}")]
    Canonicalize(#[from] crate::canon::CanonError),
}

/// Compute the exact domain-separated digest of an embedded schema set.
///
/// Input order is not authoritative. Members are sorted by `schema_id`; a
/// duplicate identity is rejected rather than silently overwritten.
///
/// # Errors
///
/// Returns [`SchemaSetDigestError::DuplicateSchemaId`] for a repeated schema
/// identity or [`SchemaSetDigestError::Canonicalize`] when the sorted member
/// array cannot satisfy Pale's RFC 8785 canonical-record contract.
pub fn schema_set_digest(
    members: impl IntoIterator<Item = SchemaSetMember>,
) -> Result<SchemaSetDigest, SchemaSetDigestError> {
    // Prevent this canonical member array from colliding with another digest
    // domain that happens to serialize the same JSON bytes.
    const DOMAIN: &[u8] = b"PALE_SCHEMA_SET_V1\0";

    let mut members = members.into_iter().collect::<Vec<_>>();
    members.sort_by(|left, right| left.schema_id.cmp(&right.schema_id));
    for pair in members.windows(2) {
        if pair[0].schema_id == pair[1].schema_id {
            return Err(SchemaSetDigestError::DuplicateSchemaId(
                pair[0].schema_id.clone(),
            ));
        }
    }
    let canonical = crate::canon::to_canonical_string(&members)?;
    let mut preimage = Vec::with_capacity(DOMAIN.len() + canonical.len());
    preimage.extend_from_slice(DOMAIN);
    preimage.extend_from_slice(canonical.as_bytes());
    Ok(SchemaSetDigest(crate::sha256_hex(&preimage)))
}
digest_newtype!(
    /// SHA-256 (hex) of the canonical target-local materialization state.
    MaterializationStateDigest
);
canonical_digest!(MaterializationStateDigest);
digest_newtype!(
    /// SHA-256 (hex) of the machine-bound materialization-lineage anchor.
    MaterializationLineageDigest
);
canonical_digest!(MaterializationLineageDigest);
digest_newtype!(
    /// SHA-256 (hex) of the canonical materialization plan (its self-digest).
    PlanDigest
);
canonical_digest!(PlanDigest);
digest_newtype!(
    /// SHA-256 (hex) of the canonical materialization receipt.
    ReceiptDigest
);
canonical_digest!(ReceiptDigest);
digest_newtype!(
    /// SHA-256 of exact artifact bytes: rendered surfaces, prompt payloads, and baselines.
    ArtifactDigest
);
byte_digest!(ArtifactDigest);
digest_newtype!(
    /// SHA-256 identity claimed or computed for executable bytes; admission is separate.
    ExecutableDigest
);
byte_digest!(ExecutableDigest);
impl ExecutableDigest {
    /// Compute executable identity from a byte stream without buffering the file.
    ///
    /// # Errors
    ///
    /// Returns the first read error reported by the supplied stream.
    pub fn hash_reader(reader: impl std::io::Read) -> std::io::Result<Self> {
        Ok(Self(crate::sha256_reader(reader)?))
    }
}
digest_newtype!(
    /// SHA-256 of `package.toml` plus every member of its admitted source inventory.
    ///
    /// The digest covers internal and exported Package resources alike. It does
    /// not attest normative Pack bytes or imply workspace selection, occurrence,
    /// activation, or native effect.
    PackageDigest
);
byte_digest!(PackageDigest);
digest_newtype!(
    /// SHA-256 of one exact attestor-owned normative Pack release.
    ///
    /// This digest is non-substitutable with [`PackageDigest`]: Pack bytes carry
    /// exact normative content, while Package bytes carry reusable procedure.
    NormativePackDigest
);
byte_digest!(NormativePackDigest);
digest_newtype!(
    /// SHA-256 identity claimed or computed for canonical target-root path bytes.
    TargetRootDigest
);
byte_digest!(TargetRootDigest);
digest_newtype!(
    /// SHA-256 (hex) of the canonical run result.
    RunDigest
);
canonical_digest!(RunDigest);
digest_newtype!(
    /// SHA-256 of a native-harness session identity bound to one activation lease.
    SessionIdentityDigest
);

impl SessionIdentityDigest {
    /// Hash a bounded host session identity so durable records do not retain its raw value.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidHostIdentity`] when the value is empty, exceeds 1,024
    /// bytes, or contains a control character.
    pub fn try_from_host_identity(value: &str) -> Result<Self, InvalidHostIdentity> {
        validate_host_identity(value, "session")?;
        Ok(Self(crate::sha256_hex(value.as_bytes())))
    }
}
digest_newtype!(
    /// SHA-256 of a native-harness turn identity bound to one operation lease.
    /// This is intentionally distinct from [`SessionIdentityDigest`].
    TurnIdentityDigest
);

impl TurnIdentityDigest {
    /// Hash a bounded host turn identity so ephemeral records do not retain its raw value.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidHostIdentity`] when the value is empty, exceeds 1,024
    /// bytes, or contains a control character.
    pub fn try_from_host_identity(value: &str) -> Result<Self, InvalidHostIdentity> {
        validate_host_identity(value, "turn")?;
        Ok(Self(crate::sha256_hex(value.as_bytes())))
    }
}

/// Privacy-preserving identity for state scoped to a native-harness activity.
/// A session identity is preferred when the host exposes one; a turn identity
/// is a distinct fallback rather than a value disguised as a session.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", content = "digest", rename_all = "snake_case")]
pub enum HarnessActivityIdentity {
    /// State is partitioned by a claimed native-harness session identity.
    Session(SessionIdentityDigest),
    /// State is partitioned by a turn identity when the host exposed no session.
    Turn(TurnIdentityDigest),
}

impl HarnessActivityIdentity {
    /// Bind the strongest identity exposed by the host without retaining its
    /// raw value. Session identity takes precedence over turn identity.
    ///
    /// A present but malformed session identity is rejected; it never silently
    /// falls back to a valid turn identity under another state scope.
    ///
    /// # Errors
    ///
    /// Rejects a selected host identity that is empty, oversized, or contains
    /// control characters.
    pub fn try_from_host(
        session_id: Option<&str>,
        turn_id: Option<&str>,
    ) -> Result<Option<Self>, InvalidHostIdentity> {
        if let Some(session_id) = session_id {
            return SessionIdentityDigest::try_from_host_identity(session_id)
                .map(Self::Session)
                .map(Some);
        }
        turn_id
            .map(TurnIdentityDigest::try_from_host_identity)
            .transpose()
            .map(|digest| digest.map(Self::Turn))
    }

    /// Borrow the digest used to partition confined state paths.
    #[must_use]
    pub fn as_digest_str(&self) -> &str {
        match self {
            Self::Session(digest) => digest.as_str(),
            Self::Turn(digest) => digest.as_str(),
        }
    }

    /// Stable discriminator used beside the digest in persisted identities.
    #[must_use]
    pub const fn kind_str(&self) -> &'static str {
        match self {
            Self::Session(_) => "session",
            Self::Turn(_) => "turn",
        }
    }
}
digest_newtype!(
    /// SHA-256 of one execution-path-specific operation contract.
    OperationContractDigest
);
canonical_digest!(OperationContractDigest);
digest_newtype!(
    /// SHA-256 of one native-harness operation catalog.
    OperationCatalogDigest
);
canonical_digest!(OperationCatalogDigest);
digest_newtype!(
    /// SHA-256 of one operation binding to a native-harness operation catalog.
    OperationBindingDigest
);
canonical_digest!(OperationBindingDigest);
digest_newtype!(
    /// SHA-256 of one exact gated tool request under an operation contract.
    ToolAuthorizationDigest
);
canonical_digest!(ToolAuthorizationDigest);
digest_newtype!(
    /// SHA-256 of one canonical decision-evidence entry, its complete ledger
    /// identity, and its predecessor link.
    DecisionEvidenceDigest
);
canonical_digest!(DecisionEvidenceDigest);
digest_newtype!(
    /// Digest of an action contract without its `contract_digest` field.
    ActionContractDigest
);
canonical_digest!(ActionContractDigest);
digest_newtype!(
    /// Digest of an action request without its `request_digest` field.
    ActionRequestDigest
);
canonical_digest!(ActionRequestDigest);
digest_newtype!(
    /// Digest of an action result without its `result_digest` field.
    ActionResultDigest
);
canonical_digest!(ActionResultDigest);
digest_newtype!(
    /// Digest of an action receipt without its `receipt_digest` field.
    ActionReceiptDigest
);
canonical_digest!(ActionReceiptDigest);
digest_newtype!(
    /// Canonical identity of one immutable compiled generation.
    CompiledGenerationDigest
);
canonical_digest!(CompiledGenerationDigest);
digest_newtype!(
    /// Canonical identity of one published-generation head.
    PublishedGenerationHeadDigest
);
canonical_digest!(PublishedGenerationHeadDigest);
digest_newtype!(
    /// Canonical identity of one advertised-generation head.
    AdvertisedGenerationHeadDigest
);
canonical_digest!(AdvertisedGenerationHeadDigest);
digest_newtype!(
    /// Canonical identity of one active-generation head.
    ActiveGenerationHeadDigest
);
canonical_digest!(ActiveGenerationHeadDigest);
digest_newtype!(
    /// Canonical identity of one deployment activation fence.
    ActivationFenceDigest
);
canonical_digest!(ActivationFenceDigest);
digest_newtype!(
    /// Canonical identity of one ambient session-admission lease.
    SessionAdmissionLeaseDigest
);
canonical_digest!(SessionAdmissionLeaseDigest);
digest_newtype!(
    /// Canonical identity of one nested operation lease.
    OperationLeaseDigest
);
canonical_digest!(OperationLeaseDigest);
digest_newtype!(
    /// Canonical identity of one immutable durable operation handle.
    OperationHandleDigest
);
canonical_digest!(OperationHandleDigest);
digest_newtype!(
    /// Canonical identity of one operation-observation revision.
    OperationObservationDigest
);
canonical_digest!(OperationObservationDigest);
digest_newtype!(
    /// Canonical identity of one admitted A2A route.
    A2aRouteAdmissionDigest
);
canonical_digest!(A2aRouteAdmissionDigest);
digest_newtype!(
    /// Canonical identity of one immutable Pale-to-A2A task binding.
    A2aTaskBindingDigest
);
canonical_digest!(A2aTaskBindingDigest);
digest_newtype!(
    /// Canonical identity of one remote A2A task-observation revision.
    A2aTaskObservationDigest
);
canonical_digest!(A2aTaskObservationDigest);
digest_newtype!(
    /// Canonical identity of one admitted event reference.
    EventReferenceDigest
);
canonical_digest!(EventReferenceDigest);
digest_newtype!(
    /// Canonical identity of one delivery-only event acknowledgement.
    EventAcknowledgementDigest
);
canonical_digest!(EventAcknowledgementDigest);
digest_newtype!(
    /// Canonical identity of one capability-session grant.
    CapabilitySessionGrantDigest
);
canonical_digest!(CapabilitySessionGrantDigest);
digest_newtype!(
    /// Canonical identity of one append-only capability-grant revocation.
    CapabilitySessionGrantRevocationDigest
);
canonical_digest!(CapabilitySessionGrantRevocationDigest);
digest_newtype!(
    /// Canonical identity of one capability-revocation anti-rollback frontier.
    CapabilityRevocationCheckpointDigest
);
canonical_digest!(CapabilityRevocationCheckpointDigest);
digest_newtype!(
    /// Canonical identity of one immutable local route generation.
    LocalRouteBindingDigest
);
canonical_digest!(LocalRouteBindingDigest);
digest_newtype!(
    /// Canonical identity of one neutral execution-owner fence generation.
    ExecutionOwnerFenceDigest
);
canonical_digest!(ExecutionOwnerFenceDigest);
digest_newtype!(
    /// Canonical identity of one deterministic interaction occurrence.
    InteractionOccurrenceDigest
);
canonical_digest!(InteractionOccurrenceDigest);
digest_newtype!(
    /// Canonical identity of one truth-classed genealogy edge.
    GenealogyEdgeDigest
);
canonical_digest!(GenealogyEdgeDigest);
digest_newtype!(
    /// Canonical identity of one independently dimensioned reconciliation frontier.
    ReconciliationFrontierDigest
);
canonical_digest!(ReconciliationFrontierDigest);
digest_newtype!(
    /// Canonical identity of one immutable reconciliation case.
    ReconciliationCaseDigest
);
canonical_digest!(ReconciliationCaseDigest);
digest_newtype!(
    /// Canonical identity of one reconciliation-custody generation.
    ReconciliationCustodyDigest
);
canonical_digest!(ReconciliationCustodyDigest);
digest_newtype!(
    /// Canonical identity of one reconciliation result revision.
    ReconciliationResultDigest
);
canonical_digest!(ReconciliationResultDigest);
digest_newtype!(
    /// Canonical identity of one effect-admission decision.
    EffectAdmissionDigest
);
canonical_digest!(EffectAdmissionDigest);
digest_newtype!(
    /// Canonical identity of one durable pre-dispatch intent.
    EffectDispatchIntentDigest
);
canonical_digest!(EffectDispatchIntentDigest);
digest_newtype!(
    /// Canonical identity of one native effect receipt.
    EffectReceiptDigest
);
canonical_digest!(EffectReceiptDigest);
digest_newtype!(
    /// Canonical identity of one signed capability result.
    CapabilityResultDigest
);
canonical_digest!(CapabilityResultDigest);
digest_newtype!(
    /// Canonical identity of one signed bounded adapter failure.
    AdapterFailureDigest
);
canonical_digest!(AdapterFailureDigest);
digest_newtype!(
    /// Canonical identity of one control-state checkpoint.
    ControlStateCheckpointDigest
);
canonical_digest!(ControlStateCheckpointDigest);
digest_newtype!(
    /// Canonical identity of one principal resident-writer fence record.
    PrincipalResidentWriterFenceDigest
);
canonical_digest!(PrincipalResidentWriterFenceDigest);
digest_newtype!(
    /// Canonical identity of one clean principal resident-writer terminal receipt.
    PrincipalResidentWriterTerminalReceiptDigest
);
canonical_digest!(PrincipalResidentWriterTerminalReceiptDigest);
digest_newtype!(
    /// Canonical identity of one authenticated principal resident-writer takeover receipt.
    PrincipalResidentWriterTakeoverReceiptDigest
);
canonical_digest!(PrincipalResidentWriterTakeoverReceiptDigest);
digest_newtype!(
    /// Canonical identity of one producer-incarnation allocation record.
    ProducerIncarnationDigest
);
canonical_digest!(ProducerIncarnationDigest);
digest_newtype!(
    /// Canonical identity of one append-only producer-incarnation retirement.
    ProducerIncarnationRetirementDigest
);
canonical_digest!(ProducerIncarnationRetirementDigest);
digest_newtype!(
    /// Canonical identity of one principal record.
    PrincipalRecordDigest
);
canonical_digest!(PrincipalRecordDigest);
digest_newtype!(
    /// Canonical identity of one principal recovery policy.
    PrincipalRecoveryPolicyDigest
);
canonical_digest!(PrincipalRecoveryPolicyDigest);
digest_newtype!(
    /// Canonical identity of one principal recovery proof, excluding its signature collection.
    PrincipalRecoveryProofDigest
);
canonical_digest!(PrincipalRecoveryProofDigest);
digest_newtype!(
    /// Canonical identity of one principal key transition, excluding both key signatures.
    PrincipalKeyTransitionDigest
);
canonical_digest!(PrincipalKeyTransitionDigest);
digest_newtype!(
    /// Canonical identity of one local-administration authority record.
    LocalAdminAuthorityDigest
);
canonical_digest!(LocalAdminAuthorityDigest);
digest_newtype!(
    /// Canonical identity of one authenticated administration command preimage.
    LocalAdminCommandDigest
);
canonical_digest!(LocalAdminCommandDigest);
digest_newtype!(
    /// Canonical identity of an administration receipt, excluding its signature.
    LocalAdminReceiptDigest
);
canonical_digest!(LocalAdminReceiptDigest);
digest_newtype!(
    /// Canonical identity of the complete ordered administration candidate array.
    LocalAdminCandidateSetDigest
);
canonical_digest!(LocalAdminCandidateSetDigest);
digest_newtype!(
    /// Canonical identity of one deployment control-store genesis receipt preimage.
    DeploymentControlStoreGenesisReceiptDigest
);
canonical_digest!(DeploymentControlStoreGenesisReceiptDigest);
digest_newtype!(
    /// Canonical identity of one producer-incarnation authority record.
    ProducerIncarnationAuthorityDigest
);
canonical_digest!(ProducerIncarnationAuthorityDigest);
digest_newtype!(
    /// Canonical identity of one checkpoint-signing authority record.
    CheckpointSigningAuthorityDigest
);
canonical_digest!(CheckpointSigningAuthorityDigest);
digest_newtype!(
    /// Canonical identity of one principal trust checkpoint.
    PrincipalTrustCheckpointDigest
);
canonical_digest!(PrincipalTrustCheckpointDigest);
digest_newtype!(
    /// Canonical identity of one principal bootstrap receipt.
    PrincipalBootstrapReceiptDigest
);
canonical_digest!(PrincipalBootstrapReceiptDigest);
digest_newtype!(
    /// Canonical identity of one admitted compatibility profile.
    CompatibilityProfileDigest
);
canonical_digest!(CompatibilityProfileDigest);
digest_newtype!(
    /// Canonical identity of one immutable deployment contract set.
    ContractSetDigest
);
canonical_digest!(ContractSetDigest);
digest_newtype!(
    /// Canonical identity of one selected distribution release.
    DistributionReleaseDigest
);
canonical_digest!(DistributionReleaseDigest);
digest_newtype!(
    /// Canonical identity of one deployment environment definition.
    DeploymentEnvironmentDigest
);
canonical_digest!(DeploymentEnvironmentDigest);
digest_newtype!(
    /// Canonical identity of one deployment processing-role definition.
    ProcessingRoleDigest
);
canonical_digest!(ProcessingRoleDigest);
digest_newtype!(
    /// Canonical identity of one deployment confidentiality-policy definition.
    ConfidentialityPolicyDigest
);
canonical_digest!(ConfidentialityPolicyDigest);
digest_newtype!(
    /// Canonical identity of one complete deployment-terms value.
    DeploymentTermsDigest
);
canonical_digest!(DeploymentTermsDigest);
digest_newtype!(
    /// Canonical identity of one deployment-binding proposal.
    DeploymentBindingProposalDigest
);
canonical_digest!(DeploymentBindingProposalDigest);
digest_newtype!(
    /// Canonical identity of one independently signed deployment acceptance.
    DeploymentAcceptanceDigest
);
canonical_digest!(DeploymentAcceptanceDigest);
digest_newtype!(
    /// Canonical identity of one deployment-acceptance revocation.
    DeploymentAcceptanceRevocationDigest
);
canonical_digest!(DeploymentAcceptanceRevocationDigest);
digest_newtype!(
    /// Canonical identity of one finalized deployment binding.
    DeploymentBindingDigest
);
canonical_digest!(DeploymentBindingDigest);
digest_newtype!(
    /// Canonical identity of one deployment-runtime-authority record.
    DeploymentRuntimeAuthorityDigest
);
canonical_digest!(DeploymentRuntimeAuthorityDigest);
digest_newtype!(
    /// Canonical identity of one checkpoint accumulator profile.
    CheckpointProfileDigest
);
canonical_digest!(CheckpointProfileDigest);
digest_newtype!(
    /// Canonical root of active producer-incarnation allocation heads.
    ProducerIncarnationHeadsRootDigest
);
canonical_digest!(ProducerIncarnationHeadsRootDigest);
digest_newtype!(
    /// Canonical root of active checkpoint-signing-authority heads.
    CheckpointSigningAuthorityHeadsRootDigest
);
canonical_digest!(CheckpointSigningAuthorityHeadsRootDigest);
digest_newtype!(
    /// Canonical root of active deployment-runtime-authority heads.
    DeploymentRuntimeAuthorityHeadsRootDigest
);
canonical_digest!(DeploymentRuntimeAuthorityHeadsRootDigest);
digest_newtype!(
    /// Canonical root of active owner-record-signing-authority heads.
    OwnerRecordSigningAuthorityHeadsRootDigest
);
canonical_digest!(OwnerRecordSigningAuthorityHeadsRootDigest);
digest_newtype!(
    /// Canonical root of owner-signer publication-frontier heads.
    OwnerSignerPublicationFrontierHeadsRootDigest
);
canonical_digest!(OwnerSignerPublicationFrontierHeadsRootDigest);
digest_newtype!(
    /// Canonical root of deployment-acceptance revocation heads.
    DeploymentAcceptanceRevocationHeadsRootDigest
);
canonical_digest!(DeploymentAcceptanceRevocationHeadsRootDigest);
digest_newtype!(
    /// Canonical root of federation-space home heads.
    FederationSpaceHomeHeadsRootDigest
);
canonical_digest!(FederationSpaceHomeHeadsRootDigest);
digest_newtype!(
    /// Canonical root of active node-delegation heads.
    NodeDelegationHeadsRootDigest
);
canonical_digest!(NodeDelegationHeadsRootDigest);
digest_newtype!(
    /// Canonical root of node-delegation revocation heads.
    NodeDelegationRevocationHeadsRootDigest
);
canonical_digest!(NodeDelegationRevocationHeadsRootDigest);
digest_newtype!(
    /// Canonical identity of one common request envelope.
    CommonRequestEnvelopeDigest
);
canonical_digest!(CommonRequestEnvelopeDigest);
digest_newtype!(
    /// Canonical identity of one admitted resource-continuity profile.
    ResourceContinuityProfileDigest
);
canonical_digest!(ResourceContinuityProfileDigest);
digest_newtype!(
    /// Canonical identity of one exact external-action body.
    ActionEnvelopeDigest
);
canonical_digest!(ActionEnvelopeDigest);
digest_newtype!(
    /// Canonical identity of one conflict-resource custody claim generation.
    EffectConflictClaimDigest
);
canonical_digest!(EffectConflictClaimDigest);
digest_newtype!(
    /// Canonical identity of one separately signed reconciliation request.
    ReconciliationRequestDigest
);
canonical_digest!(ReconciliationRequestDigest);
