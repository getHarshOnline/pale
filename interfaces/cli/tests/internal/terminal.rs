//! Required versus post-effect process-stream failure behavior.

use std::io;

use pale_sdk::PaleApi;

use super::{render_pretty_json, report_line, write_line};

struct BrokenPipe;

impl io::Write for BrokenPipe {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed reader"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed reader"))
    }
}

#[test]
fn required_output_preserves_a_broken_pipe() {
    let error = write_line(&mut BrokenPipe, format_args!("required output")).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
}

#[test]
fn post_effect_reporting_does_not_reclassify_success_on_a_broken_pipe() {
    report_line(&mut BrokenPipe, format_args!("effect already established"));
}

#[test]
fn human_json_projection_is_pretty_without_mutating_owner_canonical_bytes() {
    let identity = PaleApi::new()
        .unwrap()
        .compiled_interface_identity()
        .unwrap();
    let canonical = identity.canonical_bytes().to_vec();

    let rendered = render_pretty_json(&canonical, "compiled interface identity").unwrap();

    assert!(rendered.lines().count() > 2, "terminal JSON is multiline");
    assert!(
        rendered.lines().any(|line| line.starts_with("  \"")),
        "terminal JSON uses two-space indentation"
    );
    let rendered_value: serde_json::Value = serde_json::from_str(&rendered).unwrap();
    let canonical_value: serde_json::Value = serde_json::from_slice(&canonical).unwrap();
    assert_eq!(rendered_value, canonical_value);
    assert_eq!(identity.canonical_bytes(), canonical);
}
