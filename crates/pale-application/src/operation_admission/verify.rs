//! Self-digest and cross-record verification for operation artifacts.

use std::collections::BTreeSet;

use pale_core::records::{
    OperationCatalogEntryV1, OperationContractCatalogV1, OperationContractV1,
    OperationRouteBindingV1, PackageResourceRefV1,
};

use super::OperationAdmissionError;

/// Delegate route-binding structural and self-digest validation to its owner.
pub(super) fn binding(binding: &OperationRouteBindingV1) -> Result<(), OperationAdmissionError> {
    binding
        .verify()
        .map_err(|source| OperationAdmissionError::InvalidOperationRecord {
            family: "operation route binding",
            source,
        })
}

/// Verify the compact catalog and reject ambiguous discovery identities.
pub(super) fn catalog(catalog: &OperationContractCatalogV1) -> Result<(), OperationAdmissionError> {
    catalog
        .verify()
        .map_err(|source| OperationAdmissionError::InvalidOperationRecord {
            family: "operation catalog",
            source,
        })?;
    let mut identities = BTreeSet::new();
    let mut paths = BTreeSet::new();
    for entry in &catalog.entries {
        for identity in std::iter::once(entry.id.to_string())
            .chain(entry.dispatch.invocation().map(str::to_owned))
            .chain(entry.dispatch.aliases().iter().cloned())
        {
            if !identities.insert(identity.clone()) {
                return Err(OperationAdmissionError::DuplicateIdentity { identity });
            }
        }
        if !paths.insert(entry.contract.rel_path.clone()) {
            return Err(OperationAdmissionError::DuplicateIdentity {
                identity: entry.contract.rel_path.clone(),
            });
        }
    }
    Ok(())
}

/// Delegate contract-local validation, then enforce host observation limits.
fn contract(contract: &OperationContractV1) -> Result<(), OperationAdmissionError> {
    contract
        .verify()
        .map_err(|source| OperationAdmissionError::InvalidOperationRecord {
            family: "operation contract",
            source,
        })?;
    Ok(())
}

/// Verify that one selected path binds the exact shared catalog identity.
pub(super) fn bundle(
    binding: &OperationRouteBindingV1,
    catalog: &OperationContractCatalogV1,
) -> Result<(), OperationAdmissionError> {
    self::binding(binding)?;
    self::catalog(catalog)?;
    if binding.profile_digest != catalog.profile_digest
        || binding.repository_id != catalog.repository_id
        || binding
            .execution_path_witness
            .terminal_provider_binding
            .as_ref()
            != Some(&catalog.provider_binding)
        || catalog.catalog_digest != binding.catalog_digest
    {
        return Err(OperationAdmissionError::BindingCatalogMismatch);
    }
    Ok(())
}

/// Resolve one exact operation id from a verified catalog.
pub(super) fn entry_for_id<'a>(
    catalog: &'a OperationContractCatalogV1,
    operation_id: &PackageResourceRefV1,
) -> Result<&'a OperationCatalogEntryV1, OperationAdmissionError> {
    catalog
        .entries
        .iter()
        .find(|entry| &entry.id == operation_id)
        .ok_or_else(|| OperationAdmissionError::UnknownOperation {
            operation: operation_id.to_string(),
        })
}

/// Verify the loaded full contract against its compact lookup entry.
pub(super) fn entry_contract(
    entry: &OperationCatalogEntryV1,
    contract: &OperationContractV1,
) -> Result<(), OperationAdmissionError> {
    self::contract(contract)?;
    if entry.id != contract.id
        || entry.dispatch != contract.dispatch
        || entry.state != pale_core::records::CatalogOperationStateV1::from_contract(contract)
        || contract.contract_digest != entry.contract_digest
    {
        return Err(OperationAdmissionError::EntryContractMismatch {
            operation: entry.id.clone(),
        });
    }
    Ok(())
}
