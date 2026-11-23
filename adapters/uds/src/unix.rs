//! Darwin and Linux Unix-domain socket mechanics.

use std::cell::Cell;
use std::io::{self, Read, Write};
use std::os::fd::{AsFd, AsRawFd};
use std::os::unix::net::{UnixListener as StdUnixListener, UnixStream};
use std::path::Path;
use std::time::{Duration, Instant};

use nix::errno::Errno;
#[cfg(target_os = "macos")]
use nix::fcntl::{fcntl, FcntlArg, FdFlag};
use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
use nix::sys::socket::{
    connect as socket_connect, getsockopt, socket, sockopt, AddressFamily, SockFlag, SockType,
    UnixAddr,
};
use pale_ports::{
    LocalIpcCarrierError, LocalIpcCarrierOperation, LocalIpcFrameLimits, LocalIpcFramePort,
    LocalIpcFrameSection, LocalIpcPeerCredentials,
};

/// Fixed network-byte-order frame-length prefix width.
const LENGTH_PREFIX_BYTES: usize = size_of::<u32>();

/// Nonblocking listener for a caller-selected Unix-domain socket pathname.
///
/// The listener does not unlink a predecessor entry before binding and does not
/// remove its entry on drop. Those are deployment lifecycle decisions, not
/// carrier semantics.
pub struct UnixSocketListener {
    inner: StdUnixListener,
    limits: LocalIpcFrameLimits,
}

impl UnixSocketListener {
    /// Bind a fresh caller-selected endpoint and apply `limits` to every
    /// subsequently accepted connection.
    ///
    /// # Errors
    ///
    /// Returns a redacted [`LocalIpcCarrierError::Io`] when the host cannot bind
    /// the pathname or configure nonblocking operation. An existing entry is
    /// never removed automatically.
    pub fn bind(path: &Path, limits: LocalIpcFrameLimits) -> Result<Self, LocalIpcCarrierError> {
        let inner = StdUnixListener::bind(path)
            .map_err(|source| LocalIpcCarrierError::io(LocalIpcCarrierOperation::Bind, source))?;
        inner
            .set_nonblocking(true)
            .map_err(|source| LocalIpcCarrierError::io(LocalIpcCarrierOperation::Bind, source))?;
        Ok(Self { inner, limits })
    }

    /// Attempt one nonblocking accept without waiting for a peer.
    ///
    /// An empty queue or an interrupted attempt returns `None`. The caller
    /// controls when to offer another turn; this method does not spin or retry.
    ///
    /// # Errors
    ///
    /// Returns a redacted carrier error if accepting or configuring the
    /// connection fails. A successful accept supplies no peer authentication.
    pub fn try_accept(&self) -> Result<Option<UnixSocketCarrier>, LocalIpcCarrierError> {
        match self.inner.accept() {
            Ok((stream, _address)) => {
                UnixSocketCarrier::from_connected(stream, self.limits).map(Some)
            }
            Err(source)
                if matches!(
                    source.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                Ok(None)
            }
            Err(source) => Err(LocalIpcCarrierError::io(
                LocalIpcCarrierOperation::Accept,
                source,
            )),
        }
    }

    /// Accept one connection before the supplied monotonic `deadline`.
    ///
    /// # Errors
    ///
    /// Returns [`LocalIpcCarrierError::DeadlineElapsed`] when no peer arrives in
    /// time, or a typed host I/O failure. No peer authentication is performed.
    pub fn accept(&self, deadline: Instant) -> Result<UnixSocketCarrier, LocalIpcCarrierError> {
        loop {
            ensure_before_deadline(deadline, LocalIpcCarrierOperation::Accept, 0)?;
            match self.inner.accept() {
                Ok((stream, _address)) => {
                    return UnixSocketCarrier::from_connected(stream, self.limits);
                }
                Err(source) if source.kind() == io::ErrorKind::Interrupted => continue,
                Err(source) if source.kind() == io::ErrorKind::WouldBlock => {
                    wait_ready(
                        &self.inner,
                        PollFlags::POLLIN,
                        deadline,
                        LocalIpcCarrierOperation::Accept,
                        0,
                    )?;
                }
                Err(source) => {
                    return Err(LocalIpcCarrierError::io(
                        LocalIpcCarrierOperation::Accept,
                        source,
                    ));
                }
            }
        }
    }
}

