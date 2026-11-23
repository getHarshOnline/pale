//! Identity-preserving cleanup of atomically reserved candidate paths.

use std::os::unix::fs::PermissionsExt;

use super::*;

#[test]
fn cleanup_preserves_foreign_replacements_and_unclaimed_sidecars() {
    let directory = crate::test_support::private_tempdir();
    let location = crate::test_support::location(
        &directory,
        crate::ControlStoreKind::PrincipalTrust,
        "foreign-replacement",
    );
    let path = location.path();
    let moved_main = path.with_file_name("moved-main.sqlite3");
    let wal = sqlite_sidecar(&path, "-wal");
    let reservation = CandidateReservation::reserve_store(&location).unwrap();

    std::fs::rename(&path, &moved_main).unwrap();
    std::fs::write(&path, b"foreign main").unwrap();
    std::os::unix::fs::symlink("foreign-target", &wal).unwrap();

    let error = reservation
        .finish::<()>(Err(ControlSqliteError::SourceInvariant(
            "injected candidate failure",
        )))
        .unwrap_err();

    assert!(matches!(error, ControlSqliteError::SourceInvariant(_)));
    assert_eq!(std::fs::read(&path).unwrap(), b"foreign main");
    assert_eq!(
        std::fs::read_link(&wal).unwrap(),
        Path::new("foreign-target")
    );
    assert_eq!(std::fs::read(&moved_main).unwrap(), b"");
}

#[test]
fn explicit_discard_preserves_a_foreign_replacement() {
    let directory = crate::test_support::private_tempdir();
    let location = crate::test_support::location(
        &directory,
        crate::ControlStoreKind::PrincipalTrust,
        "discard-foreign-replacement",
    );
    let path = location.path();
    let moved_main = path.with_file_name("discarded-owned-main.sqlite3");
    let reservation = CandidateReservation::reserve_store(&location).unwrap();

    std::fs::rename(&path, &moved_main).unwrap();
    std::fs::write(&path, b"foreign replacement").unwrap();

    let error = reservation.discard().unwrap_err();
    assert!(matches!(error, ControlSqliteError::CandidateOwnershipLost(found) if found == path));
    assert_eq!(std::fs::read(&path).unwrap(), b"foreign replacement");
    assert_eq!(std::fs::read(&moved_main).unwrap(), b"");
}

#[test]
fn cleanup_uses_the_retained_parent_after_the_path_is_replaced() {
    let root = crate::test_support::private_tempdir();
    let location = crate::test_support::location(
        &root,
        crate::ControlStoreKind::PrincipalTrust,
        "retained-parent",
    );
    let path = location.path();
    let parent = path.parent().unwrap().to_path_buf();
    let moved_parent = parent.with_file_name("moved-control");
    let reservation = CandidateReservation::reserve_store(&location).unwrap();

    std::fs::rename(&parent, &moved_parent).unwrap();
    std::fs::create_dir(&parent).unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(&path, b"foreign replacement").unwrap();

    let error = reservation
        .finish::<()>(Err(ControlSqliteError::SourceInvariant(
            "injected candidate failure",
        )))
        .unwrap_err();

    assert!(matches!(error, ControlSqliteError::SourceInvariant(_)));
    assert_eq!(std::fs::read(&path).unwrap(), b"foreign replacement");
    assert!(!moved_parent.join("trust.sqlite3").exists());
}

#[test]
fn sqlite_open_rejects_a_replaced_parent_before_touching_its_candidate() {
    let root = crate::test_support::private_tempdir();
    let location = crate::test_support::location(
        &root,
        crate::ControlStoreKind::PrincipalTrust,
        "replaced-open",
    );
    let path = location.path();
    let parent = path.parent().unwrap().to_path_buf();
    let moved_parent = parent.with_file_name("moved-control");
    let reservation = CandidateReservation::reserve_store(&location).unwrap();

    std::fs::rename(&parent, &moved_parent).unwrap();
    std::fs::create_dir(&parent).unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(&path, b"foreign replacement").unwrap();

    let error = reservation
        .open_sqlite("open replaced candidate")
        .unwrap_err();
    assert!(matches!(
        error,
        ControlSqliteError::CandidateOwnershipLost(_)
    ));
    assert_eq!(std::fs::read(&path).unwrap(), b"foreign replacement");
}

#[test]
fn successful_finish_rejects_a_replaced_parent_and_cleans_only_owned_bytes() {
    let root = crate::test_support::private_tempdir();
    let location = crate::test_support::location(
        &root,
        crate::ControlStoreKind::PrincipalTrust,
        "replaced-finish",
    );
    let path = location.path();
    let parent = path.parent().unwrap().to_path_buf();
    let moved_parent = parent.with_file_name("moved-control");
    let reservation = CandidateReservation::reserve_store(&location).unwrap();

    std::fs::rename(&parent, &moved_parent).unwrap();
    std::fs::create_dir(&parent).unwrap();
    std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(&path, b"foreign replacement").unwrap();

    let error = reservation.finish(Ok(())).unwrap_err();

    assert!(matches!(
        error,
        ControlSqliteError::CandidateOwnershipLost(_)
    ));
    assert_eq!(std::fs::read(&path).unwrap(), b"foreign replacement");
    assert!(!moved_parent.join("trust.sqlite3").exists());
}

#[test]
fn cleanup_failure_retains_the_original_candidate_failure() {
    let directory = crate::test_support::private_tempdir();
    let location = crate::test_support::location(
        &directory,
        crate::ControlStoreKind::PrincipalTrust,
        "cleanup-failure",
    );
    let path = location.path();
    let parent = path.parent().unwrap();
    let reservation = CandidateReservation::reserve_store(&location).unwrap();
    std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o500)).unwrap();

    let error = reservation
        .finish::<()>(Err(ControlSqliteError::SourceInvariant(
            "primary initialization failure",
        )))
        .unwrap_err();
    std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700)).unwrap();

    assert!(matches!(
        error,
        ControlSqliteError::CandidateCleanupFailed { failure, .. }
            if matches!(*failure, ControlSqliteError::SourceInvariant("primary initialization failure"))
    ));
    assert!(path.exists());
}
