//! Façade and source-home boundary conformance for `pale-acp`.

use pale_acp::AcpConductor;
use pale_sdk::{
    capability_inventory, ApiLeaf, AuthoringIngressClass, AuthoringInspection,
    AuthoringInspectionRequest, AuthoringInspectionState, AuthoringInstancePathSegment,
    AuthoringNavigation, AuthoringNavigationRequest, AuthoringPathSegment, DiagnosticDisposition,
    DiagnosticEffectCertainty, DiagnosticProjection, DiagnosticRetry, DiagnosticStage, PaleApi,
    PortableWorkspaceChangeRootPage, PortableWorkspaceDocument, PortableWorkspaceExplanation,
    PortableWorkspaceImpactPage, PortableWorkspaceRequest, PortableWorkspaceSubject,
    StandaloneAuthoringRole,
};

const EMPTY_ROOT: &[u8] = br#"[workspace]
schema_version = 1
workspace_id = "empty-adopter"
distribution_lock = "distribution.lock.toml"
machine_binding_root = "machines"
desired_topology = { projects = [], execution_workspaces = [], repositories = [], execution_paths = [], project_repositories = [], project_execution_workspaces = [], execution_workspace_repositories = [] }
module_manifests = []
source_components = []
responsibility_derivations = []
organizational_translations = []
experience_compositions = []
resolution_lock = "resolution.lock.toml"
universe_lock = "universe.lock.toml"
"#;
const EMPTY_RESOLUTION_LOCK: &[u8] = br#"schema = "pale/workspace-resolution-lock.v1"
schema_version = 1
releases = []
resolution_digest = "945db2ec1686d74bc476a920c073a6d9bbb69ea34ab78cda178d6177ce2b855c"
"#;
const EMPTY_UNIVERSE_LOCK: &[u8] = br#"schema = "pale/workspace-universe-lock.v1"
schema_version = 1
workspace_id = "empty-adopter"
resolution_digest = "945db2ec1686d74bc476a920c073a6d9bbb69ea34ab78cda178d6177ce2b855c"
adopted_releases = []
universe_digest = "b468dd9087987d0e58f55dbe69fa4f1587270497e205150a1622d35dd69734f5"
"#;

#[test]
fn acp_eligibility_inventory_is_the_selected_eighteen_operation_subset() {
    let _conductor = AcpConductor::new().expect("ACP diagnostics must initialize");
    let eligible = capability_inventory()
        .iter()
        .filter(|capability| capability.eligible_for(ApiLeaf::Acp))
        .map(|capability| capability.id().as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        eligible,
        [
            "diagnostic.projection",
            "authoring.ingress-limits",
            "authoring.standalone-validation",
            "authoring.standalone-inspection",
            "workspace.portable-compilation",
            "workspace.portable-authoring-inspection",
            "workspace.portable-authoring-navigation",
            "workspace.portable-comparison",
            "workspace.portable-change-roots",
            "workspace.portable-impacts",
            "workspace.portable-explanation",
            "workspace.effective-profile-compilation",
            "workspace.effective-profile-inspection",
            "workspace.effective-profile-explanation",
            "workspace.effective-profile-authoring-inspection",
            "workspace.effective-profile-authoring-navigation",
            "workspace.effective-profile-authoring-join",
            "workspace.configuration-improvement",
        ]
    );
}

fn empty_workspace_request() -> PortableWorkspaceRequest {
    PortableWorkspaceRequest::new(
        "pale-workspace.toml".to_owned(),
        vec![
            PortableWorkspaceDocument::new("pale-workspace.toml".to_owned(), EMPTY_ROOT.to_vec()),
            PortableWorkspaceDocument::new(
                "resolution.lock.toml".to_owned(),
                EMPTY_RESOLUTION_LOCK.to_vec(),
            ),
            PortableWorkspaceDocument::new(
                "universe.lock.toml".to_owned(),
                EMPTY_UNIVERSE_LOCK.to_vec(),
            ),
        ],
    )
}