/// One connected, bounded, nonblocking Unix-domain socket carrier.
///
/// A connection-fatal framing failure irreversibly poisons this value. Every
/// later operation returns [`LocalIpcCarrierError::ConnectionUnusable`], so a
/// caller cannot accidentally reinterpret residual bytes as a new frame.
pub struct UnixSocketCarrier {
    stream: UnixStream,
    limits: LocalIpcFrameLimits,
    state: Cell<CarrierState>,
}

/// Whether a connected carrier still has a proved frame boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CarrierState {
    Ready,
    Unusable,
}

impl UnixSocketCarrier {
    /// Observe read readiness without consuming frame bytes or waiting.
    ///
    /// Readiness can indicate a partial frame, EOF, or a pending socket error;
    /// it proves no complete request. The subsequent bounded receive owns that
    /// distinction. An interrupted poll returns `false` without retrying.
    ///
    /// # Errors
    ///
    /// Returns a typed carrier failure if the connection is already unusable
    /// or the native readiness observation fails.
    pub fn is_readable(&self) -> Result<bool, LocalIpcCarrierError> {
        self.ensure_usable()?;
        let mut descriptors = [PollFd::new(self.stream.as_fd(), PollFlags::POLLIN)];
        let result = match poll(&mut descriptors, PollTimeout::ZERO) {
            Ok(0) | Err(Errno::EINTR) => Ok(false),
            Ok(_) => Ok(true),
            Err(source) => Err(LocalIpcCarrierError::io(
                LocalIpcCarrierOperation::ReceiveFrame,
                errno_to_io(source),
            )),
        };
        self.retain_result(result)
    }

    /// Connect to a caller-selected endpoint before the monotonic `deadline`.
    ///
    /// # Errors
    ///
    /// Returns a deadline or redacted typed host failure. Connecting proves
    /// transport reachability only; it does not authenticate either endpoint.
    pub fn connect(
        path: &Path,
        limits: LocalIpcFrameLimits,
        deadline: Instant,
    ) -> Result<Self, LocalIpcCarrierError> {
        ensure_before_deadline(deadline, LocalIpcCarrierOperation::Connect, 0)?;
        let address = UnixAddr::new(path).map_err(|source| {
            LocalIpcCarrierError::io(LocalIpcCarrierOperation::Connect, errno_to_io(source))
        })?;
        #[cfg(target_os = "linux")]
        let socket_flags = SockFlag::SOCK_CLOEXEC | SockFlag::SOCK_NONBLOCK;
        #[cfg(target_os = "macos")]
        let socket_flags = SockFlag::empty();
        let descriptor = socket(AddressFamily::Unix, SockType::Stream, socket_flags, None)
            .map_err(|source| {
                LocalIpcCarrierError::io(LocalIpcCarrierOperation::Connect, errno_to_io(source))
            })?;

        #[cfg(target_os = "macos")]
        fcntl(&descriptor, FcntlArg::F_SETFD(FdFlag::FD_CLOEXEC)).map_err(|source| {
            LocalIpcCarrierError::io(LocalIpcCarrierOperation::Connect, errno_to_io(source))
        })?;
        let stream = UnixStream::from(descriptor);
        stream.set_nonblocking(true).map_err(|source| {
            LocalIpcCarrierError::io(LocalIpcCarrierOperation::Connect, source)
        })?;

        match socket_connect(stream.as_raw_fd(), &address) {
            Ok(()) => {}
            Err(Errno::EINPROGRESS | Errno::EALREADY | Errno::EAGAIN) => {
                wait_ready(
                    &stream,
                    PollFlags::POLLOUT,
                    deadline,
                    LocalIpcCarrierOperation::Connect,
                    0,
                )?;
                let pending = getsockopt(&stream, sockopt::SocketError).map_err(|source| {
                    LocalIpcCarrierError::io(LocalIpcCarrierOperation::Connect, errno_to_io(source))
                })?;
                if pending != 0 {
                    return Err(LocalIpcCarrierError::io(
                        LocalIpcCarrierOperation::Connect,
                        io::Error::from_raw_os_error(pending),
                    ));
                }
            }
            Err(source) => {
                return Err(LocalIpcCarrierError::io(
                    LocalIpcCarrierOperation::Connect,
                    errno_to_io(source),
                ));
            }
        }

        Ok(Self {
            stream,
            limits,
            state: Cell::new(CarrierState::Ready),
        })
    }

