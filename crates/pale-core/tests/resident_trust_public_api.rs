//! Verifies that external owner crates can construct trust-head witnesses.

use pale_core::records::*;

fn active_writer_head<'a>(
    record_ref: &'a ResidentRecordRefV1,
    record: &'a PrincipalResidentWriterFenceV1,
) -> Result<PrincipalResidentWriterFenceHeadAdmissionStateV1<'a>, InvalidResidentRecordV1> {
    PrincipalResidentWriterFenceHeadAdmissionStateV1::active(record_ref, record)
}

fn retired_writer_head<'a>(
    record_ref: &'a ResidentRecordRefV1,
    record: &'a PrincipalResidentWriterFenceV1,
) -> Result<PrincipalResidentWriterFenceHeadAdmissionStateV1<'a>, InvalidResidentRecordV1> {
    PrincipalResidentWriterFenceHeadAdmissionStateV1::retired(record_ref, record)
}

fn active_producer_head<'a>(
    record_ref: &'a ResidentRecordRefV1,
    record: &'a ProducerIncarnationV1,
) -> Result<ProducerIncarnationHeadAdmissionStateV1<'a>, InvalidResidentRecordV1> {
    ProducerIncarnationHeadAdmissionStateV1::active(record_ref, record)
}

fn retired_producer_head<'a>(
    record_ref: &'a ResidentRecordRefV1,
    record: &'a ProducerIncarnationV1,
    retirement: &'a ProducerIncarnationRetirementV1,
) -> Result<ProducerIncarnationHeadAdmissionStateV1<'a>, InvalidResidentRecordV1> {
    ProducerIncarnationHeadAdmissionStateV1::retired(record_ref, record, retirement)
}

#[test]
fn trust_head_constructors_are_reachable_from_an_external_crate() {
    let _ = active_writer_head;
    let _ = retired_writer_head;
    let _ = active_producer_head;
    let _ = retired_producer_head;
}
