//! Reconstructible notification over an already-durable authority journal.
//!
//! A [`WakeTokenRing`] is a process-local performance aid. It neither stores
//! event bodies nor proves that an append committed. The writer publishes a
//! token only after the corresponding journal append is durable. Consumers
//! that miss the bounded window must recover from the durable journal before
//! resuming at the coordinate carried by
//! [`WakeReadError::DurableReplayRequired`]. Durable replay and per-consumer
//! cursor custody are deliberately outside this process-local type.

use std::collections::VecDeque;

use pale_core::records::{JournalSequenceV1, RingCursorV1, WakeEpochV1};

/// Minimum process-local notification for one durable journal append.
///
/// The token is only a coordinate in the product of [`WakeEpochV1`],
/// [`RingCursorV1`], and [`JournalSequenceV1`]:
///
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/math/wake-ring-laws.display.mathml"))]
///
/// Here `e` fences one process epoch, `c` orders publications in this ring,
/// and `j` identifies the durable journal append. The token intentionally
/// carries no event body, semantic state, consumer identity, admission proof,
/// or native-effect result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WakeToken {
    epoch: WakeEpochV1,
    cursor: RingCursorV1,
    journal_sequence: JournalSequenceV1,
}

impl WakeToken {
    /// Return the process-local epoch that fences this token.
    #[must_use]
    pub const fn epoch(self) -> WakeEpochV1 {
        self.epoch
    }

    /// Return the monotone publication cursor within [`Self::epoch`].
    #[must_use]
    pub const fn cursor(self) -> RingCursorV1 {
        self.cursor
    }

    /// Return the already-durable journal append named by this token.
    #[must_use]
    pub const fn journal_sequence(self) -> JournalSequenceV1 {
        self.journal_sequence
    }

    /// Project this token into the caller-owned cursor after it is consumed.
    ///
    /// This permits partial batch progress without registering mutable
    /// consumer state in [`WakeTokenRing`].
    #[must_use]
    pub const fn consumer_cursor(self) -> WakeConsumerCursor {
        WakeConsumerCursor::from_token(self)
    }
}

/// Caller-held position of one independent wake-ring consumer.
///
/// A cursor is reconstructible bookkeeping, not durable consumer state. The
/// journal coordinate records the last append observed by that consumer so a
/// stale epoch or overwritten token can name the exact replay interval. The
/// type has no public constructor: callers obtain honest coordinates from
/// [`WakeTokenRing::starting_cursor`] or [`WakeBatch::advanced_cursor`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WakeConsumerCursor {
    epoch: WakeEpochV1,
    cursor: RingCursorV1,
    journal_sequence: Option<JournalSequenceV1>,
}

impl WakeConsumerCursor {
    /// Return the process epoch in which this cursor was issued.
    #[must_use]
    pub const fn epoch(self) -> WakeEpochV1 {
        self.epoch
    }

    /// Return the last consumed process-local ring cursor.
    #[must_use]
    pub const fn cursor(self) -> RingCursorV1 {
        self.cursor
    }

    /// Return the last durable journal sequence observed by this consumer.
    #[must_use]
    pub const fn journal_sequence(self) -> Option<JournalSequenceV1> {
        self.journal_sequence
    }

    const fn from_token(token: WakeToken) -> Self {
        Self {
            epoch: token.epoch,
            cursor: token.cursor,
            journal_sequence: Some(token.journal_sequence),
        }
    }
}

/// Construction of a bounded [`WakeTokenRing`] failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WakeRingBuildError {
    /// A bounded ring must retain at least one token.
    #[error("wake-token ring capacity must be positive")]
    ZeroCapacity,
    /// The requested in-memory token window cannot be reserved.
    #[error("wake-token ring capacity cannot be reserved")]
    CapacityUnavailable,
}

