//! WebAssembly binding eligibility and callable-witness conformance.

use super::eligible_capability_ids;
use pale_sdk::{capability_inventory, ApiCapabilityId, ApiLeaf};

#[path = "authoring_errors.rs"]
mod authoring_errors;

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
            super::PaleWasm::contract_bundle_identity
        ),
        witness!(
            ApiCapabilityId::ContractSchemaLookup,
            super::PaleWasm::contract_schema
        ),
        witness!(
            ApiCapabilityId::ContractDocumentValidation,
            super::PaleWasm::validate_contract_document
        ),
        witness!(
            ApiCapabilityId::DiagnosticProjection,
            super::PaleWasm::project_diagnostic
        ),
        witness!(
            ApiCapabilityId::DiagnosticAdmission,
            super::PaleWasm::admit_diagnostic
        ),
        witness!(
            ApiCapabilityId::AuthoringIngressLimits,
            super::PaleWasm::authoring_ingress_limits
        ),
        witness!(
            ApiCapabilityId::StandaloneAuthoringValidation,
            super::PaleWasm::validate_authoring_document
        ),
        witness!(
            ApiCapabilityId::StandaloneAuthoringInspection,
            super::PaleWasm::inspect_standalone_authoring
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceCompilation,
            super::PaleWasm::compile_portable_workspace
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceAuthoringInspection,
            super::WasmPortableWorkspaceCompilation::inspect_authoring
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceAuthoringNavigation,
            super::WasmPortableWorkspaceCompilation::navigate_authoring
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceAuthoringOccurrences,
            super::WasmPortableWorkspaceCompilation::authoring_occurrences
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceComparison,
            super::PaleWasm::compare_portable_workspaces
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceSubjectComparison,
            super::WasmPortableWorkspaceComparison::compare_subject
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceChangeRoots,
            super::WasmPortableWorkspaceComparison::change_roots
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceImpacts,
            super::WasmPortableWorkspaceComparison::impacts
        ),
        witness!(
            ApiCapabilityId::PortableWorkspaceExplanation,
            super::WasmPortableWorkspaceComparison::explain
        ),
        witness!(
            ApiCapabilityId::NativeActionCatalog,
            super::PaleWasm::native_action_catalog
        ),
        witness!(
            ApiCapabilityId::NativeActionInvocation,
            super::PaleWasm::invoke_native_action
        ),
        witness!(
            ApiCapabilityId::NativeActionPayloadInvocation,
            super::PaleWasm::invoke_native_action_payload
        ),
    ]
}

