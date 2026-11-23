//! Exercises bounded local process probing without weakening executable identity checks.

use std::fs;
use std::io::{Cursor, Read};
use std::os::unix::fs::PermissionsExt;

use super::*;
use pale_core::records::ExecutableDigest;

/// Exact libtest filter that turns this test binary into the escaped-holder fixture.
const ESCAPED_PIPE_HOLDER_HELPER: &str = "probe::tests::escaped_pipe_holder_helper";

struct TransientReader {
    failure: Option<io::ErrorKind>,
    bytes: Cursor<&'static [u8]>,
}

impl Read for TransientReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.failure.take().map_or_else(
            || self.bytes.read(buffer),
            |kind| Err(io::Error::from(kind)),
        )
    }
}

#[test]
#[allow(
    clippy::zombie_processes,
    reason = "the finite child must outlive this helper to retain its inherited pipes"
)]
fn escaped_pipe_holder_helper() {
    // In the ordinary suite this entry is inert. The owning behavioral test
    // launches the current binary with the exact filter above.
    if !std::env::args().any(|argument| argument == ESCAPED_PIPE_HOLDER_HELPER) {
        return;
    }

    use std::os::unix::process::CommandExt;

    let parent = rustix::process::getppid().unwrap();
    let marker = escaped_holder_directory(i64::from(parent.as_raw_pid())).join("complete");
    let mut holder = Command::new("/bin/sh");
    holder
        .args([
            "-c",
            "/bin/sleep 1; : > \"$1\"; /bin/sleep 0.05; printf survived > \"$1\"",
            "holder",
        ])
        .arg(marker)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .process_group(0);
    holder.spawn().unwrap();
}

fn escaped_holder_directory(process_id: i64) -> std::path::PathBuf {
    // The probe deliberately clears `TMPDIR`; parent and helper therefore use
    // the Unix temporary root directly rather than deriving different paths.
    std::path::Path::new("/tmp").join(format!("pale-probe-escaped-holder-{process_id}"))
}

fn fixture(body: &str) -> (tempfile::TempDir, std::path::PathBuf, ExecutableDigest) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("probe");
    let bytes = format!("#!/bin/sh\n{body}\n");
    fs::write(&path, &bytes).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let digest = ExecutableDigest::hash_bytes(bytes.as_bytes());
    (directory, path, digest)
}

fn request<'a>(
    path: &'a std::path::Path,
    digest: &'a ExecutableDigest,
    limits: ProbeLimits,
) -> IdentityProbeRequest<'a> {
    request_with_argument(path, digest, "identity", limits)
}

fn request_with_argument<'a>(
    path: &'a std::path::Path,
    digest: &'a ExecutableDigest,
    argument: &'a str,
    limits: ProbeLimits,
) -> IdentityProbeRequest<'a> {
    IdentityProbeRequest {
        executable: path.to_str().unwrap(),
        expected_digest: digest,
        argument,
        limits,
    }
}

#[test]
fn verifies_the_exact_executable_without_launching_it() {
    let (_directory, path, digest) = fixture("printf 'unused'");
    let observed = LocalProcessProbe
        .verify_executable(ExecutableIdentityRequest {
            executable: path.to_str().unwrap(),
            expected_digest: &digest,
        })
        .unwrap();
    assert_eq!(observed.canonical_path, fs::canonicalize(path).unwrap());
}

#[test]
fn returns_complete_bounded_stdout() {
    let (_directory, path, digest) = fixture("printf 'identity'");
    let observed = LocalProcessProbe
        .probe(request(&path, &digest, ProbeLimits::workspace_identity()))
        .unwrap();
    assert_eq!(observed.stdout, b"identity");
}

#[test]
fn preserves_a_spawn_failure_as_a_typed_operating_system_source() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("invalid-image");
    let bytes = b"#!/definitely/missing-pale-interpreter\n";
    fs::write(&path, bytes).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let digest = ExecutableDigest::hash_bytes(bytes);

    let error = LocalProcessProbe
        .probe(request(&path, &digest, ProbeLimits::workspace_identity()))
        .unwrap_err();
    assert!(matches!(&error, ProcessProbeError::Spawn(_)));
    let source =
        std::error::Error::source(&error).and_then(|source| source.downcast_ref::<io::Error>());
    assert!(
        source.is_some(),
        "typed spawn source missing from {error:?}"
    );
}

#[test]
fn kills_and_reaps_a_probe_that_exceeds_wall_time() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("descendant-survived");
    let body = format!(
        "(sleep 0.20; printf survived > '{}') & wait",
        marker.display()
    );
    let path = directory.path().join("probe");
    let bytes = format!("#!/bin/sh\n{body}\n");
    fs::write(&path, &bytes).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let digest = ExecutableDigest::hash_bytes(bytes.as_bytes());
    let limits = ProbeLimits {
        wall_time_ms: 40,
        stdout_bytes: 64 * 1024,
        stderr_bytes: 64 * 1024,
        cleanup_time_ms: 500,
    };
    let started = Instant::now();
    let error = LocalProcessProbe
        .probe(request(&path, &digest, limits))
        .unwrap_err();
    assert!(matches!(error, ProcessProbeError::Timeout { .. }));
    assert!(started.elapsed() < Duration::from_secs(1));
    thread::sleep(Duration::from_millis(300));
    assert!(
        !marker.exists(),
        "timed-out descendant survived its process group"
    );
}

