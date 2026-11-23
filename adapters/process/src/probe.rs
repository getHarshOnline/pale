//! Bounded execution for workspace identity probes.
//!
//! Probes are short-lived synchronous subprocesses, not general command
//! execution. Standard streams are drained fairly in bounded nonblocking steps;
//! caller-selected post-spawn wall time and captured bytes are bounded. When a
//! failure requires cleanup, the adapter signals the process group where supported
//! and reaps the direct child; neither cleanup nor success proves independently
//! surviving descendants absent. A retained descriptor and the current pathname
//! are compared around launch; because the host process API consumes a pathname,
//! this does not claim descriptor-based execution across the final compare-to-spawn
//! interval.

#[cfg(unix)]
use std::io::{self, Read};
use std::path::Path;
#[cfg(unix)]
use std::process::{Command, Stdio};
#[cfg(unix)]
use std::thread;
#[cfg(unix)]
use std::time::{Duration, Instant};

use pale_ports::{
    ExecutableIdentityPort, ExecutableIdentityRequest, ExecutableObservation, IdentityProbeRequest,
    ProbeLimits, ProbeObservation, ProcessProbeError, ProcessProbePort,
};
#[cfg(unix)]
use pale_ports::{ProbeStream as Stream, ProcessTerminationPolicy};

use crate::executable::VerifiedExecutable;
#[cfg(unix)]
use crate::termination::terminate_and_reap;
use pale_ports::ExecutableIdentityError as ExecutableError;

/// Poll interval when neither nonblocking pipe makes progress.
#[cfg(unix)]
const POLL_INTERVAL: Duration = Duration::from_millis(5);
/// Maximum bytes read from each stream before supervision regains control.
#[cfg(unix)]
const CAPTURE_CHUNK_BYTES: usize = 8 * 1024;

#[cfg(unix)]
#[derive(Clone, Copy, Debug)]
/// Internal duration projection of the caller-owned [`ProbeLimits`].
struct Limits {
    wall_time: Duration,
    stdout_bytes: usize,
    stderr_bytes: usize,
    cleanup_time: Duration,
}

#[cfg(unix)]
impl From<ProbeLimits> for Limits {
    fn from(value: ProbeLimits) -> Self {
        Self {
            wall_time: Duration::from_millis(value.wall_time_ms),
            stdout_bytes: value.stdout_bytes,
            stderr_bytes: value.stderr_bytes,
            cleanup_time: Duration::from_millis(value.cleanup_time_ms),
        }
    }
}

#[cfg(unix)]
impl Limits {
    /// Derive cleanup mechanics only from the caller-owned probe request.
    const fn cleanup_policy(self) -> ProcessTerminationPolicy {
        ProcessTerminationPolicy::immediate(self.cleanup_time)
    }
}

#[cfg(unix)]
#[derive(Debug)]
/// Failure reported while reading one bounded captured stream.
enum CaptureError {
    TooLarge,
    Allocation,
    Io(io::Error),
}

#[cfg(unix)]
#[derive(Debug)]
/// Complete state of one supervision-owned captured stream.
enum CaptureState {
    Reading(Vec<u8>),
    Complete(Vec<u8>),
    Failed(CaptureError),
}

#[cfg(unix)]
impl CaptureState {
    /// Allocate only the first bounded chunk before the child is supervised.
    fn reading(limit: usize) -> Self {
        let mut bytes = Vec::new();
        if bytes
            .try_reserve_exact(limit.min(CAPTURE_CHUNK_BYTES))
            .is_err()
        {
            return Self::Failed(CaptureError::Allocation);
        }
        Self::Reading(bytes)
    }

    /// Read at most one chunk so neither stream can starve supervision.
    fn read_once(&mut self, reader: &mut impl Read, limit: usize) -> bool {
        let Self::Reading(bytes) = self else {
            return false;
        };
        let mut chunk = [0_u8; CAPTURE_CHUNK_BYTES];
        let next = match reader.read(&mut chunk) {
            Ok(0) => Self::Complete(std::mem::take(bytes)),
            Ok(count) if count > limit.saturating_sub(bytes.len()) => {
                Self::Failed(CaptureError::TooLarge)
            }
            Ok(count) if bytes.try_reserve(count).is_err() => {
                Self::Failed(CaptureError::Allocation)
            }
            Ok(count) => {
                bytes.extend_from_slice(&chunk[..count]);
                return true;
            }
            Err(source) if source.kind() == io::ErrorKind::Interrupted => return false,
            Err(source) if source.kind() == io::ErrorKind::WouldBlock => return false,
            Err(source) => Self::Failed(CaptureError::Io(source)),
        };
        *self = next;
        true
    }