    fn from_connected(
        stream: UnixStream,
        limits: LocalIpcFrameLimits,
    ) -> Result<Self, LocalIpcCarrierError> {
        stream.set_nonblocking(true).map_err(|source| {
            LocalIpcCarrierError::io(LocalIpcCarrierOperation::Connect, source)
        })?;
        Ok(Self {
            stream,
            limits,
            state: Cell::new(CarrierState::Ready),
        })
    }

    /// Reject every operation after frame alignment becomes unknowable.
    fn ensure_usable(&self) -> Result<(), LocalIpcCarrierError> {
        match self.state.get() {
            CarrierState::Ready => Ok(()),
            CarrierState::Unusable => Err(LocalIpcCarrierError::ConnectionUnusable),
        }
    }

    /// Make a connection-fatal error an irreversible carrier state transition.
    fn retain_result<T>(
        &self,
        result: Result<T, LocalIpcCarrierError>,
    ) -> Result<T, LocalIpcCarrierError> {
        if result
            .as_ref()
            .is_err_and(|error| error.invalidates_connection())
        {
            self.state.set(CarrierState::Unusable);
        }
        result
    }
}

impl LocalIpcFramePort for UnixSocketCarrier {
    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
        self.ensure_usable()?;
        self.retain_result(observe_peer_credentials(&self.stream))
    }

    fn send_frame(
        &mut self,
        payload: &[u8],
        deadline: Instant,
    ) -> Result<(), LocalIpcCarrierError> {
        self.ensure_usable()?;
        let declared_bytes = payload.len();
        let limit_bytes = self.limits.max_payload_bytes().get() as usize;
        if declared_bytes > limit_bytes {
            return Err(LocalIpcCarrierError::FrameTooLarge {
                operation: LocalIpcCarrierOperation::SendFrame,
                declared_bytes,
                limit_bytes,
            });
        }
        let declared_u32 =
            u32::try_from(declared_bytes).map_err(|_| LocalIpcCarrierError::FrameTooLarge {
                operation: LocalIpcCarrierOperation::SendFrame,
                declared_bytes,
                limit_bytes,
            })?;
        let result =
            write_before_deadline(&mut self.stream, &declared_u32.to_be_bytes(), deadline, 0)
                .and_then(|()| {
                    write_before_deadline(&mut self.stream, payload, deadline, LENGTH_PREFIX_BYTES)
                });
        self.retain_result(result)
    }

    fn receive_frame(&mut self, deadline: Instant) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        self.ensure_usable()?;
        let result = self.receive_frame_ready(deadline);
        self.retain_result(result)
    }
}

