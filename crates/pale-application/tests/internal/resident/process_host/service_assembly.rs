//! Bounded exact custody laws for public resident-service composition.

use super::*;
use crate::resident::process_host::ResidentDeploymentAuthoritySelector;

fn retained_delegate_selector(suffix: char) -> ResidentDeploymentAuthoritySelector {
    ResidentDeploymentAuthoritySelector {
        principal_id: ResidentPrincipalIdV1::parse(format!("principal-{suffix}"))
            .expect("principal"),
        deployment_binding_ref: ResidentRecordRefV1::parse(format!("deployment-binding-{suffix}"))
            .expect("binding reference"),
        deployment_binding_sha256: DeploymentBindingDigest::parse(suffix.to_string().repeat(64))
            .expect("binding digest"),
        runtime_authority_ref: ResidentRecordRefV1::parse(format!("runtime-authority-{suffix}"))
            .expect("runtime reference"),
        runtime_authority_sha256: DeploymentRuntimeAuthorityDigest::parse(
            suffix.to_string().repeat(64),
        )
        .expect("runtime digest"),
    }
}

fn retained_delegate_task_key(
    suffix: char,
) -> crate::resident::task_owner::ResidentOperationTaskKey {
    crate::resident::task_owner::ResidentOperationTaskKey::new(
        AuthorityStoreCoordinate::new(
            ControlStoreIdV1::parse(format!("control-store-{suffix}")).expect("control store"),
            ControlIncarnationIdV1::parse(format!("incarnation-{suffix}"))
                .expect("control-store incarnation"),
        ),
        ImmutableRecordReferenceV1 {
            record_ref: ResidentRecordRefV1::parse(format!("dispatch-{suffix}"))
                .expect("dispatch reference"),
            record_sha256: ArtifactDigest::parse(suffix.to_string().repeat(64))
                .expect("dispatch digest"),
        },
    )
}

#[test]
fn active_delegate_response_enters_only_the_fresh_poll_lane() {
    let pending = ResidentServiceRetainedDelegatePhase::Poll(
        super::super::ResidentProcessA2aDelegatePoll::Pending(retained_delegate_task_key('a')),
    );

    assert!(pending.is_runnable());
}

#[test]
fn carrier_free_delegate_recovery_preserves_selector_and_rejects_duplicate_custody() {
    let selector = retained_delegate_selector('a');
    let mut adapter = ResidentServiceDelegateRecoveryAdapter::new((), 2);
    let locator = ResidentOperationFrameLocator::new(
        "operation-handle".to_owned(),
        "operation-digest".to_owned(),
    );
    adapter
        .retain(
            selector.clone(),
            locator.clone(),
            ResidentServiceRetainedDelegatePhase::Poll(
                super::super::ResidentProcessA2aDelegatePoll::Unrouted {
                    selector: selector.clone(),
                    key: retained_delegate_task_key('a'),
                },
            ),
        )
        .expect("the exact carrier-free phase fits bounded custody");

    let selected = adapter
        .next_runnable_retained_delegate()
        .expect("the carrier-free phase is runnable");
    assert_eq!(selected.selector, selector);
    let selected_selector = selected.selector.clone();

    assert!(matches!(
        adapter.retain(
            selected_selector,
            locator,
            ResidentServiceRetainedDelegatePhase::Poll(
                super::super::ResidentProcessA2aDelegatePoll::Pending(retained_delegate_task_key(
                    'b'
                ),),
            ),
        ),
        Err(ResidentOperationFrameFailure::ResidentUnavailable)
    ));
}

#[test]
fn retained_phase_capacity_refuses_and_returns_the_exact_new_phase() {
    let mut custody = BoundedRetainedPhases::new(1);
    assert!(matches!(
        custody.retain("operation-a", String::from("phase-a")),
        Ok(RetainedPhaseAdmission::Retained)
    ));

    let refused = custody
        .retain("operation-b", String::from("phase-b"))
        .expect_err("bounded custody refuses a second distinct phase");

    assert_eq!(refused, "phase-b");
    assert_eq!(custody.iter().collect::<Vec<_>>(), vec!["phase-a"]);
}

#[test]
fn occupied_key_returns_the_new_phase_without_consuming_capacity() {
    let mut custody = BoundedRetainedPhases::new(2);
    assert!(matches!(
        custody.retain("operation-a", String::from("first")),
        Ok(RetainedPhaseAdmission::Retained)
    ));

    let conflict = custody
        .retain("operation-a", String::from("duplicate"))
        .expect("bounded custody reports conflicting evidence");

    let RetainedPhaseAdmission::KeyConflict(conflicting) = conflict else {
        panic!("occupied key must report a conflict")
    };
    assert_eq!(conflicting, "duplicate");
    assert!(!custody.is_full());
    assert_eq!(custody.iter().collect::<Vec<_>>(), vec!["first"]);
}

