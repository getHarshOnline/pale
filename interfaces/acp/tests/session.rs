//! Behavioral conformance for the stable-v1 ACP session boundary.

mod support;

use std::io;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

use agent_client_protocol::schema::v1::{
    Plan, PlanEntry, PlanEntryPriority, PlanEntryStatus, RequestPermissionOutcome, SessionUpdate,
    StopReason, ToolCall, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields, ToolKind,
};
use agent_client_protocol::schema::ProtocolVersion;
use futures::channel::oneshot;
use futures::future::{pending, ready};
use futures::io::Cursor;
use futures::{AsyncRead, AsyncWrite};
use pale_acp::{
    AcpCancellation, AcpConductor, AcpErrorKind, AcpSessionEvent, AcpSessionId, AcpSessionState,
    AcpStopReason, AcpStructuredUpdate, AcpTurnRequest, AcpUpdateKind,
    ACP_INCOMING_FRAME_BYTE_LIMIT, ACP_TURN_INBOUND_BYTE_LIMIT, ACP_TURN_INBOUND_EVENT_LIMIT,
    ACP_TURN_UPDATE_LIMIT, ACP_UPDATE_TEXT_BYTE_LIMIT,
};
use pale_sdk::DiagnosticStage;

use support::{spawn_agent, AgentPhase, Scenario, SESSION_ID};

fn request() -> AcpTurnRequest {
    AcpTurnRequest::new(
        PathBuf::from("/synthetic/workspace"),
        "Inspect this profile".to_owned(),
    )
}

#[test]
fn debug_projections_do_not_disclose_prompt_path_or_streamed_text() {
    let sensitive_request = AcpTurnRequest::new(
        PathBuf::from("/synthetic/private-workspace"),
        "sensitive operator instruction".to_owned(),
    );
    let request_debug = format!("{sensitive_request:?}");
    assert!(!request_debug.contains("private-workspace"));
    assert!(!request_debug.contains("sensitive operator instruction"));
    assert!(request_debug.contains("cwd_component_count"));
    assert!(request_debug.contains("prompt_byte_len"));

    let conductor = AcpConductor::new().expect("Pale diagnostics must initialize");
    let (outgoing, incoming, _probe, thread) = spawn_agent(Scenario::default());
    let mut update_debug = Vec::new();
    futures::executor::block_on(conductor.conduct_turn(
        outgoing,
        incoming,
        request(),
        AcpCancellation::new(),
        pending(),
        |update| update_debug.push(format!("{update:?}")),
    ))
    .expect("stable v1 turn must complete");
    thread.join().expect("synthetic agent must not panic");

    assert!(update_debug.iter().all(|debug| !debug.contains("first")));
    assert!(update_debug.iter().all(|debug| !debug.contains("second")));
    assert!(update_debug
        .iter()
        .all(|debug| debug.contains("text_byte_len")));
}

#[test]
fn stable_v1_stdio_handshake_preserves_order_and_omits_native_capabilities() {
    let conductor = AcpConductor::new().expect("Pale diagnostics must initialize");
    let (outgoing, incoming, probe, thread) = spawn_agent(Scenario::default());
    let mut updates = Vec::new();
    let outcome = futures::executor::block_on(conductor.conduct_turn(
        outgoing,
        incoming,
        request(),
        AcpCancellation::new(),
        pending(),
        |update| updates.push((update.sequence(), update.text().map(str::to_owned))),
    ))
    .expect("stable v1 turn must complete");
    thread.join().expect("synthetic agent must not panic");

    assert_eq!(outcome.session_id().as_str(), SESSION_ID);
    assert_eq!(outcome.stop_reason(), AcpStopReason::EndTurn);
    assert_eq!(outcome.update_count(), 2);
    assert_eq!(
        updates,
        vec![
            (1, Some("first".to_owned())),
            (2, Some("second".to_owned()))
        ]
    );

    let capabilities =
        serde_json::to_value(probe.capabilities()).expect("client capabilities must serialize");
    assert_eq!(capabilities["terminal"], false);
    assert_eq!(capabilities["fs"]["readTextFile"], false);
    assert_eq!(capabilities["fs"]["writeTextFile"], false);
    assert!(capabilities.get("session").is_none());
    assert!(capabilities.get("mcpCapabilities").is_none());
}