impl UnixSocketCarrier {
    /// Receive while the caller has already established [`CarrierState::Ready`].
    fn receive_frame_ready(
        &mut self,
        deadline: Instant,
    ) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        let mut prefix = [0_u8; LENGTH_PREFIX_BYTES];
        read_before_deadline(
            &mut self.stream,
            &mut prefix,
            deadline,
            LocalIpcFrameSection::LengthPrefix,
            0,
        )?;
        let declared_bytes = u32::from_be_bytes(prefix) as usize;
        let limit_bytes = self.limits.max_payload_bytes().get() as usize;
        if declared_bytes > limit_bytes {
            return Err(LocalIpcCarrierError::FrameTooLarge {
                operation: LocalIpcCarrierOperation::ReceiveFrame,
                declared_bytes,
                limit_bytes,
            });
        }

        let mut payload = Vec::new();
        payload
            .try_reserve_exact(declared_bytes)
            .map_err(|_| LocalIpcCarrierError::FrameAllocationFailed { declared_bytes })?;
        payload.resize(declared_bytes, 0);
        read_before_deadline(
            &mut self.stream,
            &mut payload,
            deadline,
            LocalIpcFrameSection::Payload,
            LENGTH_PREFIX_BYTES,
        )?;
        Ok(payload.into_boxed_slice())
    }
}

fn ensure_before_deadline(
    deadline: Instant,
    operation: LocalIpcCarrierOperation,
    frame_bytes_transferred: usize,
) -> Result<(), LocalIpcCarrierError> {
    if Instant::now() >= deadline {
        Err(LocalIpcCarrierError::DeadlineElapsed {
            operation,
            frame_bytes_transferred,
        })
    } else {
        Ok(())
    }
}

fn wait_ready(
    descriptor: &impl AsFd,
    interest: PollFlags,
    deadline: Instant,
    operation: LocalIpcCarrierOperation,
    frame_bytes_transferred: usize,
) -> Result<(), LocalIpcCarrierError> {
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(LocalIpcCarrierError::DeadlineElapsed {
                operation,
                frame_bytes_transferred,
            });
        }
        let timeout = poll_timeout_ceil(remaining);
        let mut descriptors = [PollFd::new(descriptor.as_fd(), interest)];
        match poll(&mut descriptors, timeout) {
            Ok(0) => {
                return Err(LocalIpcCarrierError::DeadlineElapsed {
                    operation,
                    frame_bytes_transferred,
                });
            }
            Ok(_) => return Ok(()),
            Err(Errno::EINTR) => continue,
            Err(source) => {
                return Err(LocalIpcCarrierError::io_after_progress(
                    operation,
                    frame_bytes_transferred,
                    errno_to_io(source),
                ));
            }
        }
    }
}

fn poll_timeout_ceil(duration: Duration) -> PollTimeout {
    let extra_millisecond = u128::from(!duration.subsec_nanos().is_multiple_of(1_000_000));
    let milliseconds = duration.as_millis().saturating_add(extra_millisecond);
    PollTimeout::try_from(milliseconds).unwrap_or(PollTimeout::MAX)
}

fn write_before_deadline(
    stream: &mut UnixStream,
    bytes: &[u8],
    deadline: Instant,
    preceding_frame_bytes: usize,
) -> Result<(), LocalIpcCarrierError> {
    let mut written = 0;
    while written < bytes.len() {
        let frame_bytes_transferred = preceding_frame_bytes + written;
        ensure_before_deadline(
            deadline,
            LocalIpcCarrierOperation::SendFrame,
            frame_bytes_transferred,
        )?;
        match stream.write(&bytes[written..]) {
            Ok(0) => {
                return Err(LocalIpcCarrierError::io_after_progress(
                    LocalIpcCarrierOperation::SendFrame,
                    frame_bytes_transferred,
                    io::Error::from(io::ErrorKind::WriteZero),
                ));
            }
            Ok(count) => written += count,
            Err(source) if source.kind() == io::ErrorKind::Interrupted => continue,
            Err(source) if source.kind() == io::ErrorKind::WouldBlock => wait_ready(
                stream,
                PollFlags::POLLOUT,
                deadline,
                LocalIpcCarrierOperation::SendFrame,
                frame_bytes_transferred,
            )?,
            Err(source) => {
                return Err(LocalIpcCarrierError::io_after_progress(
                    LocalIpcCarrierOperation::SendFrame,
                    frame_bytes_transferred,
                    source,
                ));
            }
        }
    }
    Ok(())
}

