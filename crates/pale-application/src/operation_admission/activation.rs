//! Exact operation selection and turn-bounded lease construction.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use pale_core::records::{
    Approval, CapabilityRequirementIdV1, OperationCatalogEntryV1, OperationContractCatalogV1,
    OperationSupport, PackageResourceRefV1, ToolAuthorizationDigest, TurnIdentityDigest,
};

use crate::hooks::DirectOwnerPrompt;

use super::{ActivationReadyOperation, OperationAdmissionError, VerifiedOperationBundle};

struct UnverifiedSelection<'a> {
    entry: &'a OperationCatalogEntryV1,
    authorized_actions: BTreeMap<CapabilityRequirementIdV1, ToolAuthorizationDigest>,
}

/// Verified operation selection that has not yet been bound to session state.
///
/// Compound dispatch consumes this proof directly. A native-harness session
/// must pass it through
/// [`SessionRuntimeState::activate_operation`](crate::session_state::SessionRuntimeState::activate_operation)
/// before any tool call can be admitted under an active lease.
pub struct OperationActivation {
    pub(crate) operation_id: PackageResourceRefV1,
    pub(crate) contract_digest: pale_core::records::OperationContractDigest,
    pub(crate) catalog_digest: pale_core::records::OperationCatalogDigest,
    pub(crate) binding_digest: pale_core::records::OperationBindingDigest,
    pub(crate) turn_digest: Option<TurnIdentityDigest>,
    pub(crate) authorized_actions: BTreeMap<CapabilityRequirementIdV1, ToolAuthorizationDigest>,
}

impl fmt::Debug for OperationActivation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OperationActivation")
            .field("operation_id", &self.operation_id)
            .field("contract_digest", &self.contract_digest)
            .field("catalog_digest", &self.catalog_digest)
            .field("binding_digest", &self.binding_digest)
            .field("turn_bound", &self.turn_digest.is_some())
            .field("authorized_action_count", &self.authorized_actions.len())
            .finish()
    }
}

/// A selection bound to verified binding and catalog digests.
///
/// Only [`VerifiedOperationBundle::select_direct_owner_prompt`],
/// [`VerifiedOperationBundle::select_dispatch`], and the bundle's internal
/// selection admission create this proof. It remains
/// valid for the immutable bundle it borrows and is rejected by
/// [`ActivationReadyOperation::activate`] when the binding, catalog, entry, or
/// contract identity differs.
pub struct VerifiedOperationSelection<'a> {
    pub(super) bundle: &'a VerifiedOperationBundle,
    pub(super) entry: &'a OperationCatalogEntryV1,
    pub(super) authorized_actions: BTreeMap<CapabilityRequirementIdV1, ToolAuthorizationDigest>,
}

impl VerifiedOperationSelection<'_> {
    /// Entry identity covered by the verified catalog digest.
    pub fn operation_id(&self) -> &PackageResourceRefV1 {
        &self.entry.id
    }
}

fn parse_authorization(
    value: &str,
) -> Result<(CapabilityRequirementIdV1, ToolAuthorizationDigest), OperationAdmissionError> {
    let (requirement, scope_digest) = value
        .split_once('=')
        .ok_or(OperationAdmissionError::MalformedAuthorization)?;
    if requirement.is_empty()
        || requirement.contains('=')
        || requirement.chars().any(char::is_control)
        || !pale_core::is_sha256_hex(scope_digest)
    {
        return Err(OperationAdmissionError::MalformedAuthorization);
    }
    Ok((
        CapabilityRequirementIdV1::parse(requirement)
            .map_err(|_| OperationAdmissionError::MalformedAuthorization)?,
        ToolAuthorizationDigest::parse(scope_digest)
            .map_err(|_| OperationAdmissionError::MalformedAuthorization)?,
    ))
}

