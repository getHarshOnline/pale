//! Exercises Unix executable admission and retained-identity revalidation.

use std::os::unix::fs::{symlink, PermissionsExt};

use super::*;

fn executable(directory: &tempfile::TempDir, name: &str) -> (PathBuf, Vec<u8>, ExecutableDigest) {
    let path = directory.path().join(name);
    let bytes = b"#!/bin/sh\nprintf identity\n".to_vec();
    fs::write(&path, &bytes).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let digest = ExecutableDigest::hash_bytes(&bytes);
    (path, bytes, digest)
}

#[test]
fn admission_rejects_every_invalid_executable_class() {
    let temporary = tempfile::tempdir().unwrap();
    let (path, _bytes, digest) = executable(&temporary, "probe");

    assert!(matches!(
        VerifiedExecutable::admit("relative-probe", &digest),
        Err(ExecutableError::NotAbsolute(_))
    ));
    let missing = temporary.path().join("missing");
    assert!(matches!(
        VerifiedExecutable::admit(missing.to_str().unwrap(), &digest),
        Err(ExecutableError::NotFound(_))
    ));

    let link = temporary.path().join("link");
    symlink(&path, &link).unwrap();
    assert!(matches!(
        VerifiedExecutable::admit(link.to_str().unwrap(), &digest),
        Err(ExecutableError::Symlink(_))
    ));

    let directory = temporary.path().join("directory");
    fs::create_dir(&directory).unwrap();
    assert!(matches!(
        VerifiedExecutable::admit(directory.to_str().unwrap(), &digest),
        Err(ExecutableError::NotRegular(_))
    ));

    let non_executable = temporary.path().join("non-executable");
    fs::write(&non_executable, b"bytes").unwrap();
    fs::set_permissions(&non_executable, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(
        VerifiedExecutable::admit(non_executable.to_str().unwrap(), &digest),
        Err(ExecutableError::NotExecutable(_))
    ));

    let wrong = ExecutableDigest::hash_bytes(b"wrong");
    assert!(matches!(
        VerifiedExecutable::admit(path.to_str().unwrap(), &wrong),
        Err(ExecutableError::DigestMismatch { .. })
    ));
}

#[test]
fn retained_and_canonical_identity_reject_path_replacement() {
    let temporary = tempfile::tempdir().unwrap();
    let (path, bytes, digest) = executable(&temporary, "probe");
    let displaced = temporary.path().join("displaced");
    let mut admitted = VerifiedExecutable::admit(path.to_str().unwrap(), &digest).unwrap();

    fs::rename(&path, &displaced).unwrap();
    fs::write(&path, &bytes).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();

    assert!(matches!(
        admitted.revalidate(),
        Err(ExecutableError::IdentityChanged(_))
    ));
    assert!(matches!(
        admitted.canonical_path(),
        Err(ExecutableError::IdentityChanged(_))
    ));
}

#[test]
fn admitted_executable_returns_its_verified_canonical_path() {
    let temporary = tempfile::tempdir().unwrap();
    let (path, _bytes, digest) = executable(&temporary, "probe");
    let mut admitted = VerifiedExecutable::admit(path.to_str().unwrap(), &digest).unwrap();
    assert_eq!(
        admitted.canonical_path().unwrap(),
        fs::canonicalize(path).unwrap()
    );
}
