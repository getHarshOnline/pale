//! Identity-replacement proof for private stop-sentinel custody.

#![cfg(unix)]
use std::fs;
use std::os::unix::fs::PermissionsExt;

use super::*;

#[test]
fn an_exact_byte_replacement_is_not_unlinked_as_the_admitted_marker() {
    let temporary = tempfile::tempdir().expect("temporary root");
    let parent = temporary.path().join("supervisor");
    fs::create_dir_all(&parent).expect("fixture parent");
    let sentinel =
        StopSentinelCustody::new(temporary.path(), "supervisor/stopped").expect("bind sentinel");
    sentinel.ensure_present().expect("publish marker");
    let admitted = sentinel
        .observe_marker()
        .expect("admit marker")
        .expect("present marker");

    let marker = parent.join("stopped");
    let original = parent.join("original-marker");
    fs::rename(&marker, &original).expect("move admitted marker");
    fs::write(&marker, MARKER_BYTES).expect("write byte-identical replacement");
    fs::set_permissions(&marker, fs::Permissions::from_mode(MARKER_MODE))
        .expect("set replacement mode");

    assert!(matches!(
        sentinel.marker.remove_file_if_identity(admitted),
        Err(FsError::Collision(_, _))
    ));
    assert_eq!(
        fs::read(&marker).expect("replacement preserved"),
        MARKER_BYTES
    );
    assert!(
        original.exists(),
        "admitted object remains separately visible"
    );
}