#[test]
fn exported_capabilities_equal_the_api_owned_wasm_allowlist() {
    let expected = capability_inventory()
        .iter()
        .filter(|capability| capability.eligible_for(ApiLeaf::Wasm))
        .map(|capability| capability.id())
        .collect::<Vec<_>>();

    let mut witnessed_ids = callable_capability_witnesses()
        .iter()
        .map(|capability| capability.as_str())
        .collect::<Vec<_>>();
    let mut expected_ids = expected
        .iter()
        .map(|capability| capability.as_str())
        .collect::<Vec<_>>();
    witnessed_ids.sort_unstable();
    expected_ids.sort_unstable();
    assert_eq!(witnessed_ids, expected_ids);
    assert_eq!(
        eligible_capability_ids(),
        expected
            .iter()
            .map(|capability| capability.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn bundle_identity_matches_the_shared_api_semantics() {
    let api = pale_sdk::PaleApi::new().expect("API initializes");
    let wasm = super::PaleWasm::new().expect("WASM leaf initializes");
    let expected = api.contract_bundle_identity();
    let actual = wasm.contract_bundle_identity();
    assert_eq!(
        actual.schema_manifest_sha256,
        expected.schema_manifest_sha256()
    );
    assert_eq!(actual.schema_set_sha256, expected.schema_set_sha256());
    assert_eq!(actual.member_count, expected.member_count());
}

#[test]
fn workspace_fact_getters_preserve_every_payload_and_absent_field() {
    use super::{PortableWorkspaceFact as Fact, WasmWorkspaceFact, WasmWorkspaceFactKind as Kind};

    let text = || "selected-output".to_owned();
    let cases = [
        (
            Fact::Component {
                component_digest: text(),
            },
            Kind::Component,
            2,
        ),
        (
            Fact::Module {
                module_digest: text(),
            },
            Kind::Module,
            3,
        ),
        (
            Fact::Declaration {
                value_digest: text(),
            },
            Kind::Declaration,
            4,
        ),
        (
            Fact::ImportSurface {
                surface_digest: text(),
            },
            Kind::ImportSurface,
            5,
        ),
        (
            Fact::Extension {
                value_digest: text(),
            },
            Kind::Extension,
            4,
        ),
        (
            Fact::EffectivePolicy {
                value_digest: text(),
            },
            Kind::EffectivePolicy,
            4,
        ),
        (
            Fact::ResponsibilityClosure {
                closure_digest: text(),
            },
            Kind::ResponsibilityClosure,
            6,
        ),
        (
            Fact::OrganizationalTranslationClosure {
                closure_digest: text(),
            },
            Kind::OrganizationalTranslationClosure,
            6,
        ),
        (
            Fact::ExperienceComposition {
                source_digest: text(),
            },
            Kind::ExperienceComposition,
            1,
        ),
        (
            Fact::Resource {
                value_digest: text(),
            },
            Kind::Resource,
            4,
        ),
        (
            Fact::ResolutionLock {
                resolution_digest: text(),
            },
            Kind::ResolutionLock,
            7,
        ),
        (
            Fact::UniverseLock {
                universe_digest: text(),
            },
            Kind::UniverseLock,
            8,
        ),
        (
            Fact::SemanticIr {
                workspace_semantic_ir_digest: text(),
            },
            Kind::SemanticIr,
            9,
        ),
    ];
    fn text_fields(value: &WasmWorkspaceFact) -> [Option<String>; 10] {
        [
            value.source_path(),
            value.source_digest(),
            value.component_digest(),
            value.module_digest(),
            value.value_digest(),
            value.surface_digest(),
            value.closure_digest(),
            value.resolution_digest(),
            value.universe_digest(),
            value.workspace_semantic_ir_digest(),
        ]
    }
    for (inner, kind, selected_field) in cases {
        let value = WasmWorkspaceFact { inner };
        assert_eq!(value.kind() as u32, kind as u32);
        assert_eq!(value.byte_len(), None);
        for (index, field) in text_fields(&value).into_iter().enumerate() {
            assert_eq!(field, (index == selected_field).then(text));
        }
    }
    let source = WasmWorkspaceFact {
        inner: Fact::SourceContent {
            source_path: "source.toml".to_owned(),
            source_digest: text(),
            byte_len: 17,
        },
    };
    assert_eq!(source.kind() as u32, Kind::SourceContent as u32);
    assert_eq!(source.byte_len(), Some(17));
    let fields = text_fields(&source);
    assert_eq!(fields[0].as_deref(), Some("source.toml"));
    assert_eq!(fields[1].as_deref(), Some("selected-output"));
    assert!(fields[2..].iter().all(Option::is_none));
}

#[test]
fn wasm_allowlist_contains_only_portable_effect_free_capabilities() {
    for capability in capability_inventory()
        .iter()
        .filter(|capability| capability.eligible_for(ApiLeaf::Wasm))
    {
        assert_eq!(capability.class(), pale_sdk::ApiCapabilityClass::Portable);
        assert_eq!(
            capability.effect(),
            pale_sdk::ApiCapabilityEffect::EffectFree
        );
    }
}

#[test]
fn standalone_authoring_projection_preserves_api_owned_limits_and_facts() {
    let wasm = super::PaleWasm::new().expect("WASM leaf initializes");
    let expected =
        pale_sdk::PaleApi::authoring_ingress_limits(pale_sdk::AuthoringIngressClass::Workspace);
    let actual = wasm.authoring_ingress_limits(super::WasmAuthoringIngressClass::Workspace);
    assert_eq!(actual.max_documents(), expected.max_documents() as u64);
    assert_eq!(actual.max_document_bytes(), expected.max_document_bytes());
    assert_eq!(actual.max_total_bytes(), expected.max_total_bytes());

    // Host tests exercise the native owner and existing result projection.
    // JavaScript path consumption is exercised by the actual ABI tests.
    let inspection = wasm
        .api
        .inspect_standalone_authoring(
            super::WasmStandaloneAuthoringRole::WorkspaceManifest.into(),
            vec![
                super::WasmAuthoringPathSegment::field("workspace".into()).inner,
                super::WasmAuthoringPathSegment::field("desired_topology".into()).inner,
            ],
        )
        .map(super::WasmAuthoringInspection::from_api)
        .expect("embedded authoring metadata is coherent");
    assert!(matches!(
        inspection.state(),
        super::WasmAuthoringInspectionState::Available
    ));
    assert_eq!(inspection.schema_id().as_deref(), Some("pale-workspace.v1"));
    assert!(inspection.node().is_some());
}

#[test]
fn derivation_page_numbers_require_exact_finite_integers_in_range() {
    for invalid in [-1.0, 0.5, f64::NAN, f64::INFINITY, 4_294_967_296.0] {
        assert_eq!(super::exact_javascript_u32(invalid, u32::MAX), None);
    }
    for invalid in [-1.0, 0.5, f64::NAN, f64::INFINITY, 65_536.0] {
        assert_eq!(
            super::exact_javascript_u32(invalid, u32::from(u16::MAX)),
            None
        );
    }
    assert_eq!(
        super::exact_javascript_u32(f64::from(u32::MAX), u32::MAX),
        Some(u32::MAX)
    );
    assert_eq!(
        super::exact_javascript_u32(f64::from(u16::MAX), u32::from(u16::MAX)),
        Some(u32::from(u16::MAX))
    );
}
