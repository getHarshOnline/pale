//! Complete-owner retention and exact private copied-string arithmetic.

use super::*;
use crate::{
    PaleApi, PortableWorkspaceDocument, PortableWorkspaceRequest,
    RecipientQualifiedWorkspaceAccessError,
};
use pale_application::workspace::{
    WorkspaceInspectionAuthority, WorkspaceInspectionBudgetLimits, WorkspaceInspectionGrant,
};
use pale_core::records::{InteractionDisclosureCeilingV1, ResidentPrincipalIdV1};
use std::time::Duration;

#[path = "organization_node_queries.rs"]
mod node_queries;

#[path = "organization_correspondence.rs"]
mod correspondence;

fn assert_portable_id_boundary<T: std::str::FromStr>() {
    assert!("x".repeat(255).parse::<T>().is_ok());
    assert!("x".repeat(256).parse::<T>().is_err());
    assert!("é".parse::<T>().is_err());
}

#[test]
fn every_shallow_string_owner_has_the_audited_portable_byte_boundary() {
    use pale_core::records::*;
    assert_portable_id_boundary::<WorkspaceNamespaceId>();
    assert_portable_id_boundary::<WorkspaceResourceId>();
    assert_portable_id_boundary::<WorkspaceControllerScopeId>();
    assert_portable_id_boundary::<WorkspaceAuthorityScopeId>();
    assert_portable_id_boundary::<WorkspaceDerivationSourceId>();
    assert_portable_id_boundary::<LogicalScopeId>();
    assert_portable_id_boundary::<LogicalOrganizationId>();
    assert_portable_id_boundary::<OrganizationalGroupId>();
    assert_portable_id_boundary::<DisciplineTreeId>();
    assert_portable_id_boundary::<OrganizationalRoleId>();
    assert_portable_id_boundary::<OrganizationalOccupantId>();
    assert_portable_id_boundary::<OrganizationalProviderId>();
    assert_portable_id_boundary::<LeadershipTopicId>();
    assert_portable_id_boundary::<OrganizationalRelationId>();
    assert_portable_id_boundary::<DirectionalServiceContractId>();
    assert_portable_id_boundary::<OrganizationalObjectClassId>();
    assert_portable_id_boundary::<OrganizationalObjectId>();
    assert_portable_id_boundary::<OrganizationalRoleClassId>();
    assert_portable_id_boundary::<GroupLensProfileId>();
    assert_portable_id_boundary::<OrganizationalLensId>();
    assert_portable_id_boundary::<OrganizationalProjectionId>();
    assert_portable_id_boundary::<OrganizationalInspectionId>();
    assert_portable_id_boundary::<CommunicationActorHandleId>();
    assert_portable_id_boundary::<ResponsibilityDerivationId>();
    assert_portable_id_boundary::<ResponsibilityOmissionReasonId>();
}

#[path = "../../support/organization_inputs.rs"]
mod inputs;

fn compile() -> PortableWorkspaceCompilation {
    PaleApi::new()
        .unwrap()
        .compile_portable_workspace(PortableWorkspaceRequest::new(
            "pale-workspace.toml".into(),
            inputs::documents()
                .into_iter()
                .map(|(label, bytes)| PortableWorkspaceDocument::new(label, bytes))
                .collect(),
        ))
        .unwrap()
}

fn inspection_principal() -> ResidentPrincipalIdV1 {
    ResidentPrincipalIdV1::parse("principal.scene").unwrap()
}

fn inspection_grant() -> WorkspaceInspectionGrant {
    WorkspaceInspectionGrant::new(
        inspection_principal(),
        InteractionDisclosureCeilingV1::parse("protected").unwrap(),
    )
}

fn inspection_authority() -> Arc<WorkspaceInspectionAuthority> {
    Arc::new(WorkspaceInspectionAuthority::new([inspection_grant()]).unwrap())
}

