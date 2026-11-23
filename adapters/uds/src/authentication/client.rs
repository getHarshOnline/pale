//! Authenticate a selected resident before releasing one application request.

use super::*;

/// One resident-authenticated carrier awaiting its only application request.
///
/// The caller supplies the carrier and both peer identities. Construction sends
/// no application bytes. The value owns that same carrier, so its verified peer
/// cannot be replaced before delivery. It establishes no application permission.
#[must_use = "exchange one request or drop the authenticated connection"]
pub struct AuthenticatedLocalClient<C> {
    carrier: C,
    transcript: Transcript,
    deadline: Instant,
}

impl<C: LocalIpcFramePort> AuthenticatedLocalClient<C> {
    /// Verify the pinned resident on this carrier using fresh challenges.
    ///
    /// # Errors
    /// Rejects an unavailable carrier, entropy failure, invalid resident proof
    /// or elapsed deadline before any application request is sent.
    pub fn authenticate(
        mut carrier: C,
        caller_key: [u8; 32],
        resident_key: [u8; 32],
        random: &dyn CryptographicRandomSourcePort,
        deadline: Instant,
    ) -> Result<Self, LocalAuthenticationError> {
        let transcript =
            authenticate_peer(&mut carrier, caller_key, resident_key, random, deadline)?;
        Ok(Self {
            carrier,
            transcript,
            deadline,
        })
    }

    /// Consume this connection to send one request and verify its exact response.
    ///
    /// The signer must match the caller selected at authentication. A later
    /// supplied deadline cannot extend the original connection deadline.
    ///
    /// # Errors
    /// Reports whether failure preceded delivery or left completion uncertain.
    /// The consumed connection cannot be reused or retry the request.
    pub fn exchange(
        mut self,
        signer: &mut dyn LocalConnectionSignerPort,
        payload: &[u8],
        deadline: Instant,
    ) -> Result<Box<[u8]>, LocalAuthenticatedExchangeError> {
        exchange_request(
            &mut self.carrier,
            &self.transcript,
            signer,
            payload,
            deadline.min(self.deadline),
        )
    }
}

/// Exchange one request with the pinned resident, without replay or fallback.
///
/// The resident proves possession of `resident_key` before receiving the request.
/// Its response binds both fresh challenges and the exact request. Authentication
/// establishes no application readiness or permission.
///
/// # Errors
/// Rejects invalid bounds, unavailable credentials or entropy, a wrong peer,
/// altered response, carrier failure or expiry. Once sending begins, failure
/// leaves application completion uncertain.
pub fn exchange_authenticated(
    carrier: &mut dyn LocalIpcFramePort,
    signer: &mut dyn LocalConnectionSignerPort,
    resident_key: [u8; 32],
    random: &dyn CryptographicRandomSourcePort,
    payload: &[u8],
    deadline: Instant,
) -> Result<Box<[u8]>, LocalAuthenticatedExchangeError> {
    let transcript = (|| {
        check_deadline(deadline)?;
        check_payload(payload)?;
        authenticate_peer(carrier, signer.public_key(), resident_key, random, deadline)
    })()
    .map_err(|cause| LocalAuthenticatedExchangeError {
        cause,
        delivery: LocalRequestDelivery::NotSent,
    })?;
    exchange_request(carrier, &transcript, signer, payload, deadline)
}

fn authenticate_peer(
    carrier: &mut dyn LocalIpcFramePort,
    caller_key: [u8; 32],
    resident_key: [u8; 32],
    random: &dyn CryptographicRandomSourcePort,
    deadline: Instant,
) -> Result<Transcript, LocalAuthenticationError> {
    check_deadline(deadline)?;
    let mut transcript = Transcript {
        caller: caller_key,
        resident: resident_key,
        caller_nonce: nonce(random)?,
        resident_nonce: [0; 32],
    };
    let mut hello = Vec::with_capacity(80);
    hello.extend_from_slice(HELLO);
    hello.extend_from_slice(&transcript.caller);
    hello.extend_from_slice(&transcript.caller_nonce);
    send(carrier, &hello, deadline)?;
    let challenge = receive(carrier, deadline)?;
    if challenge.len() != 112 || &challenge[..16] != PROOF {
        return Err(LocalAuthenticationError::Frame);
    }
    transcript.resident_nonce = array(&challenge[16..48])?;
    verify(
        &transcript.resident,
        &transcript.message(PROOF_DOMAIN, &[]),
        array(&challenge[48..])?,
    )?;
    check_deadline(deadline)?;
    Ok(transcript)
}

fn exchange_request(
    carrier: &mut dyn LocalIpcFramePort,
    transcript: &Transcript,
    signer: &mut dyn LocalConnectionSignerPort,
    payload: &[u8],
    deadline: Instant,
) -> Result<Box<[u8]>, LocalAuthenticatedExchangeError> {
    let mut delivery = LocalRequestDelivery::NotSent;
    let result = (|| {
        check_deadline(deadline)?;
        check_payload(payload)?;
        if signer.public_key() != transcript.caller {
            return Err(LocalAuthenticationError::Peer);
        }
        let request_digest: [u8; 32] = Sha256::digest(payload).into();
        let signature = signer
            .sign(
                &transcript.message(REQUEST_DOMAIN, &request_digest),
                deadline,
            )
            .map_err(|_| LocalAuthenticationError::Signing)?;
        let frame = signed_frame(REQUEST, signature, payload);
        check_deadline(deadline)?;
        delivery = LocalRequestDelivery::Uncertain;
        carrier
            .send_frame(&frame, deadline)
            .map_err(|_| LocalAuthenticationError::Carrier)?;
        let frame = receive(carrier, deadline)?;
        let (signature, response) = signed_parts(&frame, RESPONSE)?;
        let mut suffix = [0; 64];
        suffix[..32].copy_from_slice(&request_digest);
        suffix[32..].copy_from_slice(&Sha256::digest(response));
        verify(
            &transcript.resident,
            &transcript.message(RESPONSE_DOMAIN, &suffix),
            signature,
        )?;
        check_deadline(deadline)?;
        Ok(response.into())
    })();
    result.map_err(|cause| LocalAuthenticatedExchangeError { cause, delivery })
}