#[test]
fn stable_v1_structured_updates_preserve_exact_native_payloads() {
    let tool_call = ToolCall::new("tool-1", "Inspect effective profile")
        .kind(ToolKind::Read)
        .status(ToolCallStatus::InProgress)
        .raw_input(serde_json::json!({"profile": "release"}));
    let tool_call_update = ToolCallUpdate::new(
        "tool-1",
        ToolCallUpdateFields::new()
            .status(ToolCallStatus::Completed)
            .title("Effective profile inspected".to_owned())
            .raw_output(serde_json::json!({"findingCount": 2})),
    );
    let plan = Plan::new(vec![PlanEntry::new(
        "Review the admitted profile",
        PlanEntryPriority::High,
        PlanEntryStatus::InProgress,
    )]);
    let expected_tool_call = tool_call.clone();
    let expected_tool_call_update = tool_call_update.clone();
    let expected_plan = plan.clone();
    let (outgoing, incoming, _probe, thread) = spawn_agent(Scenario {
        updates: Vec::new(),
        structured_updates: vec![
            SessionUpdate::ToolCall(tool_call),
            SessionUpdate::ToolCallUpdate(tool_call_update),
            SessionUpdate::Plan(plan),
        ],
        ..Scenario::default()
    });
    let mut updates = Vec::new();
    let outcome = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                outgoing,
                incoming,
                request(),
                AcpCancellation::new(),
                pending(),
                |update| updates.push(update),
            ),
    )
    .expect("stable v1 structured updates must complete");
    thread.join().expect("synthetic agent must not panic");

    assert_eq!(outcome.update_count(), 3);
    assert_eq!(updates.len(), 3);
    assert_eq!(updates[0].sequence(), 1);
    assert_eq!(updates[0].kind(), AcpUpdateKind::ToolCall);
    assert_eq!(updates[0].text(), None);
    assert!(
        matches!(updates[0].structured(), Some(AcpStructuredUpdate::ToolCall(value)) if **value == expected_tool_call)
    );
    assert_eq!(updates[1].sequence(), 2);
    assert_eq!(updates[1].kind(), AcpUpdateKind::ToolCallUpdate);
    assert!(
        matches!(updates[1].structured(), Some(AcpStructuredUpdate::ToolCallUpdate(value)) if **value == expected_tool_call_update)
    );
    assert_eq!(updates[2].sequence(), 3);
    assert_eq!(updates[2].kind(), AcpUpdateKind::Plan);
    assert!(
        matches!(updates[2].structured(), Some(AcpStructuredUpdate::Plan(value)) if *value == expected_plan)
    );

    let debug = format!("{:?}", updates[0]);
    let structured_debug = format!("{:?}", updates[0].structured());
    assert!(!debug.contains("Inspect effective profile"));
    assert!(!debug.contains("release"));
    assert!(!structured_debug.contains("Inspect effective profile"));
    assert!(!structured_debug.contains("release"));
    assert!(debug.contains("has_structured_payload"));
}

#[test]
fn non_v1_negotiation_is_rejected_before_session_creation() {
    let version = serde_json::from_value::<ProtocolVersion>(serde_json::json!(2))
        .expect("synthetic foreign version must decode");
    let (outgoing, incoming, _probe, thread) = spawn_agent(Scenario {
        protocol_version: version,
        ..Scenario::default()
    });
    let error = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                outgoing,
                incoming,
                request(),
                AcpCancellation::new(),
                pending(),
                |_| {},
            ),
    )
    .expect_err("foreign ACP version must fail closed");
    thread.join().expect("synthetic agent must not panic");

    assert_eq!(error.kind(), AcpErrorKind::UnsupportedProtocolVersion);
    assert_eq!(error.state(), AcpSessionState::Disconnected);
    assert_eq!(
        error.diagnostic().code(),
        "acp_unsupported_protocol_version"
    );
}

