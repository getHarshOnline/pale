//! Exercises bounded, complete regular-file reads across mutation and file-kind boundaries.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use pale_ports::{SourceFilesystemError, SourceReadLimit};

use super::{read_confined_regular, read_confined_regular_with, read_regular, read_regular_with};

#[test]
fn reads_only_complete_regular_files_within_the_bound() {
    let temporary = tempfile::tempdir().unwrap();
    let file = temporary.path().join("input");
    fs::write(&file, b"complete").unwrap();
    assert_eq!(
        read_regular(&file, SourceReadLimit::new(8)).unwrap(),
        b"complete"
    );
    assert!(matches!(
        read_regular(&file, SourceReadLimit::new(7)),
        Err(SourceFilesystemError::Limit {
            limit: "file bytes",
            ..
        })
    ));
}

#[test]
fn rejects_a_file_that_grows_after_the_metadata_precheck() {
    let temporary = tempfile::tempdir().unwrap();
    let file = temporary.path().join("input");
    fs::write(&file, b"complete").unwrap();
    let result = read_regular_with(&file, SourceReadLimit::new(8), || {
        fs::OpenOptions::new()
            .append(true)
            .open(&file)
            .unwrap()
            .write_all(b"!")
            .unwrap();
    });
    assert!(matches!(
        result,
        Err(SourceFilesystemError::Limit {
            limit: "file bytes",
            value: 9,
            maximum: 8,
        })
    ));
}

#[cfg(unix)]
#[test]
fn rejects_symlinks_directories_and_special_files() {
    use std::os::unix::fs::symlink;

    let temporary = tempfile::tempdir().unwrap();
    let file = temporary.path().join("input");
    fs::write(&file, b"complete").unwrap();
    let link = temporary.path().join("link");
    symlink(&file, &link).unwrap();
    assert!(read_regular(&link, SourceReadLimit::new(8)).is_err());
    assert!(matches!(
        read_regular(temporary.path(), SourceReadLimit::new(8)),
        Err(SourceFilesystemError::InvalidEntry { .. })
    ));

    let fifo = temporary.path().join("fifo");
    assert!(Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .unwrap()
        .success());
    assert!(read_regular(&fifo, SourceReadLimit::new(8)).is_err());
}

#[cfg(unix)]
#[test]
fn ancestor_replacement_cannot_redirect_an_opened_source() {
    use std::os::unix::fs::symlink;

    let temporary = tempfile::tempdir().unwrap();
    let parent = temporary.path().join("parent");
    let retained_parent = temporary.path().join("retained-parent");
    let outside = tempfile::tempdir().unwrap();
    fs::create_dir(&parent).unwrap();
    fs::write(parent.join("input"), b"retained").unwrap();
    fs::write(outside.path().join("input"), b"redirected").unwrap();

    let bytes = read_confined_regular_with(
        temporary.path(),
        Path::new("parent/input"),
        SourceReadLimit::new(16),
        || {
            fs::rename(&parent, &retained_parent).unwrap();
            symlink(outside.path(), &parent).unwrap();
        },
    )
    .unwrap();

    assert_eq!(bytes, b"retained");
    assert!(read_confined_regular(
        temporary.path(),
        Path::new("parent/input"),
        SourceReadLimit::new(16),
    )
    .is_err());
}

#[test]
fn a_missing_file_reports_the_requested_path() {
    match read_regular(Path::new("missing-input"), SourceReadLimit::new(8)) {
        Err(SourceFilesystemError::Io { path, .. }) => {
            assert_eq!(path, Path::new("missing-input"));
        }
        result => panic!("expected missing input, got {result:?}"),
    }
}
