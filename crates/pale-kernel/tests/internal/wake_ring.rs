//! Law-oriented tests for the reconstructible resident wake-token ring.
//!
//! These tests compare independent consumer schedules against a small
//! deterministic suffix model. They exercise notification only: no test treats
//! a token as a durable append, event body, admission proof, or effect result.

use super::*;

fn sequence(value: u64) -> JournalSequenceV1 {
    JournalSequenceV1::new(value).expect("positive journal sequence")
}

fn ring(capacity: usize) -> WakeTokenRing {
    WakeTokenRing::new(capacity, WakeEpochV1::new(7), RingCursorV1::new(0), None)
        .expect("bounded ring")
}

fn publish(ring: &mut WakeTokenRing, sequence_value: u64) -> WakeToken {
    ring.publish_after_durable_append(sequence(sequence_value))
        .expect("adjacent durable publication")
}

fn token_coordinates(batch: &WakeBatch<'_>) -> Vec<(u64, u64, u64)> {
    batch
        .iter()
        .map(|token| {
            (
                token.epoch().get(),
                token.cursor().get(),
                token.journal_sequence().get(),
            )
        })
        .collect()
}

#[test]
fn construction_requires_explicit_positive_capacity_and_frontiers() {
    assert_eq!(
        WakeTokenRing::new(
            0,
            WakeEpochV1::new(2),
            RingCursorV1::new(8),
            Some(sequence(13)),
        )
        .expect_err("zero capacity must fail"),
        WakeRingBuildError::ZeroCapacity
    );
    let ring = WakeTokenRing::new(
        3,
        WakeEpochV1::new(2),
        RingCursorV1::new(8),
        Some(sequence(13)),
    )
    .expect("explicit ring");
    assert_eq!(ring.capacity(), 3);
    assert_eq!(ring.epoch(), WakeEpochV1::new(2));
    assert_eq!(ring.starting_cursor().cursor(), RingCursorV1::new(8));
    assert_eq!(
        ring.starting_cursor().journal_sequence(),
        Some(sequence(13))
    );
}

#[test]
fn independent_consumers_read_without_mutating_or_erasing_each_other() {
    let mut ring = ring(4);
    let slow = ring.starting_cursor();
    let fast = ring.starting_cursor();
    publish(&mut ring, 1);
    publish(&mut ring, 2);

    let fast_batch = ring.read_after(fast).expect("fast read");
    assert_eq!(token_coordinates(&fast_batch), vec![(7, 1, 1), (7, 2, 2)]);
    let fast = fast_batch.advanced_cursor();
    assert!(ring.read_after(fast).expect("caught up").is_empty());

    publish(&mut ring, 3);
    assert_eq!(
        token_coordinates(&ring.read_after(fast).expect("new fast token")),
        vec![(7, 3, 3)]
    );
    assert_eq!(
        token_coordinates(&ring.read_after(slow).expect("slow window retained")),
        vec![(7, 1, 1), (7, 2, 2), (7, 3, 3)]
    );
}

#[test]
fn overflow_returns_an_exact_durable_replay_instruction() {
    let mut ring = WakeTokenRing::new(
        2,
        WakeEpochV1::new(4),
        RingCursorV1::new(10),
        Some(sequence(20)),
    )
    .expect("ring");
    let slow = ring.starting_cursor();
    publish(&mut ring, 21);
    publish(&mut ring, 22);
    let fast = ring
        .read_after(slow)
        .expect("fast consumer reads retained tokens")
        .advanced_cursor();
    publish(&mut ring, 23);

    assert_eq!(
        ring.read_after(slow).expect_err("token 21 was overwritten"),
        WakeReadError::DurableReplayRequired {
            replay_after: Some(sequence(20)),
            durable_through: Some(sequence(23)),
            resume_at: ring.current_cursor(),
        }
    );
    assert_eq!(
        token_coordinates(&ring.read_after(fast).expect("fast consumer remains live")),
        vec![(4, 13, 23)]
    );
    assert_eq!(ring.current_cursor().cursor(), RingCursorV1::new(13));
}

#[test]
fn duplicate_older_and_gapped_publications_fail_without_mutation() {
    let mut ring = ring(3);
    let first = publish(&mut ring, 1);
    let head = ring.current_cursor();
    assert_eq!(
        ring.publish_after_durable_append(sequence(1)),
        Err(WakePublishError::Duplicate {
            sequence: sequence(1)
        })
    );
    assert_eq!(
        ring.publish_after_durable_append(sequence(3)),
        Err(WakePublishError::Gap {
            expected: sequence(2),
            supplied: sequence(3),
        })
    );
    publish(&mut ring, 2);
    assert_eq!(
        ring.publish_after_durable_append(sequence(1)),
        Err(WakePublishError::OutOfOrder {
            head: sequence(2),
            supplied: sequence(1),
        })
    );
    assert_eq!(first.cursor(), RingCursorV1::new(1));
    assert_eq!(head.journal_sequence(), Some(sequence(1)));
    assert_eq!(ring.current_cursor().journal_sequence(), Some(sequence(2)));
}

