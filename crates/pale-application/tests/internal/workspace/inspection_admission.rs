//! Recipient admission precedes every portable inspection call.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use super::*;
use crate::workspace::{
    compile_portable_workspace, CapturedWorkspaceDocument, CapturedWorkspaceSource,
    WorkspaceInspectionAuthority, WorkspaceInspectionAuthorityError, WorkspaceInspectionGrant,
};

struct AdmissionOwner {
    generation: AtomicU64,
    decisions: AtomicUsize,
    allow: bool,
}

struct GenerationSwapOwner {
    generation: AtomicU64,
    swap_during_recipient_decision: bool,
}

struct ComparisonAuthority {
    generation: AtomicU64,
    decisions: AtomicUsize,
}

struct SummaryAuthority {
    generation: AtomicU64,
    decisions: AtomicUsize,
    comparison: bool,
}

struct ProfileObservationAuthority {
    generation: AtomicU64,
    decisions: AtomicUsize,
    allow: bool,
}

impl WorkspaceInspectionAdmissionOwner for GenerationSwapOwner {
    fn admits_recipient(&self, request: &WorkspaceInspectionRecipientAdmissionRequest<'_>) -> bool {
        if self.swap_during_recipient_decision {
            self.generation.store(8, Ordering::SeqCst);
        }
        WorkspaceInspectionAuthorityGeneration::from_counter(self.generation.load(Ordering::SeqCst))
            == request.recipient().authority_generation()
    }

    fn admits(&self, request: &WorkspaceInspectionAdmissionRequest<'_>) -> bool {
        if !self.swap_during_recipient_decision {
            self.generation.store(8, Ordering::SeqCst);
        }
        WorkspaceInspectionAuthorityGeneration::from_counter(self.generation.load(Ordering::SeqCst))
            == request.recipient().authority_generation()
    }
}

impl WorkspaceInspectionAdmissionOwner for AdmissionOwner {
    fn admits_recipient(&self, request: &WorkspaceInspectionRecipientAdmissionRequest<'_>) -> bool {
        WorkspaceInspectionAuthorityGeneration::from_counter(self.generation.load(Ordering::SeqCst))
            == request.recipient().authority_generation()
            && self.allow
    }

    fn admits(&self, request: &WorkspaceInspectionAdmissionRequest<'_>) -> bool {
        self.decisions.fetch_add(1, Ordering::SeqCst);
        assert_eq!(request.query(), WorkspaceInspectionQuery::Organization);
        WorkspaceInspectionAuthorityGeneration::from_counter(self.generation.load(Ordering::SeqCst))
            == request.recipient().authority_generation()
            && self.allow
    }
}

impl WorkspaceInspectionAdmissionOwner for ComparisonAuthority {
    fn admits_recipient(&self, request: &WorkspaceInspectionRecipientAdmissionRequest<'_>) -> bool {
        WorkspaceInspectionAuthorityGeneration::from_counter(self.generation.load(Ordering::SeqCst))
            == request.recipient().authority_generation()
    }

    fn admits(&self, request: &WorkspaceInspectionAdmissionRequest<'_>) -> bool {
        self.decisions.fetch_add(1, Ordering::SeqCst);
        matches!(
            request.query(),
            WorkspaceInspectionQuery::ComparisonDirectDependencies
                | WorkspaceInspectionQuery::ComparisonSubjectInspection
        ) && request.prior_workspace_id().is_some()
            && WorkspaceInspectionAuthorityGeneration::from_counter(
                self.generation.load(Ordering::SeqCst),
            ) == request.recipient().authority_generation()
    }
}

impl WorkspaceInspectionAdmissionOwner for SummaryAuthority {
    fn admits_recipient(&self, request: &WorkspaceInspectionRecipientAdmissionRequest<'_>) -> bool {
        WorkspaceInspectionAuthorityGeneration::from_counter(self.generation.load(Ordering::SeqCst))
            == request.recipient().authority_generation()
    }

