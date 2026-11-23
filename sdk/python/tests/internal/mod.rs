//! Binding-internal law tests kept outside the production source tree.

use pale_sdk::{capability_inventory, ApiCapabilityId, ApiLeaf};
use pyo3::types::{PyAnyMethods, PyBytesMethods};

use crate::PalePython;

mod authoring_errors;
mod effective_profile_entries;

fn document(label: &str, bytes: &[u8]) -> crate::PyCompilationDocument {
    crate::PyCompilationDocument::new(label.to_owned(), bytes.to_vec())
}

fn private_inputs(identity: &crate::PyContractBundleIdentity) -> crate::PyOneShotPrivateInputs {
    private_inputs_at(identity, "/captured/repository")
}

fn private_inputs_at(
    identity: &crate::PyContractBundleIdentity,
    machine_root: &str,
) -> crate::PyOneShotPrivateInputs {
    let distribution_lock = String::from_utf8(
        include_bytes!(concat!(
            "../fixtures/cross-leaf.v1/one-shot",
            "/private/distribution-lock.toml"
        ))
        .to_vec(),
    )
    .expect("distribution-lock fixture is UTF-8");
    let mut lines = distribution_lock.lines();
    let header = lines.next().expect("distribution lock has a header");
    let compiler = lines.next().expect("distribution lock has a compiler");
    let _old_manifest = lines.next().expect("distribution lock has a manifest");
    let _old_set = lines.next().expect("distribution lock has a set");
    let remainder = lines.collect::<Vec<_>>().join("\n");
    let distribution_lock = format!(
        "{header}\n{compiler}\nschema_manifest_sha256 = \"{}\"\nschema_set_sha256 = \"{}\"\n{remainder}\n",
        identity.schema_manifest_sha256(),
        identity.schema_set_sha256()
    );
    let machine_binding = String::from_utf8(
        include_bytes!(concat!(
            "../fixtures/cross-leaf.v1/one-shot",
            "/private/machine-binding.toml"
        ))
        .to_vec(),
    )
    .expect("machine-binding fixture is UTF-8")
    .replace("/captured/repository", machine_root);
    crate::PyOneShotPrivateInputs::new(
        document(
            "capabilities.toml",
            include_bytes!(concat!(
                "../fixtures/cross-leaf.v1/one-shot",
                "/private/capabilities.toml"
            )),
        ),
        document("machines/machine.toml", machine_binding.as_bytes()),
        document("distribution-lock.toml", distribution_lock.as_bytes()),
        Vec::new(),
    )
}

mod effective_profile_explanation;
mod improvement;
mod profile_dispositions;

fn package_documents() -> Vec<crate::PyCompilationDocument> {
    vec![
        document(
            "engine/package.toml",
            include_bytes!(concat!(
                "../fixtures/cross-leaf.v1/one-shot",
                "/packages/engine/package.toml"
            )),
        ),
        document(
            "engine/role.md",
            include_bytes!(concat!(
                "../fixtures/cross-leaf.v1/one-shot",
                "/packages/engine/role.md"
            )),
        ),
        document(
            "engine/roles.toml",
            include_bytes!(concat!(
                "../fixtures/cross-leaf.v1/one-shot",
                "/packages/engine/roles.toml"
            )),
        ),
    ]
}

fn workspace_documents() -> Vec<crate::PyCompilationDocument> {
    vec![
        document(
            "composition.toml",
            include_bytes!(concat!(
                "../fixtures/cross-leaf.v1/one-shot",
                "/workspace/composition.toml"
            )),
        ),
        document(
            "pale-workspace.toml",
            include_bytes!(concat!(
                "../fixtures/cross-leaf.v1/one-shot",
                "/workspace/pale-workspace.toml"
            )),
        ),
        document(
            "workspace-resolution.lock.toml",
            include_bytes!(concat!(
                "../fixtures/cross-leaf.v1/one-shot",
                "/workspace/workspace-resolution.lock.toml"
            )),
        ),
        document(
            "workspace-universe.lock.toml",
            include_bytes!(concat!(
                "../fixtures/cross-leaf.v1/one-shot",
                "/workspace/workspace-universe.lock.toml"
            )),
        ),
    ]
}

