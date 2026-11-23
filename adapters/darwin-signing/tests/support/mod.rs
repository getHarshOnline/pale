//! Native compiler and owned artifact mechanics shared by the two XPC journeys.
use pale_core::records::ArtifactDigest;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) struct NativeFixture {
    pub(crate) owned: tempfile::TempDir,
    pub(crate) root: PathBuf,
    pub(crate) source: PathBuf,
    pub(crate) library: PathBuf,
}

impl NativeFixture {
    pub(crate) fn capture() -> Self {
        let selected =
            std::env::var_os("PALE_SIGNING_TEST_CORE").expect("select fixture static library");
        let expected =
            std::env::var("PALE_SIGNING_TEST_CORE_SHA256").expect("select its exact SHA-256");
        let bytes = fs::read(selected).unwrap();
        assert_eq!(ArtifactDigest::hash_bytes(&bytes).as_str(), expected);
        let owned = tempfile::tempdir().unwrap();
        let root = owned.path().canonicalize().unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let library = root.join("libfixture.a");
        fs::write(&library, bytes).unwrap();
        Self {
            owned,
            root,
            source: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
            library,
        }
    }
}

pub(crate) fn swift() -> Command {
    let mut command = Command::new("xcrun");
    command.args([
        "swiftc",
        "-swift-version",
        "5",
        "-warnings-as-errors",
        "-target",
        "arm64-apple-macos13.0",
    ]);
    command
}

pub(crate) fn sign(path: &Path, identifier: &str) -> String {
    let signed = Command::new("/usr/bin/codesign")
        .args(["--force", "--sign", "-", "--identifier", identifier])
        .arg(path)
        .output()
        .unwrap();
    assert!(
        signed.status.success(),
        "owned fixture signing: {}",
        String::from_utf8_lossy(&signed.stderr)
    );
    let inspected = Command::new("/usr/bin/codesign")
        .args(["-d", "--verbose=4"])
        .arg(path)
        .output()
        .unwrap();
    assert!(inspected.status.success());
    let text = String::from_utf8(inspected.stderr).unwrap();
    let cdhash = text
        .lines()
        .find_map(|line| line.strip_prefix("CDHash="))
        .unwrap();
    assert_eq!(cdhash.len(), 40);
    assert!(cdhash.bytes().all(|b| b.is_ascii_hexdigit()));
    format!("cdhash H\"{cdhash}\"")
}