#[test]
fn permission_requests_select_reject_once_without_authority() {
    let (outgoing, incoming, probe, thread) = spawn_agent(Scenario {
        permission_requests: 1,
        ..Scenario::default()
    });
    let outcome = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                outgoing,
                incoming,
                request(),
                AcpCancellation::new(),
                pending(),
                |_| {},
            ),
    )
    .expect("permission denial must not break the ACP turn");
    thread.join().expect("synthetic agent must not panic");

    assert_eq!(outcome.stop_reason(), AcpStopReason::EndTurn);
    assert!(
        matches!(probe.permission(), RequestPermissionOutcome::Selected(selected) if selected.option_id.0.as_ref() == "reject")
    );
}

#[test]
fn permission_requests_without_a_rejection_option_are_cancelled() {
    let (outgoing, incoming, probe, thread) = spawn_agent(Scenario {
        permission_requests: 1,
        include_reject_permission: false,
        ..Scenario::default()
    });
    let outcome = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                outgoing,
                incoming,
                request(),
                AcpCancellation::new(),
                pending(),
                |_| {},
            ),
    )
    .expect("permission cancellation must not break the ACP turn");
    thread.join().expect("synthetic agent must not panic");

    assert_eq!(outcome.stop_reason(), AcpStopReason::EndTurn);
    assert_eq!(probe.permission(), RequestPermissionOutcome::Cancelled);
}

#[test]
fn caller_cancellation_is_forwarded_and_awaits_terminal_confirmation() {
    let cancellation = AcpCancellation::new();
    let observer_cancellation = cancellation.clone();
    let (outgoing, incoming, probe, thread) = spawn_agent(Scenario {
        stop_reason: StopReason::Cancelled,
        ..Scenario::default()
    });
    let outcome = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                outgoing,
                incoming,
                request(),
                cancellation,
                pending(),
                move |update| {
                    if update.sequence() == 1 {
                        observer_cancellation.cancel();
                    }
                },
            ),
    )
    .expect("agent must confirm cancellation");
    thread.join().expect("synthetic agent must not panic");

    assert!(outcome.cancellation_requested());
    assert_eq!(outcome.stop_reason(), AcpStopReason::Cancelled);
    assert_eq!(probe.cancellation_count(), 1);
}

#[test]
fn pre_cancelled_turn_never_touches_the_supplied_connection() {
    let cancellation = AcpCancellation::new();
    cancellation.cancel();
    let error = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                Cursor::new(Vec::<u8>::new()),
                Cursor::new(Vec::<u8>::new()),
                request(),
                cancellation,
                pending(),
                |_| {},
            ),
    )
    .expect_err("pre-cancelled turn must fail before negotiation");
    assert_eq!(error.kind(), AcpErrorKind::CancelledBeforePrompt);
    assert_eq!(error.state(), AcpSessionState::Disconnected);
}

#[test]
fn cancellation_during_initialize_drops_the_pre_session_connection() {
    let cancellation = AcpCancellation::new();
    let cancellation_request = cancellation.clone();
    let (outgoing, incoming, probe, agent_thread) = spawn_agent(Scenario {
        block_initialize: true,
        ..Scenario::default()
    });
    let phase_probe = probe.clone();
    let canceller = std::thread::spawn(move || {
        phase_probe.wait_for_phase(AgentPhase::Initialize);
        cancellation_request.cancel();
    });

    let error = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                outgoing,
                incoming,
                request(),
                cancellation,
                pending(),
                |_| {},
            ),
    )
    .expect_err("initialize cancellation must not wait for the silent peer");
    canceller.join().expect("canceller must not panic");
    probe.release_blocked_request();
    agent_thread.join().expect("synthetic agent must not panic");

    assert_eq!(error.kind(), AcpErrorKind::CancelledBeforePrompt);
    assert_eq!(error.state(), AcpSessionState::Disconnected);
    assert_eq!(probe.cancellation_count(), 0);
}

