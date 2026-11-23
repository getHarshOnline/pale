//! Selected connection credentials and the one authenticated application frame.

use std::{io, time::Instant};

use pale_adapter_uds::{AuthenticatedLocalRequest, LocalAuthenticationError};
use pale_ports::{
    CryptographicRandomSourcePort, LocalConnectionSignerPort, LocalConnectionSigningError,
    LocalIpcCarrierError, LocalIpcCarrierOperation, LocalIpcFramePort, LocalIpcPeerCredentials,
};

/// Deployment-selected credentials for mutually authenticated local connections.
///
/// `select_caller` must make a bounded, nonblocking lookup of an explicitly
/// selected connection key. It runs before a challenge and again before
/// application dispatch. A matching key proves no Pale principal or permission;
/// the resident still admits the protected application request independently.
pub struct ResidentConnectionAuthentication<'owner> {
    pub(super) signer: &'owner mut (dyn LocalConnectionSignerPort + Send),
    pub(super) random: &'owner dyn CryptographicRandomSourcePort,
    pub(super) select_caller: &'owner dyn Fn(&[u8; 32]) -> bool,
}

impl<'owner> ResidentConnectionAuthentication<'owner> {
    /// Borrow the selected signer, random source and current caller-key lookup.
    #[must_use]
    pub fn new(
        signer: &'owner mut (dyn LocalConnectionSignerPort + Send),
        random: &'owner dyn CryptographicRandomSourcePort,
        select_caller: &'owner dyn Fn(&[u8; 32]) -> bool,
    ) -> Self {
        Self {
            signer,
            random,
            select_caller,
        }
    }
}

/// Preserve the existing application port while authenticating its response.
///
/// The application gets exactly one verified payload. A response consumes the
/// request before signing or sending, so a failed delivery cannot be repeated.
pub(super) struct AuthenticatedFrame<'borrow> {
    pub(super) carrier: &'borrow mut dyn LocalIpcFramePort,
    pub(super) signer: &'borrow mut (dyn LocalConnectionSignerPort + Send),
    pub(super) request: &'borrow mut Option<AuthenticatedLocalRequest>,
    pub(super) read: bool,
}

impl LocalIpcFramePort for AuthenticatedFrame<'_> {
    fn authenticated_caller_key(&self) -> Option<[u8; 32]> {
        self.request.as_ref().map(|request| *request.caller_key())
    }

    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
        self.carrier.peer_credentials()
    }

    fn receive_frame(&mut self, deadline: Instant) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        if self.read || Instant::now() >= deadline {
            return Err(closed(LocalIpcCarrierOperation::ReceiveFrame));
        }
        self.read = true;
        self.request
            .as_ref()
            .map(|request| request.payload().into())
            .ok_or_else(|| closed(LocalIpcCarrierOperation::ReceiveFrame))
    }

    fn send_frame(
        &mut self,
        payload: &[u8],
        deadline: Instant,
    ) -> Result<(), LocalIpcCarrierError> {
        let request = self
            .request
            .take()
            .ok_or_else(|| closed(LocalIpcCarrierOperation::SendFrame))?;
        if !self.read || Instant::now() >= deadline {
            return Err(closed(LocalIpcCarrierOperation::SendFrame));
        }
        let mut carrier = ResponseDeadline {
            carrier: self.carrier,
            deadline,
        };
        let mut signer = ResponseSigner {
            signer: self.signer,
            deadline,
        };
        request
            .respond(&mut carrier, &mut signer, payload)
            .map_err(|error| {
                LocalIpcCarrierError::io(
                    LocalIpcCarrierOperation::SendFrame,
                    io::Error::other(error),
                )
            })
    }
}

struct ResponseSigner<'borrow> {
    signer: &'borrow mut dyn LocalConnectionSignerPort,
    deadline: Instant,
}

impl LocalConnectionSignerPort for ResponseSigner<'_> {
    fn public_key(&self) -> [u8; 32] {
        self.signer.public_key()
    }

    fn sign(
        &mut self,
        message: &[u8],
        deadline: Instant,
    ) -> Result<[u8; 64], LocalConnectionSigningError> {
        self.signer.sign(message, deadline.min(self.deadline))
    }
}

/// Keep a tighter application deadline through response signing and delivery.
struct ResponseDeadline<'borrow> {
    carrier: &'borrow mut dyn LocalIpcFramePort,
    deadline: Instant,
}

impl LocalIpcFramePort for ResponseDeadline<'_> {
    fn peer_credentials(&self) -> Result<LocalIpcPeerCredentials, LocalIpcCarrierError> {
        self.carrier.peer_credentials()
    }

    fn send_frame(
        &mut self,
        payload: &[u8],
        deadline: Instant,
    ) -> Result<(), LocalIpcCarrierError> {
        self.carrier
            .send_frame(payload, deadline.min(self.deadline))
    }

    fn receive_frame(&mut self, _: Instant) -> Result<Box<[u8]>, LocalIpcCarrierError> {
        Err(closed(LocalIpcCarrierOperation::ReceiveFrame))
    }
}

fn closed(operation: LocalIpcCarrierOperation) -> LocalIpcCarrierError {
    LocalIpcCarrierError::io(operation, io::Error::other(LocalAuthenticationError::Frame))
}