#[test]
fn reports_nonzero_exit_with_bounded_stderr() {
    let (_directory, path, digest) = fixture("printf 'rejected' >&2; exit 7");
    let error = LocalProcessProbe
        .probe(request(&path, &digest, ProbeLimits::workspace_identity()))
        .unwrap_err();
    assert!(
        matches!(
            error,
            ProcessProbeError::NonZero { code: Some(7), ref stderr }
                if stderr.contains("rejected")
        ),
        "unexpected probe error: {error:?}"
    );
}

#[test]
fn rejects_output_over_the_complete_capture_ceiling() {
    let (_directory, path, digest) = fixture("printf '0123456789abcdef'");
    let limits = ProbeLimits {
        wall_time_ms: 5_000,
        stdout_bytes: 8,
        stderr_bytes: 8,
        cleanup_time_ms: 500,
    };
    let error = LocalProcessProbe
        .probe(request(&path, &digest, limits))
        .unwrap_err();
    assert!(
        matches!(
            error,
            ProcessProbeError::OutputTooLarge {
                stream: Stream::Stdout,
                limit: 8
            }
        ),
        "unexpected probe error: {error:?}"
    );
}

#[test]
fn rejects_stderr_over_the_complete_capture_ceiling() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join("descendant-survived");
    let body = format!(
        "(sleep 0.20; printf survived > '{}') 2>/dev/null & printf '0123456789abcdef' >&2; exit 7",
        marker.display()
    );
    let path = directory.path().join("probe");
    let bytes = format!("#!/bin/sh\n{body}\n");
    fs::write(&path, &bytes).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    let digest = ExecutableDigest::hash_bytes(bytes.as_bytes());
    let limits = ProbeLimits {
        wall_time_ms: 5_000,
        stdout_bytes: 8,
        stderr_bytes: 8,
        cleanup_time_ms: 500,
    };
    let error = LocalProcessProbe
        .probe(request(&path, &digest, limits))
        .unwrap_err();
    assert!(
        matches!(
            error,
            ProcessProbeError::OutputTooLarge {
                stream: Stream::Stderr,
                limit: 8,
            }
        ),
        "unexpected probe error: {error:?}"
    );
    thread::sleep(Duration::from_millis(300));
    assert!(
        !marker.exists(),
        "oversized-stderr descendant survived its process group"
    );
}

#[test]
fn passes_one_literal_argument_without_shell_interpretation() {
    let (_directory, path, digest) = fixture("printf '%s|%s' \"$#\" \"$1\"");
    let argument = "$(printf expanded); *";
    let observed = LocalProcessProbe
        .probe(request_with_argument(
            &path,
            &digest,
            argument,
            ProbeLimits::workspace_identity(),
        ))
        .unwrap();
    assert_eq!(observed.stdout, format!("1|{argument}").as_bytes());
}

#[test]
fn supplies_null_standard_input() {
    let (_directory, path, digest) =
        fixture("if IFS= read -r line; then printf 'open'; else printf 'eof'; fi");
    let observed = LocalProcessProbe
        .probe(request(&path, &digest, ProbeLimits::workspace_identity()))
        .unwrap();
    assert_eq!(observed.stdout, b"eof");
}

#[test]
fn clears_the_ambient_environment_before_identity_observation() {
    let mut command = Command::new("unused");
    command.env("PALE_TEST_SECRET", "secret");

    clear_probe_environment(&mut command);

    assert!(command.get_envs().next().is_none());
}

#[test]
fn rejects_an_executable_replaced_by_the_probe_itself() {
    let body = concat!(
        "replacement=\"$0.replacement\"; ",
        "printf '#!/bin/sh\\nprintf changed\\n' > \"$replacement\"; ",
        "chmod 755 \"$replacement\"; ",
        "mv \"$replacement\" \"$0\"; ",
        "printf identity"
    );
    let (_directory, path, digest) = fixture(body);
    let error = LocalProcessProbe
        .probe(request(&path, &digest, ProbeLimits::workspace_identity()))
        .unwrap_err();
    assert!(matches!(
        error,
        ProcessProbeError::Executable(ExecutableError::IdentityChanged(_))
    ));
}

