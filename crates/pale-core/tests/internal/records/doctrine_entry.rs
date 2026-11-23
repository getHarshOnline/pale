//! Checks the bounded HTTPS grammar for official doctrine-source identifiers.

use super::OfficialSourceUrl;

#[test]
fn official_source_url_rejects_non_https_empty_and_whitespace_forms() {
    assert!(OfficialSourceUrl::parse("https://doc.rust-lang.org/").is_ok());
    for invalid in [
        "http://doc.rust-lang.org/",
        "https://",
        "https://doc.rust-lang.org/a b",
    ] {
        assert!(
            OfficialSourceUrl::parse(invalid).is_err(),
            "unexpected accepted official-source URL: {invalid}"
        );
    }
}