    fn admits(&self, request: &WorkspaceInspectionAdmissionRequest<'_>) -> bool {
        self.decisions.fetch_add(1, Ordering::SeqCst);
        let selected = matches!(
            (self.comparison, request.query()),
            (false, WorkspaceInspectionQuery::WorkspaceSummary)
                | (true, WorkspaceInspectionQuery::ComparisonSummary)
        );
        selected
            && (request.prior_workspace_id().is_some() == self.comparison)
            && WorkspaceInspectionAuthorityGeneration::from_counter(
                self.generation.load(Ordering::SeqCst),
            ) == request.recipient().authority_generation()
    }
}

impl WorkspaceInspectionAdmissionOwner for ProfileObservationAuthority {
    fn admits_recipient(&self, request: &WorkspaceInspectionRecipientAdmissionRequest<'_>) -> bool {
        WorkspaceInspectionAuthorityGeneration::from_counter(self.generation.load(Ordering::SeqCst))
            == request.recipient().authority_generation()
    }

    fn admits(&self, request: &WorkspaceInspectionAdmissionRequest<'_>) -> bool {
        self.decisions.fetch_add(1, Ordering::SeqCst);
        matches!(
            request.query(),
            WorkspaceInspectionQuery::ProfileObservationAssociation
        ) && self.allow
            && request.prior_workspace_id().is_none()
            && WorkspaceInspectionAuthorityGeneration::from_counter(
                self.generation.load(Ordering::SeqCst),
            ) == request.recipient().authority_generation()
    }
}

fn owner() -> Arc<PortableWorkspaceUniverse> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspace-language/empty-adopter");
    let documents = [
        "pale-workspace.toml",
        "resolution.lock.toml",
        "universe.lock.toml",
    ]
    .into_iter()
    .map(|label| {
        CapturedWorkspaceDocument::new(label.to_owned(), std::fs::read(root.join(label)).unwrap())
            .unwrap()
    })
    .collect();
    Arc::new(
        compile_portable_workspace(
            CapturedWorkspaceSource::new(
                CapturedWorkspaceSource::admit_root_manifest("pale-workspace.toml".into()).unwrap(),
                documents,
            )
            .unwrap(),
        )
        .unwrap(),
    )
}

fn recipient() -> WorkspaceInspectionRecipient {
    WorkspaceInspectionRecipient::new(
        pale_core::records::ResidentPrincipalIdV1::parse("principal.alpha").unwrap(),
        WorkspaceInspectionAuthorityGeneration::from_counter(7),
        pale_core::records::InteractionDisclosureCeilingV1::parse("protected").unwrap(),
    )
}

fn test_workspace_inspection(
    owner: Arc<PortableWorkspaceUniverse>,
    authority: Arc<dyn WorkspaceInspectionAdmissionOwner>,
) -> AdmittedWorkspaceInspection {
    AdmittedWorkspaceInspection::new_with_budget(
        owner,
        recipient(),
        authority,
        Arc::new(WorkspaceInspectionBudget::new(
            WorkspaceInspectionBudgetLimits::default(),
        )),
    )
}

fn test_comparison_inspection(
    prior: Arc<PortableWorkspaceUniverse>,
    current: Arc<PortableWorkspaceUniverse>,
    authority: Arc<dyn WorkspaceInspectionAdmissionOwner>,
) -> AdmittedWorkspaceComparisonInspection {
    AdmittedWorkspaceComparisonInspection::new_with_budget(
        prior,
        current,
        recipient(),
        authority,
        Arc::new(WorkspaceInspectionBudget::new(
            WorkspaceInspectionBudgetLimits::default(),
        )),
    )
}

fn production_principal() -> pale_core::records::ResidentPrincipalIdV1 {
    pale_core::records::ResidentPrincipalIdV1::parse("principal.scene").unwrap()
}

fn production_grant(disclosure: &str) -> WorkspaceInspectionGrant {
    WorkspaceInspectionGrant::new(
        production_principal(),
        pale_core::records::InteractionDisclosureCeilingV1::parse(disclosure).unwrap(),
    )
}