#[test]
fn cancellation_during_session_creation_drops_without_session_cancel() {
    let cancellation = AcpCancellation::new();
    let cancellation_request = cancellation.clone();
    let (outgoing, incoming, probe, agent_thread) = spawn_agent(Scenario {
        block_new_session: true,
        ..Scenario::default()
    });
    let phase_probe = probe.clone();
    let canceller = std::thread::spawn(move || {
        phase_probe.wait_for_phase(AgentPhase::NewSession);
        cancellation_request.cancel();
    });

    let error = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                outgoing,
                incoming,
                request(),
                cancellation,
                pending(),
                |_| {},
            ),
    )
    .expect_err("session-creation cancellation must not wait for the silent peer");
    canceller.join().expect("canceller must not panic");
    probe.release_blocked_request();
    agent_thread.join().expect("synthetic agent must not panic");

    assert_eq!(error.kind(), AcpErrorKind::CancelledBeforePrompt);
    assert_eq!(error.state(), AcpSessionState::InitializedV1);
    assert_eq!(probe.cancellation_count(), 0);
}

#[test]
fn update_count_is_bounded_and_excess_fails_closed() {
    let updates = (0..=ACP_TURN_UPDATE_LIMIT)
        .map(|index| format!("update-{index}"))
        .collect();
    let (outgoing, incoming, _probe, thread) = spawn_agent(Scenario {
        updates,
        ..Scenario::default()
    });
    let error = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                outgoing,
                incoming,
                request(),
                AcpCancellation::new(),
                pending(),
                |_| {},
            ),
    )
    .expect_err("excess ACP updates must fail closed");
    thread.join().expect("synthetic agent must not panic");
    assert_eq!(error.kind(), AcpErrorKind::UpdateLimitExceeded);
    assert_eq!(error.state(), AcpSessionState::PromptActive);
    assert_eq!(
        error.session_id().map(AcpSessionId::as_str),
        Some(SESSION_ID)
    );
}

#[test]
fn aggregate_inbound_event_limit_includes_permission_traffic() {
    let (outgoing, incoming, _probe, thread) = spawn_agent(Scenario {
        permission_requests: ACP_TURN_INBOUND_EVENT_LIMIT,
        updates: Vec::new(),
        ..Scenario::default()
    });
    let error = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                outgoing,
                incoming,
                request(),
                AcpCancellation::new(),
                pending(),
                |_| {},
            ),
    )
    .expect_err("permission traffic must share the aggregate inbound event ceiling");
    thread.join().expect("synthetic agent must not panic");

    assert_eq!(error.kind(), AcpErrorKind::InboundEventLimitExceeded);
    assert_eq!(error.state(), AcpSessionState::TransportLost);
    assert_eq!(
        error.session_id().map(AcpSessionId::as_str),
        Some(SESSION_ID)
    );
}

#[test]
fn aggregate_inbound_event_limit_counts_json_rpc_batch_members() {
    let event = r#"{"jsonrpc":"2.0","method":"session/unknown"}"#;
    let event_count =
        usize::try_from(ACP_TURN_INBOUND_EVENT_LIMIT).expect("test event limit fits the host") + 1;
    let incoming = Cursor::new(format!("[{}]\n", vec![event; event_count].join(",")).into_bytes());
    let error = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                Cursor::new(Vec::<u8>::new()),
                incoming,
                request(),
                AcpCancellation::new(),
                pending(),
                |_| {},
            ),
    )
    .expect_err("one JSON-RPC batch cannot hide excess inbound events");

    assert_eq!(error.kind(), AcpErrorKind::InboundEventLimitExceeded);
    assert_eq!(error.state(), AcpSessionState::TransportLost);
    assert!(error.session_id().is_none());
}

#[test]
fn aggregate_inbound_byte_limit_includes_permission_traffic() {
    let permission_bytes = ACP_INCOMING_FRAME_BYTE_LIMIT / 2;
    let permission_requests = u32::try_from(ACP_TURN_INBOUND_BYTE_LIMIT / permission_bytes + 2)
        .expect("test request count fits the protocol counter");
    let (outgoing, incoming, _probe, thread) = spawn_agent(Scenario {
        permission_requests,
        permission_option_name_bytes: permission_bytes,
        updates: Vec::new(),
        ..Scenario::default()
    });
    let error = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                outgoing,
                incoming,
                request(),
                AcpCancellation::new(),
                pending(),
                |_| {},
            ),
    )
    .expect_err("permission traffic must share the aggregate inbound byte ceiling");
    thread.join().expect("synthetic agent must not panic");

    assert_eq!(error.kind(), AcpErrorKind::InboundByteLimitExceeded);
    assert_eq!(error.state(), AcpSessionState::TransportLost);
    assert_eq!(
        error.session_id().map(AcpSessionId::as_str),
        Some(SESSION_ID)
    );
}

