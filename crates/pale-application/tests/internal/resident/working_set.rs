//! Differential laws for the process-local resident-session projection.

use super::*;
use crate::resident::generation::tests::activation_compatibility;
use crate::resident::journal::tests::session_lease_for;
use crate::resident::lifecycle::tests::{compiled_generation, effective_profile, initial_active};
use crate::resident_fixture::session_lease_for_profile_with_native_correlation;
use crate::resident_fixture::{active, advertised, fence, published};

fn assert_projection(
    working: &ResidentSessionWorkingSet,
    lifecycle: &GenerationLifecycle,
) -> Result<(), ResidentSessionWorkingSetError> {
    assert_eq!(working.snapshot()?, lifecycle.snapshot().session_pins);
    Ok(())
}

fn projected_pin(
    lifecycle: &GenerationLifecycle,
    session_admission_id: &SessionAdmissionIdV1,
) -> SessionGenerationPin {
    lifecycle
        .session_pins()
        .find(|pin| pin.session_admission_id() == session_admission_id)
        .expect("semantic successor contains the admitted pin")
        .clone()
}

fn apply_pin(
    working: &mut ResidentSessionWorkingSet,
    pin: SessionGenerationPin,
) -> Result<(), ResidentSessionWorkingSetError> {
    let prepared = working.prepare_pin(pin)?;
    working.apply(prepared)
}

fn apply_drain(
    working: &mut ResidentSessionWorkingSet,
    session_admission_id: &SessionAdmissionIdV1,
    lease_digest: &SessionAdmissionLeaseDigest,
) -> Result<(), ResidentSessionWorkingSetError> {
    let prepared = working.prepare_drain(session_admission_id, lease_digest)?;
    working.apply(prepared)
}

fn session_lease<'a>(
    working: &'a ResidentSessionWorkingSet,
    session_admission_id: &SessionAdmissionIdV1,
) -> Result<&'a SessionAdmissionLeaseV1, ResidentSessionWorkingSetError> {
    let position = working
        .by_session
        .binary_search_by(|entry| entry.session_admission_id.cmp(session_admission_id))
        .map_err(|_| ResidentSessionWorkingSetError::Contradiction)?;
    let entry = &working.by_session[position];
    let lease = working.bodies.get(entry.handle)?;
    (lease.session_admission_id() == session_admission_id)
        .then_some(lease)
        .ok_or(ResidentSessionWorkingSetError::Contradiction)
}

#[test]
fn projection_matches_every_successful_semantic_trace_prefix(
) -> Result<(), Box<dyn std::error::Error>> {
    let old_generation = compiled_generation('4');
    let new_generation = compiled_generation('5');
    let (mut lifecycle, first_published, first_advertised, first_fence, first_active) =
        initial_active(&old_generation);
    let mut working = ResidentSessionWorkingSet::rebuild(&lifecycle, 4)?;
    assert_projection(&working, &lifecycle)?;

    let old_lease = session_lease_for(
        &old_generation,
        &first_active,
        &first_fence,
        "session-old",
        10,
    );
    let pin_transition = lifecycle.pin_session(&old_lease)?;
    apply_pin(
        &mut working,
        projected_pin(pin_transition.successor(), old_lease.session_admission_id()),
    )?;
    lifecycle = pin_transition.into_successor();
    assert_projection(&working, &lifecycle)?;
    assert_eq!(
        session_lease(&working, old_lease.session_admission_id())?,
        &old_lease
    );

    let rollback_transition =
        lifecycle.retain_for_rollback(old_generation.compiled_generation_digest().clone())?;
    lifecycle = rollback_transition.into_successor();
    assert_projection(&working, &lifecycle)?;

    let second_published = published(&new_generation, 4, Some(&first_published));
    lifecycle = lifecycle
        .publish(second_published.clone())?
        .into_successor();
    assert_projection(&working, &lifecycle)?;
    let second_advertised = advertised(&second_published, 5, Some(&first_advertised));
    lifecycle = lifecycle
        .advertise(second_advertised.clone())?
        .into_successor();
    assert_projection(&working, &lifecycle)?;
    let second_fence = fence(&new_generation, 2, Some(&first_fence));
    let second_active = active(&second_advertised, &second_fence, 6, Some(&first_active));
    lifecycle = lifecycle
        .activate(
            second_active.clone(),
            second_fence.clone(),
            &activation_compatibility(
                Some(old_generation.compiled_generation_digest().clone()),
                new_generation.compiled_generation_digest().clone(),
            ),
        )?
        .into_successor();
    assert_projection(&working, &lifecycle)?;
    assert_eq!(
        working.sessions_retaining(old_generation.compiled_generation_digest())?,
        vec![old_lease.session_admission_id().clone()]
    );

    let new_lease = session_lease_for(
        &new_generation,
        &second_active,
        &second_fence,
        "session-new",
        20,
    );
    let pin_transition = lifecycle.pin_session(&new_lease)?;
    apply_pin(
        &mut working,
        projected_pin(pin_transition.successor(), new_lease.session_admission_id()),
    )?;
    lifecycle = pin_transition.into_successor();
    assert_projection(&working, &lifecycle)?;
    assert_eq!(
        session_lease(&working, new_lease.session_admission_id())?,
        &new_lease
    );
    assert_eq!(
        working.sessions_retaining(new_generation.compiled_generation_digest())?,
        vec![new_lease.session_admission_id().clone()]
    );

    let drain_transition = lifecycle.drain_session(
        old_lease.session_admission_id(),
        old_lease.session_admission_lease_digest(),
    )?;
    apply_drain(
        &mut working,
        old_lease.session_admission_id(),
        old_lease.session_admission_lease_digest(),
    )?;
    lifecycle = drain_transition.into_successor();
    assert_projection(&working, &lifecycle)?;
    assert!(working
        .sessions_retaining(old_generation.compiled_generation_digest())?
        .is_empty());
    Ok(())
}

