//! Contract tests for neutral durable-authority byte custody.
//!
//! These tests keep adapter-owned physical boundaries separate from
//! application canonicality, fold validity, and runtime authority.

use super::*;
use pale_core::records::{ControlIncarnationIdV1, ControlStoreIdV1, JournalSequenceV1};
use std::error::Error as _;

fn store() -> AuthorityStoreCoordinate {
    AuthorityStoreCoordinate::new(
        ControlStoreIdV1::parse("control-store").expect("store id"),
        ControlIncarnationIdV1::parse("incarnation-1").expect("incarnation id"),
    )
}

fn entry(sequence: u64) -> SequencedAuthorityEntry {
    SequencedAuthorityEntry::new(
        JournalSequenceV1::new(sequence).expect("sequence"),
        BoundedAuthorityEntryBytes::new(vec![b'{', b'}'], 2).expect("bounded bytes"),
    )
}

fn limits(maximum_tail_entries: usize) -> AuthorityRecoveryReadLimits {
    AuthorityRecoveryReadLimits::new(8, 8, 2, maximum_tail_entries).expect("recovery limits")
}

#[test]
fn opaque_bytes_enforce_nonempty_explicit_bounds() {
    assert!(matches!(
        BoundedAuthorityEntryBytes::new(Vec::new(), 8).map_err(|error| error.kind()),
        Err(DurableAuthorityStoreErrorKind::Corrupt(
            AuthorityByteClass::JournalEntry
        ))
    ));
    assert!(matches!(
        BoundedRecoveryCheckpointBytes::new(vec![1, 2], 1).map_err(|error| error.kind()),
        Err(DurableAuthorityStoreErrorKind::BoundExceeded {
            class: AuthorityByteClass::RecoveryCheckpoint
        })
    ));
}

#[test]
fn recovery_budget_rejects_every_zero_dimension() {
    for limits in [(0, 1, 1, 1), (1, 0, 1, 1), (1, 1, 0, 1), (1, 1, 1, 0)] {
        assert!(AuthorityRecoveryReadLimits::new(limits.0, limits.1, limits.2, limits.3).is_err());
    }
}

#[test]
fn prepared_append_enforces_initial_and_exact_successor_sequences() {
    assert!(PreparedAuthorityAppend::new(store(), None, entry(1)).is_ok());
    assert!(PreparedAuthorityAppend::new(
        store(),
        Some(JournalSequenceV1::new(4).expect("head")),
        entry(5),
    )
    .is_ok());
    assert!(PreparedAuthorityAppend::new(store(), None, entry(2)).is_err());
    assert!(PreparedAuthorityAppend::new(
        store(),
        Some(JournalSequenceV1::new(4).expect("head")),
        entry(6),
    )
    .is_err());
}

#[test]
fn prepared_checkpoint_cannot_claim_another_journal_frontier() {
    let checkpoint = StoredRecoveryCheckpoint::new(
        JournalSequenceV1::new(4).expect("checkpoint sequence"),
        BoundedRecoveryCheckpointBytes::new(vec![b'{', b'}'], 2).expect("checkpoint bytes"),
    );
    assert!(PreparedRecoveryCheckpoint::new(
        store(),
        JournalSequenceV1::new(5).expect("head"),
        checkpoint,
    )
    .is_err());
}

#[test]
fn append_failure_contract_classifies_every_store_error() {
    for kind in [
        DurableAuthorityStoreErrorKind::Corrupt(AuthorityByteClass::JournalEntry),
        DurableAuthorityStoreErrorKind::BoundExceeded {
            class: AuthorityByteClass::JournalEntry,
        },
        DurableAuthorityStoreErrorKind::Unavailable,
    ] {
        assert_eq!(
            DurableAuthorityStoreError::from(kind).append_failure(),
            DurableAuthorityAppendFailure::RejectedNoAppend
        );
    }
    assert_eq!(
        DurableAuthorityStoreError::from(DurableAuthorityStoreErrorKind::Conflict).append_failure(),
        DurableAuthorityAppendFailure::Conflict
    );
    assert_eq!(
        DurableAuthorityStoreError::from(DurableAuthorityStoreErrorKind::AmbiguousCommit)
            .append_failure(),
        DurableAuthorityAppendFailure::OutcomeUnknown
    );
}

#[test]
fn checkpoint_failure_contract_does_not_imply_effect_uncertainty() {
    for kind in [
        DurableAuthorityStoreErrorKind::Corrupt(AuthorityByteClass::RecoveryCheckpoint),
        DurableAuthorityStoreErrorKind::BoundExceeded {
            class: AuthorityByteClass::RecoveryCheckpoint,
        },
        DurableAuthorityStoreErrorKind::Unavailable,
    ] {
        assert_eq!(
            DurableAuthorityStoreError::from(kind).checkpoint_failure(),
            DurableAuthorityCheckpointFailure::RejectedNoWrite
        );
    }
    assert_eq!(
        DurableAuthorityStoreError::from(DurableAuthorityStoreErrorKind::Conflict)
            .checkpoint_failure(),
        DurableAuthorityCheckpointFailure::Conflict
    );
    assert_eq!(
        DurableAuthorityStoreError::from(DurableAuthorityStoreErrorKind::AmbiguousCommit)
            .checkpoint_failure(),
        DurableAuthorityCheckpointFailure::OutcomeUnknown
    );
}

