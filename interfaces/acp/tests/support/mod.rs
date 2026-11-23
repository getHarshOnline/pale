//! Synthetic official-SDK ACP agent over an in-memory operating-system stream.

use std::os::unix::net::UnixStream;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use agent_client_protocol::schema::v1::{
    CancelNotification, ClientCapabilities, ContentBlock, ContentChunk, ImageContent,
    InitializeRequest, InitializeResponse, NewSessionRequest, NewSessionResponse, PermissionOption,
    PermissionOptionKind, PromptRequest, PromptResponse, RequestPermissionOutcome,
    RequestPermissionRequest, SessionId, SessionNotification, SessionUpdate, StopReason,
    TextContent, ToolCallUpdate, ToolCallUpdateFields,
};
use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::{Agent, ByteStreams, Client, ConnectionTo, Responder, UntypedMessage};
use async_io::Async;
use futures::channel::oneshot;

pub(crate) const SESSION_ID: &str = "native-acp-session";

#[derive(Clone, Debug)]
pub(crate) struct Scenario {
    pub protocol_version: ProtocolVersion,
    pub updates: Vec<String>,
    pub structured_updates: Vec<SessionUpdate>,
    pub stop_reason: StopReason,
    pub permission_requests: u32,
    pub include_reject_permission: bool,
    pub permission_option_name_bytes: usize,
    pub unknown_session_message: bool,
    pub non_text_update: bool,
    pub block_initialize: bool,
    pub block_new_session: bool,
    pub block_prompt: bool,
}

