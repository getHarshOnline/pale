//! Failure recording reuses the real shell journey and its explicit custody models.

use super::*;
use crate::actions::NativeActionInvocationError;
use crate::resident::operation_dispatch::{
    FailedNativeOperation, NativeOperationFailureInput, PreparedNativeOperationFailure,
};
use crate::resident::PreparedOperationObservationAppend;
use pale_ports::{OperationCustodyIndexMutation, OperationCustodyIndexPredecessor};

fn input(
    signing: SignedResidentAdmissionContextV1<'_>,
    observed_at: u64,
) -> NativeOperationFailureInput<'_> {
    NativeOperationFailureInput {
        recovered_history: None,
        diagnostic_ref: record_ref("native-shell/diagnostic"),
        observation_ref: record_ref("native-shell/failed"),
        completed_at: UnixTimeV1::new(68).unwrap(),
        observed_at: UnixTimeV1::new(observed_at).unwrap(),
        signing,
        maximum_diagnostic_bytes: 65_536,
    }
}

pub(super) fn prepare<'a>(
    failed: &'a FailedNativeOperation,
    signing: SignedResidentAdmissionContextV1<'_>,
    observed_at: u64,
    aliases: &[ResidentRecordRefV1],
) -> PreparedNativeOperationFailure<'a> {
    let Some(NativeActionInvocationError::Action(error)) = failed.error() else {
        panic!("schema-valid input must reach the lexical handler")
    };
    assert_eq!(error.code, ActionErrorCode::InvalidPath);
    assert_eq!(error.action_id.as_deref(), Some(PATH_INVENTORY_ACTION_ID));
    assert!(error
        .message
        .contains("parent traversal escapes the lexical root"));
    assert!(!error.message.contains("private"));
    let retained_error = serde_json::to_value(error).unwrap();
    let retained_dispatch = canon::to_canonical_string(failed.dispatch().1).unwrap();
    let baseline = failed
        .prepare_failure(
            input(signing, observed_at),
            |_, _, _, _| Ok(()),
            |_, _, _| Ok(signature()),
        )
        .unwrap();
    let diagnostic_bytes = baseline.diagnostic().canonical_bytes().to_vec();
    let observation_bytes = canon::to_canonical_string(baseline.observation().1).unwrap();
    let wire = baseline.diagnostic().wire();
    assert_eq!(wire.disposition, DiagnosticDispositionV1::InvalidInput);
    assert_eq!(wire.effect, DiagnosticEffectCertaintyV1::NotApplicable);
    assert_eq!(wire.retry, DiagnosticRetryV1::AfterCorrection);
    assert!(!String::from_utf8_lossy(&diagnostic_bytes).contains("private"));

    let mut attempted_preimage = None;
    let refused = failed.prepare_failure(
        input(signing, observed_at),
        |_, _, _, _| Ok(()),
        |domain, preimage, digest| {
            assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
            attempted_preimage = Some((
                canon::to_canonical_string(preimage).unwrap(),
                digest.clone(),
            ));
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "test observation signer",
                invariant: "unavailable",
            })
        },
    );
    assert!(refused.is_err());
    let retried = failed
        .prepare_failure(
            input(signing, observed_at),
            |_, _, _, _| Ok(()),
            |domain, preimage, digest| {
                assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
                let (expected_preimage, expected_digest) = attempted_preimage.as_ref().unwrap();
                assert_eq!(
                    &canon::to_canonical_string(preimage).unwrap(),
                    expected_preimage
                );
                assert_eq!(digest, expected_digest);
                Ok(signature())
            },
        )
        .unwrap();
    assert_eq!(retried.diagnostic().canonical_bytes(), diagnostic_bytes);
    assert_eq!(
        canon::to_canonical_string(retried.observation().1).unwrap(),
        observation_bytes
    );

    for maximum in [0, diagnostic_bytes.len() - 1] {
        let mut bounded = input(signing, observed_at);
        bounded.maximum_diagnostic_bytes = maximum;
        let signs = Cell::new(0);
        let result = failed.prepare_failure(
            bounded,
            |_, _, _, _| Ok(()),
            |_, _, _| {
                signs.set(signs.get() + 1);
                Ok(signature())
            },
        );
        assert!(matches!(result, Err(OperationAttemptError::BoundExceeded)));
        assert_eq!(signs.get(), 0);
    }
    let mut exact_bound = input(signing, observed_at);
    exact_bound.maximum_diagnostic_bytes = diagnostic_bytes.len();
    let exact = failed
        .prepare_failure(exact_bound, |_, _, _, _| Ok(()), |_, _, _| Ok(signature()))
        .unwrap();
    assert_eq!(exact.diagnostic().canonical_bytes(), diagnostic_bytes);

    for (completed_at, recorded_at) in [(66, 69), (68, 67), (68, 121)] {
        let mut changed = input(signing, recorded_at);
        changed.completed_at = UnixTimeV1::new(completed_at).unwrap();
        let signs = Cell::new(0);
        let result = failed.prepare_failure(
            changed,
            |_, _, _, _| Ok(()),
            |_, _, _| {
                signs.set(signs.get() + 1);
                Ok(signature())
            },
        );
        if recorded_at == 121 {
            assert!(matches!(
                result,
                Err(OperationAttemptError::RetentionElapsed)
            ));
        } else {
            assert!(matches!(result, Err(OperationAttemptError::TimeOrder)));
        }
        assert_eq!(signs.get(), 0);
    }
    for alias in aliases {
        for replace_diagnostic in [true, false] {
            let mut changed = input(signing, observed_at);
            if replace_diagnostic {
                changed.diagnostic_ref = alias.clone();
            } else {
                changed.observation_ref = alias.clone();
            }
            let signs = Cell::new(0);
            let result = failed.prepare_failure(
                changed,
                |_, _, _, _| Ok(()),
                |_, _, _| {
                    signs.set(signs.get() + 1);
                    Ok(signature())
                },
            );
            assert!(
                matches!(result, Err(OperationAttemptError::Association)),
                "{alias:?}"
            );
            assert_eq!(signs.get(), 0);
        }
    }
    let mut changed = input(signing, observed_at);
    changed.observation_ref = changed.diagnostic_ref.clone();
    assert!(matches!(
        failed.prepare_failure(
            changed,
            |_, _, _, _| Ok(()),
            |_, _, _| panic!("alias cannot reach signing")
        ),
        Err(OperationAttemptError::Association)
    ));
    let Some(NativeActionInvocationError::Action(error)) = failed.error() else {
        panic!("same typed cause")
    };
    assert_eq!(serde_json::to_value(error).unwrap(), retained_error);
    assert_eq!(
        canon::to_canonical_string(failed.dispatch().1).unwrap(),
        retained_dispatch
    );
    retried
}

