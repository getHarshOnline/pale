//! Deliver admitted native bytes through an explicitly supplied connection.

use pale_core::records::{
    DiagnosticDispositionV1, DiagnosticEffectCertaintyV1, DiagnosticRetryV1, DiagnosticStageV1,
};
use pale_ports::{ResidentHookFrameResponse, ResidentOperationFrameFailure};

use super::ResidentHookAdmission;
use crate::error::{ApiError, FailureSpecification};
use crate::resident_attachment::{
    resident_may_have_been_sent_error, resident_not_sent_error, response_admission_error,
    ResidentAttachmentClientError, ResidentRequestTransmissionCertainty,
};
use crate::{PaleApi, ResidentAttachment};

/// Exact admitted host output. Emitting it does not prove the host enforced it.
#[must_use = "emit the admitted native response once or keep native work blocked"]
pub struct ResidentHookOutput {
    stdout: String,
    stderr: String,
    exit_code: u8,
}

impl ResidentHookOutput {
    /// Exact standard output; do not wrap it in another result format.
    #[must_use]
    pub fn stdout(&self) -> &str {
        &self.stdout
    }
    /// Exact standard error selected by the resident.
    #[must_use]
    pub fn stderr(&self) -> &str {
        &self.stderr
    }
    /// Process status selected by the native protocol encoder.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        self.exit_code
    }
}

impl std::fmt::Debug for ResidentHookOutput {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("ResidentHookOutput(<admitted private response>)")
    }
}

impl PaleApi {
    /// Deliver one admitted hook through the supplied hook or operation attachment.
    ///
    /// This consumes the input and attempts delivery at most once. An observation
    /// attachment cannot deliver hooks. Output requires the exact signed dispatch
    /// and an independent response owner that consumes its progress once.
    /// Unknown, malformed and late responses retain an uncertain outcome.
    ///
    /// # Errors
    /// Refuses missing authority, changed bytes, replay, or an unavailable resident.
    pub fn deliver_resident_hook(
        &self,
        attachment: Option<&ResidentAttachment>,
        admission: ResidentHookAdmission,
    ) -> Result<ResidentHookOutput, ApiError> {
        let Some(client) = attachment.and_then(|attachment| attachment.hook_client.as_ref()) else {
            return Err(self.resident_hook_transport_unavailable(&admission));
        };
        let response = (|| {
            let deadline = client.deadline()?;
            let frame = pale_core::canon::to_canonical_string(&admission.frame)
                .map_err(|_| resident_not_sent_error("hook frame could not be canonicalized"))?;
            client.exchange_with(frame.as_bytes(), deadline, |bytes| {
                let response: ResidentHookFrameResponse = client.decode(bytes)?;
                if let ResidentHookFrameResponse::Prepared {
                    records,
                    response_canonical,
                } = &response
                {
                    validate_response(&admission, records, response_canonical)?;
                    client
                        .response_admission()?
                        .admit_hook_response(
                            records.operation_handle_record_ref(),
                            records.operation_handle_canonical().as_bytes(),
                            records.operation_observation_record_ref(),
                            records.operation_observation_canonical().as_bytes(),
                            response_canonical.as_bytes(),
                            deadline,
                        )
                        .map_err(response_admission_error)?;
                }
                Ok(response)
            })
        })();
        match response {
            Ok(ResidentHookFrameResponse::Prepared {
                response_canonical, ..
            }) => {
                let response: pale_ports::ResidentHookResponseRecord =
                    serde_json::from_str(&response_canonical).map_err(|_| {
                        delivery_failure(
                            self,
                            resident_may_have_been_sent_error(
                                "resident response changed after admission",
                            ),
                        )
                    })?;
                Ok(ResidentHookOutput {
                    stdout: response.stdout,
                    stderr: response.stderr,
                    exit_code: u8::try_from(response.exit_code).map_err(|_| {
                        delivery_failure(
                            self,
                            resident_may_have_been_sent_error(
                                "resident response status is invalid",
                            ),
                        )
                    })?,
                })
            }
            Ok(ResidentHookFrameResponse::HandlerUnavailable {}) => Err(diagnostic(
                self,
                "resident_hook_handler_unavailable",
                "the resident cannot handle native hook events yet",
                DiagnosticDispositionV1::Blocked,
                DiagnosticRetryV1::AfterStateChange,
                DiagnosticEffectCertaintyV1::NotStarted,
                "Keep native work blocked until the resident has a current hook handler.",
                std::io::Error::other("resident hook handler is unavailable"),
            )),
            Ok(ResidentHookFrameResponse::Rejected { failure }) => Err(rejected(self, failure)),
            Err(error) => Err(delivery_failure(self, error)),
        }
    }
}