#[test]
fn full_custody_refuses_work_from_another_deployment_before_allocation() {
    let mut custody = BoundedRetainedPhases::new(1);
    assert!(matches!(
        custody.retain(("deployment-a", "operation-a"), String::from("phase-a")),
        Ok(RetainedPhaseAdmission::Retained)
    ));
    assert!(custody.is_full());

    let refused = custody
        .retain(("deployment-b", "operation-b"), String::from("phase-b"))
        .expect_err("global capacity applies across deployment keys");
    assert_eq!(refused, "phase-b");
    assert_eq!(custody.iter().collect::<Vec<_>>(), vec!["phase-a"]);
}

#[test]
fn cancellation_reservation_occupies_capacity_before_a_phase_exists() {
    let mut custody = BoundedRetainedPhases::<&str, String>::new(1);
    let reservation = custody
        .reserve("cancel-a")
        .expect("one exact Cancel target reserves capacity");

    assert!(custody.is_full());
    assert_eq!(custody.iter().count(), 0);
    assert_eq!(
        custody.reserve("cancel-b").err(),
        Some(RetainedPhaseReservationError::Full)
    );

    custody
        .retain_reserved(reservation, String::from("a2a-handoff"))
        .expect("one typed handoff consumes the reservation");
    assert_eq!(custody.iter().collect::<Vec<_>>(), vec!["a2a-handoff"]);
}

#[test]
fn terminal_cancellation_releases_its_exact_reservation() {
    let mut custody = BoundedRetainedPhases::<&str, String>::new(1);
    let reservation = custody
        .reserve("cancel-a")
        .expect("one exact Cancel target reserves capacity");

    assert!(custody.release(reservation));
    assert!(!custody.is_full());
    assert!(custody.reserve("cancel-b").is_ok());
}

#[test]
fn cancellation_reservation_rejects_an_occupied_exact_key() {
    let mut custody = BoundedRetainedPhases::<&str, String>::new(2);
    let _reservation = custody
        .reserve("cancel-a")
        .expect("one exact Cancel target reserves capacity");

    assert_eq!(
        custody.reserve("cancel-a").err(),
        Some(RetainedPhaseReservationError::KeyConflict)
    );
    assert!(!custody.is_full());
}

#[test]
fn cancellation_lender_error_after_callback_cannot_discard_the_handoff() {
    assert_eq!(
        settle_cancellation_lend(
            Some("typed-handoff"),
            Err(ResidentOperationFrameFailure::ConfigurationUnavailable),
        ),
        Ok("typed-handoff"),
    );
}

#[test]
fn cancellation_lender_success_without_callback_fails_closed() {
    assert_eq!(
        settle_cancellation_lend::<&str>(None, Ok(())),
        Err(ResidentOperationFrameFailure::ConfigurationUnavailable),
    );
}

#[test]
fn cancellation_reservation_key_includes_request_type_digest_locator_and_selector() {
    fn key(suffix: &str) -> RetainedCancellationPhaseKey {
        RetainedCancellationPhaseKey {
            selector: ResidentDeploymentAuthoritySelector {
                principal_id: pale_core::records::ResidentPrincipalIdV1::parse(format!(
                    "principal-{suffix}"
                ))
                .expect("principal"),
                deployment_binding_ref: pale_core::records::ResidentRecordRefV1::parse(format!(
                    "deployment-binding-{suffix}"
                ))
                .expect("binding reference"),
                deployment_binding_sha256: pale_core::records::DeploymentBindingDigest::parse(
                    suffix.repeat(64),
                )
                .expect("binding digest"),
                runtime_authority_ref: pale_core::records::ResidentRecordRefV1::parse(format!(
                    "runtime-authority-{suffix}"
                ))
                .expect("runtime reference"),
                runtime_authority_sha256:
                    pale_core::records::DeploymentRuntimeAuthorityDigest::parse(suffix.repeat(64))
                        .expect("runtime digest"),
            },
            request_id: pale_core::records::ResidentRequestIdV1::parse(format!("cancel-{suffix}"))
                .expect("request id"),
            request_sha256: pale_core::records::CommonRequestEnvelopeDigest::parse(
                suffix.repeat(64),
            )
            .expect("request digest"),
            locator: pale_ports::ControlRecordCoordinate::new(
                pale_core::records::ResidentRecordRefV1::parse(format!("operation/{suffix}"))
                    .expect("operation reference"),
                pale_ports::ControlRecordDigest::parse(suffix.repeat(64)).expect("locator digest"),
            ),
        }
    }

    let exact = key("a");
    let other = key("b");
    assert_ne!(exact, other);
}
