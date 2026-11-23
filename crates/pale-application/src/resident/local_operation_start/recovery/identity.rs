//! Shared historical handle and lease admission before any protected state escapes.

use super::*;

pub(super) fn admit_handle<Authority, Signature>(
    input: LocalOperationRecoveryContext<'_>,
    request: &CommonRequestEnvelopeV1,
    parent: Option<(&ResidentRecordRefV1, &OperationHandleV1, &OperationLeaseV1)>,
    handle_record: &ControlRecord,
    operation_port: &mut dyn OperationCustodyPort,
    session_port: &dyn SessionLeaseCustodyPort,
    admission: (Authority, Signature),
) -> Result<
    (OperationHandleV1, OperationLeaseV1, SessionAdmissionLeaseV1),
    LocalOperationRecoveryError,
>
where
    Authority: FnOnce(
        &OperationHandlePreimageV1,
        Option<&OperationHandleV1>,
    ) -> Result<(), InvalidResidentRecordV1>,
    Signature: FnOnce(
        &[u8],
        &OperationHandlePreimageV1,
        &OperationHandleDigest,
        &CanonicalSignatureV1,
    ) -> Result<(), InvalidResidentRecordV1>,
{
    let (admit_historical_authority, verify_handle_signature) = admission;
    let binding = request.local_request_binding()?;
    let handle_schema = pale_contracts::record_schema_identity(OPERATION_HANDLE_V1_SCHEMA_ID)?;
    check_record(
        handle_record,
        &handle_schema.schema,
        &handle_schema.schema_sha256,
        input.maximum_record_bytes,
    )?;
    let wire: OperationHandleWireV1 = serde_json::from_value(canon::parse_strict(
        handle_record.canonical_record_jcs().as_slice(),
    )?)?;
    let supported_class = match parent {
        None => matches!(
            wire.operation_class(),
            OperationClassV1::Invoke | OperationClassV1::Consult
        ),
        Some(_) => matches!(
            wire.operation_class(),
            OperationClassV1::Invoke | OperationClassV1::Consult | OperationClassV1::Delegate
        ),
    };
    if !supported_class || wire.parent_operation().is_some() != parent.is_some() {
        return Err(LocalOperationRecoveryError::Association);
    }
    let lease_coordinate = record_projection::coordinate(
        wire.operation_lease_ref(),
        wire.operation_lease_sha256().as_str(),
    )
    .map_err(|_| LocalOperationRecoveryError::Association)?;
    let (lease, session) = recover_operation_lease_and_parent(
        operation_port,
        session_port,
        input.store,
        &lease_coordinate,
        input.maximum_operation_lease_bytes,
        input.maximum_session_lease_bytes,
        input.historical,
    )?;
    let handle = OperationHandleV1::admit_contextual_wire(
        wire,
        &OperationHandleAdmissionContextV1 {
            signed: SignedResidentAdmissionContextV1 {
                expected_schema_sha256: &handle_schema.schema_sha256,
                signing_authority: input.handle_signing_authority,
            },
            request_binding: &binding,
            operation_lease_ref: lease_coordinate.record_ref(),
            operation_lease: &lease,
            parent_operation: parent.map(|(reference, handle, lease)| {
                ParentOperationAdmissionContextV1 {
                    operation_handle_ref: reference,
                    operation_handle: handle,
                    operation_lease: lease,
                }
            }),
            compiled_generation_sha256: session.compiled_generation_digest(),
            activation_fence_sha256: session.activation_fence_digest(),
            instance_sha256: session.instance_digest(),
        },
        admit_historical_authority,
        verify_handle_signature,
    )?;
    if handle_record.coordinate().record_sha256().as_str()
        != handle.canonical_operation_handle_sha256().as_str()
        || canon::to_canonical_string(&handle)?.as_bytes()
            != handle_record.canonical_record_jcs().as_slice()
        || input.checkpoint_authority.principal_id != handle.issuer_principal_id()
    {
        return Err(LocalOperationRecoveryError::Association);
    }
    if let Some((_, parent, parent_lease)) = parent {
        parent_lease.verify_against_parent(&session)?;
        if parent.operation_lease_sha256() != parent_lease.operation_lease_digest()
            || handle.compiled_generation_sha256() != parent.compiled_generation_sha256()
            || handle.activation_fence_sha256() != parent.activation_fence_sha256()
            || handle.instance_sha256() != parent.instance_sha256()
            || handle.expires_at() > parent.expires_at()
        {
            return Err(LocalOperationRecoveryError::Association);
        }
        let key = super::super::child::membership_key(
            handle
                .parent_operation()
                .as_ref()
                .ok_or(LocalOperationRecoveryError::Association)?,
            handle.operation_handle_id(),
        )
        .map_err(|_| LocalOperationRecoveryError::Association)?;
        let member = crate::resident::operation_custody::read_index(
            operation_port,
            input.store,
            pale_ports::OperationCustodyIndexQuery::ParentChildMembership(key),
            input.maximum_record_bytes,
        )?
        .ok_or(LocalOperationRecoveryError::Missing)?;
        if &member != handle_record {
            return Err(LocalOperationRecoveryError::Association);
        }
    }
    Ok((handle, lease, session))
}