impl Default for Scenario {
    fn default() -> Self {
        Self {
            protocol_version: ProtocolVersion::V1,
            updates: vec!["first".to_owned(), "second".to_owned()],
            structured_updates: Vec::new(),
            stop_reason: StopReason::EndTurn,
            permission_requests: 0,
            include_reject_permission: true,
            permission_option_name_bytes: 0,
            unknown_session_message: false,
            non_text_update: false,
            block_initialize: false,
            block_new_session: false,
            block_prompt: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AgentPhase {
    Initialize,
    NewSession,
    Prompt,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct AgentProbe {
    capabilities: Arc<Mutex<Option<ClientCapabilities>>>,
    permission: Arc<Mutex<Option<RequestPermissionOutcome>>>,
    cancellation_count: Arc<Mutex<u32>>,
    phase: Arc<(Mutex<Option<AgentPhase>>, Condvar)>,
    release: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl AgentProbe {
    pub(crate) fn capabilities(&self) -> ClientCapabilities {
        self.capabilities
            .lock()
            .expect("capability probe must not be poisoned")
            .clone()
            .expect("client must initialize before the turn")
    }

    pub(crate) fn permission(&self) -> RequestPermissionOutcome {
        self.permission
            .lock()
            .expect("permission probe must not be poisoned")
            .clone()
            .expect("agent must request permission in this scenario")
    }

    pub(crate) fn cancellation_count(&self) -> u32 {
        *self
            .cancellation_count
            .lock()
            .expect("cancellation probe must not be poisoned")
    }

    pub(crate) fn wait_for_phase(&self, expected: AgentPhase) {
        let (phase, changed) = self.phase.as_ref();
        let mut observed = phase
            .lock()
            .expect("agent phase probe must not be poisoned");
        while *observed != Some(expected) {
            let (next, timeout) = changed
                .wait_timeout(observed, Duration::from_secs(5))
                .expect("agent phase probe must not be poisoned");
            assert!(
                !timeout.timed_out(),
                "synthetic agent did not reach {expected:?}"
            );
            observed = next;
        }
    }

    pub(crate) fn release_blocked_request(&self) {
        if let Some(release) = self
            .release
            .lock()
            .expect("agent release probe must not be poisoned")
            .take()
        {
            let _released = release.send(());
        }
    }
}

pub(crate) type SyntheticConnection = (
    Async<UnixStream>,
    Async<UnixStream>,
    AgentProbe,
    JoinHandle<()>,
);

pub(crate) fn spawn_agent(scenario: Scenario) -> SyntheticConnection {
    let (client_stream, agent_stream) =
        UnixStream::pair().expect("synthetic ACP stream pair must open");
    let client_writer = client_stream
        .try_clone()
        .expect("client stream must clone for split I/O");
    let agent_writer = agent_stream
        .try_clone()
        .expect("agent stream must clone for split I/O");
    let (release_tx, release_rx) = oneshot::channel();
    let probe = AgentProbe::default();
    *probe
        .release
        .lock()
        .expect("agent release probe must not be poisoned") = Some(release_tx);
    let thread_probe = probe.clone();
    let thread = std::thread::spawn(move || {
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let cancel_sender = Arc::new(Mutex::new(Some(cancel_tx)));
        let cancel_receiver = Arc::new(Mutex::new(Some(cancel_rx)));
        let initialize_probe = Arc::clone(&thread_probe.capabilities);
        let protocol_version = scenario.protocol_version;
        let prompt_scenario = scenario.clone();
        let prompt_cancel_receiver = Arc::clone(&cancel_receiver);
        let permission_probe = Arc::clone(&thread_probe.permission);
        let cancellation_probe = Arc::clone(&thread_probe.cancellation_count);
        let cancellation_sender = Arc::clone(&cancel_sender);
        let initialize_phase = Arc::clone(&thread_probe.phase);
        let new_session_phase = Arc::clone(&thread_probe.phase);
        let prompt_phase = Arc::clone(&thread_probe.phase);
        let block_initialize = scenario.block_initialize;
        let block_new_session = scenario.block_new_session;
        let block_prompt = scenario.block_prompt;
        let release_receiver = Arc::new(Mutex::new(Some(release_rx)));
        let initialize_release = Arc::clone(&release_receiver);
        let new_session_release = Arc::clone(&release_receiver);
        let prompt_release = Arc::clone(&release_receiver);
        let agent = Agent
            .builder()
            .on_receive_request(
                async move |request: InitializeRequest, responder, _connection| {
                    mark_phase(&initialize_phase, AgentPhase::Initialize);
                    *initialize_probe
                        .lock()
                        .expect("capability probe must not be poisoned") =
                        Some(request.client_capabilities);
                    if block_initialize {
                        wait_for_release(&initialize_release).await;
                    }
                    responder.respond(InitializeResponse::new(protocol_version))
                },
                agent_client_protocol::on_receive_request!(),
            )
            .on_receive_request(
                async move |_request: NewSessionRequest, responder, _connection| {
                    mark_phase(&new_session_phase, AgentPhase::NewSession);
                    if block_new_session {
                        wait_for_release(&new_session_release).await;
                    }
                    responder.respond(NewSessionResponse::new(SessionId::new(SESSION_ID)))
                },
                agent_client_protocol::on_receive_request!(),
            )
            .on_receive_notification(
                async move |_notification: CancelNotification, _connection| {
                    let mut count = cancellation_probe
                        .lock()
                        .expect("cancellation probe must not be poisoned");
                    *count += 1;
                    signal_cancelled(&cancellation_sender);
                    Ok(())
                },
                agent_client_protocol::on_receive_notification!(),
            )
            .on_receive_request(
                move |request: PromptRequest,
                      responder: Responder<PromptResponse>,
                      connection: ConnectionTo<Client>| {
                    let prompt_scenario = prompt_scenario.clone();
                    let prompt_cancel_receiver = Arc::clone(&prompt_cancel_receiver);
                    let permission_probe = Arc::clone(&permission_probe);
                    let prompt_phase = Arc::clone(&prompt_phase);
                    let prompt_release = Arc::clone(&prompt_release);
                    async move {
                        mark_phase(&prompt_phase, AgentPhase::Prompt);
                        if block_prompt {
                            wait_for_release(&prompt_release).await;
                        }
                        if prompt_scenario.permission_requests > 0 {
                            let permission_connection = connection.clone();
                            return connection.spawn(async move {
                                let option_name =
                                    if prompt_scenario.permission_option_name_bytes == 0 {
                                        "Allow".to_owned()
                                    } else {
                                        "p".repeat(prompt_scenario.permission_option_name_bytes)
                                    };
                                for index in 0..prompt_scenario.permission_requests {
                                    let mut options = vec![PermissionOption::new(
                                        "allow",
                                        option_name.clone(),
                                        PermissionOptionKind::AllowOnce,
                                    )];
                                    if prompt_scenario.include_reject_permission {
                                        options.push(PermissionOption::new(
                                            "reject",
                                            "Reject",
                                            PermissionOptionKind::RejectOnce,
                                        ));
                                    }
                                    let permission = permission_connection
                                        .send_request(RequestPermissionRequest::new(
                                            request.session_id.clone(),
                                            ToolCallUpdate::new(
                                                format!("tool-call-{index}"),
                                                ToolCallUpdateFields::new(),
                                            ),
                                            options,
                                        ))
                                        .block_task()
                                        .await?;
                                    *permission_probe
                                        .lock()
                                        .expect("permission probe must not be poisoned") =
                                        Some(permission.outcome);
                                }
                                responder.respond(PromptResponse::new(prompt_scenario.stop_reason))
                            });
                        }
                        for text in &prompt_scenario.updates {
                            connection.send_notification(SessionNotification::new(
                                request.session_id.clone(),
                                SessionUpdate::AgentMessageChunk(ContentChunk::new(
                                    ContentBlock::Text(TextContent::new(text.clone())),
                                )),
                            ))?;
                        }
                        for update in &prompt_scenario.structured_updates {
                            connection.send_notification(SessionNotification::new(
                                request.session_id.clone(),
                                update.clone(),
                            ))?;
                        }
                        if prompt_scenario.non_text_update {
                            connection.send_notification(SessionNotification::new(
                                request.session_id.clone(),
                                SessionUpdate::AgentMessageChunk(ContentChunk::new(
                                    ContentBlock::Image(ImageContent::new("AA==", "image/png")),
                                )),
                            ))?;
                        }
                        if prompt_scenario.unknown_session_message {
                            connection.send_notification(UntypedMessage::new(
                                "session/unknown",
                                serde_json::json!({ "sessionId": request.session_id }),
                            )?)?;
                        }
                        if prompt_scenario.stop_reason == StopReason::Cancelled {
                            let cancel_receiver = prompt_cancel_receiver
                                .lock()
                                .expect("cancellation receiver must not be poisoned")
                                .take()
                                .expect("one prompt may wait for cancellation");
                            return connection.spawn(async move {
                                let _cancelled = cancel_receiver.await;
                                responder.respond(PromptResponse::new(prompt_scenario.stop_reason))
                            });
                        }
                        responder.respond(PromptResponse::new(prompt_scenario.stop_reason))
                    }
                },
                agent_client_protocol::on_receive_request!(),
            );
        let _result = futures::executor::block_on(agent.connect_with(
            ByteStreams::new(
                Async::new(agent_writer).expect("agent writer must become nonblocking"),
                Async::new(agent_stream).expect("agent reader must become nonblocking"),
            ),
            async |connection| {
                connection.incoming_closed().await;
                Ok(())
            },
        ));
    });
    (
        Async::new(client_writer).expect("client writer must become nonblocking"),
        Async::new(client_stream).expect("client reader must become nonblocking"),
        probe,
        thread,
    )
}

fn mark_phase(phase: &Arc<(Mutex<Option<AgentPhase>>, Condvar)>, current: AgentPhase) {
    let (phase, changed) = phase.as_ref();
    *phase
        .lock()
        .expect("agent phase probe must not be poisoned") = Some(current);
    changed.notify_all();
}

async fn wait_for_release(release: &Arc<Mutex<Option<oneshot::Receiver<()>>>>) {
    let receiver = release
        .lock()
        .expect("agent release receiver must not be poisoned")
        .take()
        .expect("only one synthetic request may block");
    let _released = receiver.await;
}

fn signal_cancelled(cancelled: &Arc<Mutex<Option<oneshot::Sender<()>>>>) {
    if let Some(sender) = cancelled
        .lock()
        .expect("cancellation signal must not be poisoned")
        .take()
    {
        let _sent = sender.send(());
    }
}
