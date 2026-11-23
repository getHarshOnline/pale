//! Bounded public diagnostics for rejected authored Package navigation.

use super::*;
use pale_application::packages::MarkdownLinkViolation;
use pale_core::records::{ArtifactDigest, PackageIdV1, PackageResourceIdV1, PackageResourceRefV1};

#[test]
fn rejected_page_header_keeps_its_bounded_reason_and_source_location() {
    use pale_application::packages::PageHeaderError;
    let api = PaleApi::new().unwrap();
    for reason in [
        PageHeaderError::MissingOpening,
        PageHeaderError::MissingClosing,
        PageHeaderError::TooLarge,
        PageHeaderError::EmptyBody,
    ] {
        let failure = package_failure(
            &api.diagnostic_identity,
            PackageError::PageHeader {
                path: "pages/review.md".into(),
                reason,
            },
        );
        let diagnostic = failure.diagnostic();
        assert_eq!(
            diagnostic.code(),
            "package_distribution_source_syntax_rejected"
        );
        assert_eq!(diagnostic.stage(), crate::DiagnosticStage::InputAdmission);
        assert_eq!(
            diagnostic.disposition(),
            crate::DiagnosticDisposition::InvalidInput
        );
        assert_eq!(diagnostic.source().unwrap().label(), "pages/review.md");
        assert!(diagnostic
            .facts()
            .iter()
            .any(|fact| fact.key() == "page_header" && fact.value() == reason.to_string()));
    }
}

#[test]
fn rejected_markdown_destination_projects_only_its_bounded_identity() {
    let api = PaleApi::new().unwrap();
    let rejected = "../private?credential=do-not-disclose";
    let digest = ArtifactDigest::hash_bytes(rejected.as_bytes());
    let failure = package_failure(
        &api.diagnostic_identity,
        PackageError::InvalidMarkdownLink {
            from: PackageResourceRefV1 {
                package_id: PackageIdV1::parse("example.engine").unwrap(),
                resource_id: PackageResourceIdV1::parse("engine").unwrap(),
            },
            destination_sha256: digest.clone(),
            reason: MarkdownLinkViolation::UnconfinedDestination,
        },
    );
    let diagnostic = failure.diagnostic();
    assert_eq!(diagnostic.code(), "package_distribution_semantics_rejected");
    assert_eq!(
        diagnostic.stage(),
        crate::DiagnosticStage::SemanticAdmission
    );
    assert_eq!(
        diagnostic.disposition(),
        crate::DiagnosticDisposition::InvalidInput
    );
    let facts = diagnostic.facts();
    assert!(facts
        .iter()
        .any(|fact| fact.key() == "destination_sha256" && fact.value() == digest.as_str()));
    assert!(!format!("{failure:?}").contains("do-not-disclose"));
    assert!(diagnostic.source().is_none());
}
