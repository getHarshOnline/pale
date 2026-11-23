//! Native request meaning is checked before start signing, not by running a handler.
//! Uses the existing observed registry and real session/compiler/start owners;
//! remaining external authority and signatures remain explicit fixture models.

use super::*;
use crate::local_operation_start_fixture::{self as local, StartAdmission};
use crate::resident::{AdmittedCompiledGeneration, LocalOperationStartError};
use crate::session_admission_fixture::LocalStartContext;
use pale_core::records::{ActionErrorCode, ArtifactDigest, SchemaDigest, SchemaId};

#[derive(Clone, Copy, Debug)]
enum Case {
    Valid,
    Malformed,
    WrongKind,
    WrongSchema,
    WrongSchemaDigest,
    OwnerDenied,
    HandlerDomainFailure,
}

#[test]
fn native_ingress_checks_body_before_signing_without_impersonating_owner_or_handler() {
    let (compilation, materializations, profile) =
        local::source_generation_inputs_with(|_| {}, |_| {});
    let generation = AdmittedCompiledGeneration::admit(
        compilation,
        materializations,
        crate::resident_fixture::synthetic_set_policy(),
    )
    .unwrap();
    for case in [
        Case::Valid,
        Case::Malformed,
        Case::WrongKind,
        Case::WrongSchema,
        Case::WrongSchemaDigest,
        Case::OwnerDenied,
        Case::HandlerDomainFailure,
    ] {
        let registry = observed_registry();
        local::with_start_context(
            &generation,
            &profile,
            "native-ingress",
            |request| {
                local::native_request_body(request);
                match case {
                    Case::Malformed => request.body.payload["extra"] = serde_json::json!(true),
                    Case::WrongKind => {
                        request.body.payload = serde_json::json!({"kind":"weighted_ranking","payload":{
                            "weights":[{"id":"quality","weight_micros":1_000_000}],
                            "candidates":[{"id":"one","scores":[{"weight_id":"quality","value_micros":500_000}]}]
                        }})
                    }
                    Case::WrongSchema => {
                        request.body.schema_id = SchemaId::parse("example/other-input.v1").unwrap()
                    }
                    Case::WrongSchemaDigest => {
                        request.body.schema_sha256 = SchemaDigest::parse("0".repeat(64)).unwrap()
                    }
                    Case::HandlerDomainFailure => {
                        request.body.payload["payload"]["paths"] = serde_json::json!(["../outside"])
                    }
                    Case::Valid | Case::OwnerDenied => {}
                }
                // The common envelope remains internally valid; the ingress owns
                // selected native schema/kind admission, not a stale payload hash.
                request.body.payload_sha256 = ArtifactDigest::hash_bytes(
                    pale_core::canon::to_canonical_string(&request.body.payload)
                        .unwrap()
                        .as_bytes(),
                );
                if !matches!(case, Case::Malformed) {
                    pale_contracts::validate_value(&request.body.payload, "action-input.v1")
                        .unwrap();
                }
            },
            StartAdmission::NativeInput {
                registry: &registry,
                reject_owner: matches!(case, Case::OwnerDenied),
            },
            LocalStartContext::SelfContained,
            |result, request, owners, signatures, _| {
                assert!(
                    HANDLER_INPUTS.with_borrow(Vec::is_empty),
                    "{case:?}: ingress must not execute"
                );
                match case {
                    Case::Malformed => {
                        assert!(matches!(
                            result,
                            Err(LocalOperationStartError::NativeAction(
                                NativeActionInvocationError::MalformedInput(_)
                            ))
                        ));
                        assert_eq!((owners, signatures), (0, 0));
                    }
                    Case::WrongKind => {
                        assert!(
                            matches!(result, Err(LocalOperationStartError::NativeAction(NativeActionInvocationError::Action(error))) if error.code == ActionErrorCode::InputMismatch)
                        );
                        assert_eq!((owners, signatures), (0, 0));
                    }
                    Case::WrongSchema | Case::WrongSchemaDigest => {
                        assert!(matches!(result, Err(LocalOperationStartError::Coordinate)));
                        assert_eq!((owners, signatures), (0, 0));
                    }
                    Case::OwnerDenied => {
                        assert!(matches!(
                            result,
                            Err(LocalOperationStartError::Admission(_))
                        ));
                        assert_eq!((owners, signatures), (1, 0));
                    }
                    Case::Valid | Case::HandlerDomainFailure => {
                        let prepared = result.unwrap_or_else(|error| panic!("{case:?}: {error:?}"));
                        assert_eq!((owners, signatures), (1, 2));
                        drop(prepared);
                        assert!(HANDLER_INPUTS.with_borrow(Vec::is_empty));
                        if matches!(case, Case::HandlerDomainFailure) {
                            // This separate registry invocation proves only the
                            // handler boundary; native custody is covered by its journey.
                            let outcome = registry.execute_value(
                                PATH_INVENTORY_ACTION_ID,
                                request.body().payload.clone(),
                            );
                            assert!(
                                matches!(outcome, Err(NativeActionInvocationError::Action(error)) if error.code == ActionErrorCode::InvalidPath)
                            );
                            HANDLER_INPUTS.with_borrow(|inputs| {
                                assert_eq!(inputs.len(), 1);
                                assert_eq!(
                                    serde_json::to_value(&inputs[0]).unwrap(),
                                    request.body().payload
                                );
                            });
                        }
                    }
                }
            },
        );
    }
}
