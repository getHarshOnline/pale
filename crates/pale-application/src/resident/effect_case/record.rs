//! Canonical private index keys and immutable record projections.

use pale_core::canon;
use pale_core::records::{
    ActionEnvelopeV1, CommonRequestEnvelopeV1, ControlStateCheckpointV1, EffectAdmissionV1,
    EffectConflictClaimV1, EffectDispatchIntentV1, EffectReceiptV1, ReconciliationCaseV1,
    ReconciliationCustodyV1, ReconciliationRequestV1, ReconciliationResultV1, ResidentRecordRefV1,
};
use pale_ports::{ControlRecord, EffectCaseIndexKey};
use serde::Serialize;

use crate::resident::record_projection;

use super::{
    AdmittedEffectCaseRecord, EffectCaseDisposition, EffectCaseProjectionError,
    CASE_RESULT_KEY_DOMAIN, CONFLICT_RESOURCE_KEY_DOMAIN, LOGICAL_ACTION_KEY_DOMAIN,
    REQUEST_IDENTITY_KEY_DOMAIN,
};

#[derive(Serialize)]
struct ScopedActionKeyPreimage<'a> {
    initial_v1_domain: &'static str,
    principal_scope: &'a str,
    controller_scope: &'a str,
    relationship_scope: Option<&'a str>,
    target_system: &'a str,
    target_identity: &'a str,
    operation: &'a str,
    business_idempotency_key: &'a str,
}

#[derive(Serialize)]
struct ConflictResourceKeyPreimage<'a> {
    initial_v1_domain: &'static str,
    principal_scope: &'a str,
    controller_scope: &'a str,
    relationship_scope: Option<&'a str>,
    conflict_resource_key_schema_id: &'a str,
    conflict_resource_key_schema_sha256: &'a str,
    conflict_resource_key: &'a serde_json::Value,
}

#[derive(Serialize)]
struct CaseResultKeyPreimage<'a> {
    initial_v1_domain: &'static str,
    principal_scope: &'a str,
    controller_scope: &'a str,
    relationship_scope: Option<&'a str>,
    action_id: &'a str,
    case_id: &'a str,
}

pub(super) struct EffectCaseKeys {
    pub(super) request: EffectCaseIndexKey,
    pub(super) logical: EffectCaseIndexKey,
    pub(super) conflict: EffectCaseIndexKey,
}

impl EffectCaseKeys {
    pub(super) fn derive(
        request: &CommonRequestEnvelopeV1,
        action: &ActionEnvelopeV1,
    ) -> Result<Self, EffectCaseProjectionError> {
        Ok(Self {
            request: Self::request(request)?,
            logical: derive_key(
                LOGICAL_ACTION_KEY_DOMAIN,
                &ScopedActionKeyPreimage {
                    initial_v1_domain: "logical_action",
                    principal_scope: request.scope().principal_scope.as_str(),
                    controller_scope: request.scope().controller_scope.as_str(),
                    relationship_scope: request
                        .scope()
                        .relationship_scope
                        .as_ref()
                        .map(|value| value.as_str()),
                    target_system: action.target_system().as_str(),
                    target_identity: action.target_identity().as_str(),
                    operation: action.operation().as_str(),
                    business_idempotency_key: action.business_idempotency_key().as_str(),
                },
            )?,
            conflict: derive_key(
                CONFLICT_RESOURCE_KEY_DOMAIN,
                &ConflictResourceKeyPreimage {
                    initial_v1_domain: "conflict_resource",
                    principal_scope: request.scope().principal_scope.as_str(),
                    controller_scope: request.scope().controller_scope.as_str(),
                    relationship_scope: request
                        .scope()
                        .relationship_scope
                        .as_ref()
                        .map(|value| value.as_str()),
                    conflict_resource_key_schema_id: action
                        .conflict_resource_key_schema_id()
                        .as_str(),
                    conflict_resource_key_schema_sha256: action
                        .conflict_resource_key_schema_sha256()
                        .as_str(),
                    conflict_resource_key: action.conflict_resource_key(),
                },
            )?,
        })
    }

