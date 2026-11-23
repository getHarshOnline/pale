//! Public producer preparation, real `SQLite` custody, then controlled external anchors.
//!
//! These tests prove sequencing and replay, not production signature or anchor
//! backend qualification, current authority, or permission to publish an Event.

use super::*;
use crate::resident_fixture::producer_custody_fixture::InitialProducerFixture;
use pale_application::resident::{
    AnchorAdvanceEvidence, PrincipalProducerIncarnationCommitEvidence,
};
use pale_ports::{
    AntiRollbackAnchor, AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError,
    AntiRollbackAnchorErrorKind, AntiRollbackAnchorKey, AntiRollbackAnchorObservation,
    AntiRollbackAnchorPort, ControlDomainAntiRollbackAnchor, PreparedAntiRollbackAnchorTransition,
    PrincipalTrustMinimumAnchor,
};
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq)]
enum Call {
    Compare,
    Observe,
}

struct ControlledAnchors {
    values: BTreeMap<AntiRollbackAnchorKey, AntiRollbackAnchor>,
    advanced: Vec<AntiRollbackAnchorKey>,
    fail_at_call: Option<usize>,
    observed: Vec<AntiRollbackAnchorKey>,
    order: Vec<Call>,
}

impl ControlledAnchors {
    fn at_predecessor(
        candidate: &PreparedPrincipalProducerIncarnationTransition,
        admitted_control: &pale_core::records::ControlStateCheckpointV1,
    ) -> Self {
        let trust = candidate.expected_trust();
        let control = candidate.expected_checkpoint();
        let values: [AntiRollbackAnchor; 2] = [
            PrincipalTrustMinimumAnchor::new(
                trust.principal_id().clone(),
                trust.checkpoint_epoch(),
                trust.coordinate().clone(),
            )
            .into(),
            ControlDomainAntiRollbackAnchor::new(
                admitted_control.control_store_id().clone(),
                admitted_control.control_incarnation_id().clone(),
                control.checkpoint_domain_id().clone(),
                control.domain_epoch(),
                control.domain_sequence(),
                control.coordinate().clone(),
            )
            .into(),
        ];
        Self {
            values: values
                .into_iter()
                .map(|value| (value.key(), value))
                .collect(),
            advanced: vec![],
            fail_at_call: None,
            observed: vec![],
            order: vec![],
        }
    }
}