/// Publication after an asserted durable append violated ring succession.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WakePublishError {
    /// The supplied append is already the ring's publication frontier.
    #[error("durable journal sequence {sequence:?} was already published")]
    Duplicate {
        /// Duplicate journal coordinate.
        sequence: JournalSequenceV1,
    },
    /// The supplied append precedes the ring's publication frontier.
    #[error("durable journal sequence {supplied:?} precedes published head {head:?}")]
    OutOfOrder {
        /// Current published journal head.
        head: JournalSequenceV1,
        /// Older supplied journal coordinate.
        supplied: JournalSequenceV1,
    },
    /// One or more durable journal coordinates were skipped.
    #[error("wake publication expected {expected:?}, received {supplied:?}")]
    Gap {
        /// Exact journal successor required next.
        expected: JournalSequenceV1,
        /// Non-adjacent supplied journal coordinate.
        supplied: JournalSequenceV1,
    },
    /// The durable journal coordinate has no portable successor.
    #[error("durable journal sequence space is exhausted at {sequence:?}")]
    JournalSequenceExhausted {
        /// Terminal durable journal coordinate.
        sequence: JournalSequenceV1,
    },
    /// The process-local cursor has no representable successor.
    #[error("wake-ring cursor space is exhausted at {cursor:?}")]
    CursorExhausted {
        /// Terminal process-local cursor.
        cursor: RingCursorV1,
    },
}

/// An immutable consumer read cannot be served from the current ring window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WakeReadError {
    /// Restart invalidated a cursor from a prior process epoch.
    #[error(
        "wake cursor epoch {cursor_epoch:?} is stale; current epoch is {ring_epoch:?}; durable replay is required"
    )]
    StaleEpoch {
        /// Epoch carried by the caller-held cursor.
        cursor_epoch: WakeEpochV1,
        /// Epoch owned by the current process-local ring.
        ring_epoch: WakeEpochV1,
        /// Last journal sequence already observed by the consumer.
        replay_after: Option<JournalSequenceV1>,
        /// Current journal sequence published through this ring.
        durable_through: Option<JournalSequenceV1>,
        /// Cursor at which to resume after replay reaches `durable_through`.
        resume_at: WakeConsumerCursor,
    },
    /// Ring overflow removed at least one unread token.
    #[error("wake tokens were overwritten; durable replay is required")]
    DurableReplayRequired {
        /// Last journal sequence already observed by the consumer.
        replay_after: Option<JournalSequenceV1>,
        /// Current journal sequence published through this ring.
        durable_through: Option<JournalSequenceV1>,
        /// Cursor at which to resume after replay reaches `durable_through`.
        resume_at: WakeConsumerCursor,
    },
    /// The supplied cursor is ahead of this ring's publication frontier.
    #[error("wake cursor {supplied:?} is ahead of current cursor {current:?}")]
    CursorAhead {
        /// Caller-supplied process-local cursor.
        supplied: RingCursorV1,
        /// Current ring publication cursor.
        current: RingCursorV1,
    },
    /// Cursor and journal coordinates do not describe the same ring position.
    #[error("wake cursor does not match its journal coordinate")]
    CoordinateMismatch {
        /// Journal coordinate carried by the caller.
        supplied: Option<JournalSequenceV1>,
        /// Journal coordinate recorded at the supplied ring cursor.
        expected: Option<JournalSequenceV1>,
    },
}

/// Allocation-free view of unread tokens for one consumer.
///
/// Reading does not mutate the ring or any other consumer. After processing
/// every item from [`Self::iter`], the caller advances only its own cursor to
/// [`Self::advanced_cursor`].
#[derive(Debug, Clone, Copy)]
pub struct WakeBatch<'a> {
    tokens: &'a VecDeque<WakeToken>,
    offset: usize,
    advanced_cursor: WakeConsumerCursor,
}

impl<'a> WakeBatch<'a> {
    /// Iterate over unread tokens in exact publication order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &'a WakeToken> + DoubleEndedIterator + 'a {
        self.tokens.iter().skip(self.offset)
    }

    /// Return the number of unread tokens in this view.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tokens.len() - self.offset
    }

    /// Report whether the consumer is already caught up.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return the caller-owned cursor after all returned tokens are consumed.
    #[must_use]
    pub const fn advanced_cursor(&self) -> WakeConsumerCursor {
        self.advanced_cursor
    }
}

