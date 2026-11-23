//! Shared portable input fixtures for public laws and inspection measurements.

use super::{PortableWorkspaceDocument, PortableWorkspaceRequest};

pub(super) const ROOT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/workspace-language/empty-adopter/pale-workspace.toml"
));
pub(super) const RESOLUTION_LOCK: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/workspace-language/empty-adopter/resolution.lock.toml"
));
pub(super) const UNIVERSE_LOCK: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/workspace-language/empty-adopter/universe.lock.toml"
));

pub(super) const POPULATED_ROOT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/workspace-language/populated/pale-workspace.toml"
));
pub(super) const POPULATED_MODULE: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/workspace-language/populated/module.toml"
));
pub(super) const POPULATED_ALPHA: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/workspace-language/populated/alpha.toml"
));
pub(super) const POPULATED_BETA: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/workspace-language/populated/beta.toml"
));
pub(super) const POPULATED_RESOLUTION_LOCK: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/workspace-language/populated/resolution.lock.toml"
));
pub(super) const POPULATED_UNIVERSE_LOCK: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/workspace-language/populated/universe.lock.toml"
));

pub(super) fn empty_documents() -> Vec<PortableWorkspaceDocument> {
    vec![
        PortableWorkspaceDocument::new("pale-workspace.toml".to_string(), ROOT.to_vec()),
        PortableWorkspaceDocument::new(
            "resolution.lock.toml".to_string(),
            RESOLUTION_LOCK.to_vec(),
        ),
        PortableWorkspaceDocument::new("universe.lock.toml".to_string(), UNIVERSE_LOCK.to_vec()),
    ]
}

pub(super) fn empty_request() -> PortableWorkspaceRequest {
    PortableWorkspaceRequest::new("pale-workspace.toml".to_string(), empty_documents())
}

/// Real wide compiler input: attributable omissions, not native-ready work.
pub(super) fn wide_dependency_request() -> PortableWorkspaceRequest {
    wide_dependency_request_with_comments("")
}

/// Change captured responsibility bytes without changing their parsed records.
pub(super) fn wide_dependency_request_with_comments(comment: &str) -> PortableWorkspaceRequest {
    use pale_core::records::*;
    fn attribution() -> ResponsibilityAttributionV1 {
        ResponsibilityAttributionV1 {
            source_id: WorkspaceDerivationSourceId::parse("source.page-test").unwrap(),
            owner: WorkspaceResourceId::parse("owner.page-test").unwrap(),
            evidence_ref: WorkspaceResourceId::parse("evidence.page-test").unwrap(),
        }
    }
    fn omitted<T>() -> AttributableAnswerV1<T> {
        AttributableAnswerV1::Omitted {
            reason: ResponsibilityOmissionReasonId::parse("not-selected").unwrap(),
            attribution: attribution(),
        }
    }
    let labels = (0..300)
        .map(|index| format!("responsibility-{index}.toml"))
        .collect::<Vec<_>>();
    let source_list =
        toml::Value::Array(labels.iter().cloned().map(toml::Value::String).collect()).to_string();
    let root = std::str::from_utf8(ROOT).unwrap().replacen(
        "responsibility_derivations = []",
        &format!("responsibility_derivations = {source_list}"),
        1,
    );
    let mut documents = vec![
        PortableWorkspaceDocument::new("pale-workspace.toml".to_owned(), root.into_bytes()),
        PortableWorkspaceDocument::new("resolution.lock.toml".to_owned(), RESOLUTION_LOCK.to_vec()),
        PortableWorkspaceDocument::new("universe.lock.toml".to_owned(), UNIVERSE_LOCK.to_vec()),
    ];
    for (index, label) in labels.into_iter().enumerate() {
        let record = ResponsibilityDerivationV1 {
            schema: RESPONSIBILITY_DERIVATION_V1_SCHEMA.to_owned(),
            schema_version: RESPONSIBILITY_DERIVATION_V1_SCHEMA_VERSION,
            derivation_id: ResponsibilityDerivationId::parse(format!("page-{index}")).unwrap(),
            lifecycle: ResponsibilityDerivationLifecycleV1::ConfigurationOpen,
            provenance: attribution(),
            responsibility: omitted(),
            native_owner: omitted(),
            trigger: omitted(),
            placement: omitted(),
            structural_objects: StructuralObjectKindV1::all()
                .iter()
                .map(|kind| ResponsibilityStructuralSelectionV1 {
                    kind: *kind,
                    selection: omitted(),
                })
                .collect(),
            request_framer: omitted(),
            provider_completion: omitted(),
            provider_acceptance: omitted(),
            consumer_acceptance: omitted(),
            native_result: omitted(),
            pale_settlement: omitted(),
            lawful_silence: omitted(),
            publisher_module: omitted(),
            adopter_module: omitted(),
            ceilings: omitted(),
            incumbent: omitted(),
            readiness: omitted(),
            safe_failure: omitted(),
            direct_path: omitted(),
            residence: vec![ResponsibilityResidenceV1 {
                class: ResponsibilityResidenceClassV1::Reference,
                resource: WorkspaceResourceId::parse("reference.page-test").unwrap(),
                owner: WorkspaceResourceId::parse("owner.page-test").unwrap(),
            }],
            native_projections: Vec::new(),
            genealogy_links: Vec::new(),
        };
        let mut bytes = toml::to_string(&record).unwrap().into_bytes();
        bytes.extend_from_slice(comment.as_bytes());
        documents.push(PortableWorkspaceDocument::new(label, bytes));
    }
    PortableWorkspaceRequest::new("pale-workspace.toml".to_owned(), documents)
}

