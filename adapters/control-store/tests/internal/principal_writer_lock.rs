//! Principal writer-lock filesystem and contention laws.

use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Barrier};

use pale_core::records::ResidentPrincipalIdV1;

use crate::{acquire_principal_writer_lock, ControlSqliteError};

const CHILD_ROOT: &str = "PALE_TEST_PRINCIPAL_WRITER_LOCK_ROOT";
const CHILD_PRINCIPAL: &str = "PALE_TEST_PRINCIPAL_WRITER_LOCK_PRINCIPAL";
const CHILD_READY: &str = "PALE_PRINCIPAL_WRITER_LOCK_HELD";

#[test]
fn principal_writer_lock_child() {
    let Some(root) = std::env::var_os(CHILD_ROOT) else {
        return;
    };
    let Some(principal) = std::env::var_os(CHILD_PRINCIPAL) else {
        return;
    };
    let principal = ResidentPrincipalIdV1::parse(principal.to_string_lossy()).unwrap();
    let _guard = acquire_principal_writer_lock(Path::new(&root), principal).unwrap();
    println!("{CHILD_READY}");
    std::io::stdout().flush().unwrap();
    let mut release = [0_u8; 1];
    let _ = std::io::stdin().read(&mut release);
}

#[test]
fn another_process_is_rejected_until_the_holder_releases() {
    let directory = crate::test_support::private_tempdir();
    let principal = ResidentPrincipalIdV1::parse("principal-process-release").unwrap();
    let mut child = spawn_holder(directory.path(), &principal);
    let _child_output = wait_until_held(&mut child);

    assert!(matches!(
        acquire_principal_writer_lock(directory.path(), principal.clone()),
        Err(ControlSqliteError::PrincipalWriterLockContended)
    ));

    child.stdin.take().unwrap().write_all(b"x").unwrap();
    assert!(child.wait().unwrap().success());
    let guard = acquire_principal_writer_lock(directory.path(), principal.clone()).unwrap();
    assert_eq!(guard.principal_id(), &principal);
}

#[test]
fn operating_system_releases_the_lock_when_the_holder_crashes() {
    let directory = crate::test_support::private_tempdir();
    let principal = ResidentPrincipalIdV1::parse("principal-process-crash").unwrap();
    let mut child = spawn_holder(directory.path(), &principal);
    let _child_output = wait_until_held(&mut child);

    assert!(matches!(
        acquire_principal_writer_lock(directory.path(), principal.clone()),
        Err(ControlSqliteError::PrincipalWriterLockContended)
    ));

    child.kill().unwrap();
    let _ = child.wait().unwrap();
    let guard = acquire_principal_writer_lock(directory.path(), principal.clone()).unwrap();
    assert_eq!(guard.principal_id(), &principal);
}

#[test]
fn different_principals_use_different_private_empty_lock_files() {
    let directory = crate::test_support::private_tempdir();
    let first = ResidentPrincipalIdV1::parse("principal-first").unwrap();
    let second = ResidentPrincipalIdV1::parse("principal-second").unwrap();

    let _first = acquire_principal_writer_lock(directory.path(), first).unwrap();
    let _second = acquire_principal_writer_lock(directory.path(), second).unwrap();

    let principal_root = directory.path().join("pale/principals");
    let entries = std::fs::read_dir(principal_root)
        .unwrap()
        .map(|entry| entry.unwrap().path().join("resident-writer.lock"))
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 2);
    for lock in entries {
        let metadata = std::fs::metadata(lock).unwrap();
        assert!(metadata.is_file());
        assert_eq!(metadata.len(), 0);
        assert_eq!(metadata.uid(), rustix::process::geteuid().as_raw());
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
    }
}

#[test]
fn the_same_process_cannot_hold_two_guards_for_one_principal() {
    let directory = crate::test_support::private_tempdir();
    let principal = ResidentPrincipalIdV1::parse("principal-one-process").unwrap();
    let _first = acquire_principal_writer_lock(directory.path(), principal.clone()).unwrap();

    assert!(matches!(
        acquire_principal_writer_lock(directory.path(), principal),
        Err(ControlSqliteError::PrincipalWriterLockContended)
    ));
}

