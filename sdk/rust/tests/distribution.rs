//! Public-boundary laws for path-free Package distribution inspection.

use pale_sdk::{
    CompilationDocument, DiagnosticDisposition, DiagnosticStage, PackageDistributionRequest,
    PaleApi,
};

macro_rules! captured_fixture {
    ($member:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cross-leaf.v1/one-shot/",
            $member
        ))
    };
}

fn document(label: &str, bytes: &[u8]) -> CompilationDocument {
    CompilationDocument::new(label.to_string(), bytes.to_vec())
}

fn request(reverse: bool) -> PackageDistributionRequest {
    let mut documents = vec![
        document(
            "engine/package.toml",
            captured_fixture!("packages/engine/package.toml"),
        ),
        document(
            "engine/roles.toml",
            captured_fixture!("packages/engine/roles.toml"),
        ),
        document(
            "engine/role.md",
            captured_fixture!("packages/engine/role.md"),
        ),
    ];
    if reverse {
        documents.reverse();
    }
    PackageDistributionRequest::new(documents)
}

#[test]
fn inspection_is_order_independent_typed_and_canonical_without_a_lock() {
    let api = PaleApi::new().unwrap();
    let forward = api.inspect_package_distribution(request(false)).unwrap();
    let reverse = api.inspect_package_distribution(request(true)).unwrap();

    assert_eq!(forward, reverse);
    assert_eq!(forward.packages().len(), 1);
    assert_eq!(forward.packages()[0].id(), "example.engine");
    assert_eq!(forward.packages()[0].version(), "1.0.0");
    assert_eq!(forward.packages()[0].content_digest().len(), 64);
    assert_eq!(forward.schema_manifest_sha256().len(), 64);
    assert_eq!(forward.schema_set_sha256().len(), 64);

    let value = pale_core::canon::parse_strict(forward.canonical_bytes()).unwrap();
    assert_eq!(value["compiler"], forward.compiler());
    assert_eq!(value["packages"][0]["id"], forward.packages()[0].id());
    assert_eq!(
        pale_core::canon::to_canonical_string(&value)
            .unwrap()
            .as_bytes(),
        forward.canonical_bytes()
    );
}

#[test]
fn catalog_uses_captured_sources_and_preserves_the_authored_body() {
    let api = PaleApi::new().unwrap();
    let first = api.read_package_catalog(request(false)).unwrap();
    let reversed = api.read_package_catalog(request(true)).unwrap();
    assert_eq!(first.canonical_bytes(), reversed.canonical_bytes());
    let value: serde_json::Value = serde_json::from_slice(first.canonical_bytes()).unwrap();
    let resource = &value["resources"][0];
    let key = format!(
        "{}:{}",
        resource["package"].as_str().unwrap(),
        resource["id"].as_str().unwrap()
    );
    assert_eq!(
        first.show(&key).unwrap().as_bytes(),
        captured_fixture!("packages/engine/role.md")
    );
    assert!(first.show("example.engine:missing").is_none());
}

#[test]
fn duplicate_logical_documents_fail_at_the_shared_input_boundary() {
    let api = PaleApi::new().unwrap();
    let error = api
        .inspect_package_distribution(PackageDistributionRequest::new(vec![
            document(
                "engine/package.toml",
                captured_fixture!("packages/engine/package.toml"),
            ),
            document(
                "engine/package.toml",
                captured_fixture!("packages/engine/package.toml"),
            ),
        ]))
        .unwrap_err();

    assert_eq!(
        error.diagnostic().code(),
        "package_distribution_source_rejected"
    );
    assert_eq!(
        error.diagnostic().disposition(),
        DiagnosticDisposition::InvalidInput
    );
}

#[test]
fn package_syntax_failure_keeps_its_exact_stage_and_logical_source() {
    let api = PaleApi::new().unwrap();
    let error = api
        .inspect_package_distribution(PackageDistributionRequest::new(vec![document(
            "engine/package.toml",
            b"[package",
        )]))
        .unwrap_err();

    let diagnostic = error.diagnostic();
    assert_eq!(
        diagnostic.code(),
        "package_distribution_source_syntax_rejected"
    );
    assert_eq!(diagnostic.stage(), DiagnosticStage::InputAdmission);
    assert_eq!(
        diagnostic.source().map(|source| source.label().to_owned()),
        Some("engine/package.toml".to_owned())
    );
    assert_eq!(
        diagnostic.disposition(),
        DiagnosticDisposition::InvalidInput
    );
}
