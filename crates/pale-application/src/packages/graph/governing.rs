//! Global governing-fragment qualification with contributor provenance.

use super::qualify::{check_link, declaration_for_use};
use super::*;
use pale_core::records::GoverningResourceLinkV1;

pub(super) fn qualify_governing(
    contributor: &PackageResourceRefV1,
    source: &GoverningOrderV1,
    owner: &RetainedPackageRelease,
    declarations: &BTreeMap<PackageResourceUse, PackageResourceV1>,
) -> Result<ResolvedGoverningContributionV1, PackageError> {
    let mut diagnostics = PackageDiagnosticAccumulator::default();
    let mut constraints = Vec::new();
    let mut obligations = Vec::new();
    let mut exclusive_authority = Vec::new();
    let mut narrowing = Vec::new();
    let mut order = Vec::new();
    for constraint in &source.constraints {
        if let Some(row) = diagnostics.capture(qualify_constraint(
            constraint,
            contributor,
            owner,
            declarations,
        )) {
            constraints.push(row);
        }
    }
    for obligation in &source.obligations {
        if let Some(row) = diagnostics.capture(qualify_obligation(
            obligation,
            contributor,
            owner,
            declarations,
        )) {
            obligations.push(row);
        }
    }
    for claim in &source.exclusive_authority {
        if let Some(row) = diagnostics.capture(qualify_exclusive_authority(
            claim,
            contributor,
            owner,
            declarations,
        )) {
            exclusive_authority.push(row);
        }
    }
    for edge in &source.narrowing {
        if let Some(row) =
            diagnostics.capture(qualify_narrowing(edge, contributor, owner, declarations))
        {
            narrowing.push(row);
        }
    }
    for edge in &source.order {
        if let Some(row) =
            diagnostics.capture(qualify_order(edge, contributor, owner, declarations))
        {
            order.push(row);
        }
    }
    diagnostics.finish(ResolvedGoverningContributionV1 {
        constraints,
        obligations,
        exclusive_authority,
        narrowing,
        order,
    })
}

fn qualify_constraint(
    constraint: &pale_core::records::GoverningConstraintV1,
    contributor: &PackageResourceRefV1,
    owner: &RetainedPackageRelease,
    declarations: &BTreeMap<PackageResourceUse, PackageResourceV1>,
) -> Result<ResolvedGoverningConstraintV1, PackageError> {
    Ok(ResolvedGoverningConstraintV1 {
        scope: check_governing_link(
            &constraint.scope,
            contributor,
            owner,
            declarations,
            ResourceLinkRelation::GoverningConstraintScope,
        )?,
        subject: check_governing_link(
            &constraint.subject,
            contributor,
            owner,
            declarations,
            ResourceLinkRelation::GoverningConstraintSubject,
        )?,
        policy: check_link(
            &constraint.policy,
            contributor,
            owner,
            declarations,
            Some(PackageResourceKindV1::CapabilityPolicy),
            ResourceLinkRelation::GoverningConstraintPolicy,
        )?,
        contributor: contributor.clone(),
    })
}

fn qualify_obligation(
    obligation: &pale_core::records::GoverningObligationV1,
    contributor: &PackageResourceRefV1,
    owner: &RetainedPackageRelease,
    declarations: &BTreeMap<PackageResourceUse, PackageResourceV1>,
) -> Result<ResolvedGoverningObligationV1, PackageError> {
    Ok(ResolvedGoverningObligationV1 {
        scope: check_governing_link(
            &obligation.scope,
            contributor,
            owner,
            declarations,
            ResourceLinkRelation::GoverningObligationScope,
        )?,
        subject: check_governing_link(
            &obligation.subject,
            contributor,
            owner,
            declarations,
            ResourceLinkRelation::GoverningObligationSubject,
        )?,
        obligation: check_governing_link(
            &obligation.obligation,
            contributor,
            owner,
            declarations,
            ResourceLinkRelation::GoverningObligation,
        )?,
        contributor: contributor.clone(),
    })
}