#[test]
fn update_text_bytes_are_bounded_independently_of_update_count() {
    let (outgoing, incoming, _probe, thread) = spawn_agent(Scenario {
        updates: vec!["x".repeat(ACP_UPDATE_TEXT_BYTE_LIMIT + 1)],
        ..Scenario::default()
    });
    let error = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                outgoing,
                incoming,
                request(),
                AcpCancellation::new(),
                pending(),
                |_| {},
            ),
    )
    .expect_err("an oversized text chunk must fail independently of update count");
    thread.join().expect("synthetic agent must not panic");
    assert_eq!(error.kind(), AcpErrorKind::UpdateLimitExceeded);
    assert_eq!(error.state(), AcpSessionState::PromptActive);
}

#[test]
fn unmatched_stable_v1_session_message_fails_closed() {
    let (outgoing, incoming, _probe, thread) = spawn_agent(Scenario {
        updates: Vec::new(),
        unknown_session_message: true,
        ..Scenario::default()
    });
    let error = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                outgoing,
                incoming,
                request(),
                AcpCancellation::new(),
                pending(),
                |_| {},
            ),
    )
    .expect_err("an unmatched session message must fail closed");
    thread.join().expect("synthetic agent must not panic");
    assert_eq!(error.kind(), AcpErrorKind::UnsupportedStableV1Message);
    assert_eq!(error.state(), AcpSessionState::PromptActive);
    assert_eq!(
        error.session_id().map(AcpSessionId::as_str),
        Some(SESSION_ID)
    );
}

#[test]
fn lifecycle_is_closed_monotone_and_terminal() {
    let state = AcpSessionState::Disconnected
        .advance(AcpSessionEvent::ProtocolV1Accepted)
        .and_then(|state| state.advance(AcpSessionEvent::SessionOpened))
        .and_then(|state| state.advance(AcpSessionEvent::PromptStarted))
        .and_then(|state| state.advance(AcpSessionEvent::CancellationRequested))
        .and_then(|state| state.advance(AcpSessionEvent::TurnStopped))
        .expect("lawful lifecycle must advance");
    assert_eq!(state, AcpSessionState::TurnEnded);
    let error = state
        .advance(AcpSessionEvent::PromptStarted)
        .expect_err("terminal state must not reopen");
    assert_eq!(error.state(), AcpSessionState::TurnEnded);
    assert_eq!(error.event(), AcpSessionEvent::PromptStarted);
    let error = AcpSessionState::TransportLost
        .advance(AcpSessionEvent::TransportFailed)
        .expect_err("transport loss must remain terminal");
    assert_eq!(error.state(), AcpSessionState::TransportLost);
    assert_eq!(error.event(), AcpSessionEvent::TransportFailed);
}

#[test]
fn native_session_identity_and_stop_reason_do_not_substitute_for_settlement() {
    fn native_identity(identity: &AcpSessionId) -> &str {
        identity.as_str()
    }

    let (outgoing, incoming, _probe, thread) = spawn_agent(Scenario {
        stop_reason: StopReason::Refusal,
        updates: Vec::new(),
        ..Scenario::default()
    });
    let outcome = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                outgoing,
                incoming,
                request(),
                AcpCancellation::new(),
                pending(),
                |_| {},
            ),
    )
    .expect("refusal is a terminal ACP outcome, not a transport error");
    thread.join().expect("synthetic agent must not panic");

    assert_eq!(native_identity(outcome.session_id()), SESSION_ID);
    assert_eq!(outcome.stop_reason(), AcpStopReason::Refusal);
}

#[test]
fn caller_deadline_bounds_a_silent_peer_without_an_async_runtime() {
    let error = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                SinkWriter,
                PendingReader,
                request(),
                AcpCancellation::new(),
                ready(()),
                |_| {},
            ),
    )
    .expect_err("an elapsed caller deadline must stop a silent peer");

    assert_eq!(error.kind(), AcpErrorKind::DeadlineExceeded);
    assert_eq!(error.state(), AcpSessionState::TransportLost);
    assert_eq!(error.diagnostic().code(), "acp_deadline_exceeded");
    assert_eq!(error.diagnostic().stage(), DiagnosticStage::HostObservation);
    assert!(error.session_id().is_none());
}