#[test]
fn process_failures_are_not_masked_by_a_later_executable_change() {
    let body = concat!(
        "replacement=\"$0.replacement\"; ",
        "printf '#!/bin/sh\\nprintf changed\\n' > \"$replacement\"; ",
        "chmod 755 \"$replacement\"; ",
        "mv \"$replacement\" \"$0\"; ",
        "printf 0123456789abcdef"
    );
    let (_directory, path, digest) = fixture(body);
    let limits = ProbeLimits {
        wall_time_ms: 5_000,
        stdout_bytes: 8,
        stderr_bytes: 8,
        cleanup_time_ms: 500,
    };

    let error = LocalProcessProbe
        .probe(request(&path, &digest, limits))
        .unwrap_err();

    assert!(matches!(
        error,
        ProcessProbeError::OutputTooLarge {
            stream: Stream::Stdout,
            limit: 8,
        }
    ));
}

#[test]
fn does_not_wait_for_an_escaped_descendant_pipe_holder() {
    let holder_directory = escaped_holder_directory(i64::from(std::process::id()));
    fs::create_dir(&holder_directory).unwrap();
    fs::set_permissions(&holder_directory, fs::Permissions::from_mode(0o700)).unwrap();
    let completion_marker = holder_directory.join("complete");
    let path = std::env::current_exe().unwrap();
    let digest = ExecutableDigest::hash_bytes(&fs::read(&path).unwrap());
    let limits = ProbeLimits {
        wall_time_ms: 250,
        stdout_bytes: 64 * 1024,
        stderr_bytes: 64 * 1024,
        cleanup_time_ms: 500,
    };

    let started = Instant::now();
    let result = LocalProcessProbe.probe(request_with_argument(
        &path,
        &digest,
        ESCAPED_PIPE_HOLDER_HELPER,
        limits,
    ));
    let elapsed = started.elapsed();
    let completed_at_return = completion_marker.exists();
    let completion_deadline = Instant::now() + Duration::from_secs(2);
    while fs::read(&completion_marker).ok().as_deref() != Some(b"survived")
        && Instant::now() < completion_deadline
    {
        thread::sleep(Duration::from_millis(5));
    }
    let completion = fs::read(&completion_marker);
    let _ = fs::remove_file(&completion_marker);
    fs::remove_dir(&holder_directory).unwrap();

    let error = result.unwrap_err();

    assert!(
        matches!(error, ProcessProbeError::Timeout { millis: 250 }),
        "unexpected probe error: {error:?}"
    );
    assert!(
        elapsed < Duration::from_millis(600),
        "probe waited for escaped pipe holder for {elapsed:?}"
    );
    assert!(!completed_at_return, "escaped holder completed too early");
    assert_eq!(completion.unwrap(), b"survived");
}

#[test]
fn capture_states_preserve_stream_failure_classes_and_exact_limits() {
    let io_error = io::Error::other("capture failed");
    let error = CaptureState::Failed(CaptureError::Io(io_error))
        .finish(Stream::Stdout, 8)
        .unwrap_err();
    assert!(matches!(
        &error,
        ProcessProbeError::Capture {
            stream: Stream::Stdout,
            ..
        }
    ));
    assert!(std::error::Error::source(&error)
        .and_then(|source| source.downcast_ref::<io::Error>())
        .is_some());
    assert!(matches!(
        CaptureState::Failed(CaptureError::Allocation).finish(Stream::Stdout, 8),
        Err(ProcessProbeError::Capture {
            stream: Stream::Stdout,
            ..
        })
    ));

    let mut exact = CaptureState::reading(8);
    assert!(exact.read_once(&mut Cursor::new(b"12345678"), 8));
    assert!(exact.read_once(&mut Cursor::new([]), 8));
    assert_eq!(exact.finish(Stream::Stdout, 8).unwrap(), b"12345678");

    let mut empty = CaptureState::reading(0);
    assert!(empty.read_once(&mut Cursor::new([]), 0));
    assert!(empty.finish(Stream::Stdout, 0).unwrap().is_empty());

    let mut nonempty = CaptureState::reading(0);
    assert!(nonempty.read_once(&mut Cursor::new(b"1"), 0));
    assert!(matches!(
        nonempty.finish(Stream::Stderr, 0),
        Err(ProcessProbeError::OutputTooLarge {
            stream: Stream::Stderr,
            limit: 0,
        })
    ));

    for kind in [io::ErrorKind::Interrupted, io::ErrorKind::WouldBlock] {
        let mut capture = CaptureState::reading(1);
        let mut reader = TransientReader {
            failure: Some(kind),
            bytes: Cursor::new(b"x"),
        };
        assert!(!capture.read_once(&mut reader, 1));
        assert!(capture.read_once(&mut reader, 1));
        assert!(capture.read_once(&mut reader, 1));
        assert_eq!(capture.finish(Stream::Stdout, 1).unwrap(), b"x");
    }

    let bytes = vec![0_u8; CAPTURE_CHUNK_BYTES + 1];
    let mut reader = Cursor::new(bytes);
    let mut bounded = CaptureState::reading(CAPTURE_CHUNK_BYTES + 1);
    assert!(bounded.read_once(&mut reader, CAPTURE_CHUNK_BYTES + 1));
    assert_eq!(reader.position(), CAPTURE_CHUNK_BYTES as u64);
}
