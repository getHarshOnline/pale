//! Boundary tests for scoped physical resident-process recovery.

use pale_core::records::{ControlIncarnationIdV1, ControlStoreIdV1, ResidentPrincipalIdV1};
use pale_ports::{
    ControlStoreError, PrincipalResidentWriterReadPort, PrincipalResidentWriterSnapshot,
    PrincipalTrustStoreBinding, PrincipalWriterGuardPort,
};

use super::{recover_writer_snapshot, ResidentServicePhysicalProcessRecoveryError};

struct HeldWriter(ResidentPrincipalIdV1);

impl PrincipalWriterGuardPort for HeldWriter {
    fn principal_id(&self) -> &ResidentPrincipalIdV1 {
        &self.0
    }
}

#[derive(Default)]
struct AbsentWriterReader {
    reads: usize,
    maximum_record_bytes: Option<usize>,
}

impl PrincipalResidentWriterReadPort for AbsentWriterReader {
    fn read_current_writer(
        &mut self,
        _binding: &PrincipalTrustStoreBinding,
        maximum_record_bytes: usize,
    ) -> Result<Option<PrincipalResidentWriterSnapshot>, ControlStoreError> {
        self.reads += 1;
        self.maximum_record_bytes = Some(maximum_record_bytes);
        Ok(None)
    }
}

fn principal(value: &str) -> ResidentPrincipalIdV1 {
    ResidentPrincipalIdV1::parse(value).expect("principal")
}

fn binding(principal_id: ResidentPrincipalIdV1) -> PrincipalTrustStoreBinding {
    PrincipalTrustStoreBinding::new(
        ControlStoreIdV1::parse("trust-store-1").expect("trust store"),
        ControlIncarnationIdV1::parse("trust-incarnation-1").expect("trust incarnation"),
        principal_id,
    )
}

#[test]
fn zero_writer_bound_stops_before_physical_read() {
    let principal_id = principal("principal-1");
    let held = HeldWriter(principal_id.clone());
    let mut reader = AbsentWriterReader::default();

    let error = recover_writer_snapshot(&held, &mut reader, &binding(principal_id), 0)
        .expect_err("zero limit must be rejected");

    assert!(matches!(
        error,
        ResidentServicePhysicalProcessRecoveryError::WriterReadLimit
    ));
    assert_eq!(reader.reads, 0);
}

#[test]
fn substituted_writer_guard_stops_before_physical_read() {
    let held = HeldWriter(principal("principal-2"));
    let mut reader = AbsentWriterReader::default();

    let error =
        recover_writer_snapshot(&held, &mut reader, &binding(principal("principal-1")), 1024)
            .expect_err("substituted guard must be rejected");

    assert!(matches!(
        error,
        ResidentServicePhysicalProcessRecoveryError::WriterGuardAssociation
    ));
    assert_eq!(reader.reads, 0);
}

#[test]
fn exact_writer_bound_reaches_the_physical_reader() {
    let principal_id = principal("principal-1");
    let held = HeldWriter(principal_id.clone());
    let mut reader = AbsentWriterReader::default();

    let error = recover_writer_snapshot(&held, &mut reader, &binding(principal_id), 4096)
        .expect_err("absent writer remains explicit");

    assert!(matches!(
        error,
        ResidentServicePhysicalProcessRecoveryError::WriterUnavailable
    ));
    assert_eq!(reader.reads, 1);
    assert_eq!(reader.maximum_record_bytes, Some(4096));
}