fn portable_workspace_documents() -> Vec<(String, Vec<u8>)> {
    vec![
        (
            "composition.toml".to_owned(),
            include_bytes!(concat!(
                "../fixtures/cross-leaf.v1/one-shot",
                "/workspace/composition.toml"
            ))
            .to_vec(),
        ),
        (
            "pale-workspace.toml".to_owned(),
            include_bytes!(concat!(
                "../fixtures/cross-leaf.v1/one-shot",
                "/workspace/pale-workspace.toml"
            ))
            .to_vec(),
        ),
        (
            "workspace-resolution.lock.toml".to_owned(),
            include_bytes!(concat!(
                "../fixtures/cross-leaf.v1/one-shot",
                "/workspace/workspace-resolution.lock.toml"
            ))
            .to_vec(),
        ),
        (
            "workspace-universe.lock.toml".to_owned(),
            include_bytes!(concat!(
                "../fixtures/cross-leaf.v1/one-shot",
                "/workspace/workspace-universe.lock.toml"
            ))
            .to_vec(),
        ),
    ]
}

fn callable_capability_witnesses() -> Vec<ApiCapabilityId> {
    macro_rules! witness {
        ($id:expr, $call:path) => {{
            let _callable = $call;
            $id
        }};
    }
    vec![
        witness!(
            ApiCapabilityId::ContractBundleIdentity,
            PalePython::contract_bundle_identity
        ),
        witness!(
            ApiCapabilityId::ContractSchemaLookup,
            PalePython::contract_schema
        ),
        witness!(
            ApiCapabilityId::ContractDocumentValidation,
            PalePython::validate_contract_document
        ),
        witness!(
            ApiCapabilityId::DiagnosticProjection,
            PalePython::project_diagnostic
        ),
        witness!(
            ApiCapabilityId::DiagnosticAdmission,
            PalePython::admit_diagnostic
        ),
        witness!(
            ApiCapabilityId::PackageDistributionInspection,
            PalePython::inspect_package_distribution
        ),
        witness!(
            ApiCapabilityId::AuthoringIngressLimits,
            PalePython::authoring_ingress_limits
        ),
        witness!(
            ApiCapabilityId::StandaloneAuthoringValidation,
            PalePython::validate_authoring_document
        ),
        witness!(
            ApiCapabilityId::StandaloneAuthoringInspection,
            PalePython::inspect_standalone_authoring
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceCompilation,
            PalePython::compile_portable_workspace
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceAuthoringInspection,
            crate::PyPortableWorkspaceCompilation::inspect_authoring
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceAuthoringNavigation,
            crate::PyPortableWorkspaceCompilation::navigate_authoring
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceAuthoringOccurrences,
            crate::PyPortableWorkspaceCompilation::authoring_occurrences
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceComparison,
            PalePython::compare_portable_workspaces
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceSubjectComparison,
            crate::PyWorkspaceComparison::compare_subject
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceComparisonSubjects,
            crate::PyWorkspaceComparison::subjects
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceDirectDependencies,
            crate::PyPortableWorkspaceCompilation::direct_dependencies
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceChangeRoots,
            crate::PyWorkspaceComparison::change_roots
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceImpacts,
            crate::PyWorkspaceComparison::impacts
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceExplanation,
            crate::PyWorkspaceComparison::explain
        ),
        witness!(
            ApiCapabilityId::EffectiveProfileCompilation,
            PalePython::compile_effective_profile
        ),
        witness!(
            ApiCapabilityId::EffectiveProfileInspection,
            crate::PyEffectiveProfileCompilation::inspect
        ),
        witness!(
            ApiCapabilityId::EffectiveProfileExplanation,
            crate::PyEffectiveProfileCompilation::explain
        ),
        witness!(
            ApiCapabilityId::EffectiveProfileAuthoringInspection,
            crate::PyEffectiveProfileCompilation::inspect_authoring
        ),
        witness!(
            ApiCapabilityId::EffectiveProfileAuthoringNavigation,
            crate::PyEffectiveProfileCompilation::navigate_authoring
        ),
        witness!(
            ApiCapabilityId::EffectiveProfileAuthoringOccurrences,
            crate::PyEffectiveProfileCompilation::authoring_occurrences
        ),
        witness!(
            ApiCapabilityId::EffectiveProfileAuthoringJoin,
            crate::PyEffectiveProfileCompilation::inspect_effective_authoring
        ),
        witness!(
            ApiCapabilityId::EffectiveProfileAuthoringPages,
            crate::PyEffectiveProfileCompilation::inspect_effective_authoring_page
        ),
        witness!(
            ApiCapabilityId::ConfigurationImprovement,
            PalePython::inspect_configuration_improvement
        ),
        witness!(
            ApiCapabilityId::ObservationSnapshot,
            PalePython::observe_effective_profile
        ),
        witness!(
            ApiCapabilityId::ObservationPage,
            crate::PyObservationSnapshot::page
        ),
        witness!(
            ApiCapabilityId::ObservationProfileInspection,
            crate::PyObservationSnapshot::inspect_profile
        ),
        witness!(
            ApiCapabilityId::ObservationDelta,
            crate::PyObservationSnapshot::delta_from
        ),
        witness!(
            ApiCapabilityId::ObservationCausalPath,
            crate::PyObservationSnapshot::causal_path
        ),
        witness!(
            ApiCapabilityId::ObservationVisualLens,
            crate::PyObservationSnapshot::visual_lens
        ),
        witness!(
            ApiCapabilityId::NativeActionCatalog,
            PalePython::native_action_catalog
        ),
        witness!(
            ApiCapabilityId::NativeActionInvocation,
            PalePython::invoke_native_action
        ),
        witness!(
            ApiCapabilityId::NativeActionPayloadInvocation,
            PalePython::invoke_native_action_payload
        ),
        witness!(
            ApiCapabilityId::RepositoryNativeActionAdmission,
            PalePython::admit_repository_native_actions
        ),
        witness!(
            ApiCapabilityId::ResidentHookAdmission,
            PalePython::admit_resident_hook
        ),
        witness!(
            ApiCapabilityId::OneShotCompilation,
            PalePython::compile_one_shot
        ),
        witness!(
            ApiCapabilityId::OneShotCompilationSummary,
            PalePython::one_shot_summary
        ),
        witness!(
            ApiCapabilityId::OneShotObservationTargets,
            PalePython::one_shot_observation_targets
        ),
        witness!(ApiCapabilityId::OneShotPlanning, PalePython::plan_one_shot),
        witness!(
            ApiCapabilityId::OneShotMachineEffectBinding,
            PalePython::bind_one_shot_machine_effects
        ),
        witness!(
            ApiCapabilityId::OneShotSavedPlanAdmission,
            PalePython::admit_one_shot_saved_plan
        ),
        witness!(
            ApiCapabilityId::LocalOneShotCompilation,
            PalePython::compile_local_one_shot
        ),
        witness!(
            ApiCapabilityId::LocalPackageDistributionInspection,
            PalePython::inspect_local_package_distribution
        ),
        witness!(
            ApiCapabilityId::LocalNativeActionInputCapture,
            PalePython::capture_local_native_action_request
        ),
        witness!(
            ApiCapabilityId::LocalRepositoryStatus,
            PalePython::inspect_local_repository_status
        ),
        witness!(
            ApiCapabilityId::LocalOneShotPlanning,
            PalePython::plan_local_one_shot
        ),
        witness!(
            ApiCapabilityId::LocalOneShotPlanPublication,
            PalePython::publish_local_one_shot_plan
        ),
        witness!(
            ApiCapabilityId::LocalMaterializationRunAdmission,
            PalePython::admit_local_materialization_run
        ),
        witness!(
            ApiCapabilityId::LocalMaterializationRunSettlement,
            PalePython::settle_local_materialization_run
        ),
        witness!(
            ApiCapabilityId::LocalMaterializationReconciliationInspection,
            PalePython::inspect_local_materialization_reconciliation
        ),
        witness!(
            ApiCapabilityId::CompiledInterfaceIdentity,
            PalePython::compiled_interface_identity
        ),
        witness!(
            ApiCapabilityId::CurrentExecutableBindingVerification,
            PalePython::verify_current_executable_binding
        ),
        witness!(
            ApiCapabilityId::LocalControlStorageInspection,
            PalePython::inspect_local_control_store
        ),
        witness!(
            ApiCapabilityId::LocalConfigurationImprovementPreview,
            PalePython::preview_local_configuration_improvement
        ),
    ]
}