impl AntiRollbackAnchorPort for ControlledAnchors {
    fn read(
        &mut self,
        _: &AntiRollbackAnchorKey,
    ) -> Result<Option<AntiRollbackAnchor>, AntiRollbackAnchorError> {
        panic!("completion needs no unbound reads")
    }
    fn compare_and_advance(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorAdvanceDisposition, AntiRollbackAnchorError> {
        self.advanced.push(transition.key().clone());
        self.order.push(Call::Compare);
        if self.fail_at_call == Some(self.advanced.len()) {
            return Err(AntiRollbackAnchorError::new(
                AntiRollbackAnchorErrorKind::Unavailable,
            ));
        }
        Ok(
            match transition.classify_observed(self.values.get(transition.key())) {
                AntiRollbackAnchorObservation::ExactPredecessor => {
                    self.values
                        .insert(transition.key().clone(), transition.next().clone());
                    AntiRollbackAnchorAdvanceDisposition::Advanced
                }
                AntiRollbackAnchorObservation::ExactSuccessor => {
                    AntiRollbackAnchorAdvanceDisposition::ExactReplay
                }
                other => AntiRollbackAnchorAdvanceDisposition::NotAdvanced(other),
            },
        )
    }
    fn observe(
        &mut self,
        transition: &PreparedAntiRollbackAnchorTransition,
    ) -> Result<AntiRollbackAnchorObservation, AntiRollbackAnchorError> {
        self.observed.push(transition.key().clone());
        self.order.push(Call::Observe);
        Ok(transition.classify_observed(self.values.get(transition.key())))
    }
}

#[test]
fn public_preparation_commits_in_sqlite_before_both_anchors_and_replays_exactly() {
    let (fixture, _) = InitialProducerFixture::new();
    let prepared = fixture.prepare();
    let candidate = prepared.candidate().clone();
    let mut connection = initialize_candidate_connection(
        Connection::open_in_memory().expect("connection"),
        &candidate,
    );
    let mut anchors = ControlledAnchors::at_predecessor(&candidate, &fixture.next_control);
    assert!(anchors.advanced.is_empty());
    let pending = prepared
        .commit(&mut SqlitePrincipalProducerIncarnationCustody {
            connection: &mut connection,
        })
        .expect("real atomic database commit");
    assert!(
        anchors.advanced.is_empty(),
        "database commit cannot call an anchor"
    );
    assert_eq!(
        SqlitePrincipalProducerIncarnationCustody {
            connection: &mut connection
        }
        .observe_exact(&candidate)
        .expect("database read-back"),
        PrincipalProducerIncarnationDatabaseObservation::ExactSuccessor
    );
    let completed = pending
        .advance_anchors(&mut anchors)
        .expect("two controlled anchors");
    assert_eq!(
        completed.database_evidence(),
        PrincipalProducerIncarnationCommitEvidence::New
    );
    assert_eq!(
        completed.trust_anchor_evidence(),
        AnchorAdvanceEvidence::Advanced
    );
    assert_eq!(
        completed.control_anchor_evidence(),
        AnchorAdvanceEvidence::Advanced
    );
    assert_eq!(completed.candidate(), &candidate);
    assert_eq!(
        completed.control_store().store_id(),
        fixture.next_control.control_store_id()
    );
    assert_eq!(
        completed.control_store().incarnation_id(),
        fixture.next_control.control_incarnation_id()
    );
    assert_ne!(
        completed.control_store().store_id(),
        candidate.binding().trust_store_id()
    );
    let rows = transition_row_counts(&connection);
    assert_eq!(rows, (5, 1, 0, 1));
    let pending = fixture
        .prepare()
        .commit(&mut SqlitePrincipalProducerIncarnationCustody {
            connection: &mut connection,
        })
        .expect("exact database replay");
    let replay = pending
        .advance_anchors(&mut anchors)
        .expect("exact anchor replay");
    assert_eq!(
        replay.database_evidence(),
        PrincipalProducerIncarnationCommitEvidence::ExactReplay
    );
    assert_eq!(
        replay.trust_anchor_evidence(),
        AnchorAdvanceEvidence::ExactReplay
    );
    assert_eq!(
        replay.control_anchor_evidence(),
        AnchorAdvanceEvidence::ExactReplay
    );
    assert_eq!(anchors.advanced.len(), 2);
    assert_eq!(anchors.observed, anchors.advanced);
    assert_eq!(
        anchors.order,
        [Call::Compare, Call::Compare, Call::Observe, Call::Observe]
    );
    assert_eq!(transition_row_counts(&connection), rows);
}

#[test]
fn failed_sqlite_commit_retains_candidate_and_database_predecessor() {
    let (fixture, _) = InitialProducerFixture::new();
    let prepared = fixture.prepare();
    let candidate = prepared.candidate().clone();
    let mut connection = initialize_candidate_connection(
        Connection::open_in_memory().expect("connection"),
        &candidate,
    );
    connection.execute_batch("CREATE TRIGGER reject_producer_insert BEFORE INSERT ON pale_principal_producer_incarnation BEGIN SELECT RAISE(ABORT, 'controlled failure'); END;").expect("failure trigger");
    let failure = prepared
        .commit(&mut SqlitePrincipalProducerIncarnationCustody {
            connection: &mut connection,
        })
        .expect_err("no pending-anchor state without database proof");
    assert_eq!(failure.into_prepared().candidate(), &candidate);
    assert_eq!(
        SqlitePrincipalProducerIncarnationCustody {
            connection: &mut connection
        }
        .observe_exact(&candidate)
        .expect("rollback read-back"),
        PrincipalProducerIncarnationDatabaseObservation::Predecessor
    );
    assert_eq!(transition_row_counts(&connection), (2, 0, 0, 0));
}

#[test]
fn reopened_database_replays_the_first_anchor_before_finishing_the_second() {
    let directory = tempfile::tempdir().expect("database directory");
    let path = directory.path().join("producer.sqlite");
    let (fixture, _) = InitialProducerFixture::new();
    let prepared = fixture.prepare();
    let candidate = prepared.candidate().clone();
    let mut connection =
        initialize_candidate_connection(Connection::open(&path).expect("new database"), &candidate);
    let mut anchors = ControlledAnchors::at_predecessor(&candidate, &fixture.next_control);
    anchors.fail_at_call = Some(2);
    let pending = prepared
        .commit(&mut SqlitePrincipalProducerIncarnationCustody {
            connection: &mut connection,
        })
        .expect("durable database commit");
    let failure = pending
        .advance_anchors(&mut anchors)
        .expect_err("second anchor unavailable");
    assert_eq!(
        failure.pending().trust_anchor_evidence(),
        Some(AnchorAdvanceEvidence::Advanced)
    );
    drop(failure);
    drop(connection);

    // Reconstruct from exact database replay, not the old in-memory pending value.
    let mut connection = Connection::open(&path).expect("reopen database");
    connection
        .pragma_update(None, "foreign_keys", true)
        .expect("restore foreign keys");
    connection
        .pragma_update(None, "trusted_schema", false)
        .expect("restore untrusted schema");
    assert_eq!(
        connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
            .expect("foreign-key read-back"),
        1
    );
    assert_eq!(
        connection
            .query_row("PRAGMA trusted_schema", [], |row| row.get::<_, i64>(0))
            .expect("trusted-schema read-back"),
        0
    );
    let pending = fixture
        .prepare()
        .commit(&mut SqlitePrincipalProducerIncarnationCustody {
            connection: &mut connection,
        })
        .expect("exact committed candidate survives reopen");
    assert_eq!(
        pending.evidence(),
        PrincipalProducerIncarnationCommitEvidence::ExactReplay
    );
    assert_eq!(pending.trust_anchor_evidence(), None);
    anchors.fail_at_call = None;
    let completed = pending
        .advance_anchors(&mut anchors)
        .expect("replay first and advance second");
    assert_eq!(
        completed.trust_anchor_evidence(),
        AnchorAdvanceEvidence::ExactReplay
    );
    assert_eq!(
        completed.control_anchor_evidence(),
        AnchorAdvanceEvidence::Advanced
    );
    assert_eq!(completed.candidate(), &candidate);
    assert_eq!(anchors.advanced.len(), 3);
    assert_eq!(anchors.observed, anchors.advanced[..2]);
    assert_eq!(anchors.advanced[1], anchors.advanced[2]);
    assert_eq!(
        anchors.order,
        [
            Call::Compare,
            Call::Compare,
            Call::Observe,
            Call::Observe,
            Call::Compare
        ]
    );
    assert_eq!(transition_row_counts(&connection), (5, 1, 0, 1));
}
