//! Caller-side exact binding and one-use progress, using explicit signature models.

use super::*;
use pale_core::records::{ImmutableRecordReferenceV1, ResidentRecordRefV1, SchemaId};
use pale_ports::{ResidentHookFrameResponse, ResidentHookResponseRecord};

struct ResponseModel {
    expected: ResidentOperationFrameRecords,
    response: String,
    consumed: Mutex<bool>,
}
impl ResidentResponseAdmission for ResponseModel {
    fn admit(
        &self,
        _: &str,
        _: &[u8],
        _: &str,
        _: &[u8],
        _: Instant,
    ) -> Result<(), ResidentResponseAdmissionFailure> {
        panic!("ordinary observation admission cannot authorize native release")
    }
    fn admit_hook_response(
        &self,
        handle_ref: &str,
        handle_bytes: &[u8],
        observation_ref: &str,
        observation_bytes: &[u8],
        response: &[u8],
        deadline: Instant,
    ) -> Result<(), ResidentResponseAdmissionFailure> {
        let mut consumed = self.consumed.lock().unwrap();
        if *consumed
            || handle_ref != self.expected.operation_handle_record_ref()
            || handle_bytes != self.expected.operation_handle_canonical().as_bytes()
            || observation_ref != self.expected.operation_observation_record_ref()
            || observation_bytes != self.expected.operation_observation_canonical().as_bytes()
            || response != self.response.as_bytes()
            || Instant::now() >= deadline
        {
            return Err(ResidentResponseAdmissionFailure);
        }
        *consumed = true;
        Ok(())
    }
}

fn admission(api: &PaleApi) -> crate::ResidentHookAdmission {
    api.admit_resident_hook(ResidentHookRequest::new(
        "pre-tool-use".into(),
        "workspace-1".into(),
        "repository-1".into(),
        "terminal".into(),
        "a".repeat(64),
        EVENT.to_vec(),
    ))
    .unwrap()
}

fn response(api: &PaleApi) -> (ResidentHookFrameResponse, Arc<ResponseModel>) {
    let prepared = prepared(api);
    let body = parsed(prepared.request_body_json());
    let mut fixture = native_fixture(body.clone());
    let reference = |name: &str| ImmutableRecordReferenceV1 {
        record_ref: ResidentRecordRefV1::parse(name).unwrap(),
        record_sha256: ArtifactDigest::hash_bytes(name.as_bytes()),
    };
    let response = ResidentHookResponseRecord {
        schema_id: SchemaId::parse(pale_ports::PRIVATE_OPERATION_HOOK_RESPONSE_V1_SCHEMA).unwrap(),
        handle_sha256: OperationHandleDigest::parse(&fixture.handle_sha256).unwrap(),
        native_request_sha256: ArtifactDigest::parse(body["payload_sha256"].as_str().unwrap()).unwrap(),
        permission: reference("permission"), disclosure: reference("disclosure"),
        stdout: "{\"hookSpecificOutput\":{\"permissionDecision\":\"allow\",\"hookEventName\":\"PreToolUse\"}}".into(),
        stderr: String::new(), exit_code: 0,
    };
    let response_canonical = pale_core::canon::to_canonical_string(&response).unwrap();
    let mut observation = parsed(&fixture.observation_bytes);
    observation["revision"] = 3.into();
    observation["predecessor_ref"] = "native/preparation".into();
    observation["predecessor_sha256"] = "e".repeat(64).into();
    observation["state"] = serde_json::json!({"state":"active","phase":{"phase":"dispatched","attempt_id":"native-attempt","evidence":{"record_ref":"native/response","record_sha256":pale_core::sha256_hex(response_canonical.as_bytes())}}});
    fixture.observation_bytes = canonical(&reseal_observation(observation));
    let records = operation_records(&fixture);
    let model = Arc::new(ResponseModel {
        expected: records.clone(),
        response: response_canonical.clone(),
        consumed: Mutex::new(false),
    });
    (
        ResidentHookFrameResponse::Prepared {
            records,
            response_canonical,
        },
        model,
    )
}

fn hook_attachment(
    response: &ResidentHookFrameResponse,
    model: Option<Arc<ResponseModel>>,
) -> ResidentAttachment {
    let bytes = pale_core::canon::to_canonical_string(response)
        .unwrap()
        .into_bytes()
        .into_boxed_slice();
    let source = carrier_source(ScriptedAuthenticatedCarrier {
        sent: Arc::new(Mutex::new(Vec::new())),
        responses: VecDeque::from([Ok(bytes.clone()), Ok(bytes)]),
    });
    match model {
        Some(model) => {
            ResidentAttachment::provision_admitted_hooks(source, Duration::from_secs(2), model)
        }
        None => ResidentAttachment::provision_hooks(source, Duration::from_secs(2)),
    }
    .unwrap()
}

#[test]
fn native_response_requires_independent_authority_and_one_use_progress() {
    let api = PaleApi::new().unwrap();
    let (response, model) = response(&api);
    let absent = hook_attachment(&response, None);
    assert!(api
        .deliver_resident_hook(Some(&absent), admission(&api))
        .is_err());
    assert!(!*model.consumed.lock().unwrap());
    let attachment = hook_attachment(&response, Some(model.clone()));
    let output = api
        .deliver_resident_hook(Some(&attachment), admission(&api))
        .unwrap();
    assert_eq!(output.exit_code(), 0);
    assert!(output.stdout().contains("allow"));
    assert_eq!(output.stderr(), "");
    assert!(api
        .deliver_resident_hook(Some(&attachment), admission(&api))
        .is_err());
}

#[test]
fn native_response_substitution_never_consumes_response_authority() {
    let api = PaleApi::new().unwrap();
    for field in [
        "stdout",
        "stderr",
        "exit_code",
        "native_request_sha256",
        "handle_sha256",
    ] {
        let (mut response, model) = response(&api);
        let ResidentHookFrameResponse::Prepared {
            response_canonical, ..
        } = &mut response
        else {
            unreachable!()
        };
        let mut value: serde_json::Value = serde_json::from_str(response_canonical).unwrap();
        value[field] = match field {
            "exit_code" => 2.into(),
            "stdout" | "stderr" => "substituted".into(),
            _ => "f".repeat(64).into(),
        };
        *response_canonical = pale_core::canon::to_canonical_string(&value).unwrap();
        let attachment = hook_attachment(&response, Some(model.clone()));
        assert!(
            api.deliver_resident_hook(Some(&attachment), admission(&api))
                .is_err(),
            "{field}"
        );
        assert!(!*model.consumed.lock().unwrap(), "{field}");
    }
}
