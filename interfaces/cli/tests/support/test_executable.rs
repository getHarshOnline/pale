//! Stable executable snapshots for process-level tests.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read as _};
use std::path::{Path, PathBuf};

use sha2::{Digest as _, Sha256};

pub(crate) struct TestExecutable {
    path: PathBuf,
    sha256: String,
}

impl TestExecutable {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn sha256(&self) -> &str {
        &self.sha256
    }
}

pub(crate) fn snapshot(temporary_root: &Path) -> io::Result<TestExecutable> {
    let mut source = File::open(env!("CARGO_BIN_EXE_pale"))?;
    let permissions = source.metadata()?.permissions();
    let destination_path = temporary_root.join("pale-under-test");
    let mut destination = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination_path)?;

    io::copy(&mut source, &mut destination)?;
    destination.set_permissions(permissions)?;
    drop(source);
    drop(destination);

    let path = fs::canonicalize(destination_path)?;
    let mut copied_executable = File::open(&path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = copied_executable.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(TestExecutable {
        path,
        sha256: hex::encode(hasher.finalize()),
    })
}