/// One bounded, process-local wake-token ring for a durable journal writer.
///
/// Let `j` and `c` be the explicit starting journal and ring frontiers. Every
/// successful publication and every retained window obey:
///
/// [`WakeToken`] owns the rendered coordinate, successor, and retained-window
/// laws used here.
///
/// Here `k` is the fixed capacity and `n` is the number of tokens published.
/// A consumer before that suffix receives
/// [`WakeReadError::DurableReplayRequired`]; the ring never fills the gap from
/// cached bodies because it owns none.
///
/// The type is deliberately single-writer (`publish_after_durable_append`
/// requires `&mut self`) and lock-free by construction. Consumer reads require
/// only `&self`, retain no registration, and cannot block or erase one another.
#[derive(Debug)]
pub struct WakeTokenRing {
    capacity: usize,
    epoch: WakeEpochV1,
    origin: WakeConsumerCursor,
    tail: WakeConsumerCursor,
    tokens: VecDeque<WakeToken>,
}

impl WakeTokenRing {
    /// Construct an empty ring at explicit process-local and durable frontiers.
    ///
    /// `starting_cursor` permits exact reconstruction and makes cursor-space
    /// exhaustion testable. `durable_frontier` is the journal append already
    /// durable before this ring begins; `None` denotes an empty journal. The
    /// ring does not verify either assertion against storage.
    ///
    /// # Errors
    ///
    /// Rejects zero capacity and allocation requests that cannot be reserved.
    pub fn new(
        capacity: usize,
        epoch: WakeEpochV1,
        starting_cursor: RingCursorV1,
        durable_frontier: Option<JournalSequenceV1>,
    ) -> Result<Self, WakeRingBuildError> {
        if capacity == 0 {
            return Err(WakeRingBuildError::ZeroCapacity);
        }
        let mut tokens = VecDeque::new();
        tokens
            .try_reserve_exact(capacity)
            .map_err(|_| WakeRingBuildError::CapacityUnavailable)?;
        let origin = WakeConsumerCursor {
            epoch,
            cursor: starting_cursor,
            journal_sequence: durable_frontier,
        };
        Ok(Self {
            capacity,
            epoch,
            origin,
            tail: origin,
            tokens,
        })
    }

    /// Return the fixed token capacity selected at construction.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Return the process-local epoch that fences all cursors in this ring.
    #[must_use]
    pub const fn epoch(&self) -> WakeEpochV1 {
        self.epoch
    }

    /// Return the cursor from which a consumer begins at ring construction.
    #[must_use]
    pub const fn starting_cursor(&self) -> WakeConsumerCursor {
        self.origin
    }

    /// Return the latest successfully published wake coordinate.
    #[must_use]
    pub const fn current_cursor(&self) -> WakeConsumerCursor {
        self.tail
    }

    /// Publish one token after its exact journal append is already durable.
    ///
    /// This method checks only succession against the ring's prior publication;
    /// it performs no storage operation and cannot prove durability. The caller
    /// must invoke it only after the named append has committed successfully.
    /// When capacity is reached, the oldest token is overwritten without
    /// changing any caller-held cursor.
    ///
    /// # Errors
    ///
    /// Rejects duplicate, older, or non-adjacent journal coordinates and
    /// exhausted journal or ring-cursor domains. Failure leaves the ring
    /// unchanged.
    pub fn publish_after_durable_append(
        &mut self,
        journal_sequence: JournalSequenceV1,
    ) -> Result<WakeToken, WakePublishError> {
        let expected = expected_journal_successor(self.tail.journal_sequence, journal_sequence)?;
        debug_assert_eq!(expected, journal_sequence);
        let cursor = self
            .tail
            .cursor
            .get()
            .checked_add(1)
            .map(RingCursorV1::new)
            .ok_or(WakePublishError::CursorExhausted {
                cursor: self.tail.cursor,
            })?;
        let token = WakeToken {
            epoch: self.epoch,
            cursor,
            journal_sequence,
        };
        if self.tokens.len() == self.capacity {
            self.tokens.pop_front();
        }
        self.tokens.push_back(token);
        self.tail = WakeConsumerCursor::from_token(token);
        Ok(token)
    }