    pub(super) fn request(
        request: &CommonRequestEnvelopeV1,
    ) -> Result<EffectCaseIndexKey, EffectCaseProjectionError> {
        derive_key(
            REQUEST_IDENTITY_KEY_DOMAIN,
            &record_projection::RequestIdentityKeyPreimage::from(request),
        )
    }

    pub(super) fn case(
        request: &CommonRequestEnvelopeV1,
        action: &ActionEnvelopeV1,
        case_id: &str,
    ) -> Result<EffectCaseIndexKey, EffectCaseProjectionError> {
        derive_key(
            CASE_RESULT_KEY_DOMAIN,
            &CaseResultKeyPreimage {
                initial_v1_domain: "case_result",
                principal_scope: request.scope().principal_scope.as_str(),
                controller_scope: request.scope().controller_scope.as_str(),
                relationship_scope: request
                    .scope()
                    .relationship_scope
                    .as_ref()
                    .map(|value| value.as_str()),
                action_id: action.action_id().as_str(),
                case_id,
            },
        )
    }
}

pub(super) fn derive_key(
    domain: &[u8],
    preimage: &impl Serialize,
) -> Result<EffectCaseIndexKey, EffectCaseProjectionError> {
    let canonical = canon::to_canonical_string(preimage)?;
    let mut bytes = Vec::with_capacity(domain.len() + canonical.len());
    bytes.extend_from_slice(domain);
    bytes.extend_from_slice(canonical.as_bytes());
    EffectCaseIndexKey::parse(pale_core::sha256_hex(&bytes)).map_err(Into::into)
}

pub(super) fn require(exact: bool) -> Result<(), EffectCaseProjectionError> {
    exact
        .then_some(())
        .ok_or(EffectCaseProjectionError::Lineage)
}

pub(super) fn verify_base(
    request: &CommonRequestEnvelopeV1,
    action: &ActionEnvelopeV1,
) -> Result<(), EffectCaseProjectionError> {
    request.verify_action_body(action)?;
    Ok(())
}

pub(super) fn verify_claim(
    claim: &EffectConflictClaimV1,
    request: &CommonRequestEnvelopeV1,
    action: &ActionEnvelopeV1,
) -> Result<(), EffectCaseProjectionError> {
    claim.verify()?;
    let action_digest = action.canonical_action_body_sha256()?;
    require(
        claim.request_id() == request.request_id()
            && claim.canonical_request_sha256() == request.canonical_request_sha256()
            && claim.action_id() == action.action_id()
            && claim.action_body_sha256() == &action_digest
            && claim.principal_scope() == &request.scope().principal_scope
            && claim.controller_scope() == &request.scope().controller_scope
            && claim.relationship_scope() == &request.scope().relationship_scope
            && claim.conflict_resource_key_schema_id() == action.conflict_resource_key_schema_id()
            && claim.conflict_resource_key_schema_sha256()
                == action.conflict_resource_key_schema_sha256()
            && claim.conflict_resource_key() == action.conflict_resource_key(),
    )
}

pub(super) fn verify_claim_resource(
    claim: &EffectConflictClaimV1,
    request: &CommonRequestEnvelopeV1,
    action: &ActionEnvelopeV1,
) -> Result<(), EffectCaseProjectionError> {
    claim.verify()?;
    require(
        claim.principal_scope() == &request.scope().principal_scope
            && claim.controller_scope() == &request.scope().controller_scope
            && claim.relationship_scope() == &request.scope().relationship_scope
            && claim.conflict_resource_key_schema_id() == action.conflict_resource_key_schema_id()
            && claim.conflict_resource_key_schema_sha256()
                == action.conflict_resource_key_schema_sha256()
            && claim.conflict_resource_key() == action.conflict_resource_key(),
    )
}

pub(super) fn verify_custody_resource(
    custody: &ReconciliationCustodyV1,
    action: &ActionEnvelopeV1,
) -> Result<(), EffectCaseProjectionError> {
    custody.verify()?;
    require(
        custody.conflict_resource_key_schema_id() == action.conflict_resource_key_schema_id()
            && custody.conflict_resource_key_schema_sha256()
                == action.conflict_resource_key_schema_sha256()
            && custody.conflict_resource_key() == action.conflict_resource_key(),
    )
}

