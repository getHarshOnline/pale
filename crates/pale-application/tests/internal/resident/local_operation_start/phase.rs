//! Local operation start phase-transition tests.

use super::*;
use std::cell::Cell;

#[test]
fn observation_failure_never_reopens_the_handle_signer() {
    let (compilation, materializations, profile) =
        crate::local_operation_start_fixture::source_generation_inputs_with(|_| {}, |_| {});
    let generation = crate::resident::AdmittedCompiledGeneration::admit(
        compilation,
        materializations,
        crate::resident_fixture::synthetic_set_policy(),
    )
    .expect("actual admitted generation");
    crate::local_operation_start_fixture::with_invoke_start_input(
        &generation,
        &profile,
        "handle-phase",
        |_| {},
        |input, _, _, _| {
            let handle_signs = Cell::new(0);
            let observation_signs = Cell::new(0);
            let result = prepare_local_invoke_start_retained(
                input,
                &mut crate::session_admission_fixture::ProvingAuthority,
                |_, _, _, _, _, _, _, _| Ok(()),
                |_, _, _| {
                    handle_signs.set(handle_signs.get() + 1);
                    Ok(crate::local_operation_start_fixture::signature())
                },
                |_, _, _| {
                    observation_signs.set(observation_signs.get() + 1);
                    Err(InvalidResidentRecordV1::ContextualAdmission {
                        record: "test observation signer",
                        invariant: "first observation attempt rejected",
                    })
                },
            );
            let failure = match result {
                Ok(_) => panic!("the first observation signer must reject the record"),
                Err(failure) => failure,
            };
            let RetainedLocalOperationStartFailure::HandleIssued(failure) = failure else {
                panic!("the exact signed handle must be retained")
            };
            let _signed = failure
                .continue_observation(|_, _, _| {
                    observation_signs.set(observation_signs.get() + 1);
                    Ok(crate::local_operation_start_fixture::signature())
                })
                .expect("the same handle accepts a later observation signature");

            assert_eq!(handle_signs.get(), 1, "the handle is signed once");
            assert_eq!(
                observation_signs.get(),
                2,
                "only observation issuance is retried"
            );
        },
    );
}
