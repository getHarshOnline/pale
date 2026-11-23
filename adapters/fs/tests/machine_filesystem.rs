//! Contract tests for local machine-filesystem observations.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use pale_adapter_fs::LocalMachineFilesystem;
use pale_ports::{MachineFilesystemOperation, MachineFilesystemPort, MachinePathKind};

#[test]
fn observes_regular_file_and_directory_without_policy_decisions() {
    let root = tempfile::tempdir().expect("temporary root");
    let file = root.path().join("record.txt");
    fs::write(&file, b"retained").expect("write fixture");
    let adapter = LocalMachineFilesystem;

    let directory = adapter.observe(root.path()).expect("observe directory");
    assert_eq!(directory.canonical_path(), canonical(root.path()));
    assert_eq!(directory.requested_kind(), MachinePathKind::Directory);
    assert_eq!(directory.canonical_kind(), MachinePathKind::Directory);

    let regular = adapter.observe(&file).expect("observe regular file");
    assert_eq!(regular.canonical_path(), canonical(&file));
    assert_eq!(regular.requested_kind(), MachinePathKind::RegularFile);
    assert_eq!(regular.canonical_kind(), MachinePathKind::RegularFile);

    #[cfg(unix)]
    {
        assert!(directory.unix_mode().is_some());
        assert!(regular.unix_mode().is_some());
    }
    #[cfg(not(unix))]
    {
        assert_eq!(directory.unix_mode(), None);
        assert_eq!(regular.unix_mode(), None);
    }
}

#[test]
fn missing_path_reports_requested_inspection_stage() {
    let root = tempfile::tempdir().expect("temporary root");
    let missing = root.path().join("missing");

    let error = LocalMachineFilesystem
        .observe(&missing)
        .expect_err("missing path must fail");

    assert_eq!(
        error.operation(),
        MachineFilesystemOperation::InspectRequested
    );
    assert_eq!(error.path(), missing);
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}

#[cfg(unix)]
#[test]
fn observes_symlink_separately_from_its_canonical_target() {
    use std::os::unix::fs::{symlink, PermissionsExt};

    let root = tempfile::tempdir().expect("temporary root");
    let target = root.path().join("target.txt");
    let alias = root.path().join("alias.txt");
    fs::write(&target, b"target").expect("write target");
    symlink(&target, &alias).expect("create symlink");

    let observation = LocalMachineFilesystem
        .observe(&alias)
        .expect("observe symlink");

    assert_eq!(observation.canonical_path(), canonical(&target));
    assert_eq!(observation.requested_kind(), MachinePathKind::Symlink);
    assert_eq!(observation.canonical_kind(), MachinePathKind::RegularFile);
    let requested_mode = fs::symlink_metadata(&alias)
        .expect("inspect symlink")
        .permissions()
        .mode();
    assert_eq!(observation.unix_mode(), Some(requested_mode));
    assert_eq!(
        LocalMachineFilesystem
            .canonicalize(&alias)
            .expect("canonicalize symlink"),
        canonical(&target)
    );
}

#[cfg(unix)]
#[test]
fn requested_inspection_does_not_canonicalize_a_dangling_symlink() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().expect("temporary root");
    let alias = root.path().join("dangling");
    symlink(root.path().join("absent"), &alias).expect("create dangling symlink");

    assert_eq!(
        LocalMachineFilesystem
            .inspect_requested(&alias)
            .expect("inspect requested symlink"),
        MachinePathKind::Symlink
    );

    let error = LocalMachineFilesystem
        .canonicalize(&alias)
        .expect_err("dangling symlink cannot be canonicalized");

    assert_eq!(error.operation(), MachineFilesystemOperation::Canonicalize);
    assert_eq!(error.path(), alias);

    let complete_error = LocalMachineFilesystem
        .observe(&alias)
        .expect_err("complete observation must preserve the failed stage");
    assert_eq!(
        complete_error.operation(),
        MachineFilesystemOperation::Canonicalize
    );
    assert_eq!(complete_error.path(), alias);
    assert_eq!(complete_error.kind(), io::ErrorKind::NotFound);
}

#[cfg(unix)]
#[test]
fn observes_special_entry_without_admitting_it() {
    use std::process::Command;

    let root = tempfile::tempdir().expect("temporary root");
    let fifo = root.path().join("endpoint.fifo");
    assert!(Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("create named pipe")
        .success());

    let observation = LocalMachineFilesystem
        .observe(&fifo)
        .expect("observe special entry");

    assert_eq!(observation.requested_kind(), MachinePathKind::Other);
    assert_eq!(observation.canonical_kind(), MachinePathKind::Other);
}

fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).expect("canonical fixture path")
}
