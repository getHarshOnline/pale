//! Qualified, selection-bounded Package semantic graph.
//!
//! Source records use [`PackageResourceLinkV1`] because authors should not
//! repeat their own Package identity. This module is the sole boundary that may
//! consume that shorthand. Every retained node below carries an exact
//! [`PackageResourceUse`]; source links and filesystem coordinates are dropped
//! before the graph can be sealed or joined to private workspace mechanics.

use std::collections::{BTreeMap, BTreeSet};

use pale_contracts::CompiledSchema;
use pale_core::records::{
    AdmittedExclusiveAuthorityClaimV1, AdmittedGoverningOrderV1, AdmittedScopedOrderRelationV1,
    CapabilityFallback, CapabilityPolicy, CapabilityRequirementIdV1, CapabilityRequirementPolicy,
    DegradedOutcomeIdV1, DoctrineEntry, DoctrineFile, DoctrinePlacement, EffectClass,
    FailurePosture, GoverningOrderFile, GoverningOrderV1, LogicalRepositoryIdV1, OfficialReference,
    OnDemandOperation, OperationArchetype, OperationCapabilityRequirement, OperationsFile,
    PackageIdV1, PackageReleaseV1, PackageResourceExposureV1, PackageResourceKindV1,
    PackageResourceLinkV1, PackageResourceRefV1, PackageResourceSelectorV1, PackageResourceV1,
    PackageSemanticIrDigest, PackageSourceRoleV1, PolicyFamilyV1, ResolvedCapabilityFallbackV1,
    ResolvedCapabilityRequirementPolicyV1, ResolvedGoverningConstraintV1,
    ResolvedGoverningContributionV1, ResolvedGoverningNarrowingV1, ResolvedGoverningObligationV1,
    RoleArchetype, RolesFile, RunMode, RunModesFile, ScopedOrderRelationV1, SemanticSelection,
    SpecialistRole, Stack, StacksFile, ToolRecipe,
};
use serde::Serialize;

use super::authoring::{declaration_grammar, PackageDeclarationGrammar};
use super::source_reader::{parse_named_definition, parse_source_with_schema};
use super::{
    BodyError, DistributionSnapshot, MarkdownLinkViolation, OperationProcedureViolation,
    PackageDiagnosticAccumulator, PackageError, PackageGraphInvariant, ResourceLinkRelation,
    ResourceLinkViolation, ResourceSelectorViolation, ResourceSet, RetainedPackageRelease,
};

mod authoring;
mod body_links;
mod governing;
mod pages;
mod parse;
mod qualify;
mod select;
mod types;
mod uses;

pub use pale_core::records::PackageResourceUseV1 as PackageResourceUse;
pub use types::SealedPackageGraph;
pub(crate) use types::{
    AdmittedFallback, AdmittedMarkdown, AdmittedOperationRequirement, AdmittedPackageGraph,
    AdmittedRequirementPolicy, AdmittedResourceNode, PackageResource, RepositoryPackageSelection,
    ResidentPackageParts,
};

use governing::qualify_governing;
use parse::ParsedResource;
use qualify::{qualify_resource, validate_operation_procedures};
use types::{
    repack_markdown_bodies, requirement_policy_recipe, MarkdownSeal, PackageGraphPreimage,
    ResourceRecord, SelectedPackageGraphState, PACKAGE_SEMANTIC_IR_DOMAIN,
};
