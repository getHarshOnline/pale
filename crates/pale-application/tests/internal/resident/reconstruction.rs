//! Complete and read-only-degraded resident cold-reconstruction law tests.
//!
//! These tests keep the one-shot compiler as the oracle, prove exact retained
//! generation custody, and show that a quarantined suffix never reaches
//! generation-body reconstruction.

use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    CompiledGenerationDigest, LogicalRepositoryIdV1, RingCursorV1, WakeEpochV1,
};
use pale_kernel::{WakeReadError, WakeTokenRing};

use super::*;
use crate::resident::generation::tests::revise_profile_policy_description;
use crate::resident::journal::tests::{
    empty_journal, genesis, limits, recover_for_resident_test, session_lease_for, store,
    MemoryAuthorityStore,
};
use crate::resident::journal::AuthorityRecoveryOutcome;
use crate::resident::lifecycle::tests::compiled_generation;
use crate::resident::{
    GenerationLifecycle, GenerationLifecycleTransition, RecoveryReferenceKnowledge,
};
use crate::resident_fixture::{
    active, advertised, applied_evidence, compile_live, fence, live_workspace, published,
    synthetic_set_policy, RecoveryResolver,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct FixedGenerationInputSource {
    calls: usize,
    requested: Option<BTreeSet<CompiledGenerationDigest>>,
    inputs: Vec<ResidentGenerationReconstructionInput>,
}

impl ResidentGenerationInputSource for FixedGenerationInputSource {
    fn load_generation_inputs(
        &mut self,
        required_generations: &BTreeSet<CompiledGenerationDigest>,
    ) -> Result<Vec<ResidentGenerationReconstructionInput>, ResidentGenerationInputSourceError>
    {
        self.calls += 1;
        self.requested = Some(required_generations.clone());
        Ok(std::mem::take(&mut self.inputs))
    }
}

#[derive(Default)]
struct UnavailableGenerationInputSource {
    calls: usize,
}

impl ResidentGenerationInputSource for UnavailableGenerationInputSource {
    fn load_generation_inputs(
        &mut self,
        _required_generations: &BTreeSet<CompiledGenerationDigest>,
    ) -> Result<Vec<ResidentGenerationReconstructionInput>, ResidentGenerationInputSourceError>
    {
        self.calls += 1;
        Err(ResidentGenerationInputSourceError::Unavailable)
    }
}

struct ReconstructionFixture {
    recovery: AuthorityRecoveryOutcome,
    source: FixedGenerationInputSource,
    originals: BTreeMap<CompiledGenerationDigest, AdmittedCompiledGeneration>,
    required: BTreeSet<CompiledGenerationDigest>,
    initial_digest: CompiledGenerationDigest,
    successor_digest: CompiledGenerationDigest,
}

fn materializations(compilation: &Compilation) -> Vec<AppliedRepositoryMaterialization> {
    vec![applied_evidence(compilation).into_materialization()]
}

fn build_fixture(
    quarantine_from_start: bool,
    retain_predecessor: bool,
) -> Result<ReconstructionFixture, Box<dyn std::error::Error>> {
    let (workspace, probe) = live_workspace();

    let initial_compilation = compile_live(&workspace, &probe);
    let initial_materializations = materializations(&initial_compilation);
    let initial = AdmittedCompiledGeneration::admit(
        initial_compilation,
        initial_materializations,
        synthetic_set_policy(),
    )?;
    let initial_digest = initial.record().compiled_generation_digest().clone();
    let initial_rebuild_compilation = compile_live(&workspace, &probe);
    let initial_rebuild_materializations = materializations(&initial_rebuild_compilation);

    revise_profile_policy_description(&workspace, "Cold-rebuild successor profile");
    let successor_compilation = compile_live(&workspace, &probe);
    let successor_materializations = materializations(&successor_compilation);
    let successor = AdmittedCompiledGeneration::admit_successor(
        &initial,
        successor_compilation,
        successor_materializations,
        synthetic_set_policy(),
    )?;
    let successor_digest = successor.record().compiled_generation_digest().clone();
    let successor_rebuild_compilation = compile_live(&workspace, &probe);
    let successor_rebuild_materializations = materializations(&successor_rebuild_compilation);

    let port = MemoryAuthorityStore::default();
    let mut journal = empty_journal(&port);
    let mut resolver = RecoveryResolver::default();
    let (required, inputs, originals) = if retain_predecessor {
        let first_head = published(initial.record(), 1, None);
        let second_head = published(successor.record(), 2, Some(&first_head));
        let first_state = journal
            .commit_transition(genesis().publish(first_head.clone())?)?
            .into_parts()
            .1
            .lifecycle()
            .clone();
        let _committed = journal.commit_transition(first_state.publish(second_head.clone())?)?;
        resolver.insert_generation(initial.record());
        resolver.insert_published(&first_head);
        resolver.insert_generation(successor.record());
        resolver.insert_published(&second_head);
        (
            BTreeSet::from([initial_digest.clone(), successor_digest.clone()]),
            vec![
                ResidentGenerationReconstructionInput::new(
                    initial_digest.clone(),
                    initial_rebuild_compilation,
                    initial_rebuild_materializations,
                ),
                ResidentGenerationReconstructionInput::new(
                    successor_digest.clone(),
                    successor_rebuild_compilation,
                    successor_rebuild_materializations,
                ),
            ],
            BTreeMap::from([
                (initial_digest.clone(), initial),
                (successor_digest.clone(), successor),
            ]),
        )
    } else {
        let head = published(successor.record(), 1, None);
        let _committed = journal.commit_transition(genesis().publish(head.clone())?)?;
        resolver.insert_generation(successor.record());
        resolver.insert_published(&head);
        (
            BTreeSet::from([successor_digest.clone()]),
            vec![ResidentGenerationReconstructionInput::new(
                successor_digest.clone(),
                successor_rebuild_compilation,
                successor_rebuild_materializations,
            )],
            BTreeMap::from([(successor_digest.clone(), successor)]),
        )
    };
    drop(journal);
    if quarantine_from_start {
        port.quarantine_from(1);
    }

    let recovery = recover_for_resident_test(&port, store(), limits(), genesis(), &resolver)?;

    let source = FixedGenerationInputSource {
        calls: 0,
        requested: None,
        inputs,
    };
    Ok(ReconstructionFixture {
        recovery,
        source,
        originals,
        required,
        initial_digest,
        successor_digest,
    })
}

fn parameters(epoch: u64) -> ResidentReconstructionParameters {
    ResidentReconstructionParameters::new(synthetic_set_policy(), 8, 7, WakeEpochV1::new(epoch))
}

fn install(transition: GenerationLifecycleTransition) -> GenerationLifecycle {
    transition.into_successor()
}

#[test]
fn resident_session_capacity_rejection_precedes_generation_input_loading() -> TestResult {
    let generation = compiled_generation('4');
    let published = published(&generation, 1, None);
    let advertised = advertised(&published, 2, None);
    let fence = fence(&generation, 1, None);
    let active = active(&advertised, &fence, 3, None);
    let lease = session_lease_for(&generation, &active, &fence, "session-capacity", 10);
    let port = MemoryAuthorityStore::default();
    let mut journal = empty_journal(&port);
    let state = journal
        .commit_transition(genesis().publish(published.clone())?)?
        .into_parts()
        .1
        .lifecycle()
        .clone();
    let state = journal
        .commit_transition(state.advertise(advertised.clone())?)?
        .into_parts()
        .1
        .lifecycle()
        .clone();
    let state = journal
        .commit_transition(state.activate(
            active.clone(),
            fence.clone(),
            &crate::resident::generation::tests::activation_compatibility(
                None,
                generation.compiled_generation_digest().clone(),
            ),
        )?)?
        .into_parts()
        .1
        .lifecycle()
        .clone();
    let _ = journal.commit_transition(state.pin_session(&lease)?)?;
    drop(journal);

    let mut resolver = RecoveryResolver::default();
    resolver.insert_generation(&generation);
    resolver.insert_published(&published);
    resolver.insert_advertised(&advertised);
    resolver.insert_fence(&fence);
    resolver.insert_active(&active);
    resolver.insert_effective_profile(&crate::resident::lifecycle::tests::effective_profile());
    crate::resident::session_lease_custody::retain_session_lease_body(
        &port,
        &store(),
        &lease,
        limits().maximum_entry_bytes(),
    )
    .expect("session lease body retains before its journal pin");
    let recovery = recover_for_resident_test(&port, store(), limits(), genesis(), &resolver)?;
    let mut source = UnavailableGenerationInputSource::default();
    let error = match reconstruct_resident(
        recovery,
        &mut source,
        ResidentReconstructionParameters::new(synthetic_set_policy(), 0, 7, WakeEpochV1::new(56)),
    ) {
        Err(error) => error,
        Ok(_) => {
            return Err(
                std::io::Error::other("one recovered pin fit a zero-capacity projection").into(),
            );
        }
    };

    assert!(matches!(
        error,
        ResidentReconstructionError::ResidentSessions(
            ResidentSessionWorkingSetError::CapacityExceeded
        )
    ));
    assert_eq!(source.calls, 0);
    Ok(())
}

#[test]
fn complete_recovery_rebuilds_exact_generations_and_fresh_wake_state() -> TestResult {
    let ReconstructionFixture {
        recovery,
        mut source,
        originals,
        required,
        initial_digest,
        successor_digest,
    } = build_fixture(false, true)?;
    let old_ring = WakeTokenRing::new(
        7,
        WakeEpochV1::new(40),
        RingCursorV1::new(19),
        Some(pale_core::records::JournalSequenceV1::new(2)?),
    )?;
    let predecessor_cursor = old_ring.starting_cursor();

    let outcome = reconstruct_resident(recovery, &mut source, parameters(41))?;
    let ResidentColdRecoveryOutcome::Complete(resident) = outcome else {
        return Err(std::io::Error::other("complete recovery was quarantined").into());
    };
    assert_eq!(source.calls, 1);
    assert_eq!(source.requested.as_ref(), Some(&required));
    assert_eq!(
        resident
            .generations
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
        required
    );
    assert_eq!(
        resident.authority.journal_head().map(|value| value.get()),
        Some(2)
    );

    for (digest, original) in &originals {
        let Some(rebuilt) = resident.generations.get(digest) else {
            return Err(std::io::Error::other("retained generation was not rebuilt").into());
        };
        let repository_id = LogicalRepositoryIdV1::parse("engine").expect("repository identity");
        assert_eq!(rebuilt.record(), original.record());
        assert_eq!(
            rebuilt.workspace_derivation_sources(),
            original.workspace_derivation_sources()
        );
        assert_eq!(
            rebuilt
                .repository(&repository_id)
                .map(crate::compile::CompiledRepo::id),
            Some(&repository_id)
        );
        let rebuilt_witnesses = rebuilt
            .repository(&repository_id)
            .expect("rebuilt repository")
            .render()
            .support
            .iter()
            .map(|support| &support.execution_path_witness)
            .collect::<Vec<_>>();
        let original_witnesses = original
            .repository(&repository_id)
            .expect("original repository")
            .render()
            .support
            .iter()
            .map(|support| &support.execution_path_witness)
            .collect::<Vec<_>>();
        assert_eq!(
            rebuilt_witnesses, original_witnesses,
            "reconstruction must retain exact execution-path proof closure"
        );
    }

    let Some(rebuilt_successor) = resident.generations.get(&successor_digest) else {
        return Err(std::io::Error::other("successor was not rebuilt").into());
    };
    assert!(!rebuilt_successor
        .activation_compatibility()
        .admits(Some(&initial_digest), &successor_digest));

    let ring = &resident.wake_ring;
    assert_eq!(ring.capacity(), 7);
    assert_eq!(ring.epoch(), WakeEpochV1::new(41));
    assert_eq!(ring.starting_cursor().cursor(), RingCursorV1::new(0));
    assert_eq!(
        ring.starting_cursor()
            .journal_sequence()
            .map(|value| value.get()),
        Some(2)
    );
    assert_eq!(ring.current_cursor(), ring.starting_cursor());
    assert!(ring.read_after(ring.starting_cursor())?.is_empty());
    assert!(matches!(
        ring.read_after(predecessor_cursor),
        Err(WakeReadError::StaleEpoch {
            cursor_epoch,
            ring_epoch,
            replay_after: Some(replay_after),
            durable_through: Some(durable_through),
            ..
        }) if cursor_epoch == WakeEpochV1::new(40)
            && ring_epoch == WakeEpochV1::new(41)
            && replay_after.get() == 2
            && durable_through.get() == 2
    ));

    Ok(())
}

#[test]
fn missing_and_substituted_owner_inputs_fail_closed() -> TestResult {
    let mut missing = build_fixture(false, true)?;
    missing.source.inputs.pop();
    assert!(matches!(
        reconstruct_resident(missing.recovery, &mut missing.source, parameters(50)),
        Err(ResidentReconstructionError::MissingGeneration { .. })
    ));

    let mut substituted = build_fixture(false, true)?;
    let [initial, successor] = substituted.source.inputs.as_mut_slice() else {
        return Err(std::io::Error::other("fixture input cardinality changed").into());
    };
    std::mem::swap(&mut initial.compilation, &mut successor.compilation);
    std::mem::swap(
        &mut initial.materializations,
        &mut successor.materializations,
    );
    assert!(matches!(
        reconstruct_resident(
            substituted.recovery,
            &mut substituted.source,
            parameters(51),
        ),
        Err(ResidentReconstructionError::GenerationSubstitution { .. })
    ));
    Ok(())
}

#[test]
fn retained_successor_rebuilds_without_predecessor_custody() -> TestResult {
    let ReconstructionFixture {
        recovery,
        mut source,
        originals,
        required,
        initial_digest,
        successor_digest,
    } = build_fixture(false, false)?;
    assert_eq!(required, BTreeSet::from([successor_digest.clone()]));
    assert!(!required.contains(&initial_digest));

    let outcome = reconstruct_resident(recovery, &mut source, parameters(55))?;
    let ResidentColdRecoveryOutcome::Complete(resident) = outcome else {
        return Err(std::io::Error::other("successor-only recovery was quarantined").into());
    };
    let Some(rebuilt) = resident.generations.get(&successor_digest) else {
        return Err(std::io::Error::other("retained successor was not rebuilt").into());
    };
    let Some(original) = originals.get(&successor_digest) else {
        return Err(std::io::Error::other("successor oracle is absent").into());
    };
    assert_eq!(rebuilt.record(), original.record());
    assert!(rebuilt.derivation_receipt().is_none());
    assert!(rebuilt.successor_impact().is_none());
    assert!(!rebuilt
        .activation_compatibility()
        .admits(Some(&initial_digest), &successor_digest));
    Ok(())
}

#[test]
fn recovered_active_content_needs_no_compatibility_and_rollback_remains_separate() -> TestResult {
    let (workspace, probe) = live_workspace();
    let compilation = compile_live(&workspace, &probe);
    let applied = materializations(&compilation);
    let admitted = AdmittedCompiledGeneration::admit(compilation, applied, synthetic_set_policy())?;
    let digest = admitted.record().compiled_generation_digest().clone();
    let rebuild_compilation = compile_live(&workspace, &probe);
    let rebuild_materializations = materializations(&rebuild_compilation);

    let published_one = published(admitted.record(), 1, None);
    let advertised_one = advertised(&published_one, 2, None);
    let fence_one = fence(admitted.record(), 1, None);
    let active_one = active(&advertised_one, &fence_one, 3, None);
    let port = MemoryAuthorityStore::default();
    let mut journal = empty_journal(&port);
    let state = journal
        .commit_transition(genesis().publish(published_one.clone())?)?
        .into_parts()
        .1
        .lifecycle()
        .clone();
    let state = journal
        .commit_transition(state.advertise(advertised_one.clone())?)?
        .into_parts()
        .1
        .lifecycle()
        .clone();
    let _ = journal.commit_transition(state.activate(
        active_one.clone(),
        fence_one.clone(),
        admitted.activation_compatibility(),
    )?)?;
    drop(journal);

    let mut resolver = RecoveryResolver::default();
    resolver.insert_generation(admitted.record());
    resolver.insert_published(&published_one);
    resolver.insert_advertised(&advertised_one);
    resolver.insert_fence(&fence_one);
    resolver.insert_active(&active_one);
    let recovery = recover_for_resident_test(&port, store(), limits(), genesis(), &resolver)?;
    let mut source = FixedGenerationInputSource {
        calls: 0,
        requested: None,
        inputs: vec![ResidentGenerationReconstructionInput::new(
            digest.clone(),
            rebuild_compilation,
            rebuild_materializations,
        )],
    };
    let outcome = reconstruct_resident(recovery, &mut source, parameters(56))?;
    let ResidentColdRecoveryOutcome::Complete(resident) = outcome else {
        return Err(std::io::Error::other("active recovery was quarantined").into());
    };
    let ReconstructedResident {
        authority,
        generations,
        sessions: _,
        wake_ring: _,
    } = *resident;
    let (_, recovered_authority, _) = authority.into_parts();
    let recovered = recovered_authority.lifecycle().clone();
    assert_eq!(
        recovered.snapshot().active,
        Some(active_one.active_generation_head_digest().clone())
    );
    let Some(content) = generations.get(&digest) else {
        return Err(std::io::Error::other("active generation content is absent").into());
    };

    let state = install(recovered.retain_for_rollback(digest.clone())?);
    let published_two = published(content.record(), 4, Some(&published_one));
    let state = install(state.publish(published_two.clone())?);
    let advertised_two = advertised(&published_two, 5, Some(&advertised_one));
    let state = install(state.advertise(advertised_two.clone())?);
    let fence_two = fence(content.record(), 2, Some(&fence_one));
    let active_two = active(&advertised_two, &fence_two, 6, Some(&active_one));
    assert!(matches!(
        state.activate(
            active_two.clone(),
            fence_two.clone(),
            content.activation_compatibility(),
        ),
        Err(
            crate::resident::GenerationLifecycleError::CoordinateMismatch(
                "activation compatibility"
            )
        )
    ));
    let rolled_back = install(state.rollback(active_two.clone(), fence_two)?);
    assert_eq!(
        rolled_back.snapshot().active,
        Some(active_two.active_generation_head_digest().clone())
    );
    Ok(())
}

#[test]
fn unavailable_owner_custody_fails_before_any_projection_is_returned() -> TestResult {
    let fixture = build_fixture(false, false)?;
    let mut source = UnavailableGenerationInputSource::default();
    assert!(matches!(
        reconstruct_resident(fixture.recovery, &mut source, parameters(57)),
        Err(ResidentReconstructionError::InputSource(
            ResidentGenerationInputSourceError::Unavailable
        ))
    ));
    assert_eq!(source.calls, 1);
    Ok(())
}

#[test]
fn degraded_recovery_retains_prefix_without_requesting_generation_inputs() -> TestResult {
    let mut fixture = build_fixture(true, true)?;
    let outcome = reconstruct_resident(fixture.recovery, &mut fixture.source, parameters(60))?;
    let ResidentColdRecoveryOutcome::Degraded(degraded) = outcome else {
        return Err(std::io::Error::other("quarantine produced resident authority").into());
    };
    assert_eq!(fixture.source.calls, 0);
    assert!(fixture.source.requested.is_none());
    let (prefix, quarantine) = (*degraded).into_parts();
    assert_eq!(prefix.journal_head(), None);
    assert_eq!(
        prefix.lifecycle().snapshot(),
        GenerationLifecycle::empty(RecoveryReferenceKnowledge::Known(BTreeSet::new())).snapshot()
    );
    assert_eq!(quarantine.valid_through(), None);
    assert_eq!(quarantine.first_untrusted_sequence().get(), 1);
    Ok(())
}

#[test]
fn out_of_closure_input_cannot_create_a_partial_resident() -> TestResult {
    let mut outside = build_fixture(false, true)?;
    let unretained = CompiledGenerationDigest::hash_canonical(&"unretained generation")?;
    outside.source.inputs[1].expected_generation = unretained.clone();
    assert!(matches!(
        reconstruct_resident(outside.recovery, &mut outside.source, parameters(71)),
        Err(ResidentReconstructionError::UnexpectedGeneration { generation })
            if generation == unretained
    ));
    Ok(())
}
