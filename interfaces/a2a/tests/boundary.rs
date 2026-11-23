//! The A2A interface reuses SDK-owned Delegate operations.

#[test]
fn a2a_capabilities_are_limited_to_inspection_and_delegate_operations() {
    let capability = pale_a2a::delegate_handle_capability()
        .expect("the exact SDK capability is available to this interface");
    assert_eq!(
        capability.id(),
        pale_sdk::ApiCapabilityId::A2aDelegateHandleInspection
    );
    assert_eq!(capability.class(), pale_sdk::ApiCapabilityClass::Portable);
    assert_eq!(
        capability.effect(),
        pale_sdk::ApiCapabilityEffect::EffectFree
    );
    assert!(capability.eligible_for(pale_sdk::ApiLeaf::A2a));
    let capabilities = pale_sdk::capability_inventory()
        .iter()
        .filter(|candidate| candidate.eligible_for(pale_sdk::ApiLeaf::A2a))
        .collect::<Vec<_>>();
    assert_eq!(
        capabilities
            .iter()
            .map(|candidate| candidate.id())
            .collect::<Vec<_>>(),
        [
            pale_sdk::ApiCapabilityId::A2aDelegateHandleInspection,
            pale_sdk::ApiCapabilityId::ResidentOperationObservation,
            pale_sdk::ApiCapabilityId::ResidentDelegateStart,
            pale_sdk::ApiCapabilityId::ResidentDelegateAdvance,
            pale_sdk::ApiCapabilityId::ResidentOperationCancellation,
        ]
    );
    assert_eq!(
        capabilities
            .iter()
            .map(|candidate| candidate.class())
            .collect::<Vec<_>>(),
        [
            pale_sdk::ApiCapabilityClass::Portable,
            pale_sdk::ApiCapabilityClass::Resident,
            pale_sdk::ApiCapabilityClass::Resident,
            pale_sdk::ApiCapabilityClass::Resident,
            pale_sdk::ApiCapabilityClass::Resident,
        ]
    );
    assert_eq!(
        capabilities
            .iter()
            .map(|candidate| candidate.effect())
            .collect::<Vec<_>>(),
        [
            pale_sdk::ApiCapabilityEffect::EffectFree,
            pale_sdk::ApiCapabilityEffect::EffectFree,
            pale_sdk::ApiCapabilityEffect::ResidentEffect,
            pale_sdk::ApiCapabilityEffect::ResidentEffect,
            pale_sdk::ApiCapabilityEffect::ResidentEffect,
        ]
    );
}

#[test]
fn callable_surface_reuses_sdk_requests_and_results() {
    type Start = fn(
        &pale_sdk::PaleApi,
        &pale_sdk::ResidentAttachment,
        pale_sdk::ResidentDelegateStartRequest,
    ) -> Result<pale_sdk::ResidentOperationObservation, pale_sdk::ApiError>;
    type Advance = fn(
        &pale_sdk::PaleApi,
        &pale_sdk::ResidentAttachment,
        pale_sdk::ResidentDelegateAdvanceRequest,
    ) -> Result<pale_sdk::ResidentDelegateAdvanceOutcome, pale_sdk::ApiError>;
    type Observe = fn(
        &pale_sdk::PaleApi,
        &pale_sdk::ResidentAttachment,
        pale_sdk::ResidentOperationObservationRequest,
    ) -> Result<pale_sdk::ResidentOperationObservation, pale_sdk::ApiError>;
    type Cancel = fn(
        &pale_sdk::PaleApi,
        &pale_sdk::ResidentAttachment,
        pale_sdk::ResidentOperationCancellationRequest,
    )
        -> Result<pale_sdk::ResidentOperationCancellationOutcome, pale_sdk::ApiError>;

    let _: Start = pale_a2a::start_delegate;
    let _: Advance = pale_a2a::advance_delegate;
    let _: Observe = pale_a2a::observe_operation;
    let _: Cancel = pale_a2a::cancel_operation;
}

#[test]
fn malformed_delegate_handle_is_rejected_without_transport() {
    let api = pale_sdk::PaleApi::new().expect("embedded API dependencies are coherent");
    let error = pale_a2a::inspect_delegate_handle(
        &api,
        pale_sdk::A2aDelegateHandleRequest::new("operations/delegate-1".to_owned(), b"{}".to_vec()),
    )
    .expect_err("an empty object is not an operation handle");
    assert_eq!(error.diagnostic().code(), "a2a_delegate_handle_invalid");
}
