//! Contextual start and public observation recovery over reopened SQLite/SMT.
//! The anchor is a declared in-process model outside the database restore image;
//! signatures and historical authority resolution are fixture-owned models.
//! This does not qualify deployment admission, a key/anchor backend, power loss,
//! or independent anti-rollback authority for the store's logical mutation clock.

use crate::authority::DeploymentControlSqliteStore;
use crate::location::{ControlStorageRoot, ControlStoreScope, DeploymentStoreScope};
use crate::operation::SqliteOperationCustody;
use crate::resident_fixture::local_operation_start_fixture::StartAdmission;
use crate::resident_fixture::local_operation_start_fixture::{
    checkpoint_authority, checkpoint_preimage, issue_observation, profile, record_ref,
    schema_digest, signature, source_generation_inputs_with, store_coordinate, with_start,
};
use crate::resident_fixture::shared_resident::{
    synthetic_set_policy, FixedInputSource, RecoveryResolver,
};
use crate::store::{initialize_unadmitted_store, ControlStoreIdentity};
use pale_application::resident::{
    recover_local_operation, recover_local_operation_from_handle, recover_operation_observation,
    retain_operation_lease_body, LocalOperationHandleRecoveryInput,
    LocalOperationRecoveryAdmission, LocalOperationRecoveryContext, LocalOperationRecoveryError,
    OperationCheckpointAncestryLimits, OperationObservationReadmissionError,
    OperationObservationRecoveryAdmission, OperationObservationRecoveryInput,
    OperationObservationRecoveryLimits, OperationRecoveryError, RecoveryReferenceKnowledge,
    ResidentApplication, ResidentColdOpenInput, ResidentColdOpenOutcome,
    ResidentReconstructionParameters,
};
use pale_core::records::*;
use pale_ports::*;
use rusqlite::Connection;
use std::cell::Cell;

/// A simulated independent authority: never serialized into or restored with `SQLite`.
#[derive(Default)]
pub(crate) struct IndependentAnchor {
    value: Option<AntiRollbackAnchor>,
    advances: usize,
    reads: usize,
    recovery_only: bool,
    lose_confirmation: bool,
    replace_after_read: Option<AntiRollbackAnchor>,
}

impl IndependentAnchor {
    pub(super) fn require_read_only(&mut self) {
        self.recovery_only = true;
    }

    pub(super) fn recovery_view(&self) -> Self {
        Self {
            value: self.value.clone(),
            recovery_only: true,
            ..Self::default()
        }
    }

    pub(super) const fn advance_count(&self) -> usize {
        self.advances
    }
}

impl AntiRollbackAnchorPort for IndependentAnchor {
    fn read(
        &mut self,
        _: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        self.reads += 1;
        let observed = self.value.clone();
        if let Some(replacement) = self.replace_after_read.take() {
            self.value = Some(replacement);
        }
        Ok(observed)
    }

    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        assert!(
            !self.recovery_only,
            "locator recovery must only read the anchor"
        );
        Ok(transition.classify_observed(self.value.as_ref()))
    }

    fn compare_and_advance(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        assert!(
            !self.recovery_only,
            "locator recovery must never advance the anchor"
        );
        let observed = transition.classify_observed(self.value.as_ref());
        match (transition.expected(), observed) {
            (None, AntiRollbackAnchorObservation::Missing)
            | (Some(_), AntiRollbackAnchorObservation::ExactPredecessor) => {
                self.advances += 1;
                self.value = Some(transition.next().clone());
                if self.lose_confirmation {
                    self.lose_confirmation = false;
                    Err(AntiRollbackAnchorError::new(
                        AntiRollbackAnchorErrorKind::OutcomeUnknown,
                    ))
                } else {
                    Ok(AntiRollbackAnchorAdvanceDisposition::Advanced)
                }
            }
            (_, AntiRollbackAnchorObservation::ExactSuccessor) => {
                Ok(AntiRollbackAnchorAdvanceDisposition::ExactReplay)
            }
            (_, other) => Ok(AntiRollbackAnchorAdvanceDisposition::NotAdvanced(other)),
        }
    }
}

/// Production native reads, with the unrelated renewal actuator unavailable.
struct ReadOnlySessions(DeploymentControlSqliteStore);
impl SessionLeaseCustodyPort for ReadOnlySessions {
    fn compare_and_retain(
        &self,
        _: &SessionLeaseBody,
    ) -> Result<SessionLeaseBodyRetention, ControlStoreError> {
        panic!("locator recovery must not retain or renew a session")
    }
    fn read(
        &self,
        query: &SessionLeaseBodyRead,
    ) -> Result<Option<SessionLeaseBody>, ControlStoreError> {
        self.0.read(query)
    }
}