#[test]
fn native_causes_do_not_change_the_neutral_failure_laws() {
    for kind in [
        DurableAuthorityStoreErrorKind::Conflict,
        DurableAuthorityStoreErrorKind::Corrupt(AuthorityByteClass::JournalEntry),
        DurableAuthorityStoreErrorKind::BoundExceeded {
            class: AuthorityByteClass::RecoveryCheckpoint,
        },
        DurableAuthorityStoreErrorKind::Unavailable,
        DurableAuthorityStoreErrorKind::AmbiguousCommit,
    ] {
        let pure = DurableAuthorityStoreError::from(kind);
        let sourced = DurableAuthorityStoreError::with_source(
            kind,
            std::io::Error::other("native custody failure"),
        );
        assert_eq!(sourced.kind(), kind);
        assert_eq!(sourced.append_failure(), pure.append_failure());
        assert_eq!(sourced.checkpoint_failure(), pure.checkpoint_failure());
        assert!(pure.source().is_none());
        let native = sourced
            .source()
            .and_then(|source| source.downcast_ref::<std::io::Error>())
            .expect("the original native error remains downcastable");
        assert_eq!(native.to_string(), "native custody failure");
        assert_eq!(sourced.to_string(), pure.to_string());
    }
}

#[test]
fn rejected_checkpoint_candidate_retains_only_its_order_coordinate() {
    let sequence = JournalSequenceV1::new(7).expect("sequence");
    let candidate = RecoveryCheckpointCandidate::Rejected {
        through_sequence: sequence,
        reason: RecoveryCheckpointPhysicalRejection::Integrity,
    };

    assert_eq!(candidate.through_sequence(), sequence);
    assert!(candidate.trusted().is_none());
    assert_eq!(
        candidate.rejection(),
        Some(RecoveryCheckpointPhysicalRejection::Integrity)
    );
}

#[test]
fn fixed_head_journal_pages_continue_then_complete() {
    let through = JournalSequenceV1::new(3).expect("target head");
    let first =
        JournalTailPageRead::new(store(), None, through, limits(3), 2).expect("first request");
    let first_page = JournalTailPage::new(
        &first,
        vec![entry(1), entry(2)],
        JournalTailPageBoundary::More {
            resume_after: JournalSequenceV1::new(2).expect("resume"),
        },
    )
    .expect("first page");
    assert_eq!(first_page.entries().len(), 2);

    let second = first
        .continue_after(&first_page)
        .expect("continuation is within the cumulative budget")
        .expect("more entries remain");
    assert_eq!(second.remaining_entries(), 1);
    let second_page =
        JournalTailPage::new(&second, vec![entry(3)], JournalTailPageBoundary::Complete)
            .expect("second page");
    assert_eq!(second_page.entries()[0].sequence(), through);
}

#[test]
fn fixed_head_pages_preserve_the_first_physical_quarantine_coordinate() {
    let request = JournalTailPageRead::new(
        store(),
        Some(JournalSequenceV1::new(2).expect("after")),
        JournalSequenceV1::new(4).expect("target head"),
        limits(2),
        2,
    )
    .expect("page request");
    let first_untrusted_sequence = JournalSequenceV1::new(4).expect("quarantine");
    let page = JournalTailPage::new(
        &request,
        vec![entry(3)],
        JournalTailPageBoundary::QuarantinedFrom {
            first_untrusted_sequence,
        },
    )
    .expect("quarantined prefix");
    assert_eq!(
        page.boundary(),
        JournalTailPageBoundary::QuarantinedFrom {
            first_untrusted_sequence
        }
    );
}

#[test]
fn fixed_head_pages_reject_gaps_overflow_and_inexact_boundaries() {
    let request = JournalTailPageRead::new(
        store(),
        None,
        JournalSequenceV1::new(3).expect("target head"),
        limits(2),
        2,
    )
    .expect("page request");
    for result in [
        JournalTailPage::new(
            &request,
            vec![entry(2)],
            JournalTailPageBoundary::More {
                resume_after: JournalSequenceV1::new(2).expect("resume"),
            },
        ),
        JournalTailPage::new(
            &request,
            vec![entry(1), entry(2), entry(3)],
            JournalTailPageBoundary::Complete,
        ),
        JournalTailPage::new(
            &request,
            vec![entry(1), entry(2)],
            JournalTailPageBoundary::More {
                resume_after: JournalSequenceV1::new(1).expect("wrong resume"),
            },
        ),
    ] {
        assert!(result.is_err());
    }
    assert!(JournalTailPageRead::new(
        store(),
        Some(JournalSequenceV1::new(3).expect("after")),
        JournalSequenceV1::new(3).expect("target head"),
        limits(2),
        2,
    )
    .is_err());
}

#[test]
fn fixed_head_paging_cannot_reset_the_whole_pass_budget_during_continuation() {
    let request = JournalTailPageRead::new(
        store(),
        None,
        JournalSequenceV1::new(3).expect("target head"),
        limits(2),
        1,
    )
    .expect("page request");
    let first = JournalTailPage::new(
        &request,
        vec![entry(1)],
        JournalTailPageBoundary::More {
            resume_after: JournalSequenceV1::new(1).expect("resume"),
        },
    )
    .expect("first page");
    let second_request = request
        .continue_after(&first)
        .expect("one entry remains")
        .expect("continuation");
    let second = JournalTailPage::new(
        &second_request,
        vec![entry(2)],
        JournalTailPageBoundary::More {
            resume_after: JournalSequenceV1::new(2).expect("resume"),
        },
    );
    assert!(matches!(
        second.map_err(|error| error.kind()),
        Err(DurableAuthorityStoreErrorKind::BoundExceeded {
            class: AuthorityByteClass::JournalEntry
        })
    ));
}
