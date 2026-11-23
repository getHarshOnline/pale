//! Select the initial A2A client methods without importing the wider protocol surface.

/// A2A methods admitted by the initial client profile.
///
/// This closed projection deliberately excludes the broader method inventory
/// exposed by [`a2a::methods`]. It selects only initial request, bounded stream,
/// task read-back, and cancellation-request mechanics; method availability
/// does not authorize a transport call.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum A2aMethod {
    /// Submit one request whose response may identify a remote task or message.
    SendMessage,
    /// Submit one request and consume a bounded advertised Server-Sent Event stream.
    SendStreamingMessage,
    /// Read one already identified remote task.
    GetTask,
    /// Request cancellation of one already identified remote task.
    CancelTask,
}

impl A2aMethod {
    /// Return the exact upstream JSON-RPC method spelling.
    #[must_use]
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::SendMessage => a2a::methods::SEND_MESSAGE,
            Self::SendStreamingMessage => a2a::methods::SEND_STREAMING_MESSAGE,
            Self::GetTask => a2a::methods::GET_TASK,
            Self::CancelTask => a2a::methods::CANCEL_TASK,
        }
    }
}
