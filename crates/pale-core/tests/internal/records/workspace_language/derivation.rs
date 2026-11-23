//! Exercises workspace derivation grammar, dependency evidence, and sealed closure validation.

use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn source(id: &str, path: &str, bytes: &[u8]) -> WorkspaceDerivationNodeV1 {
    let source_id = WorkspaceDerivationSourceId::parse(id).unwrap();
    WorkspaceDerivationNodeV1::seal(
        WorkspaceDerivationOriginV1::Source {
            source_id: source_id.clone(),
        },
        WorkspaceDerivationRuleV1::SourceAdmission,
        WorkspaceDerivationSubjectV1::Source {
            source_id: source_id.clone(),
        },
        WorkspaceDerivationOutputV1::SourceContent {
            source_path: OwnedConfinedRelativePath::parse(path).unwrap(),
            source_digest: WorkspaceSourceContentDigest::hash_bytes(bytes),
            byte_len: bytes.len() as u32,
        },
        Vec::new(),
    )
    .unwrap()
}

#[test]
fn component_and_lock_rules_reject_a_swapped_source() {
    let expected = source("expected", "expected.toml", b"expected");
    let swapped = source("swapped", "swapped.toml", b"swapped");
    let nodes = BTreeMap::from([
        (&expected.node_digest, &expected),
        (&swapped.node_digest, &swapped),
    ]);
    let component = WorkspaceDerivationNodeV1::seal(
        WorkspaceDerivationOriginV1::Source {
            source_id: WorkspaceDerivationSourceId::parse("expected").unwrap(),
        },
        WorkspaceDerivationRuleV1::ComponentAdmission,
        WorkspaceDerivationSubjectV1::Component {
            component_id: WorkspaceComponentId::parse("component").unwrap(),
        },
        WorkspaceDerivationOutputV1::Component {
            component_digest: WorkspaceComponentDigest::hash_canonical(&"component").unwrap(),
        },
        vec![swapped.node_digest.clone()],
    )
    .unwrap();
    assert!(matches!(
        validate_derivation_inputs(&component, &nodes),
        Err(InvalidWorkspaceDerivationReceipt::InputGrammar)
    ));

    let resolution = WorkspaceDerivationNodeV1::seal(
        WorkspaceDerivationOriginV1::Source {
            source_id: WorkspaceDerivationSourceId::parse("expected").unwrap(),
        },
        WorkspaceDerivationRuleV1::ResolutionLocking,
        WorkspaceDerivationSubjectV1::ResolutionLock,
        WorkspaceDerivationOutputV1::ResolutionLock {
            resolution_digest: WorkspaceResolutionLockDigest::hash_canonical(&"lock").unwrap(),
        },
        vec![swapped.node_digest.clone()],
    )
    .unwrap();
    assert!(matches!(
        validate_derivation_inputs(&resolution, &nodes),
        Err(InvalidWorkspaceDerivationReceipt::InputGrammar)
    ));

    let declaration = WorkspaceDerivationNodeV1::seal(
        WorkspaceDerivationOriginV1::ModuleRelease {
            module_id: WorkspaceModuleId::parse("module").unwrap(),
            release_id: WorkspaceReleaseId::parse("release").unwrap(),
            module_digest: WorkspaceModuleDigest::hash_canonical(&"module").unwrap(),
        },
        WorkspaceDerivationRuleV1::DeclarationLinking,
        WorkspaceDerivationSubjectV1::Declaration {
            owner_module: WorkspaceModuleId::parse("module").unwrap(),
            declaration: WorkspaceDeclarationRefV1 {
                namespace: WorkspaceNamespaceId::parse("namespace").unwrap(),
                declaration_id: WorkspaceDeclarationId::parse("declaration").unwrap(),
            },
        },
        WorkspaceDerivationOutputV1::Declaration {
            value_digest: WorkspaceDerivationValueDigest::hash_canonical(&"declaration").unwrap(),
        },
        vec![expected.node_digest.clone()],
    )
    .unwrap();
    assert!(matches!(
        validate_derivation_inputs(&declaration, &nodes),
        Err(InvalidWorkspaceDerivationReceipt::InputGrammar)
    ));
}

#[test]
fn retained_sources_and_admission_facts_are_an_exact_bijection() {
    let source = WorkspaceDerivationSourceId::parse("source").unwrap();
    let extra = WorkspaceDerivationSourceId::parse("extra").unwrap();
    assert!(validate_source_admission_bijection(
        &BTreeSet::from([source.clone()]),
        &BTreeSet::from([source.clone()]),
    )
    .is_ok());
    for admitted in [BTreeSet::new(), BTreeSet::from([source, extra])] {
        assert!(matches!(
            validate_source_admission_bijection(
                &BTreeSet::from([WorkspaceDerivationSourceId::parse("source").unwrap()]),
                &admitted,
            ),
            Err(InvalidWorkspaceDerivationReceipt::SourceAdmissionBijection)
        ));
    }
}

#[test]
fn root_aggregate_accepts_its_exact_ceiling_and_rejects_the_next() {
    assert!(bounded_derivation_len(
        "root",
        MAX_WORKSPACE_DERIVATION_ROOTS,
        MAX_WORKSPACE_DERIVATION_ROOTS,
    )
    .is_ok());
    assert!(matches!(
        bounded_derivation_len(
            "root",
            MAX_WORKSPACE_DERIVATION_ROOTS + 1,
            MAX_WORKSPACE_DERIVATION_ROOTS,
        ),
        Err(InvalidWorkspaceDerivationReceipt::Limit {
            kind: "root",
            observed,
            limit: MAX_WORKSPACE_DERIVATION_ROOTS,
        }) if observed == MAX_WORKSPACE_DERIVATION_ROOTS + 1
    ));
}