/// The Python projection must track, neither widen nor narrow, API eligibility.
#[test]
fn exported_capabilities_equal_the_api_owned_python_set() {
    let expected = capability_inventory()
        .iter()
        .filter(|capability| capability.eligible_for(ApiLeaf::Python))
        .map(|capability| capability.id())
        .collect::<Vec<ApiCapabilityId>>();

    let mut callable_names = callable_capability_witnesses()
        .iter()
        .map(|capability| capability.as_str())
        .collect::<Vec<_>>();
    let mut expected_names = expected
        .iter()
        .map(|capability| capability.as_str())
        .collect::<Vec<_>>();
    callable_names.sort_unstable();
    expected_names.sort_unstable();
    assert_eq!(callable_names, expected_names);
    assert_eq!(
        crate::eligible_capability_ids(),
        expected
            .iter()
            .map(|capability| capability.as_str())
            .collect::<Vec<_>>()
    );
}

/// Every value retained while the interpreter is detached is thread-safe.
#[test]
fn python_handle_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<PalePython>();
}

#[test]
fn bundle_identity_matches_the_shared_api_semantics() {
    pyo3::Python::initialize();
    pyo3::Python::attach(|py| {
        let api = pale_sdk::PaleApi::new().expect("API initializes");
        let python = PalePython::new(py).expect("Python leaf initializes");
        let expected = api.contract_bundle_identity();
        let actual = python.contract_bundle_identity();
        assert_eq!(
            actual.schema_manifest_sha256(),
            expected.schema_manifest_sha256()
        );
        assert_eq!(actual.schema_set_sha256(), expected.schema_set_sha256());
        assert_eq!(actual.member_count(), expected.member_count());
    });
}