pub(super) fn assert_append(
    failed: &FailedNativeOperation,
    prepared: &PreparedNativeOperationFailure<'_>,
    append: &PreparedOperationObservationAppend<'_>,
) {
    let Some(OperationCustodyRecord::Diagnostic(record)) = append.associated_record() else {
        panic!("the terminal append must retain the diagnostic, not a successful result")
    };
    assert_eq!(
        record.canonical_record_jcs().as_slice(),
        prepared.diagnostic().canonical_bytes()
    );
    let OperationObservationStateV1::Failed { result, diagnostic } =
        prepared.observation().1.state()
    else {
        panic!("actual handler failure must produce the canonical failed state")
    };
    assert!(result.is_none());
    assert_eq!(&diagnostic.record_ref, record.coordinate().record_ref());
    assert_eq!(
        diagnostic.record_sha256,
        ArtifactDigest::hash_bytes(prepared.diagnostic().canonical_bytes())
    );
    assert_ne!(
        diagnostic.record_sha256.as_str(),
        prepared.diagnostic().identity(),
        "immutable record checksum and domain-separated diagnostic identity are distinct"
    );
    assert_eq!(append.predecessor(), failed.dispatch());
    let OperationCustodyIndexMutation::LatestObservation {
        expected,
        successor,
        ..
    } = append.latest_observation_mutation()
    else {
        panic!("failure must advance the existing latest-observation owner")
    };
    let OperationCustodyIndexPredecessor::Exact(prior) = expected else {
        panic!("terminal custody must compare the exact dispatched predecessor")
    };
    assert_eq!(prior.coordinate().record_ref(), failed.dispatch().0);
    assert_eq!(
        prior.canonical_record_jcs().as_slice(),
        canon::to_canonical_string(failed.dispatch().1)
            .unwrap()
            .as_bytes()
    );
    assert_eq!(successor, append.successor_record().coordinate());
}
