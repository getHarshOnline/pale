//! Service-local limits and completion laws; no endpoint or OS DNS calls.

use super::*;

fn bound(value: usize) -> NonZeroUsize {
    NonZeroUsize::new(value).unwrap()
}

fn limits() -> A2aServiceLimits {
    A2aServiceLimits {
        exchanges: bound(2),
        dns_jobs: bound(1),
        dns_addresses: bound(4),
        exchange: A2aExchangeLimits {
            request_bytes: bound(1024),
            response_bytes: bound(2048),
            evidence_bytes: bound(4096),
            line_bytes: bound(512),
            event_bytes: bound(1024),
            events: bound(4),
            queued_events: bound(2),
            connect_timeout: Duration::from_secs(1),
            read_timeout: Duration::from_secs(2),
            total_timeout: Duration::from_secs(3),
        },
        certificate_bytes: bound(64),
        certificates: bound(2),
    }
}

#[test]
fn every_request_ceiling_is_limited_by_its_service() {
    let maximum = limits().exchange;
    check_limits(maximum, maximum).unwrap();
    let mutate: [fn(&mut A2aExchangeLimits); 11] = [
        |v| v.request_bytes = bound(1025),
        |v| v.response_bytes = bound(2049),
        |v| v.evidence_bytes = bound(4097),
        |v| v.line_bytes = bound(513),
        |v| v.event_bytes = bound(1025),
        |v| v.events = bound(5),
        |v| v.queued_events = bound(3),
        |v| v.connect_timeout = Duration::from_millis(1001),
        |v| v.read_timeout = Duration::from_millis(2001),
        |v| v.total_timeout = Duration::from_millis(3001),
        |v| v.total_timeout = Duration::ZERO,
    ];
    for mutate in mutate {
        let mut changed = maximum;
        mutate(&mut changed);
        assert!(check_limits(changed, maximum).is_err());
    }
}

#[test]
fn trust_identity_preserves_certificate_boundaries_and_rejects_excess() {
    // Synthetic bytes test framing only; certificate validity belongs to TLS.
    let first = A2aTlsTrust::CertificatesOnly(vec![vec![1], vec![2, 3]]);
    let second = A2aTlsTrust::CertificatesOnly(vec![vec![1, 2], vec![3]]);
    assert_ne!(
        tls_trust_input_digest(&first, limits()).unwrap(),
        tls_trust_input_digest(&second, limits()).unwrap()
    );
    assert_eq!(
        tls_trust_input_digest(&first, limits()).unwrap(),
        tls_trust_input_digest(&first, limits()).unwrap()
    );
    for trust in [
        A2aTlsTrust::CertificatesOnly(vec![]),
        A2aTlsTrust::CertificatesOnly(vec![vec![1]; 3]),
        A2aTlsTrust::CertificatesOnly(vec![vec![1; 65]]),
    ] {
        assert!(tls_trust_input_digest(&trust, limits()).is_err());
    }
}

#[test]
fn interrupted_producer_reports_cancellation_but_preserves_an_established_result() {
    let terminal = Arc::new(Mutex::new(None));
    drop(TerminalGuard(terminal.clone()));
    assert_eq!(
        *terminal.lock().unwrap(),
        Some(Err(A2aExchangeError::Cancelled))
    );
    for result in [Ok(()), Err(A2aExchangeError::Timeout)] {
        *terminal.lock().unwrap() = Some(result);
        drop(TerminalGuard(terminal.clone()));
        assert_eq!(*terminal.lock().unwrap(), Some(result));
    }
}
