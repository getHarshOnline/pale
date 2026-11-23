//! Generated physical-store fixtures for the open-classification matrix.

use std::path::Path;
use std::{fs, os::unix::fs::PermissionsExt};

use rusqlite::Connection;

pub(crate) enum FixtureState {
    Empty,
    Corrupt,
    Partial,
    NewerVersion,
    SourceManifestMismatch,
    IncompleteMigration,
    NonWal,
}

pub(crate) fn materialize(path: &Path, state: FixtureState) {
    match state {
        FixtureState::Empty => write_private(path, []),
        FixtureState::Corrupt => write_private(path, b"not a sqlite database"),
        FixtureState::Partial => {
            Connection::open(path)
                .unwrap()
                .execute_batch(
                    "PRAGMA application_id = 1346456660; PRAGMA user_version = 1; \
                     CREATE TABLE incomplete_store (id INTEGER PRIMARY KEY) STRICT;",
                )
                .unwrap();
            make_private(path);
        }
        FixtureState::NewerVersion => {
            Connection::open(path)
                .unwrap()
                .execute_batch("PRAGMA user_version = 2;")
                .unwrap();
        }
        FixtureState::SourceManifestMismatch => {
            Connection::open(path)
                .unwrap()
                .execute(
                    "UPDATE pale_control_store_header SET storage_manifest_sha256 = ?1 \
                     WHERE singleton = 1",
                    ["0".repeat(64)],
                )
                .unwrap();
        }
        FixtureState::IncompleteMigration => {
            Connection::open(path)
                .unwrap()
                .execute("DELETE FROM pale_control_store_migration_receipt", [])
                .unwrap();
        }
        FixtureState::NonWal => {
            let connection = Connection::open(path).unwrap();
            let observed: String = connection
                .query_row("PRAGMA journal_mode = DELETE", [], |row| row.get(0))
                .unwrap();
            assert_eq!(observed, "delete");
        }
    }
}

/// Write fixture bytes with the same exact local mode required by classification.
fn write_private(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::write(path, bytes).unwrap();
    make_private(path);
}

/// Apply the exact current-UID store-file mode without changing file ownership.
fn make_private(path: &Path) {
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}