/// Parse exact `<requirement>=<tool-request-digest>` authorization values.
///
/// # Errors
///
/// Returns [`OperationAdmissionError::MalformedAuthorization`] for malformed
/// values, invalid digests, control characters, or duplicate requirements.
///
/// # Examples
///
/// ```
/// use pale_application::operation_admission::parse_authorization_values;
///
/// let digest = "0".repeat(64);
/// let authorization = format!("publish={digest}");
/// let parsed = parse_authorization_values([authorization.as_str()])?;
/// let requirement = pale_core::records::CapabilityRequirementIdV1::parse("publish")
///     .expect("static requirement identity is valid");
/// assert!(parsed.contains_key(&requirement));
/// # Ok::<(), pale_application::operation_admission::OperationAdmissionError>(())
/// ```
pub fn parse_authorization_values<'a>(
    values: impl IntoIterator<Item = &'a str>,
) -> Result<BTreeMap<CapabilityRequirementIdV1, ToolAuthorizationDigest>, OperationAdmissionError> {
    let mut authorized = BTreeMap::new();
    for value in values {
        let (requirement, digest) = parse_authorization(value)?;
        if authorized.insert(requirement, digest).is_some() {
            return Err(OperationAdmissionError::MalformedAuthorization);
        }
    }
    Ok(authorized)
}

fn prompt_authorizations<'a>(
    tokens: impl Iterator<Item = &'a str>,
) -> Result<BTreeMap<CapabilityRequirementIdV1, ToolAuthorizationDigest>, OperationAdmissionError> {
    let mut values = Vec::new();
    let mut expect_value = false;
    for token in tokens {
        if expect_value {
            values.push(token);
            expect_value = false;
        } else if token == "--authorize" {
            expect_value = true;
        } else if let Some(value) = token.strip_prefix("--authorize=") {
            values.push(value);
        }
    }
    if expect_value {
        return Err(OperationAdmissionError::MalformedAuthorization);
    }
    parse_authorization_values(values)
}

fn select_prompt_from_catalog<'a>(
    catalog: &'a OperationContractCatalogV1,
    input: &str,
    native_sigil: char,
) -> Result<Option<UnverifiedSelection<'a>>, OperationAdmissionError> {
    let mut tokens = input.split_whitespace();
    let Some(token) = tokens.next() else {
        return Ok(None);
    };
    let Some(invocation) = token.strip_prefix(native_sigil) else {
        if token.trim_start_matches(['/', '$']).starts_with("pale-op-") {
            return Err(OperationAdmissionError::UnknownOperation {
                operation: token.to_string(),
            });
        }
        return Ok(None);
    };
    if !invocation.starts_with("pale-op-") {
        return Ok(None);
    }
    let entry = catalog
        .entries
        .iter()
        .find(|entry| entry.dispatch.invocation() == Some(invocation))
        .ok_or_else(|| OperationAdmissionError::UnknownOperation {
            operation: invocation.to_string(),
        })?;
    Ok(Some(UnverifiedSelection {
        entry,
        authorized_actions: prompt_authorizations(tokens)?,
    }))
}

fn select_dispatch_from_catalog<'a>(
    catalog: &'a OperationContractCatalogV1,
    operation_id: &PackageResourceRefV1,
    authorized_actions: BTreeMap<CapabilityRequirementIdV1, ToolAuthorizationDigest>,
) -> Result<UnverifiedSelection<'a>, OperationAdmissionError> {
    Ok(UnverifiedSelection {
        entry: super::verify::entry_for_id(catalog, operation_id)?,
        authorized_actions,
    })
}

