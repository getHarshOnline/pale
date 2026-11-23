//! Verifies that executable admission fails explicitly on unsupported platforms.

use super::*;

#[test]
fn admission_reports_the_unsupported_platform() {
    let executable = std::env::current_exe().unwrap();
    let digest = ExecutableDigest::hash_bytes(b"unused");
    assert!(matches!(
        VerifiedExecutable::admit(executable.to_str().unwrap(), &digest),
        Err(ExecutableError::UnsupportedPlatform { .. })
    ));
}