fn read_before_deadline(
    stream: &mut UnixStream,
    bytes: &mut [u8],
    deadline: Instant,
    section: LocalIpcFrameSection,
    preceding_frame_bytes: usize,
) -> Result<(), LocalIpcCarrierError> {
    let mut read = 0;
    while read < bytes.len() {
        let frame_bytes_transferred = preceding_frame_bytes + read;
        ensure_before_deadline(
            deadline,
            LocalIpcCarrierOperation::ReceiveFrame,
            frame_bytes_transferred,
        )?;
        match stream.read(&mut bytes[read..]) {
            Ok(0) if section == LocalIpcFrameSection::LengthPrefix && read == 0 => {
                return Err(LocalIpcCarrierError::EndOfStream);
            }
            Ok(0) => {
                return Err(LocalIpcCarrierError::TruncatedFrame {
                    section,
                    expected_bytes: bytes.len(),
                    received_bytes: read,
                });
            }
            Ok(count) => read += count,
            Err(source) if source.kind() == io::ErrorKind::Interrupted => continue,
            Err(source) if source.kind() == io::ErrorKind::WouldBlock => wait_ready(
                stream,
                PollFlags::POLLIN,
                deadline,
                LocalIpcCarrierOperation::ReceiveFrame,
                frame_bytes_transferred,
            )?,
            Err(source) => {
                return Err(LocalIpcCarrierError::io_after_progress(
                    LocalIpcCarrierOperation::ReceiveFrame,
                    frame_bytes_transferred,
                    source,
                ));
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn observe_peer_credentials(
    stream: &UnixStream,
) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
    let credentials = getsockopt(stream, sockopt::PeerCredentials).map_err(|source| {
        LocalIpcCarrierError::io(
            LocalIpcCarrierOperation::ObservePeerCredentials,
            errno_to_io(source),
        )
    })?;
    let process_id = u32::try_from(credentials.pid()).map_err(|_| {
        LocalIpcCarrierError::io(
            LocalIpcCarrierOperation::ObservePeerCredentials,
            io::Error::new(
                io::ErrorKind::InvalidData,
                "kernel returned a negative peer process identifier",
            ),
        )
    })?;
    Ok(LocalIpcPeerCredentials::new(
        credentials.uid(),
        credentials.gid(),
        Some(process_id),
    ))
}

#[cfg(target_os = "macos")]
fn observe_peer_credentials(
    stream: &UnixStream,
) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
    let (user_id, group_id) = nix::unistd::getpeereid(stream).map_err(|source| {
        LocalIpcCarrierError::io(
            LocalIpcCarrierOperation::ObservePeerCredentials,
            errno_to_io(source),
        )
    })?;
    let raw_process_id = getsockopt(stream, sockopt::LocalPeerPid).map_err(|source| {
        LocalIpcCarrierError::io(
            LocalIpcCarrierOperation::ObservePeerCredentials,
            errno_to_io(source),
        )
    })?;
    let process_id = u32::try_from(raw_process_id).map_err(|_| {
        LocalIpcCarrierError::io(
            LocalIpcCarrierOperation::ObservePeerCredentials,
            io::Error::new(
                io::ErrorKind::InvalidData,
                "kernel returned a negative peer process identifier",
            ),
        )
    })?;
    Ok(LocalIpcPeerCredentials::new(
        user_id.as_raw(),
        group_id.as_raw(),
        Some(process_id),
    ))
}

fn errno_to_io(error: Errno) -> io::Error {
    io::Error::from_raw_os_error(error as i32)
}