fn qualify_exclusive_authority(
    claim: &pale_core::records::GoverningExclusiveAuthorityClaimV1,
    contributor: &PackageResourceRefV1,
    owner: &RetainedPackageRelease,
    declarations: &BTreeMap<PackageResourceUse, PackageResourceV1>,
) -> Result<AdmittedExclusiveAuthorityClaimV1, PackageError> {
    Ok(AdmittedExclusiveAuthorityClaimV1 {
        scope: check_governing_link(
            &claim.scope,
            contributor,
            owner,
            declarations,
            ResourceLinkRelation::GoverningExclusiveScope,
        )?,
        authority: check_governing_link(
            &claim.authority,
            contributor,
            owner,
            declarations,
            ResourceLinkRelation::GoverningExclusiveAuthority,
        )?,
        claimant: contributor.clone(),
    })
}

fn qualify_narrowing(
    edge: &pale_core::records::GoverningNarrowingV1,
    contributor: &PackageResourceRefV1,
    owner: &RetainedPackageRelease,
    declarations: &BTreeMap<PackageResourceUse, PackageResourceV1>,
) -> Result<ResolvedGoverningNarrowingV1, PackageError> {
    let scope = check_governing_link(
        &edge.scope,
        contributor,
        owner,
        declarations,
        ResourceLinkRelation::GoverningNarrowingScope,
    )?;
    let narrower = check_governing_link(
        &edge.narrower,
        contributor,
        owner,
        declarations,
        ResourceLinkRelation::GoverningNarrower,
    )?;
    let broader = check_governing_link(
        &edge.broader,
        contributor,
        owner,
        declarations,
        ResourceLinkRelation::GoverningBroader,
    )?;
    require_same_kind(
        contributor,
        &narrower,
        &broader,
        ResourceLinkRelation::GoverningBroader,
    )?;
    Ok(ResolvedGoverningNarrowingV1 {
        scope,
        narrower,
        broader,
        contributor: contributor.clone(),
    })
}

fn qualify_order(
    edge: &ScopedOrderRelationV1,
    contributor: &PackageResourceRefV1,
    owner: &RetainedPackageRelease,
    declarations: &BTreeMap<PackageResourceUse, PackageResourceV1>,
) -> Result<AdmittedScopedOrderRelationV1, PackageError> {
    let scope = check_governing_link(
        &edge.scope,
        contributor,
        owner,
        declarations,
        ResourceLinkRelation::GoverningOrderScope,
    )?;
    let before = check_governing_link(
        &edge.before,
        contributor,
        owner,
        declarations,
        ResourceLinkRelation::GoverningOrderPrerequisite,
    )?;
    let after = check_governing_link(
        &edge.after,
        contributor,
        owner,
        declarations,
        ResourceLinkRelation::GoverningOrderDependent,
    )?;
    require_same_kind(
        contributor,
        &before,
        &after,
        ResourceLinkRelation::GoverningOrderDependent,
    )?;
    Ok(AdmittedScopedOrderRelationV1 {
        scope,
        purpose: edge.purpose,
        before,
        after,
        contributor: contributor.clone(),
    })
}

/// Qualify exactly one use, preserving its visibility and authored kind.
fn check_governing_link(
    link: &GoverningResourceLinkV1,
    contributor: &PackageResourceRefV1,
    owner: &RetainedPackageRelease,
    declarations: &BTreeMap<PackageResourceUse, PackageResourceV1>,
    relation: ResourceLinkRelation,
) -> Result<PackageResourceUse, PackageError> {
    let reference = check_link(
        &link.resource_link(),
        contributor,
        owner,
        declarations,
        link.kind(),
        relation,
    )?;
    let declaration = declaration_for_use(declarations, &reference, link.kind())
        .expect("the checked link identifies exactly one declared use");
    Ok(PackageResourceUse::new(reference, declaration.kind))
}

/// Ordering and narrowing relate uses of the same kind.
fn require_same_kind(
    contributor: &PackageResourceRefV1,
    first: &PackageResourceUse,
    second: &PackageResourceUse,
    relation: ResourceLinkRelation,
) -> Result<(), PackageError> {
    if first.kind != second.kind {
        return Err(PackageError::InvalidResourceLink {
            from: contributor.clone(),
            target: second.resource.clone(),
            relation,
            reason: ResourceLinkViolation::WrongKind,
        });
    }
    Ok(())
}

#[cfg(all(test, unix))]
#[path = "../../../tests/internal/packages/graph/governing.rs"]
mod tests;
