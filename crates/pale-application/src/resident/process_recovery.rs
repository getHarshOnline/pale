//! Scoped recovery of physical evidence for one resident process.
//!
//! Deployment code supplies an already-held writer guard, exact physical
//! readers, admitted recovery owners, and caller-opened deployments. This
//! module keeps the recovered writer and trust evidence alive while the caller
//! assembles an existing resident service. It does not discover stores or
//! endpoints, open a database, choose credentials, or create deployments.

use std::num::NonZeroUsize;

use pale_ports::{
    ControlStoreError, PrincipalResidentWriterReadPort, PrincipalResidentWriterSnapshot,
    PrincipalTrustStoreBinding, PrincipalWriterGuardPort,
};

use super::{
    PrincipalProducerIncarnationRecoveryAdmission, ResidentServiceDeploymentOpen,
    ResidentServicePhysicalTrustRecoveryInput, ResidentServiceProcessInput,
    ResidentServiceTaskLimits, ResidentServiceTrustEvidencePorts, ResidentServiceTrustFrontier,
    ResidentServiceTrustPhysicalPorts, ResidentServiceTrustRecoveryError,
};

/// Physical owners and limits needed to recover one resident process scope.
pub struct ResidentServicePhysicalProcessRecoveryInput<'a, WriterGuard, WriterAdmission> {
    /// Native guard that already holds the principal writer lock.
    pub held_writer: WriterGuard,
    /// Physical reader for the current writer and its checkpoint.
    pub writer_reader: &'a mut dyn PrincipalResidentWriterReadPort,
    /// Maximum combined bytes copied for the writer and checkpoint records.
    pub maximum_writer_record_bytes: usize,
    /// Record-specific admission owner retained by the assembled service.
    pub writer_admission: WriterAdmission,
    /// Bootstrap evidence and limits for physical trust recovery.
    pub trust_input: ResidentServicePhysicalTrustRecoveryInput,
    /// Physical trust readers selected by deployment code.
    pub trust_physical: ResidentServiceTrustPhysicalPorts<'a>,
    /// Admission owner for recovered producer-incarnation records.
    pub producer_admission: &'a mut dyn PrincipalProducerIncarnationRecoveryAdmission,
    /// Historical keys, signatures, and deployment-authority admission.
    pub trust_evidence: ResidentServiceTrustEvidencePorts<'a>,
    /// Complete caller-opened deployment set.
    pub deployments: Vec<ResidentServiceDeploymentOpen<'a>>,
    /// Maximum ready deployments retained by the process.
    pub maximum_ready_deployments: NonZeroUsize,
    /// Bounded process-local task ownership.
    pub task_limits: ResidentServiceTaskLimits,
}

/// Why physical evidence could not form one scoped resident process input.
#[derive(Debug, thiserror::Error)]
pub enum ResidentServicePhysicalProcessRecoveryError {
    /// The writer-record byte limit was zero.
    #[error("resident writer read limit must be positive")]
    WriterReadLimit,
    /// The native guard belongs to another principal.
    #[error("resident writer guard does not match the admitted principal")]
    WriterGuardAssociation,
    /// The physical writer snapshot could not be read completely.
    #[error("resident writer evidence could not be read")]
    WriterRead(#[source] ControlStoreError),
    /// No current writer exists for the admitted principal and store.
    #[error("resident writer evidence is unavailable")]
    WriterUnavailable,
    /// The physical writer snapshot names another store or principal.
    #[error("resident writer evidence does not match the admitted trust-store binding")]
    WriterAssociation,
    /// Physical trust evidence did not form one admitted frontier.
    #[error(transparent)]
    Trust(#[from] ResidentServiceTrustRecoveryError),
}

/// Recover physical writer and trust evidence, then lend one process input.
///
/// The callback can use the process only during this recovery scope. Its return
/// type cannot borrow the writer snapshot, trust frontier, or process input. A
/// callback can return its own `Result`; this function changes only physical
/// recovery failures.
///
/// # Errors
///
/// Returns an error for an empty writer-read bound, a substituted writer guard
/// or snapshot, an absent or failed writer read, or failed trust recovery.
pub fn with_recovered_resident_service_process<
    'physical,
    WriterGuard,
    WriterAdmission,
    Output,
    UseProcess,
>(
    input: ResidentServicePhysicalProcessRecoveryInput<'physical, WriterGuard, WriterAdmission>,
    use_process: UseProcess,
) -> Result<Output, ResidentServicePhysicalProcessRecoveryError>
where
    WriterGuard: PrincipalWriterGuardPort,
    UseProcess: for<'evidence> FnOnce(
        ResidentServiceProcessInput<'evidence, 'physical, WriterGuard, WriterAdmission>,
    ) -> Output,
{
    let ResidentServicePhysicalProcessRecoveryInput {
        held_writer,
        writer_reader,
        maximum_writer_record_bytes,
        writer_admission,
        trust_input,
        trust_physical,
        producer_admission,
        trust_evidence,
        deployments,
        maximum_ready_deployments,
        task_limits,
    } = input;

    let writer_binding = trust_store_binding(&trust_input);
    let writer_snapshot = recover_writer_snapshot(
        &held_writer,
        writer_reader,
        &writer_binding,
        maximum_writer_record_bytes,
    )?;
    let trust = ResidentServiceTrustFrontier::recover_from_physical(
        trust_input,
        trust_physical,
        producer_admission,
        trust_evidence,
    )?;

    Ok(use_process(ResidentServiceProcessInput {
        held_writer,
        writer_snapshot: &writer_snapshot,
        writer_admission,
        trust: &trust,
        deployments,
        maximum_ready_deployments,
        task_limits,
    }))
}

fn trust_store_binding(
    input: &ResidentServicePhysicalTrustRecoveryInput,
) -> PrincipalTrustStoreBinding {
    let bootstrap = input.bootstrap.atom_view();
    let (_, checkpoint) = bootstrap.trust_checkpoint_entry();
    PrincipalTrustStoreBinding::new(
        checkpoint.trust_store_id().clone(),
        checkpoint.trust_store_incarnation_id().clone(),
        checkpoint.principal_id().clone(),
    )
}

fn recover_writer_snapshot<WriterGuard>(
    held_writer: &WriterGuard,
    writer_reader: &mut dyn PrincipalResidentWriterReadPort,
    binding: &PrincipalTrustStoreBinding,
    maximum_record_bytes: usize,
) -> Result<PrincipalResidentWriterSnapshot, ResidentServicePhysicalProcessRecoveryError>
where
    WriterGuard: PrincipalWriterGuardPort,
{
    if maximum_record_bytes == 0 {
        return Err(ResidentServicePhysicalProcessRecoveryError::WriterReadLimit);
    }
    if held_writer.principal_id() != binding.principal_id() {
        return Err(ResidentServicePhysicalProcessRecoveryError::WriterGuardAssociation);
    }
    let snapshot = writer_reader
        .read_current_writer(binding, maximum_record_bytes)
        .map_err(ResidentServicePhysicalProcessRecoveryError::WriterRead)?
        .ok_or(ResidentServicePhysicalProcessRecoveryError::WriterUnavailable)?;
    if snapshot.binding() != binding {
        return Err(ResidentServicePhysicalProcessRecoveryError::WriterAssociation);
    }
    Ok(snapshot)
}

#[cfg(test)]
#[path = "../../tests/internal/resident/process_recovery.rs"]
mod tests;
