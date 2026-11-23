//! Bounded no-follow reads for standalone and root-confined source files.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use pale_ports::{SourceFilesystemError, SourceReadLimit};

#[cfg(unix)]
use super::root::OpenedSourceRoot;

/// Reused stream chunk; [`SourceReadLimit`] remains the policy bound.
const BUFFER_BYTES: usize = 16 * 1024;

/// Read one complete regular file without following its leaf.
pub(super) fn read_regular(
    path: &Path,
    limit: SourceReadLimit,
) -> Result<Vec<u8>, SourceFilesystemError> {
    read_regular_with(path, limit, || {})
}

fn read_regular_with(
    path: &Path,
    limit: SourceReadLimit,
    before_stream: impl FnOnce(),
) -> Result<Vec<u8>, SourceFilesystemError> {
    let file = open_nofollow(path)?;
    read_opened(file, path, limit, before_stream)
}

/// Read one relative member from a retained, identity-checked directory root.
pub(super) fn read_confined_regular(
    root: &Path,
    relative: &Path,
    limit: SourceReadLimit,
) -> Result<Vec<u8>, SourceFilesystemError> {
    read_confined_regular_with(root, relative, limit, || {})
}

#[cfg(unix)]
fn read_confined_regular_with(
    root: &Path,
    relative: &Path,
    limit: SourceReadLimit,
    before_stream: impl FnOnce(),
) -> Result<Vec<u8>, SourceFilesystemError> {
    use rustix::fs::{openat, Mode, OFlags};

    let relative = relative
        .to_str()
        .and_then(pale_core::paths::ConfinedRelativePath::parse)
        .ok_or_else(|| SourceFilesystemError::InvalidEntry {
            path: root.join(relative),
        })?;
    let opened_root = OpenedSourceRoot::open(root)?;
    let mut components = relative.components().peekable();
    let mut directory =
        opened_root
            .directory()
            .try_clone()
            .map_err(|source| SourceFilesystemError::Io {
                path: root.to_path_buf(),
                source,
            })?;
    let mut walked = std::path::PathBuf::new();
    let file = loop {
        let component = components
            .next()
            .expect("a confined relative path has at least one component");
        walked.push(component);
        let path = root.join(&walked);
        if components.peek().is_none() {
            break File::from(
                openat(
                    &directory,
                    component,
                    OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
                    Mode::empty(),
                )
                .map_err(|source| errno(path.as_path(), source))?,
            );
        }
        directory = File::from(
            openat(
                &directory,
                component,
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|source| errno(path.as_path(), source))?,
        );
    };
    let path = root.join(relative.as_str());
    let bytes = read_opened(file, &path, limit, before_stream)?;
    opened_root.verify()?;
    Ok(bytes)
}

#[cfg(not(unix))]
fn read_confined_regular_with(
    root: &Path,
    relative: &Path,
    limit: SourceReadLimit,
    before_stream: impl FnOnce(),
) -> Result<Vec<u8>, SourceFilesystemError> {
    let _ = (root, relative, limit, before_stream);
    Err(SourceFilesystemError::UnsupportedPlatform {
        platform: std::env::consts::OS,
    })
}

/// Stream one already-opened regular-file handle under the caller's bound.
fn read_opened(
    file: File,
    path: &Path,
    limit: SourceReadLimit,
    before_stream: impl FnOnce(),
) -> Result<Vec<u8>, SourceFilesystemError> {
    let metadata = file
        .metadata()
        .map_err(|source| SourceFilesystemError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    if !metadata.is_file() {
        return Err(SourceFilesystemError::InvalidEntry {
            path: path.to_path_buf(),
        });
    }
    let maximum = limit.max_bytes();
    let stream_limit = u64::try_from(maximum).unwrap_or(u64::MAX);
    if metadata.len() > stream_limit {
        return Err(SourceFilesystemError::Limit {
            limit: "file bytes",
            value: metadata.len(),
            maximum: stream_limit,
        });
    }
    before_stream();

    let capacity = usize::try_from(metadata.len())
        .unwrap_or(usize::MAX)
        .min(maximum)
        .min(BUFFER_BYTES);
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|source| SourceFilesystemError::Allocation {
            path: path.to_path_buf(),
            source,
        })?;
    let mut reader = file.take(stream_limit.saturating_add(1));
    let mut buffer = [0_u8; BUFFER_BYTES];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|source| SourceFilesystemError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        if count == 0 {
            break;
        }
        if bytes
            .len()
            .checked_add(count)
            .is_none_or(|length| length > maximum)
        {
            return Err(SourceFilesystemError::Limit {
                limit: "file bytes",
                value: u64::try_from(maximum).unwrap_or(u64::MAX).saturating_add(1),
                maximum: stream_limit,
            });
        }
        bytes
            .try_reserve(count)
            .map_err(|source| SourceFilesystemError::Allocation {
                path: path.to_path_buf(),
                source,
            })?;
        bytes.extend_from_slice(&buffer[..count]);
    }
    Ok(bytes)
}

#[cfg(unix)]
fn open_nofollow(path: &Path) -> Result<File, SourceFilesystemError> {
    use rustix::fs::{open, Mode, OFlags};

    open(
        path,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::empty(),
    )
    .map(File::from)
    .map_err(|source| errno(path, source))
}

#[cfg(not(unix))]
fn open_nofollow(path: &Path) -> Result<File, SourceFilesystemError> {
    let _ = path;
    Err(SourceFilesystemError::UnsupportedPlatform {
        platform: std::env::consts::OS,
    })
}

#[cfg(unix)]
fn errno(path: &Path, source: rustix::io::Errno) -> SourceFilesystemError {
    SourceFilesystemError::Io {
        path: path.to_path_buf(),
        source: std::io::Error::from_raw_os_error(source.raw_os_error()),
    }
}

#[cfg(test)]
#[path = "../../tests/internal/source/read.rs"]
mod tests;
