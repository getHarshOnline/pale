//! Repository-bound admission of selected pure native actions.

use std::collections::BTreeMap;

use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1,
    DiagnosticSubjectV1Wire, LogicalRepositoryIdV1, RepositoryNativeActionCatalogV1,
};

use crate::error::{ApiError, FailureSpecification};
use crate::{NativeActionDescriptor, PaleApi};

/// Maximum repository action catalog admitted at the path-free API boundary.
const REPOSITORY_CATALOG_LIMIT: usize = 16 << 20;

/// Owned repository-catalog bytes and the exact context expected by a host leaf.
///
/// Construction records claims only. [`PaleApi::admit_repository_native_actions`]
/// validates the strict JSON document, its generated contract, the three context
/// coordinates, stable contract order, and parity with the compiled action registry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepositoryNativeActionCatalogRequest {
    bytes: Vec<u8>,
    profile_digest: String,
    instance_digest: String,
    repository_id: String,
}

impl RepositoryNativeActionCatalogRequest {
    /// Bind one untrusted catalog to the context selected by its host process.
    ///
    /// This constructor performs no parsing, hashing, admission, filesystem
    /// access, or runtime observation.
    #[must_use]
    pub fn new(
        bytes: Vec<u8>,
        profile_digest: String,
        instance_digest: String,
        repository_id: String,
    ) -> Self {
        Self {
            bytes,
            profile_digest,
            instance_digest,
            repository_id,
        }
    }
}

/// Selected action descriptors admitted for one exact repository instance.
///
/// This value proves catalog shape, context equality, stable unique ordering,
/// and equality with the current compiled action contracts. It does not prove
/// route authority, provider readiness, installation, activation, or an effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdmittedRepositoryNativeActions {
    profile_digest: String,
    instance_digest: String,
    repository_id: String,
    descriptors: Vec<NativeActionDescriptor>,
}

impl AdmittedRepositoryNativeActions {
    /// Profile digest carried by both the request expectation and admitted catalog.
    #[must_use]
    pub fn profile_digest(&self) -> &str {
        &self.profile_digest
    }

    /// Instance digest carried by both the request expectation and admitted catalog.
    #[must_use]
    pub fn instance_digest(&self) -> &str {
        &self.instance_digest
    }

    /// Repository identity carried by both the request expectation and admitted catalog.
    #[must_use]
    pub fn repository_id(&self) -> &str {
        &self.repository_id
    }

    /// Borrow the selected action descriptors in stable action-ID order.
    #[must_use]
    pub fn descriptors(&self) -> &[NativeActionDescriptor] {
        &self.descriptors
    }

    /// Consume the admission proof and return its owned selected descriptors.
    #[must_use]
    pub fn into_descriptors(self) -> Vec<NativeActionDescriptor> {
        self.descriptors
    }
}