#[test]
fn restart_rejects_stale_epoch_and_starts_fresh_ring_empty() {
    let mut old = ring(2);
    publish(&mut old, 1);
    let stale = old.current_cursor();
    let restarted = WakeTokenRing::new(
        2,
        WakeEpochV1::new(8),
        RingCursorV1::new(0),
        Some(sequence(1)),
    )
    .expect("restarted ring");

    assert!(restarted
        .read_after(restarted.starting_cursor())
        .expect("fresh consumer")
        .is_empty());
    assert_eq!(
        restarted.read_after(stale).expect_err("prior epoch"),
        WakeReadError::StaleEpoch {
            cursor_epoch: WakeEpochV1::new(7),
            ring_epoch: WakeEpochV1::new(8),
            replay_after: Some(sequence(1)),
            durable_through: Some(sequence(1)),
            resume_at: restarted.current_cursor(),
        }
    );
}

#[test]
fn cursor_exhaustion_and_terminal_journal_inputs_are_closed_and_non_mutating() {
    let mut cursor_exhausted =
        WakeTokenRing::new(1, WakeEpochV1::new(1), RingCursorV1::new(u64::MAX), None)
            .expect("ring");
    assert_eq!(
        cursor_exhausted.publish_after_durable_append(sequence(1)),
        Err(WakePublishError::CursorExhausted {
            cursor: RingCursorV1::new(u64::MAX)
        })
    );
    assert_eq!(
        cursor_exhausted.current_cursor(),
        cursor_exhausted.starting_cursor()
    );

    let terminal_sequence = pale_core::canon::JSON_SAFE_INTEGER_MAX;
    let mut journal_exhausted = WakeTokenRing::new(
        1,
        WakeEpochV1::new(1),
        RingCursorV1::new(0),
        Some(sequence(terminal_sequence)),
    )
    .expect("ring");
    assert_eq!(
        journal_exhausted.publish_after_durable_append(sequence(terminal_sequence - 1)),
        Err(WakePublishError::JournalSequenceExhausted {
            sequence: sequence(terminal_sequence),
        })
    );
    assert_eq!(
        journal_exhausted.publish_after_durable_append(sequence(terminal_sequence)),
        Err(WakePublishError::JournalSequenceExhausted {
            sequence: sequence(terminal_sequence),
        })
    );
    assert_eq!(
        expected_journal_successor(
            Some(sequence(terminal_sequence)),
            sequence(terminal_sequence)
        ),
        Err(WakePublishError::JournalSequenceExhausted {
            sequence: sequence(terminal_sequence),
        })
    );
}

#[test]
fn token_shape_is_coordinate_only_and_coordinate_mismatch_fails_closed() {
    assert_eq!(
        std::mem::size_of::<WakeToken>(),
        std::mem::size_of::<(WakeEpochV1, RingCursorV1, JournalSequenceV1)>()
    );
    let mut ring = ring(2);
    let token = publish(&mut ring, 1);
    assert_eq!(token.consumer_cursor(), ring.current_cursor());
    let dishonest = WakeConsumerCursor {
        epoch: WakeEpochV1::new(7),
        cursor: RingCursorV1::new(1),
        journal_sequence: None,
    };
    assert_eq!(
        ring.read_after(dishonest).expect_err("coordinate mismatch"),
        WakeReadError::CoordinateMismatch {
            supplied: None,
            expected: Some(sequence(1)),
        }
    );
}

#[test]
fn deterministic_reference_model_matches_every_capacity_and_schedule() {
    for capacity in 1..=5 {
        for publication_count in 0_u64..=12 {
            let mut ring = ring(capacity);
            let origin = ring.starting_cursor();
            for value in 1..=publication_count {
                publish(&mut ring, value);
            }
            let retained_start = publication_count
                .saturating_sub(capacity as u64)
                .saturating_add(1);
            for consumed in 0..=publication_count {
                let cursor = WakeConsumerCursor {
                    epoch: WakeEpochV1::new(7),
                    cursor: RingCursorV1::new(consumed),
                    journal_sequence: (consumed > 0).then(|| sequence(consumed)),
                };
                let result = ring.read_after(cursor);
                if consumed + 1 < retained_start {
                    assert!(matches!(
                        result,
                        Err(WakeReadError::DurableReplayRequired { .. })
                    ));
                } else {
                    let actual = result
                        .expect("cursor remains within retained suffix")
                        .iter()
                        .map(|token| token.journal_sequence().get())
                        .collect::<Vec<_>>();
                    let expected = ((consumed + 1)..=publication_count).collect::<Vec<_>>();
                    assert_eq!(actual, expected);
                }
            }
            assert_eq!(origin.journal_sequence(), None);
        }
    }
}