#[test]
fn production_authority_rejects_ambiguous_or_absent_recipient_grants() {
    assert!(matches!(
        WorkspaceInspectionAuthority::new([
            production_grant("protected"),
            production_grant("public"),
        ]),
        Err(WorkspaceInspectionAuthorityError::DuplicatePrincipal)
    ));

    let authority = Arc::new(WorkspaceInspectionAuthority::new([]).unwrap());
    assert!(matches!(
        authority.issue_workspace(owner(), &production_principal()),
        Err(WorkspaceInspectionDenied)
    ));
}

#[test]
fn production_authority_invalidates_old_handles_and_preserves_exact_disclosure() {
    let authority =
        Arc::new(WorkspaceInspectionAuthority::new([production_grant("protected")]).unwrap());
    let retained = owner();
    let issued = authority
        .issue_workspace(Arc::clone(&retained), &production_principal())
        .unwrap();
    assert_eq!(
        issued.recipient().disclosure_ceiling().as_str(),
        "protected"
    );
    issued.authorize_summary().unwrap();

    authority
        .replace_grants([production_grant("public")])
        .unwrap();
    assert!(matches!(
        issued.authorize_summary(),
        Err(WorkspaceInspectionAccessError::Denied(_))
    ));

    let replacement = authority
        .issue_workspace(retained, &production_principal())
        .unwrap();
    assert_eq!(
        replacement.recipient().disclosure_ceiling().as_str(),
        "public"
    );
    replacement.authorize_summary().unwrap();
}

#[test]
fn production_pair_binds_both_owners_and_one_aggregate_budget() {
    let authority =
        Arc::new(WorkspaceInspectionAuthority::new([production_grant("protected")]).unwrap());
    let prior_owner = owner();
    let current_owner = owner();
    let issued = authority
        .issue_comparison_with_budget(
            Arc::clone(&prior_owner),
            Arc::clone(&current_owner),
            &production_principal(),
            WorkspaceInspectionBudgetLimits::new(
                2,
                u64::MAX,
                u64::MAX,
                std::time::Duration::from_secs(30),
            ),
        )
        .unwrap();
    let (prior, current, comparison) = issued.into_parts();
    assert!(prior.belongs_to(&prior_owner));
    assert!(current.belongs_to(&current_owner));
    assert!(comparison.belongs_to(&prior_owner, &current_owner));

    prior.authorize_summary().unwrap();
    current.authorize_summary().unwrap();
    assert!(matches!(
        comparison.authorize_summary(),
        Err(WorkspaceInspectionAccessError::Budget(
            WorkspaceInspectionBudgetExceeded::Work
        ))
    ));
}

#[test]
fn denial_and_stale_authority_stop_before_enumeration_without_leaking_frontiers() {
    let compilation = owner();
    let denied_owner = Arc::new(AdmissionOwner {
        generation: AtomicU64::new(7),
        decisions: AtomicUsize::new(0),
        allow: false,
    });
    let denied = test_workspace_inspection(Arc::clone(&compilation), denied_owner.clone());
    assert!(matches!(
        denied.inspect_organization(None, 1, 1),
        Err(WorkspaceInspectionAccessError::Denied(
            WorkspaceInspectionDenied
        ))
    ));
    assert_eq!(denied_owner.decisions.load(Ordering::SeqCst), 0);

    let admitted_owner = Arc::new(AdmissionOwner {
        generation: AtomicU64::new(7),
        decisions: AtomicUsize::new(0),
        allow: true,
    });
    let admitted = test_workspace_inspection(Arc::clone(&compilation), admitted_owner.clone());
    let page = admitted.inspect_organization(None, 1, 1).unwrap().unwrap();
    assert!(page.entries().is_empty());
    admitted_owner.generation.store(8, Ordering::SeqCst);
    assert!(matches!(
        admitted.inspect_organization(page.continuation(), 1, 1),
        Err(WorkspaceInspectionAccessError::Denied(
            WorkspaceInspectionDenied
        ))
    ));
    assert_eq!(admitted_owner.decisions.load(Ordering::SeqCst), 1);
}