fn qualified(
    compilation: PortableWorkspaceCompilation,
    authority: &Arc<WorkspaceInspectionAuthority>,
    limits: WorkspaceInspectionBudgetLimits,
) -> crate::RecipientQualifiedWorkspaceInspection {
    let issued = authority
        .issue_workspace_with_budget(
            Arc::clone(&compilation.universe),
            &inspection_principal(),
            limits,
        )
        .unwrap();
    crate::RecipientQualifiedWorkspaceInspection::from_issued(compilation, issued).unwrap()
}

#[test]
fn recipient_qualified_continuation_reenters_authority_before_disclosing_position() {
    let authority = inspection_authority();
    let inspection = qualified(
        compile(),
        &authority,
        WorkspaceInspectionBudgetLimits::default(),
    );
    let first = inspection
        .inspect_organization(None, 1, 1, MAX_ORGANIZATION_COPY_BYTES)
        .unwrap();
    let continuation = first
        .continuation()
        .expect("the actual organization fixture must continue")
        .clone();
    authority.replace_grants([inspection_grant()]).unwrap();
    assert!(matches!(
        inspection.inspect_organization(Some(&continuation), 1, 1, MAX_ORGANIZATION_COPY_BYTES),
        Err(crate::RecipientQualifiedWorkspaceInspectionError::Access(
            RecipientQualifiedWorkspaceAccessError::Denied
        ))
    ));
}

#[test]
fn qualified_workspace_and_pair_summaries_reenter_authority() {
    let workspace_authority = inspection_authority();
    let workspace = qualified(
        compile(),
        &workspace_authority,
        WorkspaceInspectionBudgetLimits::default(),
    );
    let summary = workspace.summary().unwrap();
    assert_eq!(summary.workspace_id(), "empty-adopter");
    workspace_authority
        .replace_grants([inspection_grant()])
        .unwrap();
    assert!(matches!(
        workspace.summary(),
        Err(RecipientQualifiedWorkspaceAccessError::Denied)
    ));
    let prior = compile();
    let current = compile();
    let comparison = PaleApi::new()
        .unwrap()
        .compare_portable_workspaces(&prior, &current)
        .unwrap();
    let pair_authority = inspection_authority();
    let (_, _, issued_comparison) = pair_authority
        .issue_comparison(
            Arc::clone(&prior.universe),
            Arc::clone(&current.universe),
            &inspection_principal(),
        )
        .unwrap()
        .into_parts();
    let pair = crate::RecipientQualifiedWorkspaceComparisonInspection::from_issued(
        prior,
        current,
        comparison,
        issued_comparison,
    )
    .unwrap();
    let summaries = pair.summaries().unwrap();
    assert_eq!(
        summaries.prior().semantic_ir_digest(),
        summaries.comparison().prior_semantic_ir_digest()
    );
    assert_eq!(
        summaries.current().semantic_ir_digest(),
        summaries.comparison().current_semantic_ir_digest()
    );
    pair_authority.replace_grants([inspection_grant()]).unwrap();
    assert!(matches!(
        pair.summaries(),
        Err(crate::RecipientQualifiedWorkspaceInspectionError::Access(
            RecipientQualifiedWorkspaceAccessError::Denied
        ))
    ));
}

#[test]
fn recipient_qualified_pages_share_one_aggregate_application_budget() {
    let authority = inspection_authority();
    let inspection = qualified(
        compile(),
        &authority,
        WorkspaceInspectionBudgetLimits::new(1, u64::MAX, u64::MAX, Duration::from_secs(30)),
    );
    let first = inspection
        .inspect_organization(None, 1, 1, MAX_ORGANIZATION_COPY_BYTES)
        .unwrap();
    let continuation = first
        .continuation()
        .expect("the actual organization fixture must continue")
        .clone();
    assert!(matches!(
        inspection.inspect_organization(Some(&continuation), 1, 1, MAX_ORGANIZATION_COPY_BYTES),
        Err(crate::RecipientQualifiedWorkspaceInspectionError::Access(
            RecipientQualifiedWorkspaceAccessError::WorkBudgetExhausted
        ))
    ));
}