/// A schema-valid repository action catalog contradicted its admitted context.
#[derive(Debug, thiserror::Error)]
enum RepositoryCatalogError {
    /// The generated schema and Rust wire decoder disagree.
    #[error("schema-valid repository action catalog could not be decoded: {0}")]
    Decode(#[source] serde_json::Error),
    /// A caller expectation was not a valid context coordinate.
    #[error("expected repository action {coordinate} is invalid: {source}")]
    InvalidExpectation {
        coordinate: &'static str,
        #[source]
        source: pale_core::records::InvalidDigest,
    },
    /// The caller's repository expectation is not a canonical logical identity.
    #[error("expected repository action repository identity is invalid")]
    InvalidRepositoryExpectation(#[source] pale_core::records::InvalidWorkspaceLanguageId),
    /// The catalog belongs to a different profile, instance, or repository.
    #[error("repository action catalog does not match the expected context")]
    ContextMismatch,
    /// Contract order is not strictly increasing and therefore not unique.
    #[error("repository action contracts are not in unique stable identity order")]
    UnstableContractOrder,
    /// A selected contract is absent from the compiled pure-action registry.
    #[error("repository action contract is absent from the compiled registry")]
    UnknownContract,
    /// A selected contract differs from the compiled registry contract.
    #[error("repository action contract differs from the compiled registry")]
    ContractMismatch,
}

/// Closed recovery policy for path-free repository-catalog admission.
#[derive(Clone, Copy)]
enum RepositoryAdmissionClass {
    /// Correct the supplied bytes or expected context before retrying.
    InvalidInput,
    /// Retry only after the selected admitted state changes.
    Blocked,
    /// The compiled application state and its generated contracts contradict one another.
    Internal,
}

impl RepositoryAdmissionClass {
    /// Project the class once into the portable disposition and retry posture.
    const fn diagnostic_policy(self) -> (DiagnosticDispositionV1, DiagnosticRetryV1) {
        match self {
            Self::InvalidInput => (
                DiagnosticDispositionV1::InvalidInput,
                DiagnosticRetryV1::AfterCorrection,
            ),
            Self::Blocked => (
                DiagnosticDispositionV1::Blocked,
                DiagnosticRetryV1::AfterStateChange,
            ),
            Self::Internal => (
                DiagnosticDispositionV1::InternalFailure,
                DiagnosticRetryV1::NotIndicated,
            ),
        }
    }
}

/// Typed primary coordinate for one repository-catalog failure.
#[derive(Clone, Copy)]
enum RepositoryCatalogSubject<'a> {
    /// Repository expectation or catalog-wide relationship.
    Repository(&'a str),
    /// Exact action contract whose selection or bytes failed admission.
    Action(&'a str),
}

impl RepositoryCatalogSubject<'_> {
    /// Project the typed coordinate into the portable diagnostic subject.
    fn into_wire(self) -> Option<DiagnosticSubjectV1Wire> {
        let (kind, id) = match self {
            Self::Repository("") => return None,
            Self::Repository(id) => ("repository", id),
            Self::Action(id) => ("action", id),
        };
        Some(DiagnosticSubjectV1Wire {
            kind: kind.to_string(),
            id: id.to_string(),
        })
    }
}

impl PaleApi {
    /// Admit one repository-bound selection of compiled pure native actions.
    ///
    /// The host leaf remains responsible for confined reading, exact artifact
    /// digest comparison, executable observation, transport naming, and serving.
    /// This method owns only path-free contract and context admission.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] when the input is oversized or not strict JSON, when
    /// it violates `repository-native-action-catalog.v1`, when the expected
    /// context is malformed or unequal, or when its ordered contracts differ
    /// from the compiled pure-action registry.
    pub fn admit_repository_native_actions(
        &self,
        request: RepositoryNativeActionCatalogRequest,
    ) -> Result<AdmittedRepositoryNativeActions, ApiError> {
        let RepositoryNativeActionCatalogRequest {
            bytes,
            profile_digest,
            instance_digest,
            repository_id,
        } = request;
        if bytes.len() > REPOSITORY_CATALOG_LIMIT {
            return Err(self.repository_catalog_failure(
                RepositoryCatalogSubject::Repository(&repository_id),
                crate::error::BoundaryError::InputTooLarge {
                    limit: REPOSITORY_CATALOG_LIMIT,
                },
                "input_too_large",
                DiagnosticStageV1::InputAdmission,
                RepositoryAdmissionClass::InvalidInput,
                "repository action catalog exceeds the bounded API input",
            ));
        }
        let value = pale_core::canon::parse_strict(&bytes).map_err(|error| {
            self.repository_catalog_failure(
                RepositoryCatalogSubject::Repository(&repository_id),
                error,
                "invalid_json",
                DiagnosticStageV1::InputAdmission,
                RepositoryAdmissionClass::InvalidInput,
                "repository action catalog is not strict JSON",
            )
        })?;
        pale_contracts::validate_value(&value, "repository-native-action-catalog.v1")
            .map_err(|error| self.contract_failure("repository-native-action-catalog.v1", error))?;
        let catalog: RepositoryNativeActionCatalogV1 =
            serde_json::from_value(value).map_err(|error| {
                self.repository_catalog_failure(
                    RepositoryCatalogSubject::Repository(&repository_id),
                    RepositoryCatalogError::Decode(error),
                    "repository_action_catalog_invariant",
                    DiagnosticStageV1::Internal,
                    RepositoryAdmissionClass::Internal,
                    "schema-valid repository action catalog could not be decoded",
                )
            })?;

        let expected_profile =
            pale_core::records::ProfileDigest::parse(profile_digest).map_err(|source| {
                self.repository_catalog_failure(
                    RepositoryCatalogSubject::Repository(&repository_id),
                    RepositoryCatalogError::InvalidExpectation {
                        coordinate: "profile digest",
                        source,
                    },
                    "invalid_repository_action_context",
                    DiagnosticStageV1::InputAdmission,
                    RepositoryAdmissionClass::InvalidInput,
                    "expected repository action context is malformed",
                )
            })?;
        let expected_instance = pale_core::records::InstanceDigest::parse(instance_digest)
            .map_err(|source| {
                self.repository_catalog_failure(
                    RepositoryCatalogSubject::Repository(&repository_id),
                    RepositoryCatalogError::InvalidExpectation {
                        coordinate: "instance digest",
                        source,
                    },
                    "invalid_repository_action_context",
                    DiagnosticStageV1::InputAdmission,
                    RepositoryAdmissionClass::InvalidInput,
                    "expected repository action context is malformed",
                )
            })?;
        let expected_repository =
            LogicalRepositoryIdV1::parse(repository_id.clone()).map_err(|source| {
                self.repository_catalog_failure(
                    RepositoryCatalogSubject::Repository(&repository_id),
                    RepositoryCatalogError::InvalidRepositoryExpectation(source),
                    "invalid_repository_action_context",
                    DiagnosticStageV1::InputAdmission,
                    RepositoryAdmissionClass::InvalidInput,
                    "expected repository action context is malformed",
                )
            })?;
        if catalog.profile_digest != expected_profile
            || catalog.instance_digest != expected_instance
            || catalog.repository_id != expected_repository
        {
            return Err(self.repository_catalog_failure(
                RepositoryCatalogSubject::Repository(&repository_id),
                RepositoryCatalogError::ContextMismatch,
                "repository_action_context_mismatch",
                DiagnosticStageV1::SemanticAdmission,
                RepositoryAdmissionClass::Blocked,
                "repository action catalog does not match the expected context",
            ));
        }

        let mut descriptors = self
            .native_action_descriptors()?
            .into_iter()
            .map(|descriptor| (descriptor.action_id().to_owned(), descriptor))
            .collect::<BTreeMap<_, _>>();
        let mut previous = None;
        let mut admitted = Vec::with_capacity(catalog.contracts.len());
        for contract in &catalog.contracts {
            if previous.is_some_and(|id| id >= contract.id.as_str()) {
                return Err(self.repository_catalog_failure(
                    RepositoryCatalogSubject::Repository(&repository_id),
                    RepositoryCatalogError::UnstableContractOrder,
                    "repository_action_catalog_unstable",
                    DiagnosticStageV1::SemanticAdmission,
                    RepositoryAdmissionClass::InvalidInput,
                    "repository action contracts are not in unique stable identity order",
                ));
            }
            previous = Some(contract.id.as_str());
            let descriptor = descriptors.remove(&contract.id).ok_or_else(|| {
                self.repository_catalog_failure(
                    RepositoryCatalogSubject::Action(&contract.id),
                    RepositoryCatalogError::UnknownContract,
                    "repository_action_unavailable",
                    DiagnosticStageV1::SemanticAdmission,
                    RepositoryAdmissionClass::Blocked,
                    "repository action contract is unavailable from this runtime",
                )
            })?;
            let contract_bytes =
                pale_core::canon::to_canonical_string(contract).map_err(|error| {
                    self.repository_catalog_failure(
                        RepositoryCatalogSubject::Action(&contract.id),
                        error,
                        "repository_action_catalog_invariant",
                        DiagnosticStageV1::Internal,
                        RepositoryAdmissionClass::Internal,
                        "repository action contract could not be canonicalized",
                    )
                })?;
            if descriptor.canonical_contract_bytes() != contract_bytes.as_bytes() {
                return Err(self.repository_catalog_failure(
                    RepositoryCatalogSubject::Action(&contract.id),
                    RepositoryCatalogError::ContractMismatch,
                    "repository_action_contract_mismatch",
                    DiagnosticStageV1::SemanticAdmission,
                    RepositoryAdmissionClass::Blocked,
                    "repository action contract does not match this runtime",
                ));
            }
            admitted.push(descriptor);
        }

        Ok(AdmittedRepositoryNativeActions {
            profile_digest: catalog.profile_digest.to_string(),
            instance_digest: catalog.instance_digest.to_string(),
            repository_id: catalog.repository_id.to_string(),
            descriptors: admitted,
        })
    }

    /// Project one repository-catalog failure through the shared diagnostic owner.
    fn repository_catalog_failure<E>(
        &self,
        subject: RepositoryCatalogSubject<'_>,
        error: E,
        code: &str,
        stage: DiagnosticStageV1,
        class: RepositoryAdmissionClass,
        summary: &str,
    ) -> ApiError
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        let (disposition, retry) = class.diagnostic_policy();
        ApiError::new(
            &self.diagnostic_identity,
            error,
            FailureSpecification {
                code,
                stage,
                disposition,
                summary,
                subject: subject.into_wire(),
                schema_coordinate: None,
                location: None,
                retry,
                effect: DiagnosticEffectCertaintyV1::NotApplicable,
                request: None,
                notes: Vec::new(),
                help: Vec::new(),
                facts: Vec::new(),
            },
        )
    }
}
