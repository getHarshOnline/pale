//! Historical comparisons use core-admitted result wires and signed observation fixtures.
//! The fixtures do not prove actual dispatch, checkpoint membership or native custody.

use super::*;
use crate::actions::NativeActionRegistry;
use crate::local_operation_start_fixture::issue_observation;
use crate::resident::operation_dispatch::{
    validate_operation_attempt_result, OperationAttemptError, OperationAttemptHistoryInput,
    OperationAttemptResultInput,
};

#[test]
fn historical_result_matches_the_retained_selection_instead_of_selecting_its_own_expectations() {
    with_selection_context(|selection, parents, _, authority| {
        let other_signing = SigningAuthorityBindingV1::integrated_runtime(
            authority.signing.principal_id().clone(),
            ResidentKeyIdV1::parse("other-result-key").unwrap(),
            *authority.signing.signer_key_epoch(),
            authority
                .signing
                .integrated_runtime_authority()
                .unwrap()
                .clone(),
        )
        .unwrap();
        let record = selection
            .record(record_ref("attempt-selection"), 65_536)
            .unwrap();
        let (handle_ref, handle) = parents.operation.handle();
        let schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let initial_ref = record_ref("result-initial");
        let initial = issue_observation(
            authority.signing,
            handle_ref,
            handle,
            schema.clone(),
            OperationObservationRevisionV1::new(1).unwrap(),
            None,
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Resolving,
            },
            60,
        );
        for case in [
            "exact",
            "wrong-dispatch-reference",
            "wrong-dispatch-attempt",
            "not-dispatched",
            "wrong-implementation-kind",
            "wrong-implementation-version",
            "wrong-signer",
            "wrong-outer-schema",
            "wrong-request",
            "wrong-generation",
            "wrong-instance",
            "wrong-operation",
            "wrong-provider",
            "wrong-attempt",
            "bound",
        ] {
            let signing = if case == "wrong-signer" {
                &other_signing
            } else {
                authority.signing
            };
            let success = provider::success(
                parents.request,
                parents.generation,
                signing,
                selection.attempt_id(),
                case,
            );
            let mut dispatch_evidence = evidence(&record);
            if case == "wrong-dispatch-reference" {
                dispatch_evidence.record_ref = record_ref("different-selection");
            }
            let attempt_id = if case == "wrong-dispatch-attempt" {
                CapabilityAttemptIdV1::parse("different-dispatch-attempt").unwrap()
            } else {
                selection.attempt_id().clone()
            };
            let phase = if case == "not-dispatched" {
                OperationActivePhaseV1::DispatchPrepared {
                    attempt_id,
                    evidence: dispatch_evidence,
                }
            } else {
                OperationActivePhaseV1::Dispatched {
                    attempt_id,
                    evidence: dispatch_evidence,
                }
            };
            let dispatched = issue_observation(
                authority.signing,
                handle_ref,
                handle,
                schema.clone(),
                OperationObservationRevisionV1::new(2).unwrap(),
                Some((&initial_ref, &initial)),
                OperationObservationStateV1::Active { phase },
                65,
            );
            let history = OperationAttemptHistoryInput {
                operation: parents.operation,
                request: parents.request,
                session: parents.session,
                lease: parents.lease,
                generation: parents.generation,
                dispatched: &dispatched,
                selection: &record,
                maximum_selection_bytes: if case == "bound" { 1 } else { 65_536 },
            };
            if case == "exact" {
                assert!(matches!(
                    history.classify_owner().unwrap(),
                    crate::resident::operation_dispatch::OperationAttemptOwner::Provider
                ));
            }
            let checked = validate_operation_attempt_result(OperationAttemptResultInput {
                history,
                success: &success,
            });
            if case == "exact" {
                checked.unwrap();
            } else {
                assert!(checked.is_err(), "{case}");
            }
            assert_eq!(
                handle.expires_at().get(),
                80,
                "comparison does not renew execution"
            );
        }
    });
}

#[test]
fn provider_selection_cannot_be_reopened_as_a_pure_native_call() {
    with_selection_context(|selection, parents, _, authority| {
        let record = selection
            .record(record_ref("provider-attempt-selection"), 65_536)
            .unwrap();
        let (handle_ref, handle) = parents.operation.handle();
        let schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
        let initial_ref = record_ref("provider-recovery-initial");
        let initial = issue_observation(
            authority.signing,
            handle_ref,
            handle,
            schema.clone(),
            OperationObservationRevisionV1::new(1).unwrap(),
            None,
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Resolving,
            },
            60,
        );
        let dispatched = issue_observation(
            authority.signing,
            handle_ref,
            handle,
            schema,
            OperationObservationRevisionV1::new(2).unwrap(),
            Some((&initial_ref, &initial)),
            OperationObservationStateV1::Active {
                phase: OperationActivePhaseV1::Dispatched {
                    attempt_id: selection.attempt_id().clone(),
                    evidence: evidence(&record),
                },
            },
            65,
        );
        let registry = NativeActionRegistry::new().unwrap();
        let contracts = crate::session_admission_fixture::local_start_contract_set();
        let error = OperationAttemptHistoryInput {
            operation: parents.operation,
            request: parents.request,
            session: parents.session,
            lease: parents.lease,
            generation: parents.generation,
            dispatched: &dispatched,
            selection: &record,
            maximum_selection_bytes: 65_536,
        }
        .recover_native_attempt(&registry, &contracts)
        .err()
        .expect("provider selection cannot reconstruct a native call");
        assert!(matches!(error, OperationAttemptError::Association));
    });
}