pub(super) fn verify_disposition(
    disposition: EffectCaseDisposition<'_>,
    action: &ActionEnvelopeV1,
) -> Result<(), EffectCaseProjectionError> {
    let action_digest = action.canonical_action_body_sha256()?;
    let exact = match disposition {
        EffectCaseDisposition::Admission(value) => {
            value.record.verify()?;
            value.record.action_id() == action.action_id()
                && value.record.action_body_sha256() == &action_digest
        }
        EffectCaseDisposition::Claim(value) => {
            value.record.verify()?;
            value.record.action_id() == action.action_id()
                && value.record.action_body_sha256() == &action_digest
        }
        EffectCaseDisposition::DispatchIntent(value) => {
            value.record.verify()?;
            value.record.action_id() == action.action_id()
                && value.record.action_body_sha256() == &action_digest
        }
        EffectCaseDisposition::Receipt(value) => {
            value.record.verify()?;
            value.record.action_id() == action.action_id()
                && value.record.action_body_sha256() == &action_digest
        }
        EffectCaseDisposition::ReconciliationCase(value) => {
            value.record.verify()?;
            value.record.action_id() == action.action_id()
                && value.record.conflict_resource_key_schema_id()
                    == action.conflict_resource_key_schema_id()
                && value.record.conflict_resource_key_schema_sha256()
                    == action.conflict_resource_key_schema_sha256()
                && value.record.conflict_resource_key() == action.conflict_resource_key()
        }
        EffectCaseDisposition::ReconciliationResult(value) => {
            value.record.verify()?;
            value.record.action_id() == action.action_id()
        }
    };
    require(exact)
}

pub(super) fn verify_disposition_admission(
    disposition: EffectCaseDisposition<'_>,
    admission: AdmittedEffectCaseRecord<'_, EffectAdmissionV1>,
) -> Result<(), EffectCaseProjectionError> {
    let exact = match disposition {
        EffectCaseDisposition::Admission(value) => {
            value.record_ref == admission.record_ref
                && value.record.canonical_effect_admission_sha256()
                    == admission.record.canonical_effect_admission_sha256()
        }
        EffectCaseDisposition::Claim(_) => true,
        EffectCaseDisposition::DispatchIntent(value) => {
            value.record.effect_admission_ref() == admission.record_ref
                && value.record.effect_admission_sha256()
                    == admission.record.canonical_effect_admission_sha256()
        }
        EffectCaseDisposition::Receipt(value) => {
            value.record.effect_admission_ref() == admission.record_ref
                && value.record.effect_admission_sha256()
                    == admission.record.canonical_effect_admission_sha256()
        }
        EffectCaseDisposition::ReconciliationCase(value) => {
            value.record.effect_admission_ref() == admission.record_ref
                && value.record.effect_admission_sha256()
                    == admission.record.canonical_effect_admission_sha256()
        }
        EffectCaseDisposition::ReconciliationResult(value) => {
            value.record.effect_admission_ref() == admission.record_ref
                && value.record.effect_admission_sha256()
                    == admission.record.canonical_effect_admission_sha256()
        }
    };
    require(exact)
}

fn project_typed_record<R>(
    entry: AdmittedEffectCaseRecord<'_, R>,
    digest: &str,
) -> Result<ControlRecord, EffectCaseProjectionError>
where
    R: Serialize + record_projection::ResidentRecordSchema,
{
    let coordinate = record_projection::coordinate(entry.record_ref, digest)?;
    record_projection::record(coordinate, entry.record).map_err(Into::into)
}

pub(super) fn project_request(
    entry: AdmittedEffectCaseRecord<'_, CommonRequestEnvelopeV1>,
) -> Result<ControlRecord, EffectCaseProjectionError> {
    record_projection::request_record(entry.record_ref, entry.record).map_err(Into::into)
}

