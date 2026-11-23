//! Cancellation preparation over the existing admitted start and finite custody model.
//! No native tree, production signature, external caller or handler is simulated here.

use super::*;
use crate::resident::{NativeOperationCancellationInput, PreparedNativeOperationAttempt};

pub(super) fn check(
    prepared: &PreparedNativeOperationAttempt<'_, '_>,
    selected: NativeOperationCancellationInput<'_>,
    request: &CommonRequestEnvelopeV1,
    checkpoint: &ControlStateCheckpointV1,
    requires_evidence: bool,
) {
    let input = || NativeOperationCancellationInput {
        completed: selected.completed,
        current: selected.current,
        signing: selected.signing,
        diagnostic_ref: selected.diagnostic_ref.clone(),
        observation_ref: selected.observation_ref.clone(),
        maximum_diagnostic_bytes: selected.maximum_diagnostic_bytes,
        resolve_store: selected.resolve_store,
    };
    let signs = Cell::new(0);
    let prepare = |input| {
        prepared.prepare_cancellation(
            input,
            &mut ProvingAuthority,
            |_, _, _, _| Ok(()),
            |domain, _, _| {
                assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
                signs.set(signs.get() + 1);
                Ok(signature())
            },
        )
    };
    if requires_evidence {
        assert!(matches!(
            prepare(input()),
            Err(OperationAttemptError::ContextRequired)
        ));
        assert_eq!(signs.get(), 0);
        return;
    }
    let expired = selected.current.at_time(UnixTimeV1::new(80).unwrap());
    let mut altered = input();
    altered.current = &expired;
    assert!(matches!(
        prepare(altered),
        Err(OperationAttemptError::Association)
    ));
    let mut without_grant = selected.current.at_time(selected.current.now);
    without_grant.grant = None;
    let mut altered = input();
    altered.current = &without_grant;
    assert!(matches!(
        prepare(altered),
        Err(OperationAttemptError::Session(_))
    ));
    let mut without_runtime = selected.current.at_time(selected.current.now);
    without_runtime.runtime_authority = None;
    let mut altered = input();
    altered.current = &without_runtime;
    assert!(matches!(
        prepare(altered),
        Err(OperationAttemptError::Session(_))
    ));
    for reference in [
        selected.observation_ref.clone(),
        prepared.observation().0.clone(),
        record_ref("native-shell/selection"),
        prepared.observation().1.operation_handle_ref().clone(),
    ] {
        let mut altered = input();
        altered.diagnostic_ref = reference;
        assert!(matches!(
            prepare(altered),
            Err(OperationAttemptError::Association)
        ));
    }
    let mut altered = input();
    altered.maximum_diagnostic_bytes = 1;
    assert!(matches!(
        prepare(altered),
        Err(OperationAttemptError::BoundExceeded)
    ));
    assert_eq!(signs.get(), 0, "preparation refusals occur before signing");
    let rejected = prepared.prepare_cancellation(
        input(),
        &mut ProvingAuthority,
        |_, _, _, _| Ok(()),
        |_, _, _| {
            Err(InvalidResidentRecordV1::ContextualAdmission {
                record: "cancellation test",
                invariant: "controlled signer refusal",
            })
        },
    );
    assert!(rejected.is_err());
    let cancelled = prepare(input()).unwrap();
    assert_eq!(
        signs.get(),
        1,
        "same retained attempt remains usable after refusal"
    );
    assert_eq!(
        cancelled.observation().1.predecessor_ref().as_ref(),
        Some(prepared.observation().0)
    );
    assert_eq!(
        cancelled.observation().1.predecessor_sha256().as_ref(),
        Some(
            prepared
                .observation()
                .1
                .canonical_operation_observation_sha256()
        )
    );
    let OperationObservationStateV1::Cancelled { diagnostic } = cancelled.observation().1.state()
    else {
        panic!("confirmed cancellation shape")
    };
    assert_eq!(&diagnostic.record_ref, &selected.diagnostic_ref);
    assert_eq!(
        diagnostic.record_sha256,
        ArtifactDigest::hash_bytes(cancelled.diagnostic().canonical_bytes())
    );
    assert_eq!(
        cancelled.diagnostic().wire().effect,
        DiagnosticEffectCertaintyV1::NotApplicable
    );
    let append = cancelled.prepare_append(selected.resolve_store).unwrap();
    assert!(matches!(
        append.associated_record(),
        Some(OperationCustodyRecord::Diagnostic(_))
    ));
    let (terminal, _) = custody::complete(
        append,
        request,
        (
            selected
                .completed
                .candidate()
                .checkpoint()
                .successor()
                .coordinate()
                .record_ref(),
            checkpoint,
        ),
        OperationCustodyDatabaseEvidence::New,
        64,
    );
    let mut altered = input();
    altered.completed = &terminal;
    assert!(matches!(
        prepare(altered),
        Err(OperationAttemptError::Custody(_))
    ));
    assert_eq!(
        signs.get(),
        1,
        "terminal custody cannot replace the exact preparation"
    );
    let mut exact = input();
    exact.maximum_diagnostic_bytes = cancelled.diagnostic().canonical_bytes().len();
    assert!(prepare(exact).is_ok());
    let mut too_small = input();
    too_small.maximum_diagnostic_bytes = cancelled.diagnostic().canonical_bytes().len() - 1;
    assert!(matches!(
        prepare(too_small),
        Err(OperationAttemptError::BoundExceeded)
    ));
}