#[test]
fn authoring_projection_reuses_the_retained_api_compilation() {
    pyo3::Python::initialize();
    pyo3::Python::attach(|py| {
        let python = PalePython::new(py).expect("Python leaf initializes");
        let bundle = python.contract_bundle_identity();
        let manifest = include_bytes!(concat!(
            "../fixtures/cross-leaf.v1/one-shot",
            "/workspace/pale-workspace.toml"
        ));

        let _limits = python.authoring_ingress_limits(crate::PyAuthoringIngressClass::Workspace);
        python
            .validate_authoring_document(
                py,
                crate::PyStandaloneAuthoringRole::WorkspaceManifest,
                "pale-workspace.toml".to_owned(),
                manifest.to_vec(),
            )
            .expect("standalone manifest validates through pale-sdk");
        let standalone = python
            .inspect_standalone_authoring(
                py,
                crate::PyStandaloneAuthoringRole::WorkspaceManifest,
                vec![crate::PyAuthoringPathSegment::field("workspace".to_owned())],
            )
            .expect("generated standalone grammar is coherent");
        assert!(standalone.state() == crate::PyAuthoringInspectionState::Available);

        let workspace = python
            .compile_portable_workspace(
                py,
                "pale-workspace.toml".to_owned(),
                portable_workspace_documents(),
            )
            .expect("workspace fixture compiles");
        let portable_inspection = workspace
            .inspect_authoring(
                py,
                "pale-workspace.toml".to_owned(),
                vec![crate::PyAuthoringPathSegment::field("workspace".to_owned())],
            )
            .expect("retained workspace grammar is coherent");
        assert!(portable_inspection.state() == crate::PyAuthoringInspectionState::Available);
        let portable_navigation = workspace.navigate_authoring(
            py,
            "pale-workspace.toml".to_owned(),
            vec![
                crate::PyAuthoringInstancePathSegment::field("workspace".to_owned()),
                crate::PyAuthoringInstancePathSegment::field("workspace_id".to_owned()),
            ],
        );
        assert_eq!(
            portable_navigation
                .identity()
                .expect("workspace identity resolves")
                .value(),
            "act-ws"
        );
        let profile = python
            .compile_effective_profile(
                py,
                &workspace,
                crate::PyEffectiveProfileCompilationRequest::new(
                    "demo".to_owned(),
                    "default".to_owned(),
                    "machine".to_owned(),
                    private_inputs(&bundle),
                    package_documents(),
                ),
            )
            .unwrap_or_else(|error| {
                let diagnostic = error
                    .value(py)
                    .getattr("diagnostic")
                    .and_then(|value| value.extract::<Vec<u8>>())
                    .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
                panic!("profile fixture compiles: {diagnostic:?}")
            });
        let effective_inspection = profile
            .inspect_authoring(
                py,
                "pale-workspace.toml".to_owned(),
                vec![crate::PyAuthoringPathSegment::field("workspace".to_owned())],
            )
            .expect("retained profile grammar is coherent");
        assert!(effective_inspection.state() == crate::PyAuthoringInspectionState::Available);
        let path = vec![
            crate::PyAuthoringInstancePathSegment::field("workspace".to_owned()),
            crate::PyAuthoringInstancePathSegment::field("desired_topology".to_owned()),
            crate::PyAuthoringInstancePathSegment::field("projects".to_owned()),
            crate::PyAuthoringInstancePathSegment::index(0),
            crate::PyAuthoringInstancePathSegment::field("run_policy".to_owned()),
        ];
        let navigation = profile.navigate_authoring(py, "pale-workspace.toml".to_owned(), path);
        let identity = navigation.identity().expect("run policy resolves");
        assert_eq!(identity.value(), "standard");
        let inspection = profile.inspect_effective_authoring(py, &identity);
        let _coordinates = inspection.coordinates();
        assert!(!inspection.entries().is_empty());
        assert!(!inspection.truncated());
        let explanation = profile
            .explain(py, inspection.entries()[0].explanation_handle().to_owned())
            .expect("handle is valid")
            .expect("handle belongs to the retained profile");
        assert_eq!(
            explanation.handle(),
            inspection.entries()[0].explanation_handle()
        );
    });
}

