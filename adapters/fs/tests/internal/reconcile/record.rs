//! Ownership checks for materialization state and lineage records.

use super::inspect;
use crate::confine::ConfinedRoot;
use crate::errors::FsError;

#[test]
fn planted_or_symlinked_internal_record_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let state = tmp.path().join("materialization-state.json");
    let lineage = tmp.path().join("materialization-lineage.json");
    let root = ConfinedRoot::open(tmp.path()).unwrap();
    for (path, relative) in [
        (&state, "materialization-state.json"),
        (&lineage, "materialization-lineage.json"),
    ] {
        std::fs::write(path, b"planted").unwrap();
        assert!(matches!(
            inspect(&root.location(relative).unwrap(), "internal-record", None),
            Err(FsError::Collision(_, _))
        ));
        std::fs::remove_file(path).unwrap();
    }

    use std::os::unix::fs::symlink;

    let outside = tmp.path().join("outside.json");
    std::fs::write(&outside, b"outside").unwrap();
    symlink(&outside, &state).unwrap();
    assert!(inspect(
        &root.location("materialization-state.json").unwrap(),
        "materialization-state",
        Some(b"expected")
    )
    .is_err());
    assert_eq!(std::fs::read(outside).unwrap(), b"outside");
}