#[test]
fn recipient_qualified_handle_rejects_an_equal_but_separately_retained_owner() {
    let first = compile();
    let second = compile();
    assert_eq!(first.summary(), second.summary());
    let authority = inspection_authority();
    let issued = authority
        .issue_workspace(Arc::clone(&first.universe), &inspection_principal())
        .unwrap();
    assert_eq!(
        crate::RecipientQualifiedWorkspaceInspection::from_issued(second, issued).unwrap_err(),
        crate::RecipientQualifiedWorkspaceOwnerMismatch
    );
}

#[test]
fn recipient_qualified_comparison_rejects_equal_substitute_owners() {
    let prior = compile();
    let current = compile();
    let substitute_prior = compile();
    let comparison = PaleApi::new()
        .unwrap()
        .compare_portable_workspaces(&prior, &current)
        .unwrap();
    let authority = inspection_authority();
    let (_, _, issued) = authority
        .issue_comparison(
            Arc::clone(&prior.universe),
            Arc::clone(&current.universe),
            &inspection_principal(),
        )
        .unwrap()
        .into_parts();
    assert_eq!(
        crate::RecipientQualifiedWorkspaceComparisonInspection::from_issued(
            substitute_prior,
            current,
            comparison,
            issued,
        )
        .unwrap_err(),
        crate::RecipientQualifiedWorkspaceComparisonOwnerMismatch
    );
}

#[test]
fn qualified_comparison_checks_exact_sdk_custody_without_reentering_authority() {
    let prior = compile();
    let current = compile();
    let substitute_prior = compile();
    assert_eq!(prior.summary(), substitute_prior.summary());

    let authority = inspection_authority();
    let (prior_issued, current_issued, comparison_issued) = authority
        .issue_comparison(
            Arc::clone(&prior.universe),
            Arc::clone(&current.universe),
            &inspection_principal(),
        )
        .unwrap()
        .into_parts();
    let prior_inspection =
        crate::RecipientQualifiedWorkspaceInspection::from_issued(prior.clone(), prior_issued)
            .unwrap();
    let current_inspection =
        crate::RecipientQualifiedWorkspaceInspection::from_issued(current.clone(), current_issued)
            .unwrap();
    let substitute_authority = inspection_authority();
    let substitute_inspection = qualified(
        substitute_prior,
        &substitute_authority,
        WorkspaceInspectionBudgetLimits::default(),
    );

    let comparison = PaleApi::new()
        .unwrap()
        .compare_portable_workspaces(&prior, &current)
        .unwrap();
    let qualified_comparison = crate::RecipientQualifiedWorkspaceComparisonInspection::from_issued(
        prior,
        current,
        comparison,
        comparison_issued,
    )
    .unwrap();

    authority.replace_grants([]).unwrap();
    substitute_authority.replace_grants([]).unwrap();

    assert!(qualified_comparison.compares_exact_inspections(&prior_inspection, &current_inspection));
    assert!(!qualified_comparison
        .compares_exact_inspections(&substitute_inspection, &current_inspection));
    assert!(
        !qualified_comparison.compares_exact_inspections(&current_inspection, &prior_inspection)
    );
}

fn selected(
    owner: &PortableWorkspaceCompilation,
    predicate: impl Fn(&OrganizationRow) -> bool,
) -> OrganizationRecordHandle {
    let mut after = None;
    for _ in 0..10_000 {
        let page = owner
            .inspect_organization(after.as_ref(), 256, 256, MAX_ORGANIZATION_COPY_BYTES)
            .unwrap();
        if let Some(entry) = page.entries().iter().find(|entry| predicate(entry.row())) {
            return entry.handle().clone();
        }
        after = Some(
            page.continuation()
                .expect("actual fixture must contain the selected record")
                .clone(),
        );
    }
    panic!("fixture did not terminate");
}

fn retains_independently<T>(make: impl FnOnce(&PortableWorkspaceCompilation) -> T) {
    let owner = compile();
    let weak = Arc::downgrade(&owner.universe);
    let retained = make(&owner);
    drop(owner);
    assert!(
        weak.upgrade().is_some(),
        "complete compilation, not just inner semantic records, must survive"
    );
    drop(retained);
    assert!(
        weak.upgrade().is_none(),
        "the final independent result must release the complete owner"
    );
}

