//! Shared runtime-signer and observation issuance checks for operation attempts.

use super::*;

pub(super) fn validate_runtime_signer(
    signer: &SigningAuthorityBindingV1,
    current: &AmbientSessionAdmissionContext<'_>,
    schema: &str,
) -> Result<(), OperationAttemptError> {
    let runtime = signer
        .integrated_runtime_authority()
        .ok_or(OperationAttemptError::Association)?;
    let current_runtime = current
        .runtime_authority
        .ok_or(OperationAttemptError::Association)?;
    if &runtime.deployment_runtime_authority_ref != current_runtime.authority_ref
        || runtime.deployment_runtime_authority_sha256.as_str()
            != current_runtime
                .authority
                .canonical_deployment_runtime_authority_sha256()
                .as_str()
        || signer.principal_id() != current_runtime.authority.principal_id()
        || signer.signer_key_id() != current_runtime.authority.subject_key_id().as_str()
        || signer.signer_key_epoch() != current_runtime.authority.subject_key_epoch()
        || !current_runtime
            .authority
            .permitted_record_schema_ids()
            .iter()
            .any(|permitted| permitted.as_str() == schema)
    {
        return Err(OperationAttemptError::Association);
    }
    Ok(())
}

pub(in crate::resident) fn issue_observation<Rotation, Sign>(
    context: &OperationObservationAdmissionContextV1<'_>,
    state: OperationObservationStateV1,
    observed_at: UnixTimeV1,
    admit_rotation: Rotation,
    sign: Sign,
) -> Result<OperationObservationV1, OperationAttemptError>
where
    Rotation: FnOnce(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
    Sign: FnOnce(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    issue_observation_with_state_admission(
        context,
        state.clone(),
        observed_at,
        |proposed| {
            if proposed == &state {
                Ok(())
            } else {
                Err(InvalidResidentRecordV1::ContextualAdmission {
                    record: "operation attempt",
                    invariant: "exact source-selected transition",
                })
            }
        },
        admit_rotation,
        sign,
    )
}

pub(super) fn issue_observation_with_state_admission<Evidence, Rotation, Sign>(
    context: &OperationObservationAdmissionContextV1<'_>,
    state: OperationObservationStateV1,
    observed_at: UnixTimeV1,
    admit_state: Evidence,
    admit_rotation: Rotation,
    sign: Sign,
) -> Result<OperationObservationV1, OperationAttemptError>
where
    Evidence: FnOnce(&OperationObservationStateV1) -> Result<(), InvalidResidentRecordV1>,
    Rotation: FnOnce(
        &ResidentRecordRefV1,
        &ArtifactDigest,
        &ResidentRecordRefV1,
        &ArtifactDigest,
    ) -> Result<(), InvalidResidentRecordV1>,
    Sign: FnOnce(
        &[u8],
        &OperationObservationPreimageV1,
        &OperationObservationDigest,
    ) -> Result<CanonicalSignatureV1, InvalidResidentRecordV1>,
{
    let predecessor = context
        .predecessor
        .ok_or(OperationAttemptError::Association)?;
    let signer = context.signed.signing_authority;
    let runtime = signer
        .integrated_runtime_authority()
        .ok_or(OperationAttemptError::Association)?;
    let observation = OperationObservationV1::issue_contextual(
        OperationObservationPreimageV1 {
            schema_id: SchemaId::parse(OPERATION_OBSERVATION_V1_SCHEMA)
                .map_err(|_| OperationAttemptError::Encoding)?,
            schema_sha256: context.signed.expected_schema_sha256.clone(),
            operation_handle_id: context.handle.operation_handle_id().clone(),
            operation_handle_ref: context.handle_ref.clone(),
            operation_handle_sha256: context.handle.canonical_operation_handle_sha256().clone(),
            revision: predecessor
                .1
                .revision()
                .checked_successor()
                .ok_or(OperationAttemptError::Association)?,
            predecessor_ref: Some(predecessor.0.clone()),
            predecessor_sha256: Some(
                predecessor
                    .1
                    .canonical_operation_observation_sha256()
                    .clone(),
            ),
            state: state.clone(),
            observed_at,
            deployment_runtime_authority_ref: runtime.deployment_runtime_authority_ref.clone(),
            deployment_runtime_authority_sha256: runtime
                .deployment_runtime_authority_sha256
                .clone(),
            issuer_principal_id: signer.principal_id().clone(),
            issuer_key_id: ResidentKeyIdV1::parse(signer.signer_key_id())?,
            issuer_key_epoch: *signer.signer_key_epoch(),
        },
        context,
        admit_state,
        admit_rotation,
        sign,
    )?;
    Ok(observation)
}