#[test]
fn concurrent_first_acquisition_creates_one_lock_and_one_holder() {
    const CONTENDERS: usize = 8;

    let directory = crate::test_support::private_tempdir();
    let root = directory.path().to_path_buf();
    let principal = ResidentPrincipalIdV1::parse("principal-concurrent-create").unwrap();
    let start = Arc::new(Barrier::new(CONTENDERS + 1));
    let attempted = Arc::new(Barrier::new(CONTENDERS + 1));
    let workers = (0..CONTENDERS)
        .map(|_| {
            let root = root.clone();
            let principal = principal.clone();
            let start = Arc::clone(&start);
            let attempted = Arc::clone(&attempted);
            std::thread::spawn(move || {
                start.wait();
                let result = acquire_principal_writer_lock(&root, principal);
                attempted.wait();
                match result {
                    Ok(_guard) => true,
                    Err(ControlSqliteError::PrincipalWriterLockContended) => false,
                    Err(error) => panic!("unexpected acquisition failure: {error}"),
                }
            })
        })
        .collect::<Vec<_>>();

    start.wait();
    attempted.wait();
    assert_eq!(
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .filter(|acquired| *acquired)
            .count(),
        1
    );
    assert_eq!(lock_paths(directory.path()).len(), 1);
}

#[test]
fn unsafe_existing_lock_entries_are_rejected_without_adoption() {
    assert_unsafe_lock_entry("principal-lock-symlink", |root, lock| {
        std::fs::remove_file(lock).unwrap();
        let target = root.join("symlink-target");
        std::fs::write(&target, []).unwrap();
        std::os::unix::fs::symlink(&target, lock).unwrap();
    });
    assert_unsafe_lock_entry("principal-lock-directory", |_, lock| {
        std::fs::remove_file(lock).unwrap();
        std::fs::create_dir(lock).unwrap();
    });
    assert_unsafe_lock_entry("principal-lock-visible", |_, lock| {
        std::fs::set_permissions(lock, std::fs::Permissions::from_mode(0o640)).unwrap();
    });
    assert_unsafe_lock_entry("principal-lock-nonempty", |_, lock| {
        std::fs::write(lock, b"unexpected").unwrap();
        std::fs::set_permissions(lock, std::fs::Permissions::from_mode(0o600)).unwrap();
    });
    assert_unsafe_lock_entry("principal-lock-hard-link", |root, lock| {
        std::fs::remove_file(lock).unwrap();
        let target = root.join("hard-link-target");
        std::fs::write(&target, []).unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::hard_link(target, lock).unwrap();
    });
}

fn spawn_holder(root: &Path, principal: &ResidentPrincipalIdV1) -> Child {
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "principal_writer_lock_tests::principal_writer_lock_child",
            "--nocapture",
        ])
        .env(CHILD_ROOT, root)
        .env(CHILD_PRINCIPAL, principal.as_str())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap()
}

fn wait_until_held(child: &mut Child) -> BufReader<std::process::ChildStdout> {
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let mut line = String::new();
    loop {
        line.clear();
        assert_ne!(output.read_line(&mut line).unwrap(), 0);
        if line.trim() == CHILD_READY {
            break;
        }
    }
    output
}

fn assert_unsafe_lock_entry(principal: &str, mutate: impl FnOnce(&Path, &Path)) {
    let directory = crate::test_support::private_tempdir();
    let principal = ResidentPrincipalIdV1::parse(principal).unwrap();
    drop(acquire_principal_writer_lock(directory.path(), principal.clone()).unwrap());
    let lock = lock_paths(directory.path()).pop().unwrap();
    mutate(directory.path(), &lock);

    let result = acquire_principal_writer_lock(directory.path(), principal);
    assert!(
        matches!(
            result,
            Err(ControlSqliteError::UnsafePrincipalWriterLock(ref path)) if path == &lock
        ),
        "unsafe lock entry was not rejected at {}: {result:?}",
        lock.display()
    );
}

fn lock_paths(root: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(root.join("pale/principals"))
        .unwrap()
        .map(|entry| entry.unwrap().path().join("resident-writer.lock"))
        .collect()
}