fn validate_response(
    admission: &ResidentHookAdmission,
    records: &pale_ports::ResidentOperationFrameRecords,
    canonical: &str,
) -> Result<(), ResidentAttachmentClientError> {
    use crate::resident_observation::{
        parse_canonical_record, validate_response as validate_records, ResidentObservationResponse,
    };
    let invalid = || resident_may_have_been_sent_error("native hook response is misbound");
    let value = pale_core::canon::parse_strict(canonical.as_bytes()).map_err(|_| invalid())?;
    if pale_core::canon::to_canonical_string(&value).map_err(|_| invalid())? != canonical {
        return Err(invalid());
    }
    let response: pale_ports::ResidentHookResponseRecord =
        serde_json::from_value(value).map_err(|_| invalid())?;
    let native = pale_application::resident::ResidentNativeToolStart::prepare(
        admission._request.routing().clone(),
        admission.frame.native_event(),
    )
    .map_err(|_| invalid())?;
    if response.schema_id.as_str() != pale_ports::PRIVATE_OPERATION_HOOK_RESPONSE_V1_SCHEMA
        || response.native_request_sha256 != native.body().payload_sha256
        || !(0..=255).contains(&response.exit_code)
    {
        return Err(invalid());
    }
    validate_records(
        None,
        &ResidentObservationResponse {
            operation_handle_record_ref: records.operation_handle_record_ref().to_owned(),
            operation_handle_canonical_bytes: records
                .operation_handle_canonical()
                .as_bytes()
                .to_vec(),
            operation_observation_record_ref: records.operation_observation_record_ref().to_owned(),
            operation_observation_canonical_bytes: records
                .operation_observation_canonical()
                .as_bytes()
                .to_vec(),
        },
    )
    .map_err(|_| invalid())?;
    let observation = parse_canonical_record(
        records.operation_observation_canonical().as_bytes(),
        "operation-observation.v1",
        "schema_sha256",
    )
    .map_err(|_| invalid())?;
    if observation["operation_handle_sha256"].as_str() != Some(response.handle_sha256.as_str())
        || observation["state"]["state"].as_str() != Some("active")
        || observation["state"]["phase"]["phase"].as_str() != Some("dispatched")
        || observation["state"]["phase"]["evidence"]["record_sha256"].as_str()
            != Some(pale_core::sha256_hex(canonical.as_bytes()).as_str())
    {
        return Err(invalid());
    }
    Ok(())
}

fn rejected(api: &PaleApi, failure: ResidentOperationFrameFailure) -> ApiError {
    let (code, summary, disposition, retry, effect, help) = match failure {
        ResidentOperationFrameFailure::InvalidRequest => (
            "resident_hook_request_rejected",
            "the resident rejected the native event or its operation association",
            DiagnosticDispositionV1::InvalidInput,
            DiagnosticRetryV1::AfterCorrection,
            DiagnosticEffectCertaintyV1::NotStarted,
            "Correct the native event and route without substituting another occurrence.",
        ),
        ResidentOperationFrameFailure::ReplayConflict => (
            "resident_hook_replay_conflict",
            "the native event conflicts with retained operation state",
            DiagnosticDispositionV1::EffectOutcomeUnknown,
            DiagnosticRetryV1::ReconcileFirst,
            DiagnosticEffectCertaintyV1::OutcomeUnknown,
            "Reconcile the original occurrence and operation; do not create a replacement identity or replay native work.",
        ),
        ResidentOperationFrameFailure::AdmissionDenied => (
            "resident_hook_admission_denied",
            "the resident denied current native hook admission",
            DiagnosticDispositionV1::Blocked,
            DiagnosticRetryV1::AfterStateChange,
            DiagnosticEffectCertaintyV1::NotStarted,
            "Restore current caller, session and operation admission before submitting the event again.",
        ),
        ResidentOperationFrameFailure::ConfigurationUnavailable => (
            "resident_hook_configuration_unavailable",
            "the native event's selected resident configuration is unavailable",
            DiagnosticDispositionV1::Blocked,
            DiagnosticRetryV1::AfterStateChange,
            DiagnosticEffectCertaintyV1::NotStarted,
            "Restore the exact compiled configuration selected for this operation.",
        ),
        ResidentOperationFrameFailure::ResidentUnavailable => (
            "resident_hook_recovery_unavailable",
            "the resident could not establish the native operation's current protected state",
            DiagnosticDispositionV1::EffectOutcomeUnknown,
            DiagnosticRetryV1::ReconcileFirst,
            DiagnosticEffectCertaintyV1::OutcomeUnknown,
            "Keep native work blocked and reconcile the retained operation before another delivery attempt.",
        ),
    };
    diagnostic(
        api,
        code,
        summary,
        disposition,
        retry,
        effect,
        help,
        std::io::Error::other("resident refused native hook admission"),
    )
}

fn delivery_failure(api: &PaleApi, error: ResidentAttachmentClientError) -> ApiError {
    match error.transmission() {
        ResidentRequestTransmissionCertainty::NotSent => diagnostic(
            api,
            "resident_hook_not_sent",
            "the native hook event was not sent to the resident",
            DiagnosticDispositionV1::Blocked,
            DiagnosticRetryV1::AfterStateChange,
            DiagnosticEffectCertaintyV1::NotStarted,
            "Check the authenticated connection and request size before issuing a new request.",
            error,
        ),
        ResidentRequestTransmissionCertainty::MayHaveBeenSent => diagnostic(
            api,
            "resident_hook_outcome_unknown",
            "the resident did not return a valid, timely hook response",
            DiagnosticDispositionV1::EffectOutcomeUnknown,
            DiagnosticRetryV1::ReconcileFirst,
            DiagnosticEffectCertaintyV1::OutcomeUnknown,
            "Keep native work blocked and reconcile the resident's state before retrying the event.",
            error,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn diagnostic(
    api: &PaleApi,
    code: &'static str,
    summary: &'static str,
    disposition: DiagnosticDispositionV1,
    retry: DiagnosticRetryV1,
    effect: DiagnosticEffectCertaintyV1,
    help: &'static str,
    error: impl std::error::Error + Send + Sync + 'static,
) -> ApiError {
    ApiError::new_with_opaque_source(
        &api.diagnostic_identity,
        error,
        FailureSpecification {
            code,
            stage: DiagnosticStageV1::ResidentAdmission,
            disposition,
            summary,
            subject: None,
            schema_coordinate: None,
            location: None,
            retry,
            effect,
            request: None,
            notes: vec!["This response authorizes no native work or context disclosure."],
            help: vec![help],
            facts: Vec::new(),
        },
    )
}