#[test]
fn portable_handle_operations_remain_in_the_canonical_sdk() {
    let api = PaleApi::new().expect("Pale façade must initialize");
    let prior = api
        .compile_portable_workspace(empty_workspace_request())
        .expect("empty portable workspace must compile");
    let current = api
        .compile_portable_workspace(empty_workspace_request())
        .expect("same portable workspace must compile again");

    let inspection: AuthoringInspection = prior
        .inspect_authoring(AuthoringInspectionRequest::new(
            "pale-workspace.toml".to_owned(),
            vec![AuthoringPathSegment::Field("workspace".to_owned())],
        ))
        .expect("admitted workspace authoring must be inspectable");
    assert_eq!(inspection.state(), AuthoringInspectionState::Available);
    let navigation: AuthoringNavigation =
        prior.navigate_authoring(AuthoringNavigationRequest::new(
            "pale-workspace.toml".to_owned(),
            vec![
                AuthoringInstancePathSegment::Field("workspace".to_owned()),
                AuthoringInstancePathSegment::Field("workspace_id".to_owned()),
            ],
        ));
    assert!(navigation.identity().is_some());

    let comparison = api
        .compare_portable_workspaces(&prior, &current)
        .expect("identical admitted workspaces must compare");
    let roots: PortableWorkspaceChangeRootPage = comparison.change_roots(0, 0);
    assert!(roots.entries().is_empty());
    let impacts: PortableWorkspaceImpactPage = comparison.impacts(0, 0);
    let subject: &PortableWorkspaceSubject = impacts
        .entries()
        .first()
        .expect("an admitted workspace has derivation subjects")
        .subject();
    let _explanation: PortableWorkspaceExplanation = comparison
        .explain(subject)
        .expect("retained proof must explain its own subject");
}

#[test]
fn effect_free_authoring_and_compilation_dispatch_through_the_facade() {
    let api = PaleApi::new().expect("Pale façade must initialize");
    let limits = PaleApi::authoring_ingress_limits(AuthoringIngressClass::Workspace);
    assert!(limits.max_documents() > 0);
    assert!(limits.max_document_bytes() > 0);
    assert!(limits.max_total_bytes() >= limits.max_document_bytes());

    let inspection = api
        .inspect_standalone_authoring(StandaloneAuthoringRole::WorkspaceManifest, Vec::new())
        .expect("the embedded workspace grammar must be inspectable");
    assert_eq!(inspection.state(), AuthoringInspectionState::Available);

    let error = match api.compile_portable_workspace(PortableWorkspaceRequest::new(
        "pale-workspace.toml".to_owned(),
        vec![PortableWorkspaceDocument::new(
            "pale-workspace.toml".to_owned(),
            b"[".to_vec(),
        )],
    )) {
        Ok(_) => panic!("invalid workspace TOML must not compile"),
        Err(error) => error,
    };
    assert_eq!(
        error.diagnostic().effect(),
        DiagnosticEffectCertainty::NotApplicable
    );
}

#[test]
fn authoring_validation_dispatches_through_the_canonical_facade() {
    let api = PaleApi::new().expect("Pale façade must initialize");
    let error = api
        .validate_authoring_document(
            StandaloneAuthoringRole::WorkspaceManifest,
            "workspace.toml".to_owned(),
            b"[".to_vec(),
        )
        .expect_err("invalid TOML must be rejected by the owning validator");
    assert!(!error.diagnostic().code().is_empty());
    assert_eq!(
        error.diagnostic().effect(),
        DiagnosticEffectCertainty::NotApplicable
    );

    let diagnostic = api
        .project_diagnostic(DiagnosticProjection::new(
            "acp_projection_attestation".to_owned(),
            DiagnosticStage::InputAdmission,
            DiagnosticDisposition::Blocked,
            "ACP public projection attestation".to_owned(),
            DiagnosticRetry::NotIndicated,
            DiagnosticEffectCertainty::NotApplicable,
        ))
        .expect("the canonical SDK must project its diagnostic");
    assert_eq!(diagnostic.code(), "acp_projection_attestation");
}