pub(super) fn unadopted_documents(alpha_label: &str) -> Vec<PortableWorkspaceDocument> {
    use pale_core::paths::OwnedConfinedRelativePath;
    use pale_core::records::WorkspaceManifestFile;

    let mut manifest: WorkspaceManifestFile =
        toml::from_str(std::str::from_utf8(POPULATED_ROOT).unwrap()).unwrap();
    for path in &mut manifest.workspace.source_components {
        if path.as_str() == "alpha.toml" {
            *path = OwnedConfinedRelativePath::parse(alpha_label.to_owned()).unwrap();
        }
    }
    let root = toml::to_string(&manifest).unwrap();
    let universe = unadopted_universe_bytes();
    [
        ("pale-workspace.toml", root.as_bytes()),
        ("module.toml", POPULATED_MODULE),
        (alpha_label, POPULATED_ALPHA),
        ("beta.toml", POPULATED_BETA),
        ("resolution.lock.toml", POPULATED_RESOLUTION_LOCK),
        ("universe.lock.toml", universe.as_slice()),
    ]
    .into_iter()
    .map(|(label, bytes)| PortableWorkspaceDocument::new(label.to_owned(), bytes.to_vec()))
    .collect()
}

/// The same Core-sealed empty adoption used by the retained source fixture.
fn unadopted_universe_bytes() -> Vec<u8> {
    use pale_core::records::WorkspaceUniverseLockV1;

    let existing: WorkspaceUniverseLockV1 =
        toml::from_str(std::str::from_utf8(POPULATED_UNIVERSE_LOCK).unwrap()).unwrap();
    let universe = WorkspaceUniverseLockV1::seal(
        existing.workspace_id,
        existing.resolution_digest,
        Vec::new(),
    )
    .unwrap();
    toml::to_string(&universe).unwrap().into_bytes()
}

#[test]
fn unadopted_navigation_lock_matches_existing_sealing_owner() {
    use pale_core::records::WorkspaceUniverseLockV1;

    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/workspace-language/unadopted/universe.lock.toml"
    ))
    .unwrap();
    assert_eq!(bytes, unadopted_universe_bytes());
    let lock: WorkspaceUniverseLockV1 =
        toml::from_str(std::str::from_utf8(&bytes).unwrap()).unwrap();
    lock.verify().unwrap();
    assert!(lock.adopted_releases.is_empty());
    let published: WorkspaceUniverseLockV1 =
        toml::from_str(std::str::from_utf8(POPULATED_UNIVERSE_LOCK).unwrap()).unwrap();
    assert_eq!(lock.workspace_id, published.workspace_id);
    assert_eq!(lock.resolution_digest, published.resolution_digest);
    assert_ne!(lock.universe_digest, published.universe_digest);
}

/// Development generator only; normal tests independently verify its bytes.
/// Adopt the exported lock into `contracts/conformance/workspace-language.v1/unadopted`,
/// then run `tools/constellation.py --write` from the repository root to update projections.
#[test]
#[ignore = "development-only fixture export into a new explicit directory"]
fn export_unadopted_navigation_lock() {
    use std::io::Write as _;

    let output = std::path::PathBuf::from(
        std::env::var_os("PALE_UNADOPTED_FIXTURE_OUTPUT").expect("explicit output directory"),
    );
    assert!(output.is_absolute());
    std::fs::create_dir(&output).expect("output directory must not exist");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("universe.lock.toml"))
        .unwrap();
    file.write_all(&unadopted_universe_bytes()).unwrap();
    file.sync_all().unwrap();
}