    /// Whether this stream has reached an exact end-of-file boundary.
    const fn complete(&self) -> bool {
        matches!(self, Self::Complete(_))
    }

    /// Whether capture has reached a typed terminal failure.
    const fn failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }

    /// Project a terminal capture into the public process-port error algebra.
    fn finish(self, stream: Stream, limit: usize) -> Result<Vec<u8>, ProcessProbeError> {
        match self {
            Self::Complete(bytes) => Ok(bytes),
            Self::Failed(source) => Err(capture_error(source, stream, limit)),
            Self::Reading(_) => Err(ProcessProbeError::Capture {
                stream,
                source: io::Error::other(
                    "probe stream remained incomplete after successful supervision",
                ),
            }),
        }
    }
}

#[cfg(unix)]
#[derive(Debug)]
/// Supervisor condition that requires termination and reaping.
enum StopReason {
    Timeout,
    CaptureFailed,
    Wait(io::Error),
}

/// Run one probe under the caller-selected limits.
///
/// Standard output is returned only when complete. Standard error is retained
/// only for a non-zero-exit diagnostic. A successful observation is returned
/// only after the admitted descriptor and pathname retain their exact identity;
/// a process or cleanup failure is not masked by a later identity observation.
fn run_identity_probe(
    executable: &mut VerifiedExecutable,
    argument: &str,
    limits: ProbeLimits,
) -> Result<Vec<u8>, ProcessProbeError> {
    executable.revalidate()?;
    let stdout = run_with_limits(executable.path(), argument, limits)?;
    executable.revalidate()?;
    Ok(stdout)
}

/// Clear inherited environment variables before one identity-only observation.
///
/// This prevents the helper from receiving unrelated credentials or obsolete
/// capability tokens through the environment. It is not a sandbox: other
/// process context, such as the working directory, remains inherited.
#[cfg(unix)]
fn clear_probe_environment(command: &mut Command) {
    command.env_clear();
}

/// Local bounded implementation of [`ProcessProbePort`].
///
/// The unit value owns no policy or mutable state: requests supply exact identity
/// and resource ceilings. Its success remains a process observation, not
/// execution-path or provider admission.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalProcessProbe;

impl ExecutableIdentityPort for LocalProcessProbe {
    fn verify_executable(
        &self,
        request: ExecutableIdentityRequest<'_>,
    ) -> Result<ExecutableObservation, ExecutableError> {
        let mut executable =
            VerifiedExecutable::admit(request.executable, request.expected_digest)?;
        executable.revalidate()?;
        Ok(ExecutableObservation {
            canonical_path: executable.canonical_path()?,
        })
    }
}

impl ProcessProbePort for LocalProcessProbe {
    fn probe(
        &self,
        request: IdentityProbeRequest<'_>,
    ) -> Result<ProbeObservation, ProcessProbeError> {
        let mut executable =
            VerifiedExecutable::admit(request.executable, request.expected_digest)?;
        let stdout = run_identity_probe(&mut executable, request.argument, request.limits)?;
        Ok(ProbeObservation { stdout })
    }
}

