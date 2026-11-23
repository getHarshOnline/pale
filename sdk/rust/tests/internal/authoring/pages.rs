//! Page framing, generated grammar and source rejection through the public API.

use crate::{
    AuthoringInspectionState, AuthoringPathSegment, DiagnosticDisposition, PaleApi,
    StandaloneAuthoringRole,
};

#[test]
fn page_role_validates_only_the_header_and_requires_a_real_body() {
    let api = PaleApi::new().unwrap();
    let page = "+++\r\ndescription = 'Review λ.'\r\n[guide]\r\n+++\r\n# Review\r\nNot TOML: [unfinished\r\n";
    let role = StandaloneAuthoringRole::PackagePage;
    api.validate_authoring_document(role, "review.md".into(), page.as_bytes().to_vec())
        .unwrap();
    assert_eq!(
        &page[role.declaration_range(page).unwrap()],
        "description = 'Review λ.'\r\n[guide]\r\n"
    );
    let help = api
        .inspect_standalone_authoring(
            role,
            vec![AuthoringPathSegment::Field("description".into())],
        )
        .unwrap();
    assert_eq!(help.state(), AuthoringInspectionState::Available);
    for source in [
        "+++\ndescription = 'Review.'\n",
        "+++\ndescription = 'Review.'\n+++\n",
        "# Plain Markdown\n",
    ] {
        let error = api
            .validate_authoring_document(role, "review.md".into(), source.as_bytes().to_vec())
            .unwrap_err();
        assert_eq!(
            error.diagnostic().disposition(),
            DiagnosticDisposition::InvalidInput
        );
    }
}

#[test]
fn data_role_uses_the_typed_package_data_grammar() {
    let api = PaleApi::new().unwrap();
    let role = StandaloneAuthoringRole::PackageData;
    let data = "kind = 'capability_policy'\n[policy]\npermitted_capabilities = ['repository.read']\npermitted_effects = ['read']\n";
    api.validate_authoring_document(role, "policy.toml".into(), data.as_bytes().to_vec())
        .unwrap();
    assert_eq!(role.declaration_range(data), Some(0..data.len()));
    assert_eq!(
        api.inspect_standalone_authoring(role, vec![AuthoringPathSegment::Field("kind".into())])
            .unwrap()
            .state(),
        AuthoringInspectionState::Available
    );
    assert!(api
        .validate_authoring_document(role, "policy.toml".into(), b"kind = 'unknown'".to_vec())
        .is_err());
}