#[test]
fn exact_retained_owner_rejects_equal_byte_recompilation() {
    let first = owner();
    let second = owner();
    assert_eq!(first.semantic_ir_digest(), second.semantic_ir_digest());
    let authority = Arc::new(AdmissionOwner {
        generation: AtomicU64::new(7),
        decisions: AtomicUsize::new(0),
        allow: true,
    });
    let admitted = test_workspace_inspection(first.clone(), authority);
    assert!(admitted.belongs_to(&first));
    assert!(!admitted.belongs_to(&second));
}

#[test]
fn aggregate_work_is_not_refunded_after_exhaustion() {
    let compilation = owner();
    let authority = Arc::new(AdmissionOwner {
        generation: AtomicU64::new(7),
        decisions: AtomicUsize::new(0),
        allow: true,
    });
    let budget = Arc::new(WorkspaceInspectionBudget::new(
        WorkspaceInspectionBudgetLimits::new(1, 64, 64, std::time::Duration::from_secs(5)),
    ));
    let admitted =
        AdmittedWorkspaceInspection::new_with_budget(compilation, recipient(), authority, budget);
    admitted.inspect_organization(None, 1, 1).unwrap().unwrap();
    assert!(matches!(
        admitted.inspect_organization(None, 1, 1),
        Err(WorkspaceInspectionAccessError::Budget(
            WorkspaceInspectionBudgetExceeded::Work
        ))
    ));
    assert!(matches!(
        admitted.inspect_organization(None, 1, 1),
        Err(WorkspaceInspectionAccessError::Budget(
            WorkspaceInspectionBudgetExceeded::Work
        ))
    ));
}

#[test]
fn generation_swap_inside_either_atomic_decision_fails_closed() {
    for swap_during_recipient_decision in [true, false] {
        let admitted = test_workspace_inspection(
            owner(),
            Arc::new(GenerationSwapOwner {
                generation: AtomicU64::new(7),
                swap_during_recipient_decision,
            }),
        );
        let result = if swap_during_recipient_decision {
            admitted
                .preflight()
                .map_err(WorkspaceInspectionAccessError::from)
        } else {
            admitted.inspect_organization(None, 1, 1).map(|_| ())
        };
        assert!(matches!(
            result,
            Err(WorkspaceInspectionAccessError::Denied(
                WorkspaceInspectionDenied
            ))
        ));
    }
}

#[test]
fn workspace_and_comparison_summaries_require_fresh_authority() {
    let workspace_authority = Arc::new(SummaryAuthority {
        generation: AtomicU64::new(7),
        decisions: AtomicUsize::new(0),
        comparison: false,
    });
    let workspace = test_workspace_inspection(owner(), workspace_authority.clone());
    workspace.authorize_summary().unwrap();
    workspace_authority.generation.store(8, Ordering::SeqCst);
    assert!(matches!(
        workspace.authorize_summary(),
        Err(WorkspaceInspectionAccessError::Denied(_))
    ));
    assert_eq!(workspace_authority.decisions.load(Ordering::SeqCst), 1);

    let comparison_authority = Arc::new(SummaryAuthority {
        generation: AtomicU64::new(7),
        decisions: AtomicUsize::new(0),
        comparison: true,
    });
    let comparison = test_comparison_inspection(owner(), owner(), comparison_authority.clone());
    comparison.authorize_summary().unwrap();
    comparison_authority.generation.store(8, Ordering::SeqCst);
    assert!(matches!(
        comparison.authorize_summary(),
        Err(WorkspaceInspectionAccessError::Denied(_))
    ));
    assert_eq!(comparison_authority.decisions.load(Ordering::SeqCst), 1);
}