#[test]
fn deadline_after_session_creation_preserves_the_native_correlation() {
    let (deadline_sender, deadline) = oneshot::channel();
    let (outgoing, incoming, probe, agent_thread) = spawn_agent(Scenario {
        updates: Vec::new(),
        block_prompt: true,
        ..Scenario::default()
    });
    let phase_probe = probe.clone();
    let timer = std::thread::spawn(move || {
        phase_probe.wait_for_phase(AgentPhase::Prompt);
        let _elapsed = deadline_sender.send(());
    });

    let error = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                outgoing,
                incoming,
                request(),
                AcpCancellation::new(),
                async move {
                    let _elapsed = deadline.await;
                },
                |_| {},
            ),
    )
    .expect_err("deadline must abandon the established native session");
    timer.join().expect("deadline trigger must not panic");
    probe.release_blocked_request();
    agent_thread.join().expect("synthetic agent must not panic");

    assert_eq!(error.kind(), AcpErrorKind::DeadlineExceeded);
    assert_eq!(error.state(), AcpSessionState::TransportLost);
    assert_eq!(
        error.session_id().map(AcpSessionId::as_str),
        Some(SESSION_ID)
    );
}

#[test]
fn incoming_json_rpc_frame_is_bounded_before_sdk_parsing() {
    let incoming = Cursor::new(vec![b' '; ACP_INCOMING_FRAME_BYTE_LIMIT + 1]);
    let error = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                Cursor::new(Vec::<u8>::new()),
                incoming,
                request(),
                AcpCancellation::new(),
                pending(),
                |_| {},
            ),
    )
    .expect_err("an oversized frame must fail before JSON parsing");

    assert_eq!(error.kind(), AcpErrorKind::FrameLimitExceeded);
    assert_eq!(error.state(), AcpSessionState::TransportLost);
    assert!(error.session_id().is_none());
    assert_eq!(error.diagnostic().code(), "acp_frame_limit_exceeded");
    assert_eq!(error.diagnostic().stage(), DiagnosticStage::InputAdmission);
}

#[test]
fn non_text_content_is_rejected_instead_of_silently_erased() {
    let (outgoing, incoming, _probe, thread) = spawn_agent(Scenario {
        non_text_update: true,
        ..Scenario::default()
    });
    let error = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                outgoing,
                incoming,
                request(),
                AcpCancellation::new(),
                pending(),
                |_| {},
            ),
    )
    .expect_err("unsupported content must fail closed");
    thread.join().expect("synthetic agent must not panic");

    assert_eq!(error.kind(), AcpErrorKind::UnsupportedStableV1Message);
    assert_eq!(error.state(), AcpSessionState::PromptActive);
    assert_eq!(
        error.session_id().map(AcpSessionId::as_str),
        Some(SESSION_ID)
    );
    assert_eq!(error.diagnostic().stage(), DiagnosticStage::Internal);
}

#[test]
fn carrier_termination_is_distinct_from_deadline_expiry() {
    let error = futures::executor::block_on(
        AcpConductor::new()
            .expect("Pale diagnostics must initialize")
            .conduct_turn(
                Cursor::new(Vec::<u8>::new()),
                Cursor::new(Vec::<u8>::new()),
                request(),
                AcpCancellation::new(),
                pending(),
                |_| {},
            ),
    )
    .expect_err("an exhausted carrier cannot reach a terminal response");

    assert_eq!(error.kind(), AcpErrorKind::Transport);
    assert_eq!(error.state(), AcpSessionState::TransportLost);
    assert_eq!(error.diagnostic().code(), "acp_transport_failure");
    assert_eq!(error.diagnostic().stage(), DiagnosticStage::HostObservation);
}

#[derive(Debug)]
struct PendingReader;

impl AsyncRead for PendingReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        _buffer: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Pending
    }
}

#[derive(Debug)]
struct SinkWriter;

impl AsyncWrite for SinkWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        buffer: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(Ok(buffer.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
