//! Per-message allocation bound for the SDK's stdio transport.

use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, ReadBuf};

/// Reject a JSON-RPC line before the SDK's growable line buffer can exceed the
/// same outer boundary used by direct native-action input.
///
/// The newline delimiter does not consume the limit. Once a line crosses the
/// boundary, subsequent reads fail rather than resynchronizing on attacker-
/// controlled input.
pub(super) struct BoundedLines<R> {
    inner: R,
    current_len: usize,
    max_len: usize,
    rejected: bool,
}

impl<R> BoundedLines<R> {
    /// Wrap a transport and reject any single line above `max_len` bytes.
    ///
    /// A zero limit therefore admits only an empty line.
    pub(super) fn new(inner: R, max_len: usize) -> Self {
        Self {
            inner,
            current_len: 0,
            max_len,
            rejected: false,
        }
    }
}

impl<R: AsyncRead + Unpin> AsyncRead for BoundedLines<R> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.rejected {
            return Poll::Ready(Err(message_too_large()));
        }
        if buffer.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }

        let mut staging = [0_u8; 8 * 1024];
        let capacity = buffer.remaining().min(staging.len());
        let mut staged = ReadBuf::new(&mut staging[..capacity]);
        match Pin::new(&mut self.inner).poll_read(context, &mut staged) {
            Poll::Ready(Ok(())) => {
                let mut admitted = 0;
                for byte in staged.filled() {
                    if *byte == b'\n' {
                        self.current_len = 0;
                    } else {
                        self.current_len = self.current_len.saturating_add(1);
                        if self.current_len > self.max_len {
                            self.rejected = true;
                            break;
                        }
                    }
                    admitted += 1;
                }
                if admitted != 0 {
                    buffer.put_slice(&staged.filled()[..admitted]);
                    Poll::Ready(Ok(()))
                } else if self.rejected {
                    Poll::Ready(Err(message_too_large()))
                } else {
                    Poll::Ready(Ok(()))
                }
            }
            other => other,
        }
    }
}

/// Construct the stable terminal transport error after line admission closes.
fn message_too_large() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "MCP message exceeds the bounded input size",
    )
}

#[cfg(test)]
#[path = "../tests/internal/transport.rs"]
mod tests;
