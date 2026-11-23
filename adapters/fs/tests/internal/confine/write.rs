//! Staging cleanup identity and failure preservation.

use super::{cleanup_temporary, StagingIdentity};
use crate::errors::FsError;
use std::os::unix::fs::PermissionsExt;

#[test]
fn staging_cleanup_preserves_failed_or_replaced_entries() {
    let temporary = tempfile::tempdir().unwrap();
    let parent_path = temporary.path().join("parent");
    std::fs::create_dir(&parent_path).unwrap();
    let parent = std::fs::File::open(&parent_path).unwrap();
    let staging_path = parent_path.join("stage");

    std::fs::write(&staging_path, b"staged").unwrap();
    let identity =
        StagingIdentity(rustix::fs::fstat(std::fs::File::open(&staging_path).unwrap()).unwrap());
    std::fs::set_permissions(&parent_path, std::fs::Permissions::from_mode(0o500)).unwrap();
    let failure = cleanup_temporary(&parent, "stage", &staging_path, identity, true).unwrap_err();
    std::fs::set_permissions(&parent_path, std::fs::Permissions::from_mode(0o700)).unwrap();

    match failure {
        FsError::Io {
            operation,
            path,
            source,
        } => {
            assert_eq!(operation, "remove staging file");
            assert_eq!(path, staging_path);
            assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied);
        }
        other => panic!("unexpected cleanup failure: {other:?}"),
    }
    assert!(staging_path.exists(), "failed cleanup remains observable");

    let replacement_path = parent_path.join("replacement");
    std::fs::write(&replacement_path, b"replacement").unwrap();
    std::fs::remove_file(&staging_path).unwrap();
    std::fs::rename(&replacement_path, &staging_path).unwrap();
    assert!(matches!(
        cleanup_temporary(&parent, "stage", &staging_path, identity, true),
        Err(FsError::Collision(_, _))
    ));
    assert_eq!(std::fs::read(&staging_path).unwrap(), b"replacement");
}
