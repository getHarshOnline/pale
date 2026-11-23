//! Resident service peer-admission tests.

use std::{cell::Cell, rc::Rc};

use super::*;

struct AdmitEveryPeer {
    calls: Rc<Cell<usize>>,
}

impl ResidentPeerAdmissionPort for AdmitEveryPeer {
    fn admit_peer(
        &mut self,
        credentials: LocalIpcPeerCredentials,
    ) -> Result<ResidentPeerAdmissionWitness, InvalidResidentRecordV1> {
        self.calls.set(self.calls.get() + 1);
        Ok(ResidentPeerAdmissionWitness::new(credentials))
    }
}

struct RejectCurrentEvidence {
    reads: Rc<Cell<usize>>,
}

impl ResidentServiceCurrentRequestEvidencePort for RejectCurrentEvidence {
    fn read_current_request_evidence(
        &mut self,
        _: ResidentServiceCurrentRequestSelector<'_>,
    ) -> Result<ResidentServiceCurrentRequestEvidence, InvalidResidentRecordV1> {
        self.reads.set(self.reads.get() + 1);
        Err(InvalidResidentRecordV1::ContextualAdmission {
            record: "current request authority",
            invariant: "the test rejects current authority after peer admission",
        })
    }
}

fn operation_request() -> ResidentOperationFrameRequest {
    ResidentOperationFrameRequest::StartInvoke {
        signed_common_request: "{}".to_owned(),
    }
}

#[test]
fn peer_admission_is_evaluated_for_each_connection() {
    let calls = Rc::new(Cell::new(0));
    let mut adapter = PeerAdmissionAdapter(Box::new(AdmitEveryPeer {
        calls: Rc::clone(&calls),
    }));
    let request = operation_request();
    let first = LocalIpcPeerCredentials::new(501, 20, Some(8_081));
    let second = LocalIpcPeerCredentials::new(501, 20, Some(8_082));

    let first_witness =
        program::ResidentProgramPeerAdmissionPort::admit_peer(&mut adapter, first, &request)
            .expect("first connected process is admitted");
    let second_witness =
        program::ResidentProgramPeerAdmissionPort::admit_peer(&mut adapter, second, &request)
            .expect("second connected process is admitted independently");

    assert_eq!(calls.get(), 2);
    assert_eq!(first_witness.credentials(), first);
    assert_eq!(second_witness.credentials(), second);
    assert_ne!(first_witness, second_witness);
}

#[test]
fn peer_witness_does_not_replace_current_authority_admission() {
    let peer_calls = Rc::new(Cell::new(0));
    let peer = LocalIpcPeerCredentials::new(501, 20, Some(8_081));
    let mut adapter = PeerAdmissionAdapter(Box::new(AdmitEveryPeer {
        calls: Rc::clone(&peer_calls),
    }));
    let witness = program::ResidentProgramPeerAdmissionPort::admit_peer(
        &mut adapter,
        peer,
        &operation_request(),
    )
    .expect("local peer is admitted");
    let reads = Rc::new(Cell::new(0));
    let mut evidence = CurrentEvidenceAdapter(RejectCurrentEvidence {
        reads: Rc::clone(&reads),
    });
    let session_id = SessionAdmissionIdV1::parse("session-1").expect("session id");
    let lease_digest = SessionAdmissionLeaseDigest::parse("a".repeat(64)).expect("session digest");
    let grant_id = SessionGrantIdV1::parse("grant-1").expect("grant id");
    let binding_ref = ResidentRecordRefV1::parse("deployment-binding-1").expect("binding ref");
    let binding_digest = ArtifactDigest::parse("b".repeat(64)).expect("binding digest");

    let result = current_request::ResidentCurrentRequestEvidencePort::read_current_request_evidence(
        &mut evidence,
        current_request::ResidentCurrentRequestSelector {
            peer_witness: &witness,
            session_admission_id: &session_id,
            session_admission_lease_digest: &lease_digest,
            session_grant_id: &grant_id,
            deployment_binding_ref: &binding_ref,
            deployment_binding_sha256: &binding_digest,
        },
    );

    assert!(matches!(
        result,
        Err(current_request::ResidentCurrentRequestEvidenceError::Record(_))
    ));
    assert_eq!(peer_calls.get(), 1);
    assert_eq!(reads.get(), 1);
}