#[test]
fn rejected_semantic_transitions_never_change_the_projection(
) -> Result<(), Box<dyn std::error::Error>> {
    let generation = compiled_generation('4');
    let (lifecycle, _, _, fence, active) = initial_active(&generation);
    let lease = session_lease_for(&generation, &active, &fence, "session-1", 10);
    let transition = lifecycle.pin_session(&lease)?;
    let working = ResidentSessionWorkingSet::rebuild(transition.successor(), 2)?;
    let lifecycle = transition.into_successor();
    let before = working.snapshot()?;

    assert!(lifecycle.pin_session(&lease).is_err());
    let conflicting = session_lease_for(&generation, &active, &fence, "session-1", 11);
    assert!(lifecycle.pin_session(&conflicting).is_err());
    assert_eq!(working.snapshot()?, before);

    let unknown = SessionAdmissionIdV1::parse("unknown-session")?;
    assert!(working
        .prepare_drain(&unknown, lease.session_admission_lease_digest())
        .is_err());
    assert_eq!(working.snapshot()?, before);
    Ok(())
}

#[test]
fn capacity_failure_returns_no_partial_projection() -> Result<(), Box<dyn std::error::Error>> {
    let generation = compiled_generation('4');
    let (lifecycle, _, _, fence, active) = initial_active(&generation);
    let first = session_lease_for(&generation, &active, &fence, "session-a", 10);
    let first_transition = lifecycle.pin_session(&first)?;
    let working = ResidentSessionWorkingSet::rebuild(first_transition.successor(), 1)?;
    let lifecycle = first_transition.into_successor();
    let before = working.snapshot()?;
    let second = session_lease_for(&generation, &active, &fence, "session-b", 10);
    let second_transition = lifecycle.pin_session(&second)?;

    assert!(matches!(
        working.prepare_pin(projected_pin(
            second_transition.successor(),
            second.session_admission_id(),
        )),
        Err(ResidentSessionWorkingSetError::CapacityExceeded)
    ));
    assert_eq!(working.snapshot()?, before);
    assert!(matches!(
        ResidentSessionWorkingSet::rebuild(second_transition.successor(), 1),
        Err(ResidentSessionWorkingSetError::CapacityExceeded)
    ));
    Ok(())
}