impl VerifiedOperationBundle {
    /// Select an invocation proved to originate from this exact direct path.
    ///
    /// # Errors
    ///
    /// Rejects a proof minted for another path, malformed authorization values,
    /// or an unknown explicit operation invocation. Delegated paths cannot mint
    /// the direct-owner proof and must use [`Self::select_dispatch`].
    pub fn select_direct_owner_prompt(
        &self,
        prompt: DirectOwnerPrompt<'_>,
    ) -> Result<Option<VerifiedOperationSelection<'_>>, OperationAdmissionError> {
        if !self.execution_path_witness().is_direct() {
            return Err(OperationAdmissionError::InvocationPathMismatch);
        }
        let Some(provider_binding) = self
            .execution_path_witness()
            .terminal_provider_binding
            .as_ref()
        else {
            return Err(OperationAdmissionError::InvocationOriginMismatch);
        };
        if !prompt.binds_provider(provider_binding) {
            return Err(OperationAdmissionError::InvocationOriginMismatch);
        }
        select_prompt_from_catalog(self.catalog(), prompt.invocation(), prompt.sigil())?
            .map(|selection| self.admit_selection(selection))
            .transpose()
    }

    /// Select an exact operation under this bundle's verified catalog digest.
    ///
    /// # Errors
    ///
    /// Rejects an operation identity absent from the admitted catalog or an
    /// entry that no longer matches the immutable bundle snapshot.
    pub fn select_dispatch(
        &self,
        operation_id: &PackageResourceRefV1,
        authorized_actions: BTreeMap<CapabilityRequirementIdV1, ToolAuthorizationDigest>,
    ) -> Result<VerifiedOperationSelection<'_>, OperationAdmissionError> {
        self.admit_selection(select_dispatch_from_catalog(
            self.catalog(),
            operation_id,
            authorized_actions,
        )?)
    }

    fn admit_selection<'a>(
        &'a self,
        selection: UnverifiedSelection<'_>,
    ) -> Result<VerifiedOperationSelection<'a>, OperationAdmissionError> {
        let entry = super::verify::entry_for_id(self.catalog(), &selection.entry.id)?;
        if entry != selection.entry {
            return Err(OperationAdmissionError::EntryContractMismatch {
                operation: selection.entry.id.clone(),
            });
        }
        Ok(VerifiedOperationSelection {
            bundle: self,
            entry,
            authorized_actions: selection.authorized_actions,
        })
    }
}

impl ActivationReadyOperation<'_> {
    /// Create a lease carrying this operation's verified binding, catalog, and
    /// contract digests.
    ///
    /// # Errors
    ///
    /// Rejects a selection from another binding/catalog snapshot, a compact
    /// entry that differs from the admitted contract, an unavailable
    /// operation, or authorization for a requirement this route cannot gate.
    pub fn activate(
        &self,
        selection: VerifiedOperationSelection<'_>,
        turn_digest: Option<TurnIdentityDigest>,
    ) -> Result<OperationActivation, OperationAdmissionError> {
        let operation = &self.operation;
        if selection.bundle.binding_digest() != operation.bundle.binding_digest()
            || selection.bundle.catalog_digest() != operation.bundle.catalog_digest()
        {
            return Err(OperationAdmissionError::BindingCatalogMismatch);
        }
        if selection.entry.id != operation.entry.id
            || selection.entry.contract_digest != operation.contract_digest
        {
            return Err(OperationAdmissionError::EntryContractMismatch {
                operation: selection.entry.id.clone(),
            });
        }
        if operation.contract.support() == OperationSupport::Blocked {
            return Err(OperationAdmissionError::Unavailable {
                operation: operation.contract.id.clone(),
            });
        }
        let authorizable = self
            .operation
            .contract
            .capabilities()
            .iter()
            .filter(|capability| {
                matches!(
                    capability.provider_state.approval(),
                    Some(Approval::Ask | Approval::Gated)
                )
            })
            .map(|capability| &capability.requirement_id)
            .collect::<BTreeSet<_>>();
        for requirement in selection.authorized_actions.keys() {
            if !authorizable.contains(requirement) {
                return Err(OperationAdmissionError::UnknownAuthorization {
                    operation: operation.contract.id.clone(),
                    requirement: requirement.clone(),
                });
            }
        }
        Ok(OperationActivation {
            operation_id: operation.contract.id.clone(),
            contract_digest: operation.contract_digest.clone(),
            catalog_digest: operation.bundle.catalog_digest().clone(),
            binding_digest: operation.bundle.binding_digest().clone(),
            turn_digest,
            authorized_actions: selection.authorized_actions,
        })
    }
}
