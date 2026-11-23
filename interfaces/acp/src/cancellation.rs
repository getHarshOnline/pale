//! Cooperative cancellation signal for one ACP prompt turn.

use std::future::poll_fn;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures::task::AtomicWaker;

/// Cloneable cooperative cancellation signal for one [`crate::AcpTurnRequest`].
///
/// Cancellation is monotone: once requested, it cannot be withdrawn. The ACP
/// client drops a pre-prompt connection without fabricating a native session
/// cancellation. Once the prompt starts, it forwards `session/cancel` and
/// still waits for the agent's terminal stop reason. Requesting cancellation
/// does not itself prove that the turn stopped or that any external effect
/// settled.
#[derive(Clone, Debug, Default)]
pub struct AcpCancellation {
    inner: Arc<CancellationState>,
}

#[derive(Debug, Default)]
struct CancellationState {
    requested: AtomicBool,
    waker: AtomicWaker,
}

impl AcpCancellation {
    /// Create an unset cancellation signal.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Request cooperative cancellation and wake the active turn, if any.
    pub fn cancel(&self) {
        self.inner.requested.store(true, Ordering::Release);
        self.inner.waker.wake();
    }

    /// Report whether cancellation has been requested.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.inner.requested.load(Ordering::Acquire)
    }

    pub(crate) async fn cancelled(&self) {
        poll_fn(|context| {
            if self.is_cancelled() {
                return std::task::Poll::Ready(());
            }
            self.inner.waker.register(context.waker());
            if self.is_cancelled() {
                std::task::Poll::Ready(())
            } else {
                std::task::Poll::Pending
            }
        })
        .await;
    }
}