pub(super) fn project_action(
    request: &CommonRequestEnvelopeV1,
    entry: AdmittedEffectCaseRecord<'_, ActionEnvelopeV1>,
) -> Result<ControlRecord, EffectCaseProjectionError> {
    let digest = entry.record.canonical_action_body_sha256()?;
    let coordinate = record_projection::coordinate(entry.record_ref, digest.as_str())?;
    record_projection::record_with_schema(
        coordinate,
        request.body().schema_id.clone(),
        request.body().schema_sha256.clone(),
        entry.record,
    )
    .map_err(Into::into)
}

pub(super) fn project_claim(
    entry: AdmittedEffectCaseRecord<'_, EffectConflictClaimV1>,
) -> Result<ControlRecord, EffectCaseProjectionError> {
    project_typed_record(
        entry,
        entry
            .record
            .canonical_effect_conflict_claim_sha256()
            .as_str(),
    )
}

pub(super) fn project_admission(
    entry: AdmittedEffectCaseRecord<'_, EffectAdmissionV1>,
) -> Result<ControlRecord, EffectCaseProjectionError> {
    project_typed_record(
        entry,
        entry.record.canonical_effect_admission_sha256().as_str(),
    )
}

pub(super) fn project_intent(
    entry: AdmittedEffectCaseRecord<'_, EffectDispatchIntentV1>,
) -> Result<ControlRecord, EffectCaseProjectionError> {
    project_typed_record(
        entry,
        entry.record.canonical_dispatch_intent_sha256().as_str(),
    )
}

pub(super) fn project_receipt(
    entry: AdmittedEffectCaseRecord<'_, EffectReceiptV1>,
) -> Result<ControlRecord, EffectCaseProjectionError> {
    project_typed_record(entry, entry.record.canonical_receipt_sha256().as_str())
}

pub(super) fn project_case(
    entry: AdmittedEffectCaseRecord<'_, ReconciliationCaseV1>,
) -> Result<ControlRecord, EffectCaseProjectionError> {
    project_typed_record(entry, entry.record.canonical_case_sha256().as_str())
}

pub(super) fn project_reconciliation_request(
    request: &CommonRequestEnvelopeV1,
    entry: AdmittedEffectCaseRecord<'_, ReconciliationRequestV1>,
) -> Result<ControlRecord, EffectCaseProjectionError> {
    let digest = entry.record.canonical_reconciliation_request_sha256()?;
    let coordinate = record_projection::coordinate(entry.record_ref, digest.as_str())?;
    record_projection::record_with_schema(
        coordinate,
        request.body().schema_id.clone(),
        request.body().schema_sha256.clone(),
        entry.record,
    )
    .map_err(Into::into)
}

pub(super) fn project_custody(
    entry: AdmittedEffectCaseRecord<'_, ReconciliationCustodyV1>,
) -> Result<ControlRecord, EffectCaseProjectionError> {
    project_typed_record(entry, entry.record.canonical_custody_sha256().as_str())
}

pub(super) fn project_result(
    entry: AdmittedEffectCaseRecord<'_, ReconciliationResultV1>,
) -> Result<ControlRecord, EffectCaseProjectionError> {
    project_typed_record(
        entry,
        entry.record.canonical_reconciliation_sha256().as_str(),
    )
}

pub(super) fn project_checkpoint_record(
    record_ref: &ResidentRecordRefV1,
    checkpoint: &ControlStateCheckpointV1,
) -> Result<ControlRecord, EffectCaseProjectionError> {
    project_typed_record(
        AdmittedEffectCaseRecord::new(record_ref, checkpoint),
        checkpoint.canonical_control_checkpoint_sha256().as_str(),
    )
}

pub(super) fn project_disposition(
    disposition: EffectCaseDisposition<'_>,
) -> Result<ControlRecord, EffectCaseProjectionError> {
    match disposition {
        EffectCaseDisposition::Admission(value) => project_admission(value),
        EffectCaseDisposition::Claim(value) => project_claim(value),
        EffectCaseDisposition::DispatchIntent(value) => project_intent(value),
        EffectCaseDisposition::Receipt(value) => project_receipt(value),
        EffectCaseDisposition::ReconciliationCase(value) => project_case(value),
        EffectCaseDisposition::ReconciliationResult(value) => project_result(value),
    }
}