#[test]
fn one_shot_admission_preserves_linear_phase_consumption() {
    pyo3::Python::initialize();
    pyo3::Python::attach(|py| {
        let python = PalePython::new(py).expect("Python leaf initializes");
        let bundle = python.contract_bundle_identity();
        let compilation = python
            .compile_one_shot(
                py,
                crate::PyOneShotCompilationRequest::new(
                    "demo".to_owned(),
                    "default".to_owned(),
                    "machine".to_owned(),
                    private_inputs(&bundle),
                    package_documents(),
                    crate::PyOneShotWorkspaceInputs::new(
                        "pale-workspace.toml".to_owned(),
                        workspace_documents(),
                    ),
                ),
            )
            .unwrap_or_else(|error| {
                let diagnostic = error
                    .value(py)
                    .getattr("diagnostic")
                    .and_then(|value| value.extract::<Vec<u8>>())
                    .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
                panic!("one-shot fixture compiles: {diagnostic:?}")
            });
        let observation = crate::PyOneShotObservationRequest::new(
            "engine".to_owned(),
            crate::PyOneShotPlanKind::Apply,
            crate::PyOneShotPriorMaterialization::unowned(),
        );
        let targets = python
            .one_shot_observation_targets(py, &compilation, observation)
            .expect("observation targets are derived");
        let planning = crate::PyOneShotPlanningRequest::new(
            crate::PyOneShotObservationRequest::new(
                "engine".to_owned(),
                crate::PyOneShotPlanKind::Apply,
                crate::PyOneShotPriorMaterialization::unowned(),
            ),
            targets
                .into_iter()
                .map(crate::PyBaselineEntry::absent)
                .collect(),
        );
        let candidate = python
            .plan_one_shot(py, &compilation, planning)
            .expect("candidate is planned");
        let bound = python
            .bind_one_shot_machine_effects(py, &candidate, Vec::new())
            .expect("empty machine-local set is bound");
        assert!(python
            .bind_one_shot_machine_effects(py, &candidate, Vec::new())
            .is_err());
        let saved = bound.canonical_bytes(py).bind(py).as_bytes().to_vec();
        let admitted = python
            .admit_one_shot_saved_plan(py, &bound, saved)
            .expect("exact saved bytes are admitted");
        assert_eq!(admitted.repository_id(), "engine");
        assert!(python
            .admit_one_shot_saved_plan(py, &bound, Vec::new())
            .is_err());
    });
}