#[cfg(unix)]
fn run_with_limits(
    exec: &Path,
    argument: &str,
    limits: ProbeLimits,
) -> Result<Vec<u8>, ProcessProbeError> {
    let limits = Limits::from(limits);
    let mut command = Command::new(exec);
    command
        .arg(argument)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    clear_probe_environment(&mut command);
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(ProcessProbeError::Spawn)?;
    let started = Instant::now();

    let Some(mut stdout) = child.stdout.take() else {
        terminate_and_reap(&mut child, limits.cleanup_policy())
            .map_err(ProcessProbeError::Terminate)?;
        return Err(ProcessProbeError::PipeUnavailable {
            stream: Stream::Stdout,
        });
    };
    let Some(mut stderr) = child.stderr.take() else {
        terminate_and_reap(&mut child, limits.cleanup_policy())
            .map_err(ProcessProbeError::Terminate)?;
        return Err(ProcessProbeError::PipeUnavailable {
            stream: Stream::Stderr,
        });
    };
    if let Err(error) = configure_nonblocking(&stdout, Stream::Stdout)
        .and_then(|()| configure_nonblocking(&stderr, Stream::Stderr))
    {
        terminate_and_reap(&mut child, limits.cleanup_policy())
            .map_err(ProcessProbeError::Terminate)?;
        return Err(error);
    }
    let mut stdout_capture = CaptureState::reading(limits.stdout_bytes);
    let mut stderr_capture = CaptureState::reading(limits.stderr_bytes);

    let mut status = None;
    let stopped = loop {
        // Bitwise OR deliberately polls both streams without short-circuiting.
        let made_progress = stdout_capture.read_once(&mut stdout, limits.stdout_bytes)
            | stderr_capture.read_once(&mut stderr, limits.stderr_bytes);
        if stdout_capture.failed() || stderr_capture.failed() {
            break Some(StopReason::CaptureFailed);
        }
        // Do not reap the direct child until both streams have reached EOF. A
        // later capture failure still needs the retained
        // child and process-group coordinate for bounded cleanup.
        if status.is_none() && stdout_capture.complete() && stderr_capture.complete() {
            match child.try_wait() {
                Ok(observed) => status = observed,
                Err(error) => break Some(StopReason::Wait(error)),
            }
        }
        if status.is_some() && stdout_capture.complete() && stderr_capture.complete() {
            break None;
        }
        if started.elapsed() >= limits.wall_time {
            break Some(StopReason::Timeout);
        }
        if !made_progress {
            thread::sleep(POLL_INTERVAL.min(limits.wall_time.saturating_sub(started.elapsed())));
        }
    };

    if stopped.is_some() {
        terminate_and_reap(&mut child, limits.cleanup_policy())
            .map_err(ProcessProbeError::Terminate)?;
    }

    match stopped {
        Some(StopReason::Timeout) => {
            return Err(ProcessProbeError::Timeout {
                millis: limits.wall_time.as_millis(),
            });
        }
        Some(StopReason::Wait(error)) => return Err(ProcessProbeError::Wait(error)),
        Some(StopReason::CaptureFailed) => {
            return Err(capture_failure(stdout_capture, stderr_capture, limits));
        }
        None => {}
    }
    let stdout = stdout_capture.finish(Stream::Stdout, limits.stdout_bytes)?;
    let stderr = stderr_capture.finish(Stream::Stderr, limits.stderr_bytes)?;
    let status = status.ok_or_else(|| {
        ProcessProbeError::Wait(io::Error::other(
            "probe stopped without a captured process status",
        ))
    })?;
    if !status.success() {
        let stderr = String::from_utf8_lossy(&stderr);
        let stderr = if stderr.is_empty() {
            String::new()
        } else {
            format!(": {}", stderr.trim_end())
        };
        return Err(ProcessProbeError::NonZero {
            code: status.code(),
            stderr,
        });
    }
    Ok(stdout)
}

#[cfg(not(unix))]
fn run_with_limits(
    exec: &Path,
    argument: &str,
    limits: ProbeLimits,
) -> Result<Vec<u8>, ProcessProbeError> {
    let _ = (exec, argument, limits);
    Err(ProcessProbeError::Executable(
        ExecutableError::UnsupportedPlatform {
            platform: std::env::consts::OS,
        },
    ))
}

#[cfg(unix)]
/// Make one retained child pipe nonblocking before bounded supervision begins.
fn configure_nonblocking(
    reader: &impl std::os::fd::AsFd,
    stream: Stream,
) -> Result<(), ProcessProbeError> {
    use rustix::fs::{fcntl_getfl, fcntl_setfl, OFlags};

    let io_error = |source: rustix::io::Errno| ProcessProbeError::Capture {
        stream,
        source: io::Error::from_raw_os_error(source.raw_os_error()),
    };
    let flags = fcntl_getfl(reader).map_err(io_error)?;
    fcntl_setfl(reader, flags | OFlags::NONBLOCK).map_err(io_error)
}

/// Preserve stdout-first precedence when both bounded streams fail together.
#[cfg(unix)]
fn capture_failure(
    stdout: CaptureState,
    stderr: CaptureState,
    limits: Limits,
) -> ProcessProbeError {
    match (stdout, stderr) {
        (CaptureState::Failed(source), _) => {
            capture_error(source, Stream::Stdout, limits.stdout_bytes)
        }
        (_, CaptureState::Failed(source)) => {
            capture_error(source, Stream::Stderr, limits.stderr_bytes)
        }
        _ => ProcessProbeError::Capture {
            stream: Stream::Stdout,
            source: io::Error::other("probe stopped without a captured stream failure"),
        },
    }
}

/// Translate one internal capture failure without exposing buffered bytes.
#[cfg(unix)]
fn capture_error(source: CaptureError, stream: Stream, limit: usize) -> ProcessProbeError {
    match source {
        CaptureError::TooLarge => ProcessProbeError::OutputTooLarge { stream, limit },
        CaptureError::Allocation => ProcessProbeError::Capture {
            stream,
            source: io::Error::other("cannot allocate bounded probe output"),
        },
        CaptureError::Io(source) => ProcessProbeError::Capture { stream, source },
    }
}

#[cfg(all(test, unix))]
#[path = "../tests/internal/probe.rs"]
mod tests;
