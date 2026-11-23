//! Package occurrence, use, invalidation, and normative-Pack reference laws.
//!
//! These application records contain only exact coordinates and immutable
//! references. They do not copy Package/Pack bodies or native-owner facts, and
//! they deliberately keep release, occurrence, materialization, activation,
//! use claim, claimed native effect, and settlement claim as distinct records.
//! Local admission validates their structural joins; it does not authenticate
//! the immutable references or establish owner-native truth.

mod invalidation;
mod normative_pack;
mod occurrence;
mod use_records;

pub use invalidation::{
    propagate_package_invalidation, PackageGeneratedDependencySource, PackageInvalidation,
    PackageInvalidationClosure, PackageInvalidationKind, PackageLineageEdge, PackageLineageError,
    PackageLineageGraph, PackageLineageNode, MAX_PACKAGE_LINEAGE_MEMBERS,
};
pub use normative_pack::{
    admit_normative_pack_reference, admit_normative_pack_use, AdmittedNormativePackReference,
    AdmittedNormativePackUse, NormativePackReferenceAdmission, NormativePackReferenceError,
    NormativePackUse, NormativePackUseError, NormativePackUsePurpose, NormativePackUseRights,
    PackPrecedence,
};
pub use occurrence::{
    admit_root_occurrence, derive_child_occurrence, AdmittedPackageOccurrence,
    ChildOccurrenceError, OccurrenceCeilings, OccurrenceContextDerivation, PackageOccurrence,
    PackageOccurrenceContext, RootOccurrenceError,
};
pub use use_records::{
    admit_package_activation, admit_package_materialization, admit_package_native_effect,
    admit_package_settlement, admit_package_use, AdmittedPackageActivation,
    AdmittedPackageMaterialization, AdmittedPackageNativeEffect, AdmittedPackageSettlement,
    AdmittedPackageUse, PackageActivation, PackageActivationError, PackageMaterialization,
    PackageMaterializationError, PackageNativeEffect, PackageNativeEffectError, PackageSettlement,
    PackageSettlementError, PackageUse, PackageUseDisposition, PackageUseError,
};

#[cfg(test)]
#[path = "../../tests/internal/semantic/package_lineage.rs"]
mod tests;