    /// Read every token after one caller-held cursor without mutating the ring.
    ///
    /// A successful empty batch means the consumer is caught up. A stale epoch
    /// or overwritten cursor names the durable replay interval and the cursor
    /// at which the caller may resume after replay. No consumer registration or
    /// identity is stored by the ring.
    ///
    /// # Errors
    ///
    /// Returns a typed error for stale epochs, overwritten tokens, cursors ahead
    /// of the ring, or an inconsistent ring/journal coordinate pair.
    pub fn read_after(&self, cursor: WakeConsumerCursor) -> Result<WakeBatch<'_>, WakeReadError> {
        if cursor.epoch != self.epoch {
            return Err(WakeReadError::StaleEpoch {
                cursor_epoch: cursor.epoch,
                ring_epoch: self.epoch,
                replay_after: cursor.journal_sequence,
                durable_through: self.tail.journal_sequence,
                resume_at: self.tail,
            });
        }
        if cursor.cursor > self.tail.cursor {
            return Err(WakeReadError::CursorAhead {
                supplied: cursor.cursor,
                current: self.tail.cursor,
            });
        }
        let Some(oldest) = self.tokens.front().copied() else {
            validate_coordinate(cursor, self.origin)?;
            return Ok(WakeBatch {
                tokens: &self.tokens,
                offset: 0,
                advanced_cursor: self.tail,
            });
        };
        let predecessor_of_oldest = oldest.cursor.get() - 1;
        if cursor.cursor.get() < predecessor_of_oldest {
            return Err(WakeReadError::DurableReplayRequired {
                replay_after: cursor.journal_sequence,
                durable_through: self.tail.journal_sequence,
                resume_at: self.tail,
            });
        }
        let expected = if cursor.cursor.get() == predecessor_of_oldest {
            Some(WakeConsumerCursor {
                epoch: self.epoch,
                cursor: cursor.cursor,
                journal_sequence: oldest
                    .journal_sequence
                    .get()
                    .checked_sub(1)
                    .and_then(|value| (value > 0).then_some(value))
                    .and_then(|value| JournalSequenceV1::new(value).ok()),
            })
        } else {
            let relative = cursor.cursor.get() - oldest.cursor.get();
            usize::try_from(relative)
                .ok()
                .and_then(|index| self.tokens.get(index).copied())
                .map(WakeConsumerCursor::from_token)
        };
        if let Some(expected) = expected {
            validate_coordinate(cursor, expected)?;
        }
        let offset = self
            .tokens
            .iter()
            .take_while(|token| token.cursor <= cursor.cursor)
            .count();
        Ok(WakeBatch {
            tokens: &self.tokens,
            offset,
            advanced_cursor: self.tail,
        })
    }
}

fn expected_journal_successor(
    head: Option<JournalSequenceV1>,
    supplied: JournalSequenceV1,
) -> Result<JournalSequenceV1, WakePublishError> {
    let Some(head) = head else {
        let first = JournalSequenceV1::new(1).expect("one is a valid positive journal sequence");
        return if supplied == first {
            Ok(first)
        } else {
            Err(WakePublishError::Gap {
                expected: first,
                supplied,
            })
        };
    };
    let expected = head
        .checked_successor()
        .ok_or(WakePublishError::JournalSequenceExhausted { sequence: head })?;
    if supplied == head {
        return Err(WakePublishError::Duplicate { sequence: supplied });
    }
    if supplied < head {
        return Err(WakePublishError::OutOfOrder { head, supplied });
    }
    if supplied != expected {
        return Err(WakePublishError::Gap { expected, supplied });
    }
    Ok(expected)
}

fn validate_coordinate(
    supplied: WakeConsumerCursor,
    expected: WakeConsumerCursor,
) -> Result<(), WakeReadError> {
    if supplied.journal_sequence != expected.journal_sequence {
        return Err(WakeReadError::CoordinateMismatch {
            supplied: supplied.journal_sequence,
            expected: expected.journal_sequence,
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/internal/wake_ring.rs"]
mod tests;