#[test]
fn page_cursor_record_and_source_results_each_retain_the_complete_owner_independently() {
    retains_independently(|owner| {
        owner
            .inspect_organization(None, 1, 1, MAX_ORGANIZATION_COPY_BYTES)
            .unwrap()
    });
    retains_independently(|owner| {
        owner
            .inspect_organization(None, 1, 1, MAX_ORGANIZATION_COPY_BYTES)
            .unwrap()
            .continuation()
            .unwrap()
            .clone()
    });
    retains_independently(|owner| {
        selected(owner, |row| matches!(row, OrganizationRow::Role { .. }))
    });
    retains_independently(|owner| {
        selected(owner, |row| matches!(row, OrganizationRow::Role { .. }))
            .sources(None, 1, 1, MAX_ORGANIZATION_COPY_BYTES)
            .unwrap()
    });
    retains_independently(|owner| {
        let source = selected(owner, |row| matches!(row, OrganizationRow::Role { .. }))
            .sources(None, 1, 1, MAX_ORGANIZATION_COPY_BYTES)
            .unwrap();
        let OrganizationSourceMapping::Named {
            continuation: Some(cursor),
            ..
        } = source.mapping()
        else {
            panic!("bounded actual source scan must continue");
        };
        cursor.clone()
    });
}

#[test]
fn nested_handle_detail_cursor_and_empty_terminal_page_each_retain_the_complete_owner() {
    retains_independently(|owner| {
        let page = selected(owner, |row| matches!(row, OrganizationRow::Service { .. }))
            .service_assignments(None, 1, 1, MAX_ORGANIZATION_COPY_BYTES)
            .unwrap();
        let OrganizationDetailRows::ServiceAssignments(values) = page.rows() else {
            panic!("wrong section");
        };
        values[0].handle().clone()
    });
    retains_independently(|owner| {
        selected(owner, |row| matches!(row, OrganizationRow::Service { .. }))
            .service_assignments(None, 1, 1, MAX_ORGANIZATION_COPY_BYTES)
            .unwrap()
            .continuation()
            .unwrap()
            .clone()
    });
    retains_independently(|owner| {
        let record = selected(owner, |row| matches!(row, OrganizationRow::Service { .. }));
        let mut after = None;
        for _ in 0..10_000 {
            let page = record
                .service_assignments(after.as_ref(), 1, 1, MAX_ORGANIZATION_COPY_BYTES)
                .unwrap();
            if page.is_complete() {
                let OrganizationDetailRows::ServiceAssignments(values) = page.rows() else {
                    panic!("wrong section");
                };
                assert!(values.is_empty());
                return page;
            }
            after = page.continuation().cloned();
        }
        panic!("finite section did not terminate");
    });
}

#[test]
fn private_copy_accounting_uses_utf8_and_failed_charge_does_not_change_used_bytes() {
    let mut budget = CopyBudget::new(3);
    assert_eq!(budget.text("é").unwrap(), "é");
    assert_eq!(budget.used, 2);
    assert!(matches!(
        budget.text("é"),
        Err(OrganizationFailure::CopyBudget)
    ));
    assert_eq!(budget.used, 2);
    assert_eq!(budget.text("a").unwrap(), "a");
    assert_eq!(budget.used, 3);
    assert!(matches!(
        budget.charge(usize::MAX),
        Err(OrganizationFailure::CopyBudget)
    ));
    assert_eq!(budget.used, 3);
}

#[test]
fn locality_projection_preserves_both_canonical_dispositions() {
    use pale_core::records::OrganizationalRelationLocalityV1 as Core;
    for (source, expected) in [
        (Core::Internal, OrganizationRelationLocality::Internal),
        (
            Core::NativeBoundary,
            OrganizationRelationLocality::NativeBoundary,
        ),
    ] {
        assert_eq!(projection::relation_locality(source), expected);
    }
}