pub(super) fn connection(file: &std::path::Path) -> Connection {
    let value = Connection::open(file).unwrap();
    value.pragma_update(None, "foreign_keys", true).unwrap();
    value.pragma_update(None, "synchronous", "FULL").unwrap();
    assert_eq!(
        value
            .pragma_query_value(None, "journal_mode", |r| r.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
    assert_eq!(
        value
            .pragma_query_value(None, "synchronous", |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    value
}

fn checkpoint_database(value: &Connection) {
    let (busy, _, remaining): (i64, i64, i64) = value
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .unwrap();
    assert_eq!((busy, remaining), (0, 0));
}

/// Damage one throwaway recovery input after the locator has been recovered.
fn damage_observation_input(
    sql: &Connection,
    fault: &str,
    first: &ControlRecord,
    appended: &OperationCustodyCandidate,
    authority: &ControlCheckpointSigningAuthorityAdmissionContextV1<'_>,
) {
    let next = appended
        .records()
        .iter()
        .map(|entry| entry.record())
        .find(|record| record.schema_id().as_str() == OPERATION_OBSERVATION_V1_SCHEMA)
        .unwrap();
    let changed = match fault {
        "observation-history-missing" => {
            sql.pragma_update(None, "foreign_keys", false).unwrap();
            let changed = sql.execute(
                "DELETE FROM pale_deployment_control_record WHERE record_ref=?1",
                [first.coordinate().record_ref().as_str()],
            ).unwrap();
            sql.pragma_update(None, "foreign_keys", true).unwrap();
            changed
        }
        "observation-history-substituted" => sql.execute(
            "UPDATE pale_deployment_control_record SET canonical_record_jcs=?1 WHERE record_ref=?2",
            rusqlite::params![next.canonical_record_jcs().as_slice(), first.coordinate().record_ref().as_str()],
        ).unwrap(),
        "observation-checkpoint-missing" => sql.execute(
            "DELETE FROM pale_deployment_operation_index WHERE index_role='observation_checkpoint' AND index_key=?1",
            [OperationCustodyIndexKey::for_observation(next.coordinate()).as_str()],
        ).unwrap(),
        "observation-checkpoint-misdirected" => sql.execute(
            "UPDATE pale_deployment_operation_index SET record_ref=?1, record_sha256=?2 \
             WHERE index_role='observation_checkpoint' AND index_key=?3",
            rusqlite::params![first.coordinate().record_ref().as_str(), first.coordinate().record_sha256().as_str(),
                OperationCustodyIndexKey::for_observation(next.coordinate()).as_str()],
        ).unwrap(),
        "observation-checkpoint-fork" => {
            // This is a valid signed fixture checkpoint, not a wrong-schema row.
            // Its identity differs from the actual committed successor, so it
            // cannot be an ancestor of the independently anchored head.
            let original = appended.checkpoint().successor_record();
            let mut value = pale_core::canon::parse_strict(original.canonical_record_jcs().as_slice()).unwrap();
            value["control_checkpoint_id"] = serde_json::json!("checkpoint/fork");
            let fields = value.as_object_mut().unwrap();
            fields.remove("canonical_control_checkpoint_sha256").unwrap();
            let signature_value = fields.remove("signature").unwrap();
            let digest = ControlStateCheckpointDigest::hash_canonical(&value).unwrap();
            value["canonical_control_checkpoint_sha256"] = serde_json::to_value(&digest).unwrap();
            value["signature"] = signature_value;
            let signed = ControlStateCheckpointV1::admit_signed_wire(
                serde_json::from_value(value).unwrap(),
                original.schema_sha256(),
                authority,
                |domain, _, _, actual| {
                    assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                    assert_eq!(actual, &signature());
                    Ok(())
                },
            ).unwrap();
            let fork = ControlRecord::new(
                ControlRecordCoordinate::new(
                    record_ref("checkpoint/fork"),
                    ControlRecordDigest::parse(digest.as_str()).unwrap(),
                ),
                original.schema_id().clone(),
                original.schema_sha256().clone(),
                ControlRecordBytes::new(pale_core::canon::to_canonical_string(&signed).unwrap().into_bytes()).unwrap(),
            );
            super::insert_record(sql, &fork);
            sql.execute(
                "UPDATE pale_deployment_operation_index SET record_ref=?1, record_sha256=?2 \
                 WHERE index_role='observation_checkpoint' AND index_key=?3",
                rusqlite::params![fork.coordinate().record_ref().as_str(), fork.coordinate().record_sha256().as_str(),
                    OperationCustodyIndexKey::for_observation(next.coordinate()).as_str()],
            ).unwrap()
        }
        "observation-leaf-missing" => {
            let leaf = appended.accumulator().state().mutations().iter()
                .find(|entry| matches!(entry.position(), AccumulatorPosition::Leaf(_))).unwrap();
            sql.execute(
                "DELETE FROM pale_deployment_operation_accumulator_position \
                 WHERE checkpoint_domain_id=?1 AND tree_kind='state' AND position_kind='leaf' AND node_bytes=?2",
                rusqlite::params![appended.checkpoint().successor().checkpoint_domain_id().as_str(), leaf.successor().unwrap()],
            ).unwrap()
        }
        _ => return,
    };
    assert_eq!(changed, 1, "corrupt only the selected {fault} input");
}

pub(super) type GenerationInputs = (
    pale_application::compile::Compilation,
    Vec<pale_application::resident::AppliedRepositoryMaterialization>,
    EffectiveProjectProfileV1,
);

pub(super) fn with_generation_copies<R>(
    copies: usize,
    run: impl FnOnce(
        &pale_application::resident::AdmittedCompiledGeneration,
        &EffectiveProjectProfileV1,
        &mut Vec<GenerationInputs>,
    ) -> R,
) -> R {
    let mut inputs =
        crate::resident_fixture::local_operation_start_fixture::source_generation_input_copies_with(
            |_| {},
            |_| {},
            copies + 1,
        );
    let first = inputs.pop().unwrap();
    with_generation_inputs(first, |generation, effective| {
        run(generation, effective, &mut inputs)
    })
}

pub(crate) fn with_generation<R>(
    run: impl FnOnce(
        &pale_application::resident::AdmittedCompiledGeneration,
        &EffectiveProjectProfileV1,
    ) -> R,
) -> R {
    let inputs = source_generation_inputs_with(|_| {}, |_| {});
    with_generation_inputs(inputs, run)
}

pub(super) fn with_host_generation<R>(
    tool_name: &str,
    run: impl FnOnce(
        &pale_application::resident::AdmittedCompiledGeneration,
        &EffectiveProjectProfileV1,
    ) -> R,
) -> R {
    let inputs =
        crate::resident_fixture::local_operation_start_fixture::host_source_generation_inputs(
            tool_name,
        );
    with_generation_inputs(inputs, run)
}

fn with_generation_inputs<R>(
    inputs: (
        pale_application::compile::Compilation,
        Vec<pale_application::resident::AppliedRepositoryMaterialization>,
        EffectiveProjectProfileV1,
    ),
    run: impl FnOnce(
        &pale_application::resident::AdmittedCompiledGeneration,
        &EffectiveProjectProfileV1,
    ) -> R,
) -> R {
    // The adapter obtains admitted generation custody through the public resident
    // owner, not a private application constructor or a test-only production API.
    let generation_directory = crate::test_support::private_tempdir();
    let (generation_location, generation_identity, _) = crate::test_support::initialize(
        &generation_directory,
        crate::ControlStoreKind::DeploymentControl,
        "local-recovery-generation",
    );
    let generation_coordinate = AuthorityStoreCoordinate::new(
        generation_identity.logical_store_id().clone(),
        generation_identity.control_incarnation_id().clone(),
    );
    let generation_store = DeploymentControlSqliteStore::open_unadmitted_candidate(
        &generation_location,
        generation_coordinate.clone(),
    )
    .unwrap();
    let generation_resolver = RecoveryResolver::default();
    let mut generation_source = FixedInputSource::default();
    let opened = ResidentApplication::cold_open(
        &generation_store,
        &generation_store,
        &mut &generation_store,
        1 << 20,
        ResidentColdOpenInput::new(
            generation_coordinate,
            AuthorityRecoveryReadLimits::new(1 << 20, 1 << 20, 8, 64).unwrap(),
            1 << 20,
            RecoveryReferenceKnowledge::Known(std::collections::BTreeSet::new()),
            &generation_resolver,
            &mut generation_source,
            ResidentReconstructionParameters::new(
                synthetic_set_policy(),
                8,
                8,
                WakeEpochV1::new(1),
            ),
        ),
    )
    .unwrap();
    let ResidentColdOpenOutcome::Ready(mut resident) = opened else {
        panic!("empty generation fixture must not be degraded");
    };
    let (compilation, materializations, effective) = inputs;
    let generation_digest = resident
        .admit_generation(compilation, materializations, synthetic_set_policy())
        .unwrap();
    let generation = resident.generation(&generation_digest).unwrap();
    run(generation, &effective)
}

#[test]
fn contextual_start_native_commit_anchor_reopen_and_locator_recovery() {
    with_generation(|generation, effective| {
        let mut domains = std::collections::BTreeSet::new();
        for fault in [
            "none",
            "saved-handle",
            "observation-saved-handle",
            "saved-handle-request-alias",
            "lost-anchor-confirmation",
            "unconfirmed-anchor",
            "restored-database",
            "foreign-anchor-key",
            "profile",
            "request",
            "observation-replaced",
            "cancel-requested",
            "checkpoint-insertion-stamp",
            "observation-history-missing",
            "observation-history-substituted",
            "observation-checkpoint-missing",
            "observation-checkpoint-misdirected",
            "observation-checkpoint-fork",
            "observation-leaf-missing",
            "observation-revision-bound",
            "observation-chain-byte-bound",
            "observation-record-byte-bound",
            "observation-membership-work-bound",
            "observation-anchor-changed",
            "observation-leaf-advanced",
        ] {
            with_start(
                generation,
                effective,
                fault,
                |_| {},
                StartAdmission::AcceptedOwnerModel,
                |prepared, request, owner_calls, record_signatures, retained| {
                    assert_eq!((owner_calls, record_signatures), (1, 2));
                    let prepared = prepared.unwrap();
                    let directory = crate::test_support::private_tempdir();
                    let binding = retained.current.deployment_binding;
                    let scope = ControlStoreScope::Deployment(DeploymentStoreScope::new(
                        ResidentWorkspaceIdV1::parse(
                            binding.deployment_terms().workspace_id().as_str(),
                        )
                        .unwrap(),
                        retained.current.deployment_binding_ref.clone(),
                        ArtifactDigest::parse(binding.canonical_binding_sha256().as_str()).unwrap(),
                    ));
                    let location = ControlStorageRoot::retain(directory.path())
                        .unwrap()
                        .prepare_store_location(scope.clone())
                        .unwrap();
                    let coordinate = store_coordinate();
                    let initialized = initialize_unadmitted_store(
                        &location,
                        ControlStoreIdentity::new(
                            scope,
                            coordinate.store_id().clone(),
                            coordinate.incarnation_id().clone(),
                        ),
                    )
                    .unwrap();
                    let file = initialized.path();
                    let sessions = DeploymentControlSqliteStore::open_unadmitted_candidate(
                        &location,
                        coordinate.clone(),
                    )
                    .unwrap();
                    let session = retained.session.lease();
                    let body = SessionLeaseBody::new(
                        coordinate.clone(),
                        session.session_admission_lease_digest().clone(),
                        session.schema_id().clone(),
                        session.schema_sha256().clone(),
                        ControlRecordBytes::new(
                            pale_core::canon::to_canonical_string(
                                &SessionAdmissionLeaseWireV1::from(session),
                            )
                            .unwrap()
                            .into_bytes(),
                        )
                        .unwrap(),
                    );
                    assert_eq!(
                        sessions.compare_and_retain(&body).unwrap(),
                        SessionLeaseBodyRetention::Retained
                    );
                    drop(sessions);
                    let mut sql = connection(file);
                    retain_operation_lease_body(
                        &mut SqliteOperationCustody {
                            connection: &mut sql,
                            coordinate: coordinate.clone(),
                        },
                        &coordinate,
                        &record_ref("start-lease"),
                        retained.lease,
                        1 << 20,
                    )
                    .unwrap();
                    checkpoint_database(&sql);
                    drop(sql);
                    let before_start = std::fs::read(file).unwrap();

                    let mut sql = connection(file);
                    let mut port = SqliteOperationCustody {
                        connection: &mut sql,
                        coordinate: coordinate.clone(),
                    };
                    let prepared = prepared
                        .prepare_checkpoint(
                            AccumulatorPreparationLimits::new(4, 10_000, 10_000, 4096, 100_000)
                                .unwrap(),
                            &mut port,
                        )
                        .unwrap();
                    let mut checkpoint = checkpoint_preimage(1, None);
                    checkpoint.committed_transaction_sequence = prepared.transaction_sequence();
                    checkpoint.issued_at = UnixTimeV1::new(61).unwrap();
                    checkpoint.checkpoint_profile_sha256 = pale_application::resident::SupportedOperationCheckpointProfile::content_digest().unwrap();
                    checkpoint.checkpoint_domain_id = pale_application::resident::SupportedOperationCheckpointProfile::conflict_domain(request).unwrap();
                    assert!(
                        domains.insert(checkpoint.checkpoint_domain_id.clone()),
                        "distinct request identities have distinct domains"
                    );
                    checkpoint.state_root_sha256 = prepared.state_root().clone();
                    checkpoint.unresolved_custody_root_sha256 = prepared.unresolved_root().clone();
                    checkpoint.changed_leaf_keys = prepared.changed_keys().to_vec();
                    checkpoint.leaf_count = ControlLeafCountV1::new(4).unwrap();
                    let checkpoint_signatures = Cell::new(0);
                    let (physical, pending) = prepared
                        .issue_checkpoint(
                            &record_ref("operation/checkpoint-1"),
                            checkpoint.clone(),
                            &checkpoint.schema_sha256,
                            &checkpoint_authority(&checkpoint),
                            |domain, _, _| {
                                assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                                checkpoint_signatures.set(checkpoint_signatures.get() + 1);
                                Ok(signature())
                            },
                        )
                        .unwrap();
                    let mut anchor = IndependentAnchor {
                        lose_confirmation: fault == "lost-anchor-confirmation",
                        ..Default::default()
                    };
                    let database = physical.commit(&mut port, &mut anchor, 1 << 20).unwrap();
                    let candidate = database.candidate().clone();
                    assert_eq!(candidate.store(), &coordinate);
                    let runtime = retained.current.runtime_authority.unwrap().authority;
                    assert_eq!(candidate.store().store_id(), runtime.control_store_id());
                    assert_eq!(
                        candidate.store().incarnation_id(),
                        runtime.control_incarnation_id()
                    );
                    let mut checked_runtime_records = 0;
                    for record in candidate.records() {
                        if record.record().schema_id().as_str() == OPERATION_HANDLE_V1_SCHEMA
                            || record.record().schema_id().as_str()
                                == OPERATION_OBSERVATION_V1_SCHEMA
                        {
                            checked_runtime_records += 1;
                            let value = pale_core::canon::parse_strict(
                                record.record().canonical_record_jcs().as_slice(),
                            )
                            .unwrap();
                            assert_eq!(
                                value["deployment_runtime_authority_ref"],
                                serde_json::to_value(retained.session.runtime_authority_ref())
                                    .unwrap()
                            );
                            assert_eq!(
                                value["deployment_runtime_authority_sha256"],
                                serde_json::to_value(retained.session.runtime_authority_sha256())
                                    .unwrap()
                            );
                            assert_eq!(
                                value["issuer_key_id"],
                                serde_json::to_value(
                                    retained
                                        .current
                                        .runtime_authority
                                        .unwrap()
                                        .authority
                                        .subject_key_id()
                                )
                                .unwrap()
                            );
                        }
                    }
                    assert_eq!(
                        checked_runtime_records, 2,
                        "actual handle and revision-one observation were both checked"
                    );
                    let mut continuation_selection = None;
                    let mut appended_candidate = None;
                    let mut before_append = None;
                    let mut before_append_anchor = None;
                    let mut after_append_anchor = None;
                    let expected = if fault == "unconfirmed-anchor" {
                        drop(database);
                        drop(pending);
                        None
                    } else {
                        let completed = database.advance_anchor(&mut anchor).unwrap();
                        let issued = pending
                            .finish(completed)
                            .unwrap_or_else(|_| panic!("matching actual commit and anchor"));
                        let expected = (issued.handle().0.clone(), issued.handle().1.clone());
                        continuation_selection = Some(issued.checkpoint_selection());
                        drop(issued);
                        Some(expected)
                    };
                    let committed_changes = port.connection.total_changes();
                    assert_eq!(
                        crate::store::transaction::read(port.connection, &coordinate).unwrap(),
                        checkpoint.committed_transaction_sequence,
                    );
                    for record in candidate
                        .records()
                        .iter()
                        .map(|entry| entry.record())
                        .chain(std::iter::once(candidate.checkpoint().successor_record()))
                    {
                        assert_eq!(
                            crate::store::transaction::record_stamp(
                                port.connection,
                                crate::ControlStoreKind::DeploymentControl,
                                record.coordinate()
                            )
                            .unwrap(),
                            Some(checkpoint.committed_transaction_sequence),
                        );
                    }
                    assert_eq!(
                        port.compare_and_commit(&candidate).unwrap(),
                        OperationCustodyCommitOutcome::ExactReplay
                    );
                    assert_eq!(
                        port.connection.total_changes(),
                        committed_changes,
                        "exact start replay has no physical writes"
                    );
                    let appends_observation =
                        fault.starts_with("observation-") || fault == "cancel-requested";
                    if appends_observation {
                        if fault == "observation-leaf-advanced" {
                            checkpoint_database(port.connection);
                            before_append = Some(std::fs::read(file).unwrap());
                            before_append_anchor = anchor.value.clone();
                        }
                        let (handle_ref, handle) = expected.as_ref().unwrap();
                        let first_record = candidate
                            .records()
                            .iter()
                            .map(|entry| entry.record())
                            .find(|record| {
                                record.schema_id().as_str() == OPERATION_OBSERVATION_V1_SCHEMA
                            })
                            .unwrap();
                        let observation_schema = schema_digest(OPERATION_OBSERVATION_V1_SCHEMA_ID);
                        let first = OperationObservationV1::admit_contextual_wire(
                            serde_json::from_value(
                                pale_core::canon::parse_strict(
                                    first_record.canonical_record_jcs().as_slice(),
                                )
                                .unwrap(),
                            )
                            .unwrap(),
                            &OperationObservationAdmissionContextV1 {
                                signed: SignedResidentAdmissionContextV1 {
                                    expected_schema_sha256: &observation_schema,
                                    signing_authority: retained.signing,
                                },
                                handle_ref,
                                handle,
                                predecessor: None,
                            },
                            |state| {
                                assert!(matches!(
                                    state,
                                    OperationObservationStateV1::Active {
                                        phase: OperationActivePhaseV1::Resolving
                                    }
                                ));
                                Ok(())
                            },
                            |_, _, _, _| Ok(()),
                            |domain, _, _, actual_signature| {
                                assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
                                assert_eq!(actual_signature, &signature());
                                Ok(())
                            },
                        )
                        .unwrap();
                        assert_eq!(
                            first.canonical_operation_observation_sha256().as_str(),
                            first_record.coordinate().record_sha256().as_str()
                        );
                        let next_state = if fault == "cancel-requested" {
                            // Declared fixture evidence only: no remote stop, effect
                            // cancellation, or public Cancel service is asserted.
                            OperationObservationStateV1::CancellationRequested {
                                request: ImmutableRecordReferenceV1 {
                                    record_ref: record_ref("fixture/cancellation-request"),
                                    record_sha256: ArtifactDigest::hash_bytes(
                                        b"declared cancellation request",
                                    ),
                                },
                            }
                        } else {
                            OperationObservationStateV1::Active {
                                phase: OperationActivePhaseV1::Resolving,
                            }
                        };
                        let next = issue_observation(
                            retained.signing,
                            handle_ref,
                            handle,
                            observation_schema,
                            OperationObservationRevisionV1::new(2).unwrap(),
                            Some((first_record.coordinate().record_ref(), &first)),
                            next_state,
                            handle.created_at().get() + 1,
                        );
                        assert!(
                            !next.state().is_terminal(),
                            "a cancellation request is not confirmed cancellation"
                        );
                        let initial_record = candidate.checkpoint().successor_record();
                        let prior_checkpoint = ControlStateCheckpointV1::admit_initial_wire(
                            serde_json::from_value(
                                pale_core::canon::parse_strict(
                                    initial_record.canonical_record_jcs().as_slice(),
                                )
                                .unwrap(),
                            )
                            .unwrap(),
                            &checkpoint.schema_sha256,
                            &checkpoint_authority(&checkpoint),
                            |domain, _, _, actual_signature| {
                                assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                                assert_eq!(actual_signature, &signature());
                                Ok(())
                            },
                        )
                        .unwrap();
                        let next_ref = record_ref("operation/observation-2");
                        let prior_ref = initial_record.coordinate().record_ref();
                        let resolve_store = |observation: &OperationObservationV1| {
                            assert_eq!(
                                observation.deployment_runtime_authority_ref(),
                                retained.session.runtime_authority_ref()
                            );
                            assert_eq!(
                                observation.deployment_runtime_authority_sha256().as_str(),
                                retained.session.runtime_authority_sha256().as_str()
                            );
                            assert_eq!(
                                observation.issuer_principal_id(),
                                handle.issuer_principal_id()
                            );
                            assert_eq!(observation.issuer_key_id(), handle.issuer_key_id());
                            assert_eq!(observation.issuer_key_epoch(), handle.issuer_key_epoch());
                            Ok(coordinate.clone())
                        };
                        let prepared =
                            pale_application::resident::prepare_operation_observation_append(
                                pale_application::resident::OperationObservationTransition::new(
                                    handle_ref,
                                    handle,
                                    first_record.coordinate().record_ref(),
                                    &first,
                                    &next_ref,
                                    &next,
                                ),
                                &resolve_store,
                            )
                            .unwrap()
                            .prepare_checkpoint(
                                continuation_selection.take().unwrap(),
                                prior_ref,
                                &prior_checkpoint,
                                AccumulatorPreparationLimits::new(1, 10_000, 10_000, 4096, 100_000)
                                    .unwrap(),
                                &mut port,
                            )
                            .unwrap();
                        checkpoint.control_checkpoint_id =
                            ControlCheckpointIdV1::parse("operation-checkpoint-2").unwrap();
                        checkpoint.domain_sequence = ControlDomainSequenceV1::new(2).unwrap();
                        checkpoint.previous_control_checkpoint_ref = Some(prior_ref.clone());
                        checkpoint.previous_control_checkpoint_sha256 = Some(
                            prior_checkpoint
                                .canonical_control_checkpoint_sha256()
                                .clone(),
                        );
                        checkpoint.committed_transaction_sequence = prepared.transaction_sequence();
                        checkpoint.issued_at = UnixTimeV1::new(62).unwrap();
                        checkpoint.state_root_sha256 = prepared.state_root().clone();
                        checkpoint.unresolved_custody_root_sha256 =
                            prepared.unresolved_root().clone();
                        checkpoint.changed_leaf_keys = vec![prepared.observation_key().clone()];
                        let (physical, issued_checkpoint) = prepared
                            .issue_checkpoint(
                                &record_ref("operation/checkpoint-2"),
                                checkpoint.clone(),
                                &checkpoint.schema_sha256,
                                &checkpoint_authority(&checkpoint),
                                |domain, preimage, digest| {
                                    assert_eq!(
                                        domain,
                                        CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN
                                    );
                                    assert_eq!(preimage, &checkpoint);
                                    assert_eq!(
                                        digest,
                                        &ControlStateCheckpointDigest::hash_canonical(preimage)
                                            .unwrap()
                                    );
                                    checkpoint_signatures.set(checkpoint_signatures.get() + 1);
                                    Ok(signature())
                                },
                            )
                            .unwrap();
                        let pending = physical.commit(&mut port, &mut anchor).unwrap();
                        let successor_candidate = pending.candidate().clone();
                        assert_eq!(
                            successor_candidate
                                .checkpoint()
                                .successor_record()
                                .canonical_record_jcs()
                                .as_slice(),
                            pale_core::canon::to_canonical_string(&issued_checkpoint)
                                .unwrap()
                                .as_bytes()
                        );
                        let completed = pending.advance_anchor(&mut anchor).unwrap();
                        assert_eq!(completed.candidate(), &successor_candidate);
                        assert_eq!(
                            successor_candidate
                                .checkpoint()
                                .successor()
                                .domain_sequence()
                                .get(),
                            2
                        );
                        let latest_key = successor_candidate
                            .indexes()
                            .iter()
                            .find_map(|index| {
                                if let OperationCustodyIndexMutation::LatestObservation {
                                    key,
                                    ..
                                } = index
                                {
                                    Some(key.clone())
                                } else {
                                    None
                                }
                            })
                            .unwrap();
                        let latest = port
                            .read_index(
                                &coordinate,
                                &OperationCustodyIndexQuery::LatestObservation(latest_key),
                                1 << 20,
                            )
                            .unwrap();
                        assert_eq!(
                            latest.record().unwrap().canonical_record_jcs().as_slice(),
                            pale_core::canon::to_canonical_string(&next)
                                .unwrap()
                                .as_bytes()
                        );
                        let writes = port.connection.total_changes();
                        assert_eq!(
                            crate::store::transaction::read(port.connection, &coordinate).unwrap(),
                            checkpoint.committed_transaction_sequence,
                        );
                        assert_eq!(
                            port.compare_and_commit(&successor_candidate).unwrap(),
                            OperationCustodyCommitOutcome::ExactReplay
                        );
                        assert_eq!(port.connection.total_changes(), writes);
                        assert_eq!(anchor.advances, 2);
                        after_append_anchor = anchor.value.clone();
                        appended_candidate = Some(successor_candidate);
                    }
                    let expected_checkpoint_signatures = if appends_observation { 2 } else { 1 };
                    assert_eq!(checkpoint_signatures.get(), expected_checkpoint_signatures);
                    checkpoint_database(&sql);
                    drop(sql);
                    if let Some(image) = before_append {
                        // Keep the actual signed successor candidate for a second
                        // connection to commit at the observation admission boundary.
                        std::fs::write(file, image).unwrap();
                        anchor.value = before_append_anchor;
                    }
                    if fault == "restored-database" {
                        assert_ne!(std::fs::read(file).unwrap(), before_start);
                        std::fs::write(file, &before_start).unwrap();
                        assert!(
                            anchor.value.is_some(),
                            "independent anchor is not part of the restored SQLite image"
                        );
                    }
                    if fault == "foreign-anchor-key" {
                        let head = candidate.checkpoint().successor();
                        anchor.value = Some(AntiRollbackAnchor::ControlDomain(
                            ControlDomainAntiRollbackAnchor::new(
                                coordinate.store_id().clone(),
                                coordinate.incarnation_id().clone(),
                                ControlCheckpointDomainIdV1::parse("operation:foreign").unwrap(),
                                head.domain_epoch(),
                                head.domain_sequence(),
                                head.coordinate().clone(),
                            ),
                        ));
                    }
                    anchor.recovery_only = true;
                    let mut before_anchor = anchor.value.clone();
                    let advances = anchor.advances;
                    let sessions = ReadOnlySessions(
                        DeploymentControlSqliteStore::open_unadmitted_candidate(
                            &location,
                            coordinate.clone(),
                        )
                        .unwrap(),
                    );
                    let mut sql = connection(file);
                    if fault == "saved-handle-request-alias" {
                        let original = candidate
                            .record(OperationCustodyRecordKind::CommonRequest)
                            .unwrap();
                        let alias = ControlRecord::new(
                            ControlRecordCoordinate::new(
                                record_ref("request/alias"),
                                original.coordinate().record_sha256().clone(),
                            ),
                            original.schema_id().clone(),
                            original.schema_sha256().clone(),
                            original.canonical_record_jcs().clone(),
                        );
                        super::insert_record(&sql, &alias);
                        assert_eq!(sql.execute(
                        "UPDATE pale_deployment_operation_index SET record_ref=?1 \
                         WHERE index_role='request_for_handle' AND record_ref=?2 AND record_sha256=?3",
                        rusqlite::params![alias.coordinate().record_ref().as_str(), original.coordinate().record_ref().as_str(), original.coordinate().record_sha256().as_str()],
                    ).unwrap(), 1);
                    }
                    if fault == "checkpoint-insertion-stamp" {
                        assert!(checkpoint.committed_transaction_sequence.get() > 1);
                        assert_eq!(
                        sql.execute(
                            "UPDATE pale_deployment_control_record SET inserted_transaction_sequence=1 \
                             WHERE record_ref=?1 AND record_sha256=?2",
                            rusqlite::params![
                                candidate.checkpoint().successor_record().coordinate().record_ref().as_str(),
                                candidate.checkpoint().successor_record().coordinate().record_sha256().as_str(),
                            ],
                        ).unwrap(),
                        1,
                        "corrupt exactly the actual checkpoint insertion stamp, not its signed bytes",
                    );
                        assert_eq!(
                            crate::store::transaction::read(&sql, &coordinate).unwrap(),
                            checkpoint.committed_transaction_sequence,
                        );
                    }
                    sql.pragma_update(None, "query_only", true).unwrap();
                    let writes = sql.total_changes();
                    let mut data_version: i64 = sql
                        .pragma_query_value(None, "data_version", |r| r.get(0))
                        .unwrap();
                    let mut resolver = RecoveryResolver::default();
                    resolver.insert_generation(retained.current.generation.compiled_generation);
                    resolver.insert_active(retained.current.generation.active_head);
                    resolver.insert_fence(retained.current.generation.activation_fence);
                    resolver
                        .insert_effective_profile(retained.current.generation.effective_profile);
                    let supported = if fault == "profile" {
                        pale_application::resident::SupportedOperationCheckpointProfile::admit(record_ref("unlike-profile"), &checkpoint.checkpoint_profile_sha256, pale_application::resident::SupportedOperationCheckpointProfile::canonical_content().unwrap().as_bytes()).unwrap()
                    } else {
                        profile()
                    };
                    let mut changed = request.to_preimage();
                    changed.body.payload = serde_json::json!({"query":"unlike"});
                    changed.body.payload_sha256 = ArtifactDigest::hash_bytes(
                        pale_core::canon::to_canonical_string(&changed.body.payload)
                            .unwrap()
                            .as_bytes(),
                    );
                    let changed =
                        crate::resident_fixture::effect_case_lineage_fixture::request(&changed);
                    assert_eq!(pale_application::resident::SupportedOperationCheckpointProfile::conflict_domain(&changed).unwrap(), checkpoint.checkpoint_domain_id);
                    let selected_request = if fault == "request" {
                        &changed
                    } else {
                        request
                    };
                    let mut port = SqliteOperationCustody {
                        connection: &mut sql,
                        coordinate: coordinate.clone(),
                    };
                    let recovery_context = LocalOperationRecoveryContext {
                        store: &coordinate,
                        profile: &supported,
                        handle_signing_authority: retained.signing,
                        checkpoint_authority: &checkpoint_authority(&checkpoint),
                        historical: &resolver,
                        maximum_record_bytes: 1 << 20,
                        maximum_operation_lease_bytes: 1 << 20,
                        maximum_session_lease_bytes: 1 << 20,
                        membership_limits: AccumulatorMembershipLimits::new(3, 10_000, 100_000)
                            .unwrap(),
                        observation_limits: OperationObservationRecoveryLimits::new(
                            8,
                            1 << 20,
                            1 << 20,
                        )
                        .unwrap(),
                        checkpoint_limits: OperationCheckpointAncestryLimits::new(8, 1 << 20, 8)
                            .unwrap(),
                    };
                    let request_calls = Cell::new(0);
                    let caller_signature_calls = Cell::new(0);
                    let historical_calls = Cell::new(0);
                    let handle_signature_calls = Cell::new(0);
                    let recovery_checkpoint_calls = Cell::new(0);
                    let recovery_admission = LocalOperationRecoveryAdmission {
                        admit_historical_authority:
                            |handle: &OperationHandlePreimageV1,
                             parent: Option<&OperationHandleV1>| {
                                historical_calls.set(historical_calls.get() + 1);
                                assert!(parent.is_none());
                                assert_eq!(handle.operation_class, OperationClassV1::Invoke);
                                Ok(())
                            },
                        verify_handle_signature:
                            |domain: &[u8],
                             _: &OperationHandlePreimageV1,
                             _: &OperationHandleDigest,
                             actual_signature: &CanonicalSignatureV1| {
                                handle_signature_calls.set(handle_signature_calls.get() + 1);
                                assert_eq!(domain, OPERATION_HANDLE_V1_SIGNATURE_DOMAIN);
                                assert_eq!(actual_signature, &signature());
                                Ok(())
                            },
                        verify_checkpoint_signature:
                            |domain: &[u8],
                             _: &ControlStateCheckpointPreimageV1,
                             _: &ControlStateCheckpointDigest,
                             actual_signature: &CanonicalSignatureV1| {
                                recovery_checkpoint_calls.set(recovery_checkpoint_calls.get() + 1);
                                assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                                assert_eq!(actual_signature, &signature());
                                Ok(())
                            },
                        initial_observation: OperationObservationRecoveryAdmission {
                            admit_checkpoint:
                                |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                                    ControlStateCheckpointV1::admit_signed_wire(
                                        wire,
                                        &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                                        &checkpoint_authority(&checkpoint),
                                        |domain, _, _, actual| {
                                            assert_eq!(
                                                domain,
                                                CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN
                                            );
                                            assert_eq!(actual, &signature());
                                            Ok(())
                                        },
                                    )
                                },
                            resolve_signer:
                                |_: &ResidentRecordRefV1, wire: &OperationObservationWireV1| {
                                    assert_eq!(
                                        wire.deployment_runtime_authority_ref(),
                                        retained.session.runtime_authority_ref()
                                    );
                                    assert_eq!(
                                        wire.deployment_runtime_authority_sha256().as_str(),
                                        retained.session.runtime_authority_sha256().as_str()
                                    );
                                    Ok(retained.signing.clone())
                                },
                            admit_state: |_: &OperationObservationStateV1| Ok(()),
                            admit_rotation:
                                |_: &ResidentRecordRefV1,
                                 _: &ArtifactDigest,
                                 _: &ResidentRecordRefV1,
                                 _: &ArtifactDigest| {
                                    panic!("local recovery fixture has no authority rotation")
                                },
                            verify_signature:
                                |domain: &[u8],
                                 _: &OperationObservationPreimageV1,
                                 _: &OperationObservationDigest,
                                 actual: &CanonicalSignatureV1| {
                                    assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
                                    assert_eq!(actual, &signature());
                                    Ok(())
                                },
                        },
                    };
                    let saved_handle = matches!(
                        fault,
                        "saved-handle" | "observation-saved-handle" | "saved-handle-request-alias"
                    );
                    let saved_result;
                    let request_result;
                    let result = if saved_handle {
                        let original = request.to_preimage();
                        let request_context = CommonRequestAdmissionContextV1 {
                            expected_schema_sha256: &original.envelope_schema_sha256,
                            expected_body_schema_id: &original.body.schema_id,
                            expected_body_schema_sha256: &original.body.schema_sha256,
                            caller: &original.caller,
                            signing_authority: &original.signing_authority_binding,
                            route: &original.route,
                            session_admission: &original.session_admission,
                            operation_target: original.operation_target.as_ref(),
                            execution_owner_fence: None,
                            federation_claim: None,
                            workspace_id: &original.workspace_id,
                            capability_id: &original.capability_id,
                            operation: &original.operation,
                            scope: &original.scope,
                            risk_class: &original.risk_class,
                            resource_constraints: &original.resource_constraints,
                            expected_precondition: None,
                            idempotency_key: &original.idempotency_key,
                            issued_at: original.issued_at,
                            expires_at: original.expires_at,
                            generation: original.generation.as_ref(),
                            fence_token: original.fence_token.as_ref(),
                        };
                        let locator = candidate
                            .record(OperationCustodyRecordKind::OperationHandle)
                            .unwrap()
                            .coordinate();
                        saved_result = recover_local_operation_from_handle(
                            LocalOperationHandleRecoveryInput {
                                context: recovery_context,
                                expected_handle: locator,
                                maximum_parent_depth: 0,
                            },
                            &mut port,
                            &sessions,
                            &mut anchor,
                            |reference, wire| {
                                request_calls.set(request_calls.get() + 1);
                                let expected_ref = if fault == "saved-handle-request-alias" {
                                    record_ref("request/alias")
                                } else {
                                    candidate
                                        .record(OperationCustodyRecordKind::CommonRequest)
                                        .unwrap()
                                        .coordinate()
                                        .record_ref()
                                        .clone()
                                };
                                assert_eq!(reference, &expected_ref);
                                CommonRequestEnvelopeV1::admit_contextual_wire(
                                    wire,
                                    &request_context,
                                    |preimage, digest, actual| {
                                        caller_signature_calls
                                            .set(caller_signature_calls.get() + 1);
                                        assert_eq!(preimage, &original);
                                        assert_eq!(digest, request.canonical_request_sha256());
                                        assert_eq!(actual, request.caller_signature());
                                        Ok(())
                                    },
                                )
                            },
                            recovery_admission,
                        );
                        saved_result.as_ref().map(|recovered| {
                            assert_eq!(recovered.request(), request);
                            recovered.operation()
                        })
                    } else {
                        request_result = recover_local_operation(
                            recovery_context,
                            selected_request,
                            &mut port,
                            &sessions,
                            &mut anchor,
                            recovery_admission,
                        );
                        request_result.as_ref()
                    };
                    if saved_handle {
                        assert_eq!(
                            (
                                request_calls.get(),
                                caller_signature_calls.get(),
                                historical_calls.get(),
                                handle_signature_calls.get(),
                                recovery_checkpoint_calls.get()
                            ),
                            (1, 1, 1, 1, 2)
                        );
                        assert_eq!(
                            anchor.reads, 4,
                            "saved-handle recovery reaches native current membership and the second anchor fence"
                        );
                    }
                    match (fault, result) {
                        (fault, Ok(recovered))
                            if matches!(
                                fault,
                                "none"
                                    | "saved-handle"
                                    | "lost-anchor-confirmation"
                                    | "cancel-requested"
                            ) || fault.starts_with("observation-") =>
                        {
                            let expected = expected.as_ref().unwrap();
                            assert_eq!(recovered.handle(), (&expected.0, &expected.1));
                            crate::resident_fixture::local_operation_start_fixture::assert_original_session_revalidation(recovered, &retained);
                            assert_eq!(anchor.reads, 4);
                            let first = candidate
                                .records()
                                .iter()
                                .map(|entry| entry.record())
                                .find(|record| {
                                    record.schema_id().as_str() == OPERATION_OBSERVATION_V1_SCHEMA
                                })
                                .unwrap();
                            let mut writer = connection(file);
                            if let Some(appended) = appended_candidate.as_ref() {
                                damage_observation_input(
                                    &writer,
                                    fault,
                                    first,
                                    appended,
                                    &checkpoint_authority(&checkpoint),
                                );
                                if fault == "observation-checkpoint-fork" {
                                    let physical = pale_application::resident::read_operation_observation_for_handle(
                                    &mut port, &coordinate, &expected.0, &expected.1, 1 << 20,
                                ).unwrap().unwrap();
                                    assert_eq!(
                                        physical.checkpoint().coordinate().record_ref(),
                                        &record_ref("checkpoint/fork")
                                    );
                                    assert_ne!(
                                        physical.checkpoint().coordinate(),
                                        appended.checkpoint().successor().coordinate()
                                    );
                                }
                            }
                            data_version = port
                                .connection
                                .pragma_query_value(None, "data_version", |r| r.get(0))
                                .unwrap();
                            let mut delayed_append = if fault == "observation-leaf-advanced" {
                                // The independent model reports the already-qualified
                                // successor after its concurrent database commit.
                                anchor.value = after_append_anchor;
                                before_anchor = anchor.value.clone();
                                appended_candidate.take()
                            } else {
                                None
                            };
                            if fault == "observation-anchor-changed" {
                                let head = candidate.checkpoint().successor();
                                let unlike = AntiRollbackAnchor::ControlDomain(
                                    ControlDomainAntiRollbackAnchor::new(
                                        coordinate.store_id().clone(),
                                        coordinate.incarnation_id().clone(),
                                        ControlCheckpointDomainIdV1::parse("operation:foreign")
                                            .unwrap(),
                                        head.domain_epoch(),
                                        head.domain_sequence(),
                                        head.coordinate().clone(),
                                    ),
                                );
                                anchor.replace_after_read = Some(unlike.clone());
                                before_anchor = Some(unlike);
                            }
                            let limits = OperationObservationRecoveryLimits::new(
                                if fault == "observation-revision-bound" {
                                    1
                                } else {
                                    2
                                },
                                if fault == "observation-chain-byte-bound" {
                                    1
                                } else {
                                    1 << 20
                                },
                                if fault == "observation-record-byte-bound" {
                                    1
                                } else {
                                    1 << 20
                                },
                            )
                            .unwrap();
                            let signatures = Cell::new(0);
                            let states = Cell::new(0);
                            let writer_changes = writer.total_changes();
                            let admission = OperationObservationRecoveryAdmission {
                            admit_checkpoint:
                                |_: &ResidentRecordRefV1, wire: ControlStateCheckpointWireV1| {
                                    ControlStateCheckpointV1::admit_signed_wire(
                                        wire,
                                        &schema_digest(CONTROL_STATE_CHECKPOINT_V1_SCHEMA_ID),
                                        &checkpoint_authority(&checkpoint),
                                        |domain, _, _, actual| {
                                            assert_eq!(
                                                domain,
                                                CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN
                                            );
                                            assert_eq!(actual, &signature());
                                            Ok(())
                                        },
                                    )
                                },
                            resolve_signer:
                                |_: &ResidentRecordRefV1, wire: &OperationObservationWireV1| {
                                    if let Some(append) = delayed_append.take() {
                                        assert_eq!(
                                            SqliteOperationCustody {
                                                connection: &mut writer,
                                                coordinate: coordinate.clone()
                                            }
                                            .compare_and_commit(&append)
                                            .unwrap(),
                                            OperationCustodyCommitOutcome::Committed
                                        );
                                    }
                                    assert_eq!(
                                        wire.deployment_runtime_authority_ref(),
                                        retained.session.runtime_authority_ref()
                                    );
                                    assert_eq!(
                                        wire.deployment_runtime_authority_sha256().as_str(),
                                        retained.session.runtime_authority_sha256().as_str()
                                    );
                                    assert_eq!(
                                        wire.issuer_principal_id(),
                                        expected.1.issuer_principal_id()
                                    );
                                    assert_eq!(wire.issuer_key_id(), expected.1.issuer_key_id());
                                    assert_eq!(
                                        wire.issuer_key_epoch(),
                                        expected.1.issuer_key_epoch()
                                    );
                                    assert!(wire.observed_at() >= expected.1.created_at());
                                    Ok(retained.signing.clone())
                                },
                            admit_state: |state: &OperationObservationStateV1| {
                                states.set(states.get() + 1);
                                match state {
                                    OperationObservationStateV1::Active {
                                        phase: OperationActivePhaseV1::Resolving,
                                    } => {}
                                    OperationObservationStateV1::CancellationRequested {
                                        request,
                                    } => {
                                        assert_eq!(fault, "cancel-requested");
                                        assert_eq!(
                                            request.record_ref,
                                            record_ref("fixture/cancellation-request")
                                        );
                                        assert_eq!(
                                            request.record_sha256,
                                            ArtifactDigest::hash_bytes(
                                                b"declared cancellation request"
                                            )
                                        );
                                    }
                                    _ => {
                                        panic!("this fixture has no other admitted state evidence")
                                    }
                                }
                                Ok(())
                            },
                            admit_rotation:
                                |_: &ResidentRecordRefV1,
                                 _: &ArtifactDigest,
                                 _: &ResidentRecordRefV1,
                                 _: &ArtifactDigest| {
                                    panic!("this fixture does not rotate runtime authority")
                                },
                            verify_signature:
                                |domain: &[u8],
                                 _: &OperationObservationPreimageV1,
                                 _: &OperationObservationDigest,
                                 actual: &CanonicalSignatureV1| {
                                    assert_eq!(domain, OPERATION_OBSERVATION_V1_SIGNATURE_DOMAIN);
                                    assert_eq!(actual, &signature());
                                    signatures.set(signatures.get() + 1);
                                    Ok(())
                                },
                        };
                            let observed = recover_operation_observation(
                            OperationObservationRecoveryInput {
                                store: &coordinate,
                                operation: recovered,
                                observation: pale_application::resident::OperationObservationSelection::SelectedHead,
                                checkpoint_authority: &checkpoint_authority(&checkpoint),
                                limits,
                                checkpoint_limits: OperationCheckpointAncestryLimits::new(8, 1 << 20, 8).unwrap(),
                                membership_limits: AccumulatorMembershipLimits::new(
                                    1,
                                    if fault == "observation-membership-work-bound" {
                                        1
                                    } else {
                                        10_000
                                    },
                                    100_000,
                                )
                                .unwrap(),
                            },
                            &mut port,
                            &mut anchor,
                            admission,
                            |domain, _, _, actual| {
                                assert_eq!(domain, CONTROL_STATE_CHECKPOINT_V1_SIGNATURE_DOMAIN);
                                assert_eq!(actual, &signature());
                                Ok(())
                            },
                        );
                            match (fault, observed) {
                                (
                                    "none"
                                    | "saved-handle"
                                    | "observation-saved-handle"
                                    | "lost-anchor-confirmation"
                                    | "observation-replaced"
                                    | "cancel-requested",
                                    Ok(proof),
                                ) => {
                                    let (reference, observation) = proof.selected();
                                    assert_eq!(proof.selected_head(), (reference, observation));
                                    assert_eq!(proof.store(), &coordinate);
                                    let revision = if appends_observation { 2 } else { 1 };
                                    assert_eq!(observation.revision().get(), revision);
                                    assert_eq!(observation.operation_handle_ref(), &expected.0);
                                    assert_eq!(
                                        observation.operation_handle_sha256(),
                                        expected.1.canonical_operation_handle_sha256()
                                    );
                                    assert_eq!(
                                        *reference,
                                        if appends_observation {
                                            record_ref("operation/observation-2")
                                        } else {
                                            first.coordinate().record_ref().clone()
                                        }
                                    );
                                    assert_eq!(
                                        observation.predecessor_ref().as_ref(),
                                        appends_observation
                                            .then(|| first.coordinate().record_ref())
                                    );
                                    assert_eq!(signatures.get(), revision);
                                    assert_eq!(states.get(), revision);
                                    assert!(!observation.state().is_terminal());
                                    assert_eq!(
                                        matches!(
                                            observation.state(),
                                            OperationObservationStateV1::CancellationRequested { .. }
                                        ),
                                        fault == "cancel-requested"
                                    );
                                    assert_eq!(anchor.reads, 6);
                                }
                                (
                                    "observation-history-missing",
                                    Err(OperationObservationReadmissionError::Missing),
                                )
                                | (
                                    "observation-history-substituted",
                                    Err(OperationObservationReadmissionError::Lineage),
                                )
                                | (
                                    "observation-checkpoint-missing",
                                    Err(OperationObservationReadmissionError::Lookup(
                                        OperationRecoveryError::IncompleteObservationIndexes,
                                    )),
                                )
                                | (
                                    "observation-revision-bound" | "observation-chain-byte-bound",
                                    Err(OperationObservationReadmissionError::Bounds),
                                )
                                | (
                                    "observation-record-byte-bound",
                                    Err(OperationObservationReadmissionError::Lookup(
                                        OperationRecoveryError::Store(_),
                                    )),
                                )
                                | (
                                    "observation-checkpoint-misdirected",
                                    Err(OperationObservationReadmissionError::Lookup(
                                        OperationRecoveryError::Store(_),
                                    )),
                                )
                                | (
                                    "observation-checkpoint-fork",
                                    Err(OperationObservationReadmissionError::Checkpoint(
                                        LocalOperationRecoveryError::Association,
                                    )),
                                )
                                | (
                                    "observation-leaf-missing"
                                    | "observation-membership-work-bound"
                                    | "observation-leaf-advanced",
                                    Err(OperationObservationReadmissionError::Checkpoint(
                                        LocalOperationRecoveryError::Membership(_),
                                    )),
                                )
                                | (
                                    "observation-anchor-changed",
                                    Err(OperationObservationReadmissionError::Checkpoint(
                                        LocalOperationRecoveryError::Association,
                                    )),
                                ) => {}
                                (_, value) => {
                                    panic!("{fault}: unexpected observation recovery {value:?}")
                                }
                            }
                            if fault == "observation-leaf-advanced" {
                                assert!(
                                    delayed_append.is_none(),
                                    "the second connection advanced once"
                                );
                                assert!(writer.total_changes() > writer_changes);
                                assert_ne!(
                                    port.connection
                                        .pragma_query_value(None, "data_version", |r| r
                                            .get::<_, i64>(0))
                                        .unwrap(),
                                    data_version
                                );
                                data_version = port
                                    .connection
                                    .pragma_query_value(None, "data_version", |r| r.get(0))
                                    .unwrap();
                            } else {
                                assert_eq!(
                                    writer.total_changes(),
                                    writer_changes,
                                    "recovery does not write through the second connection"
                                );
                            }
                        }
                        (
                            "unconfirmed-anchor" | "restored-database",
                            Err(LocalOperationRecoveryError::Missing),
                        ) => {}
                        (
                            "foreign-anchor-key"
                            | "profile"
                            | "checkpoint-insertion-stamp"
                            | "saved-handle-request-alias",
                            Err(LocalOperationRecoveryError::Association),
                        ) => {
                            if fault == "checkpoint-insertion-stamp" {
                                assert_eq!(
                                    anchor.reads, 0,
                                    "reject before anchor or membership reads"
                                );
                            }
                        }
                        (
                            "request",
                            Err(LocalOperationRecoveryError::Lookup(
                                OperationRecoveryError::RequestConflict,
                            )),
                        ) => {}
                        (_, value) => panic!(
                            "{fault}: wrong recovery disposition: {}",
                            value.err().map_or_else(
                                || "unexpected locator".to_string(),
                                |e| format!("{e:?}")
                            )
                        ),
                    }
                    assert_eq!(sql.total_changes(), writes);
                    assert_eq!(
                        sql.pragma_query_value(None, "data_version", |r| r.get::<_, i64>(0))
                            .unwrap(),
                        data_version
                    );
                    assert_eq!(
                        (anchor.advances, checkpoint_signatures.get()),
                        (advances, expected_checkpoint_signatures)
                    );
                    assert_eq!(anchor.value, before_anchor);
                },
            );
        }
    });
}
