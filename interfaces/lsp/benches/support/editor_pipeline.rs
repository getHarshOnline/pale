//! Deterministic editor-text fixtures shared by the LSP benchmark cases.

use lsp_types::{Position, Range};
use sha2::{Digest, Sha256};

/// Canonical portable root manifest already used by LSP analysis tests.
const ROOT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cross-leaf.v1/portable/pale-workspace.toml"
));
/// Canonical portable resolution lock retained in every benchmark store.
pub(super) const RESOLUTION_LOCK: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cross-leaf.v1/portable/resolution.lock.toml"
));
/// Canonical portable universe lock retained in every benchmark store.
pub(super) const UNIVERSE_LOCK: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/cross-leaf.v1/portable/universe.lock.toml"
));

/// Small interactive buffer used by smoke and characterization runs.
pub(super) const SMALL_DOCUMENT_BYTES: usize = 4 * 1_024;
/// Large buffer paired with the small buffer in characterization runs.
pub(super) const LARGE_DOCUMENT_BYTES: usize = 512 * 1_024;
/// Number of rapidly superseded editor generations in the scheduling case.
pub(super) const DEBOUNCE_BURST: usize = 8;

/// One exact, schema-valid TOML document and its deterministic edit vectors.
#[derive(Clone, Debug)]
pub(super) struct EditorFixture {
    /// Exact initial open-buffer bytes.
    pub(super) initial: String,
    /// Same-length full replacement preserving the admitted TOML meaning.
    pub(super) full_replacement: String,
    /// One UTF-16 range replacing an ASCII byte inside a comment.
    pub(super) incremental_range: Range,
    /// Expected bytes after the incremental edit.
    pub(super) incremental_result: String,
}

impl EditorFixture {
    /// Build one exact-size document using bounded-width TOML comment lines.
    pub(super) fn new(target_bytes: usize) -> Self {
        assert!(
            target_bytes > ROOT.len(),
            "the benchmark size must retain the canonical root fixture"
        );
        let initial = padded_root(target_bytes, 'a');
        let full_replacement = padded_root(target_bytes, 'b');
        assert_eq!(initial.len(), target_bytes);
        assert_eq!(full_replacement.len(), target_bytes);

        let edit_byte = initial
            .as_bytes()
            .iter()
            .enumerate()
            .skip(target_bytes / 2)
            .find_map(|(index, byte)| (*byte == b'a').then_some(index))
            .expect("the bounded-width comment padding contains an editable byte");
        let mut incremental_result = initial.clone();
        incremental_result.replace_range(edit_byte..edit_byte + 1, "b");
        let prefix = &initial[..edit_byte];
        let comment_line = u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count())
            .expect("the bounded fixture line count fits the LSP coordinate");
        let line_start = prefix.rfind('\n').map_or(0, |newline| newline + 1);
        let comment_character = u32::try_from(edit_byte - line_start)
            .expect("the bounded fixture line width fits the LSP coordinate");
        let incremental_range = Range::new(
            Position::new(comment_line, comment_character),
            Position::new(comment_line, comment_character + 1),
        );

        Self {
            initial,
            full_replacement,
            incremental_range,
            incremental_result,
        }
    }
}

fn padded_root(target_bytes: usize, fill: char) -> String {
    const COMMENT_LINE_BYTES: usize = 80;
    const COMMENT_PREFIX: &str = "# ";

    let mut text = String::with_capacity(target_bytes);
    text.push_str(ROOT);
    while target_bytes - text.len() >= COMMENT_LINE_BYTES {
        text.push_str(COMMENT_PREFIX);
        text.extend(std::iter::repeat_n(
            fill,
            COMMENT_LINE_BYTES - COMMENT_PREFIX.len() - 1,
        ));
        text.push('\n');
    }
    let remainder = target_bytes - text.len();
    if remainder > 0 {
        text.push('#');
        text.extend(std::iter::repeat_n(fill, remainder - 1));
    }
    text
}

/// Exact buffer sizes selected without a Cartesian fixture expansion.
pub(super) fn selected_document_sizes(smoke: bool) -> &'static [usize] {
    if smoke {
        &[SMALL_DOCUMENT_BYTES]
    } else {
        &[SMALL_DOCUMENT_BYTES, LARGE_DOCUMENT_BYTES]
    }
}

/// Digest the exact selected bytes and case coordinates.
pub(super) fn corpus_sha256(smoke: bool) -> String {
    let mut digest = Sha256::new();
    digest.update(b"pale-lsp-editor-pipeline-v1\0");
    digest.update([u8::from(smoke)]);
    digest.update(RESOLUTION_LOCK.as_bytes());
    digest.update([0]);
    digest.update(UNIVERSE_LOCK.as_bytes());
    digest.update([0]);
    for &size in selected_document_sizes(smoke) {
        let fixture = EditorFixture::new(size);
        digest.update(
            u64::try_from(size)
                .expect("fixture size fits u64")
                .to_be_bytes(),
        );
        digest.update(fixture.initial.as_bytes());
        digest.update([0]);
        digest.update(fixture.full_replacement.as_bytes());
        digest.update([0]);
        digest.update(fixture.incremental_result.as_bytes());
        digest.update([0]);
    }
    digest.update(
        u64::try_from(DEBOUNCE_BURST)
            .expect("burst size fits u64")
            .to_be_bytes(),
    );
    encode_hex(digest.finalize())
}

fn encode_hex(bytes: impl AsRef<[u8]>) -> String {
    let mut output = String::with_capacity(bytes.as_ref().len() * 2);
    for byte in bytes.as_ref() {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to a String is infallible");
    }
    output
}