#[test]
fn profile_observation_association_requires_its_own_fresh_decision() {
    let authority = Arc::new(ProfileObservationAuthority {
        generation: AtomicU64::new(7),
        decisions: AtomicUsize::new(0),
        allow: true,
    });
    let inspection = test_workspace_inspection(owner(), authority.clone());
    inspection
        .authorize_profile_observation_association()
        .unwrap();
    assert_eq!(authority.decisions.load(Ordering::SeqCst), 1);

    authority.generation.store(8, Ordering::SeqCst);
    assert!(matches!(
        inspection.authorize_profile_observation_association(),
        Err(WorkspaceInspectionAccessError::Denied(_))
    ));
    assert_eq!(authority.decisions.load(Ordering::SeqCst), 1);

    let denied_authority = Arc::new(ProfileObservationAuthority {
        generation: AtomicU64::new(7),
        decisions: AtomicUsize::new(0),
        allow: false,
    });
    let denied = test_workspace_inspection(owner(), denied_authority.clone());
    assert!(matches!(
        denied.authorize_profile_observation_association(),
        Err(WorkspaceInspectionAccessError::Denied(_))
    ));
    assert_eq!(denied_authority.decisions.load(Ordering::SeqCst), 1);
}

#[test]
fn comparison_side_is_preserved_in_dependency_and_subject_permits() {
    let prior = owner();
    let current = owner();
    let authority = Arc::new(ComparisonAuthority {
        generation: AtomicU64::new(7),
        decisions: AtomicUsize::new(0),
    });
    let admitted = test_comparison_inspection(prior, current, authority.clone());
    let subject = WorkspaceDerivationSubjectV1::Workspace {
        workspace_id: pale_core::records::WorkspaceId::parse("workspace.empty").unwrap(),
    };

    let dependency = admitted
        .authorize_direct_dependencies(WorkspaceInspectionComparisonSide::Prior, &subject, true, 3)
        .unwrap();
    assert!(dependency.matches(WorkspaceInspectionComparisonSide::Prior, &subject, true, 3,));
    assert!(!dependency.matches(
        WorkspaceInspectionComparisonSide::Current,
        &subject,
        true,
        3,
    ));

    let inspection = admitted
        .authorize_subject_inspection(
            WorkspaceInspectionComparisonSide::Prior,
            &subject,
            false,
            2,
            4,
            5,
        )
        .unwrap();
    assert!(inspection.matches(
        WorkspaceInspectionComparisonSide::Prior,
        &subject,
        false,
        2,
        4,
        5,
    ));
    assert!(!inspection.matches(
        WorkspaceInspectionComparisonSide::Current,
        &subject,
        false,
        2,
        4,
        5,
    ));
    let current_dependency = admitted
        .authorize_direct_dependencies(
            WorkspaceInspectionComparisonSide::Current,
            &subject,
            true,
            3,
        )
        .unwrap();
    assert!(current_dependency.matches(
        WorkspaceInspectionComparisonSide::Current,
        &subject,
        true,
        3,
    ));
    let current_inspection = admitted
        .authorize_subject_inspection(
            WorkspaceInspectionComparisonSide::Current,
            &subject,
            false,
            2,
            4,
            5,
        )
        .unwrap();
    assert!(current_inspection.matches(
        WorkspaceInspectionComparisonSide::Current,
        &subject,
        false,
        2,
        4,
        5,
    ));
    assert_eq!(authority.decisions.load(Ordering::SeqCst), 4);
}

#[test]
fn stale_comparison_authority_stops_before_side_or_continuation_disclosure() {
    let prior = owner();
    let current = owner();
    let authority = Arc::new(ComparisonAuthority {
        generation: AtomicU64::new(8),
        decisions: AtomicUsize::new(0),
    });
    let admitted = test_comparison_inspection(prior, current, authority.clone());
    let subject = WorkspaceDerivationSubjectV1::Workspace {
        workspace_id: pale_core::records::WorkspaceId::parse("workspace.empty").unwrap(),
    };

    assert!(matches!(
        admitted.authorize_direct_dependencies(
            WorkspaceInspectionComparisonSide::Current,
            &subject,
            true,
            3,
        ),
        Err(WorkspaceInspectionAccessError::Denied(_))
    ));
    assert!(matches!(
        admitted.authorize_subject_inspection(
            WorkspaceInspectionComparisonSide::Current,
            &subject,
            true,
            2,
            4,
            5,
        ),
        Err(WorkspaceInspectionAccessError::Denied(_))
    ));
    assert_eq!(
        authority.decisions.load(Ordering::SeqCst),
        0,
        "recipient preflight must fail before side, subject or continuation admission"
    );
}