#[test]
fn drain_and_reuse_rejects_the_old_body_handle() -> Result<(), Box<dyn std::error::Error>> {
    let generation = compiled_generation('4');
    let (lifecycle, _, _, fence, active) = initial_active(&generation);
    let first = session_lease_for(&generation, &active, &fence, "session-reused", 10);
    let transition = lifecycle.pin_session(&first)?;
    let mut working = ResidentSessionWorkingSet::rebuild(transition.successor(), 1)?;
    let lifecycle = transition.into_successor();
    let old_handle = working.by_session[0].handle;

    let transition = lifecycle.drain_session(
        first.session_admission_id(),
        first.session_admission_lease_digest(),
    )?;
    apply_drain(
        &mut working,
        first.session_admission_id(),
        first.session_admission_lease_digest(),
    )?;
    let lifecycle = transition.into_successor();
    assert_eq!(working.bodies.get(old_handle), Err(ArenaError::Stale));

    let replacement = session_lease_for(&generation, &active, &fence, "session-reused", 11);
    let transition = lifecycle.pin_session(&replacement)?;
    apply_pin(
        &mut working,
        projected_pin(transition.successor(), replacement.session_admission_id()),
    )?;
    assert_eq!(working.bodies.get(old_handle), Err(ArenaError::Stale));
    assert_projection(&working, transition.successor())?;
    Ok(())
}

#[test]
fn rebuild_is_deterministic_owned_and_invalidates_prior_handles(
) -> Result<(), Box<dyn std::error::Error>> {
    let generation = compiled_generation('4');
    let (lifecycle, _, _, fence, active) = initial_active(&generation);
    let first = session_lease_for(&generation, &active, &fence, "session-z", 10);
    let lifecycle = lifecycle.pin_session(&first)?.into_successor();
    let second = session_lease_for(&generation, &active, &fence, "session-a", 20);
    let lifecycle = lifecycle.pin_session(&second)?.into_successor();
    let original = ResidentSessionWorkingSet::rebuild(&lifecycle, 2)?;
    let old_handle = original.by_session[0].handle;
    let expected = original.snapshot()?;

    let rebuilt = ResidentSessionWorkingSet::rebuild(&lifecycle, 2)?;
    assert_eq!(
        session_lease(&rebuilt, first.session_admission_id())?,
        &first
    );
    assert_eq!(
        session_lease(&rebuilt, second.session_admission_id())?,
        &second
    );
    assert_eq!(rebuilt.snapshot()?, expected);
    assert_eq!(rebuilt.bodies.get(old_handle), Err(ArenaError::Foreign));
    assert_eq!(
        rebuilt
            .snapshot()?
            .into_keys()
            .map(|identity| identity.as_str().to_owned())
            .collect::<Vec<_>>(),
        vec!["session-a".to_owned(), "session-z".to_owned()]
    );
    Ok(())
}

#[test]
fn native_activity_lookup_is_exact_bounded_and_refuses_ambiguity(
) -> Result<(), Box<dyn std::error::Error>> {
    let generation = compiled_generation('4');
    let (lifecycle, _, _, fence, active) = initial_active(&generation);
    let profile = effective_profile();
    let activity = HarnessActivityIdentity::try_from_host(Some("native-session"), None)?
        .expect("native session identity is present");
    let observed = NativeSessionCorrelationV1::Observed {
        native_session_identity: activity.clone(),
    };
    let first = session_lease_for_profile_with_native_correlation(
        &generation,
        &active,
        &fence,
        &profile,
        "session-a",
        10,
        observed.clone(),
    );
    let lifecycle = lifecycle.pin_session(&first)?.into_successor();
    let working = ResidentSessionWorkingSet::rebuild(&lifecycle, 2)?;
    assert_eq!(
        working.session_by_native_activity(&activity)?,
        ResidentNativeSessionLookup::Exact(first.session_admission_lease_digest().clone())
    );
    let other = HarnessActivityIdentity::try_from_host(Some("other-session"), None)?
        .expect("other native session identity is present");
    assert_eq!(
        working.session_by_native_activity(&other)?,
        ResidentNativeSessionLookup::Missing
    );

    let second = session_lease_for_profile_with_native_correlation(
        &generation,
        &active,
        &fence,
        &profile,
        "session-b",
        10,
        observed,
    );
    let lifecycle = lifecycle.pin_session(&second)?.into_successor();
    let working = ResidentSessionWorkingSet::rebuild(&lifecycle, 2)?;
    assert_eq!(
        working.session_by_native_activity(&activity)?,
        ResidentNativeSessionLookup::Ambiguous
    );
    Ok(())
}
