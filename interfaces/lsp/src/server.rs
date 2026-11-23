//! LSP transport and scheduling over the API-owned compiler façade.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use lsp_server::{Connection, Message, Request, RequestId, Response, ResponseError};
use lsp_types::{
    notification::{
        Cancel, DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument,
        DidOpenTextDocument, DidSaveTextDocument, Notification as _, PublishDiagnostics,
    },
    request::{Request as _, Shutdown},
    *,
};
use pale_sdk::PaleApi;

use crate::analysis::{
    AnalysisJob, AnalysisResult, AnalysisWorker, AnalysisWorkerError, QueryFailure, QueryJob,
    QueryResult, WorkerResult,
};
use crate::authoring::{
    correction_range, AuthoringClientCapabilities, AuthoringMarkupKind, AuthoringQueryError,
};
use crate::document::{BaselineDocument, DocumentError, DocumentStore};
use crate::protocol::{
    decode_query, experimental_capabilities, AnalysisAdmission, AnalysisPublication,
    AuthoringLimitErrorData, AuthoringQueryResult, ProtocolError, WorkspaceQuery,
    WorkspaceQueryResult, ANALYSIS_PUBLISHED_NOTIFICATION,
};
use crate::scheduling::{AnalysisClock, AnalysisClockError};
use crate::syntax::AuthoringSyntaxSnapshot;

mod references;

/// Maximum custom requests retained while one API operation owns the worker.
const MAX_PENDING_QUERIES: usize = 64;

/// Typed initialization options for the configuration-only leaf.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "protocol-codegen", derive(schemars::JsonSchema))]
struct InitializationOptions {
    /// Logical root manifest passed unchanged to
    /// [`PortableWorkspaceRequest`](pale_sdk::PortableWorkspaceRequest).
    root_manifest: String,
    /// Explicit machine selector used only for effective-profile inspection.
    #[serde(default)]
    machine_id: Option<String>,
    /// Complete bounded configuration baseline captured by the editor host.
    documents: Vec<BaselineDocument>,
}

/// Typed subset of LSP initialization used by this transport.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitializationInput {
    root_uri: Uri,
    #[serde(default)]
    capabilities: ClientCapabilities,
    initialization_options: InitializationOptions,
}

/// Configuration-only transport state.
struct ServerState {
    documents: DocumentStore,
    clock: AnalysisClock,
    root_manifest: String,
    machine_id: Option<String>,
    authoring_capabilities: AuthoringClientCapabilities,
    pending_analysis: Option<PendingAnalysis>,
    pending_queries: VecDeque<QueryJob>,
    analyzed_generation: Option<u64>,
    /// Source-local structure for the exact accepted analysis publication.
    authoring_syntax: AuthoringSyntaxSnapshot,
    in_flight: Option<InFlight>,
    /// One streaming request; all later queries stay in the existing queue.
    references: Option<references::ReferenceDelivery>,
}

/// One debounced generation whose immutable text snapshot is not captured yet.
struct PendingAnalysis {
    generation: u64,
}

/// Exactly one worker operation can be in flight.
enum InFlight {
    /// Compilation/comparison for one editor generation.
    Analysis { generation: u64 },
    /// One explicit request. Cancellation suppresses only its result; it does
    /// not pretend to interrupt the API operation.
    Query {
        request_id: RequestId,
        generation: u64,
        document_revision: u64,
        cancelled: bool,
    },
}

enum QueryAcceptance {
    Current,
    Cancelled,
    Stale,
    Unknown,
}

enum CancellationDisposition {
    Pending,
    InFlight,
    Unknown,
}

fn authoring_capabilities(client: &ClientCapabilities) -> AuthoringClientCapabilities {
    let versioned_document_changes = client
        .workspace
        .as_ref()
        .and_then(|workspace| workspace.workspace_edit.as_ref())
        .and_then(|workspace_edit| workspace_edit.document_changes)
        .unwrap_or(false);
    let completion_markup = client
        .text_document
        .as_ref()
        .and_then(|text| text.completion.as_ref())
        .and_then(|completion| completion.completion_item.as_ref())
        .and_then(|item| item.documentation_format.as_deref())
        .map_or(AuthoringMarkupKind::PlainText, preferred_markup);
    let hover_markup = client
        .text_document
        .as_ref()
        .and_then(|text| text.hover.as_ref())
        .and_then(|hover| hover.content_format.as_deref())
        .map_or(AuthoringMarkupKind::PlainText, preferred_markup);
    AuthoringClientCapabilities {
        versioned_document_changes,
        completion_markup,
        hover_markup,
    }
}

fn preferred_markup(formats: &[MarkupKind]) -> AuthoringMarkupKind {
    formats
        .iter()
        .find_map(|format| {
            if *format == MarkupKind::Markdown {
                Some(AuthoringMarkupKind::Markdown)
            } else if *format == MarkupKind::PlainText {
                Some(AuthoringMarkupKind::PlainText)
            } else {
                None
            }
        })
        .unwrap_or_default()
}

impl ServerState {
    fn new(input: InitializationInput) -> Result<Self, ServerError> {
        let authoring_capabilities = authoring_capabilities(&input.capabilities);
        let InitializationOptions {
            root_manifest,
            machine_id,
            documents,
        } = input.initialization_options;
        Ok(Self {
            documents: DocumentStore::new(&input.root_uri, documents)?,
            clock: AnalysisClock::default(),
            root_manifest,
            machine_id,
            authoring_capabilities,
            pending_analysis: None,
            pending_queries: VecDeque::new(),
            analyzed_generation: None,
            authoring_syntax: AuthoringSyntaxSnapshot::default(),
            in_flight: None,
            references: None,
        })
    }

    fn capabilities() -> ServerCapabilities {
        ServerCapabilities {
            position_encoding: Some(PositionEncodingKind::UTF16),
            text_document_sync: Some(TextDocumentSyncCapability::Options(
                TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::INCREMENTAL),
                    save: Some(
                        SaveOptions {
                            include_text: Some(true),
                        }
                        .into(),
                    ),
                    ..TextDocumentSyncOptions::default()
                },
            )),
            semantic_tokens_provider: Some(
                SemanticTokensOptions {
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    legend: SemanticTokensLegend {
                        token_types: vec![
                            SemanticTokenType::NAMESPACE,
                            SemanticTokenType::PROPERTY,
                            SemanticTokenType::ENUM_MEMBER,
                            SemanticTokenType::VARIABLE,
                        ],
                        token_modifiers: vec![SemanticTokenModifier::DEFINITION],
                    },
                    range: None,
                    full: Some(SemanticTokensFullOptions::Bool(true)),
                }
                .into(),
            ),
            completion_provider: Some(CompletionOptions::default()),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            document_symbol_provider: Some(OneOf::Left(true)),
            definition_provider: Some(OneOf::Left(true)),
            references_provider: Some(OneOf::Right(ReferencesOptions {
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: Some(true),
                },
            })),
            code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
            experimental: Some(experimental_capabilities()),
            ..ServerCapabilities::default()
        }
    }

    /// Supersede the scheduled editor state and return queued stale requests.
    fn schedule(&mut self, now: Instant) -> Result<Vec<RequestId>, AnalysisClockError> {
        let generation = self.clock.schedule(now)?;
        self.pending_analysis = Some(PendingAnalysis { generation });
        self.analyzed_generation = None;
        Ok(self
            .pending_queries
            .drain(..)
            .map(|query| query.request_id)
            .collect())
    }

    /// Claim due work while preserving the one-worker bound.
    ///
    /// If `I` is the set of in-flight analyses, the transport invariant is:
    #[doc = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/docs/math/analysis-in-flight-bound.inline.mathml"
    ))]
    /// New editor states replace
    /// [`Self::pending_analysis`] while that single job runs; they never create
    /// another compiler owner.
    fn claim_due(&mut self, now: Instant) -> Option<AnalysisJob> {
        if self.in_flight.is_some() {
            return None;
        }
        let pending = self.pending_analysis.take()?;
        if !self.clock.claim(pending.generation, now) {
            self.pending_analysis = Some(pending);
            return None;
        }
        self.in_flight = Some(InFlight::Analysis {
            generation: pending.generation,
        });
        Some(AnalysisJob {
            generation: pending.generation,
            snapshot: self.documents.snapshot(),
        })
    }

    fn accept_analysis(&mut self, result: &AnalysisResult) -> bool {
        let Some(InFlight::Analysis { generation }) = self.in_flight else {
            return false;
        };
        if generation != result.generation {
            return false;
        }
        self.in_flight = None;
        let current = result.generation == self.clock.generation
            && self.documents.is_current(&result.snapshot);
        if current {
            self.analyzed_generation = Some(result.generation);
            self.authoring_syntax = result.syntax.clone();
        }
        current
    }

    fn queue_query(
        &mut self,
        request_id: RequestId,
        query: WorkspaceQuery,
    ) -> Result<(), RequestId> {
        let job = QueryJob {
            request_id,
            generation: self.clock.generation,
            document_revision: self.documents.revision(),
            query,
            authoring_capabilities: self.authoring_capabilities,
        };
        if self.pending_queries.len() >= MAX_PENDING_QUERIES {
            return Err(job.request_id);
        }
        self.pending_queries.push_back(job);
        Ok(())
    }

    fn claim_query(&mut self) -> Option<QueryJob> {
        if self.in_flight.is_some()
            || self.pending_analysis.is_some()
            || self.analyzed_generation != Some(self.clock.generation)
        {
            return None;
        }
        let job = self.pending_queries.pop_front()?;
        self.references = references::ReferenceDelivery::for_job(&job);
        self.in_flight = Some(InFlight::Query {
            request_id: job.request_id.clone(),
            generation: job.generation,
            document_revision: job.document_revision,
            cancelled: false,
        });
        Some(job)
    }

    fn accept_query(&mut self, result: &QueryResult) -> QueryAcceptance {
        let Some(InFlight::Query {
            request_id,
            generation,
            document_revision,
            cancelled,
        }) = self.in_flight.take()
        else {
            return QueryAcceptance::Unknown;
        };
        if request_id != result.request_id
            || generation != result.generation
            || document_revision != result.document_revision
        {
            self.in_flight = Some(InFlight::Query {
                request_id,
                generation,
                document_revision,
                cancelled,
            });
            return QueryAcceptance::Unknown;
        }
        if cancelled {
            QueryAcceptance::Cancelled
        } else if result.generation != self.clock.generation
            || result.document_revision != self.documents.revision()
            || self.analyzed_generation != Some(result.generation)
        {
            QueryAcceptance::Stale
        } else {
            QueryAcceptance::Current
        }
    }

    fn cancel_query(&mut self, request_id: &RequestId) -> CancellationDisposition {
        if let Some(index) = self
            .pending_queries
            .iter()
            .position(|query| &query.request_id == request_id)
        {
            self.pending_queries.remove(index);
            return CancellationDisposition::Pending;
        }
        if let Some(InFlight::Query {
            request_id: current,
            cancelled,
            ..
        }) = &mut self.in_flight
        {
            if current == request_id {
                *cancelled = true;
                return CancellationDisposition::InFlight;
            }
        }
        CancellationDisposition::Unknown
    }

    fn cancel_pending(&mut self) -> Vec<RequestId> {
        self.pending_analysis = None;
        if let Some(InFlight::Query { cancelled, .. }) = &mut self.in_flight {
            *cancelled = true;
        }
        self.pending_queries
            .drain(..)
            .map(|query| query.request_id)
            .collect()
    }
}

/// Failure to initialize or operate the transport leaf.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    /// The API façade could not verify its embedded contracts.
    #[error(transparent)]
    ApiInitialization(#[from] pale_sdk::ApiInitializationError),
    /// Initialization bytes did not form a confined configuration baseline.
    #[error("LSP initialization failed: {0}")]
    Initialization(String),
    /// A process-local analysis coordinate had no representable successor.
    #[error("analysis generation is exhausted")]
    AnalysisCoordinateExhausted,
    /// The client sent `exit` before the required successful shutdown request.
    #[error("LSP client exited before shutdown")]
    ExitBeforeShutdown,
    /// JSON-RPC transport failed.
    #[error("LSP transport failed: {0}")]
    Transport(String),
}

impl From<DocumentError> for ServerError {
    fn from(error: DocumentError) -> Self {
        Self::Initialization(document_error_message(&error))
    }
}

impl From<AnalysisClockError> for ServerError {
    fn from(_: AnalysisClockError) -> Self {
        Self::AnalysisCoordinateExhausted
    }
}

impl From<AnalysisWorkerError> for ServerError {
    fn from(error: AnalysisWorkerError) -> Self {
        Self::Transport(error.to_string())
    }
}

/// Serve the configuration projection over standard input/output.
///
/// # Errors
///
/// Returns [`ServerError`] for initialization, overlay, coordinate, protocol,
/// or transport failures. It may read an exact initialization-registered source
/// after a standard watched-file event; it performs no filesystem mutation or
/// native effect.
pub fn run_stdio() -> Result<(), ServerError> {
    let (connection, io_threads) = Connection::stdio();
    let capabilities = serde_json::to_value(ServerState::capabilities())
        .map_err(|error| ServerError::Transport(error.to_string()))?;
    let initialization: InitializationInput = serde_json::from_value(
        connection
            .initialize(capabilities)
            .map_err(|error| ServerError::Transport(error.to_string()))?,
    )
    .map_err(|error| ServerError::Initialization(error.to_string()))?;
    let api = PaleApi::new()?;
    let mut state = ServerState::new(initialization)?;
    let worker = AnalysisWorker::spawn(
        api,
        state.root_manifest.clone(),
        state.machine_id.clone(),
        state.documents.baseline_snapshot(),
    );
    let stale = state.schedule(Instant::now())?;
    debug_assert!(stale.is_empty());
    let service_result = serve(&connection, &worker, &mut state);
    let worker_result = worker.stop();
    let io_result = io_threads
        .join()
        .map_err(|error| ServerError::Transport(error.to_string()));
    service_result?;
    worker_result?;
    io_result
}

fn serve(
    connection: &Connection,
    worker: &AnalysisWorker,
    state: &mut ServerState,
) -> Result<(), ServerError> {
    let mut shutdown_requested = false;
    loop {
        while let Ok(result) = worker.results.try_recv() {
            match result {
                WorkerResult::Analysis(result) => {
                    if state.accept_analysis(&result) && !shutdown_requested {
                        publish_analysis(connection, &state.root_manifest, &result)?;
                        publish_analysis_state(connection, state.documents.revision(), &result)?;
                    }
                }
                WorkerResult::Query(result) => match state.accept_query(&result) {
                    QueryAcceptance::Current if !shutdown_requested => {
                        publish_query_diagnostic(connection, state, &result)?;
                        send_query_result(connection, *result)?;
                    }
                    QueryAcceptance::Cancelled => send_request_error(
                        connection,
                        result.request_id,
                        lsp_server::ErrorCode::RequestCanceled,
                        "request cancelled; an in-flight API call may have completed without publication",
                    )?,
                    QueryAcceptance::Stale => send_request_error(
                        connection,
                        result.request_id,
                        lsp_server::ErrorCode::ContentModified,
                        "editor state changed before the query result was published",
                    )?,
                    QueryAcceptance::Current | QueryAcceptance::Unknown => {}
                },
                WorkerResult::References(result) => state.accept_reference_reply(*result),
            }
        }
        state.pump_references(connection, worker)?;
        if !shutdown_requested {
            if let Some(job) = state.claim_due(Instant::now()) {
                worker.analyze(job)?;
            } else if let Some(job) = state.claim_query() {
                worker.query(job)?;
            }
        }
        match connection.receiver.recv_timeout(Duration::from_millis(10)) {
            Ok(Message::Notification(notification))
                if notification.method == notification::Exit::METHOD =>
            {
                return if shutdown_requested {
                    Ok(())
                } else {
                    Err(ServerError::ExitBeforeShutdown)
                };
            }
            Ok(Message::Notification(notification)) if notification.method == Cancel::METHOD => {
                if let Err(error) = handle_cancellation(connection, state, notification) {
                    publish_editor_error(connection, &error)?;
                }
            }
            Ok(Message::Notification(notification)) if !shutdown_requested => {
                if let Err(error) = handle_editor_notification(connection, state, notification) {
                    publish_editor_error(connection, &error)?;
                }
            }
            Ok(Message::Request(request)) => {
                if request.method == Shutdown::METHOD {
                    send_shutdown_response(connection, request)?;
                    shutdown_requested = true;
                    for request_id in state.cancel_pending() {
                        send_request_error(
                            connection,
                            request_id,
                            lsp_server::ErrorCode::RequestCanceled,
                            "request cancelled by server shutdown",
                        )?;
                    }
                } else if shutdown_requested {
                    send_request_error(
                        connection,
                        request.id,
                        lsp_server::ErrorCode::InvalidRequest,
                        "server shutdown has already been requested",
                    )?;
                } else {
                    handle_workspace_request(connection, state, request)?;
                }
            }
            Ok(_) => {}
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
}

fn handle_workspace_request(
    connection: &Connection,
    state: &mut ServerState,
    request: Request,
) -> Result<(), ServerError> {
    let Request { id, method, params } = request;
    match decode_query(&method, params) {
        Ok(Some(query)) => {
            if let Err(rejected) = state.queue_query(id, query) {
                send_limit_error(
                    connection,
                    rejected,
                    "bounded Pale query queue is full",
                    AuthoringLimitErrorData::query_queue(MAX_PENDING_QUERIES),
                )?;
            }
        }
        Ok(None) => send_request_error(
            connection,
            id,
            lsp_server::ErrorCode::MethodNotFound,
            "capability is not owned by the current pale-sdk façade",
        )?,
        Err(error) => send_protocol_error(connection, id, error)?,
    }
    Ok(())
}

fn handle_cancellation(
    connection: &Connection,
    state: &mut ServerState,
    notification: lsp_server::Notification,
) -> Result<(), String> {
    let params: CancelParams =
        serde_json::from_value(notification.params).map_err(|error| error.to_string())?;
    let request_id = match params.id {
        NumberOrString::Number(id) => RequestId::from(id),
        NumberOrString::String(id) => RequestId::from(id),
    };
    if let CancellationDisposition::Pending = state.cancel_query(&request_id) {
        send_request_error(
            connection,
            request_id,
            lsp_server::ErrorCode::RequestCanceled,
            "request cancelled before API dispatch",
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn send_shutdown_response(connection: &Connection, request: Request) -> Result<(), ServerError> {
    connection
        .sender
        .send(Message::Response(Response::new_ok(request.id, ())))
        .map_err(|error| ServerError::Transport(error.to_string()))
}

fn handle_editor_notification(
    connection: &Connection,
    state: &mut ServerState,
    notification: lsp_server::Notification,
) -> Result<(), String> {
    match notification.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params: DidOpenTextDocumentParams =
                serde_json::from_value(notification.params).map_err(|error| error.to_string())?;
            let item = params.text_document;
            state
                .documents
                .open(item.uri, item.language_id, item.version, item.text)
                .map_err(|error| document_error_message(&error))?;
            schedule_editor_analysis(connection, state)
        }
        DidChangeTextDocument::METHOD => {
            let params: DidChangeTextDocumentParams =
                serde_json::from_value(notification.params).map_err(|error| error.to_string())?;
            state
                .documents
                .change(
                    &params.text_document.uri,
                    params.text_document.version,
                    params.content_changes,
                )
                .map_err(|error| document_error_message(&error))?;
            schedule_editor_analysis(connection, state)
        }
        DidSaveTextDocument::METHOD => {
            let params: DidSaveTextDocumentParams =
                serde_json::from_value(notification.params).map_err(|error| error.to_string())?;
            state
                .documents
                .save(&params.text_document.uri, params.text.as_deref())
                .map_err(|error| document_error_message(&error))
        }
        DidCloseTextDocument::METHOD => {
            let params: DidCloseTextDocumentParams =
                serde_json::from_value(notification.params).map_err(|error| error.to_string())?;
            state
                .documents
                .close(&params.text_document.uri)
                .map_err(|error| document_error_message(&error))?;
            publish_empty(connection, params.text_document.uri)
                .map_err(|error| error.to_string())?;
            schedule_editor_analysis(connection, state)
        }
        DidChangeWatchedFiles::METHOD => {
            let params: DidChangeWatchedFilesParams =
                serde_json::from_value(notification.params).map_err(|error| error.to_string())?;
            synchronize_watched_sources(connection, state, params)
        }
        _ => Ok(()),
    }
}

/// Refresh only initialization-registered source URIs from the host filesystem.
fn synchronize_watched_sources(
    connection: &Connection,
    state: &mut ServerState,
    params: DidChangeWatchedFilesParams,
) -> Result<(), String> {
    let mut changed = false;
    let mut failures = Vec::new();
    for event in params.changes {
        if !state.documents.is_registered(&event.uri) {
            failures.push(format!(
                "ignored filesystem event for an unregistered Pale source: {}",
                event.uri.as_str()
            ));
            continue;
        }
        match read_watched_source(&state.documents, &event.uri) {
            Ok(text) => match state.documents.replace_saved(&event.uri, text) {
                Ok(replaced) => changed |= replaced,
                Err(error) => {
                    changed |= state
                        .documents
                        .mark_unavailable(&event.uri)
                        .map_err(|unavailable| document_error_message(&unavailable))?;
                    let error = document_error_message(&error);
                    failures.push(format!(
                        "configured Pale source is unavailable after filesystem change: {}: {error}",
                        event.uri.as_str()
                    ));
                }
            },
            Err(error) => {
                changed |= state
                    .documents
                    .mark_unavailable(&event.uri)
                    .map_err(|unavailable| document_error_message(&unavailable))?;
                failures.push(format!(
                    "configured Pale source is unavailable after filesystem change: {}: {error}",
                    event.uri.as_str()
                ));
            }
        }
    }
    if changed {
        schedule_editor_analysis(connection, state)?;
    }
    for failure in failures {
        publish_editor_error(connection, &failure).map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Read one already-registered host file without scanning its workspace.
fn read_watched_source(documents: &DocumentStore, uri: &Uri) -> Result<String, String> {
    let path = registered_host_path(documents, uri)?;
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    String::from_utf8(bytes).map_err(|error| error.to_string())
}

/// Map an exact registered source URI to the filesystem of this extension host.
///
/// A workspace-kind extension runs beside the language server on the selected
/// local or remote host. `file` identifies a local workspace. `vscode-remote`
/// identifies the same host path through VS Code; its authority must match the
/// admitted initialization root. Other virtual-workspace schemes remain
/// unsupported.
fn registered_host_path(documents: &DocumentStore, uri: &Uri) -> Result<PathBuf, String> {
    if !documents.is_registered(uri) || uri.query().is_some() || uri.fragment().is_some() {
        return Err("configured source is not registered for this workspace".to_owned());
    }
    let root = documents.root_uri();
    let scheme = uri
        .scheme()
        .ok_or_else(|| "configured source has no URI scheme".to_owned())?;
    let is_file = scheme.eq_lowercase("file");
    let is_remote = scheme.eq_lowercase("vscode-remote")
        && root
            .scheme()
            .is_some_and(|root_scheme| root_scheme.eq_lowercase("vscode-remote"))
        && uri.authority().map(|authority| authority.as_str())
            == root.authority().map(|authority| authority.as_str())
        && uri.authority().is_some();
    if !is_file && !is_remote {
        return Err("configured source is not on the admitted local or remote host".to_owned());
    }
    let decoded = uri
        .path()
        .as_estr()
        .decode()
        .into_string()
        .map_err(|_| "configured source path is not valid UTF-8".to_owned())?
        .into_owned();
    let authority = uri.authority().map_or("", |authority| authority.as_str());
    #[cfg(windows)]
    {
        if is_file && !authority.is_empty() {
            return Ok(PathBuf::from(format!("//{authority}{decoded}")));
        }
        let path = decoded
            .strip_prefix('/')
            .filter(|path| path.as_bytes().get(1) == Some(&b':'))
            .unwrap_or(&decoded);
        Ok(PathBuf::from(path))
    }
    #[cfg(not(windows))]
    {
        if is_file && !authority.is_empty() {
            return Err("configured source file URI has a non-local authority".to_owned());
        }
        Ok(PathBuf::from(decoded))
    }
}

fn schedule_editor_analysis(
    connection: &Connection,
    state: &mut ServerState,
) -> Result<(), String> {
    let stale = state
        .schedule(Instant::now())
        .map_err(|error| error.to_string())?;
    for request_id in stale {
        send_request_error(
            connection,
            request_id,
            lsp_server::ErrorCode::ContentModified,
            "editor state changed before the query was dispatched",
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn publish_editor_error(connection: &Connection, message: &str) -> Result<(), ServerError> {
    let params = LogMessageParams {
        typ: MessageType::ERROR,
        message: message.to_owned(),
    };
    let notification =
        lsp_server::Notification::new(notification::LogMessage::METHOD.to_owned(), params);
    connection
        .sender
        .send(Message::Notification(notification))
        .map_err(|error| ServerError::Transport(error.to_string()))
}

fn document_limit_data(error: &DocumentError) -> Option<AuthoringLimitErrorData> {
    match *error {
        DocumentError::DocumentBytesExceeded {
            class,
            observed,
            maximum,
        } => Some(AuthoringLimitErrorData::document_bytes(
            class, observed, maximum,
        )),
        DocumentError::DocumentCountExceeded {
            class,
            observed,
            maximum,
        } => Some(AuthoringLimitErrorData::document_count(
            class, observed, maximum,
        )),
        DocumentError::TotalBytesExceeded {
            class,
            observed,
            maximum,
        } => Some(AuthoringLimitErrorData::total_bytes(
            class, observed, maximum,
        )),
        DocumentError::UnsupportedLanguage(_)
        | DocumentError::OutsideRoot
        | DocumentError::DuplicateLogicalLabel(_)
        | DocumentError::UnregisteredRole
        | DocumentError::LanguageMismatch
        | DocumentError::RoleLanguageMismatch
        | DocumentError::InvalidRoot
        | DocumentError::InvalidEncoding
        | DocumentError::NotOpen
        | DocumentError::SaveTextMismatch
        | DocumentError::StaleVersion { .. }
        | DocumentError::InvalidRange
        | DocumentError::RevisionExhausted => None,
    }
}

fn document_error_message(error: &DocumentError) -> String {
    document_limit_data(error).map_or_else(
        || error.to_string(),
        |data| format!("{error}. {}", data.recovery()),
    )
}

fn publish_empty(connection: &Connection, uri: Uri) -> Result<(), ServerError> {
    let params = PublishDiagnosticsParams::new(uri, Vec::new(), None);
    let notification = lsp_server::Notification::new(PublishDiagnostics::METHOD.to_owned(), params);
    connection
        .sender
        .send(Message::Notification(notification))
        .map_err(|error| ServerError::Transport(error.to_string()))
}

fn publish_analysis(
    connection: &Connection,
    root_manifest: &str,
    publication: &AnalysisResult,
) -> Result<(), ServerError> {
    for document in publication.snapshot.documents() {
        let diagnostics = match &publication.outcome {
            Ok(()) => Vec::new(),
            Err(errors) => errors
                .iter()
                .filter_map(|error| {
                    let diagnostic = error.diagnostic();
                    diagnostic_targets_document(diagnostic, root_manifest, document.logical_label())
                        .then(|| {
                            let text = document.text();
                            let correction_range =
                                error.authoring_corrections().iter().find_map(|correction| {
                                    correction_range(
                                        document,
                                        publication.syntax.document(document.logical_label())?,
                                        correction,
                                    )
                                });
                            let authoring_range = authoring_diagnostic_range(
                                document,
                                publication.syntax.document(document.logical_label()),
                                error.authoring_location(),
                            );
                            vec![project_diagnostic(
                                diagnostic,
                                &text,
                                correction_range.or(authoring_range),
                            )]
                        })
                })
                .flatten()
                .collect(),
        };
        let params =
            PublishDiagnosticsParams::new(document.uri().clone(), diagnostics, document.version());
        let notification =
            lsp_server::Notification::new(PublishDiagnostics::METHOD.to_owned(), params);
        connection
            .sender
            .send(Message::Notification(notification))
            .map_err(|error| ServerError::Transport(error.to_string()))?;
    }
    Ok(())
}

fn publish_analysis_state(
    connection: &Connection,
    document_revision: u64,
    publication: &AnalysisResult,
) -> Result<(), ServerError> {
    let params = AnalysisPublication {
        editor: crate::protocol::EditorCoordinate {
            generation: publication.generation,
            document_revision,
        },
        admission: if publication.outcome.is_ok() {
            AnalysisAdmission::Admitted
        } else {
            AnalysisAdmission::Rejected
        },
    };
    let notification =
        lsp_server::Notification::new(ANALYSIS_PUBLISHED_NOTIFICATION.to_owned(), params);
    connection
        .sender
        .send(Message::Notification(notification))
        .map_err(|error| ServerError::Transport(error.to_string()))
}

fn publish_query_diagnostic(
    connection: &Connection,
    state: &ServerState,
    result: &QueryResult,
) -> Result<(), ServerError> {
    let Ok(WorkspaceQueryResult::EffectiveProfile(response)) = &result.outcome else {
        return Ok(());
    };
    let diagnostic = response.diagnostic();
    for document in state.documents.snapshot().documents() {
        let diagnostics = diagnostic
            .filter(|diagnostic| {
                diagnostic_targets_document(
                    diagnostic,
                    &state.root_manifest,
                    document.logical_label(),
                )
            })
            .map_or_else(Vec::new, |diagnostic| {
                let range = authoring_diagnostic_range(
                    document,
                    state.authoring_syntax.document(document.logical_label()),
                    response.authoring_location(),
                );
                vec![project_diagnostic(diagnostic, &document.text(), range)]
            });
        let params =
            PublishDiagnosticsParams::new(document.uri().clone(), diagnostics, document.version());
        let notification =
            lsp_server::Notification::new(PublishDiagnostics::METHOD.to_owned(), params);
        connection
            .sender
            .send(Message::Notification(notification))
            .map_err(|error| ServerError::Transport(error.to_string()))?;
    }
    Ok(())
}

fn diagnostic_targets_document(
    diagnostic: &pale_sdk::DiagnosticV1,
    root_manifest: &str,
    document_label: &str,
) -> bool {
    diagnostic
        .source()
        .map_or(document_label == root_manifest, |source| {
            source.label() == document_label
        })
}

fn authoring_diagnostic_range(
    document: &crate::document::AnalysisDocument,
    syntax: Option<&crate::syntax::AuthoringSyntax>,
    location: Option<&pale_sdk::AuthoringSourceLocation>,
) -> Option<Range> {
    let location = location?;
    if location.source_label() != document.logical_label() {
        return None;
    }
    let fact = syntax?.fact_for_instance_path(location.path())?;
    document.range_at_bytes(fact.range.start, fact.range.end)
}

/// Project one admitted API diagnostic without becoming another policy owner.
///
/// LSP's native fields receive the exact range, severity, code, and summary
/// they can represent. The `data` member retains the complete admitted
/// [`pale_sdk::DiagnosticV1`] JSON, including stage, disposition, retry,
/// effect certainty, source/value path, schema coordinate, causes, notes,
/// help, facts, correlations, and truncation accounting. Those fields are not
/// re-rendered into transport prose or fabricated as location-bearing related
/// information, because their meaning remains owned by [`PaleApi`].
fn project_diagnostic(
    diagnostic: &pale_sdk::DiagnosticV1,
    text: &str,
    exact_authoring_range: Option<Range>,
) -> Diagnostic {
    let range = exact_authoring_range
        .or_else(|| {
            diagnostic
                .source()
                .as_ref()
                .and_then(|source| diagnostic_range(source, text))
        })
        .unwrap_or_else(|| Range::new(Position::new(0, 0), Position::new(0, 0)));
    let severity = match diagnostic.severity() {
        pale_sdk::DiagnosticSeverity::Error => DiagnosticSeverity::ERROR,
    };
    let complete: serde_json::Value = serde_json::from_slice(diagnostic.canonical_bytes())
        .expect("an admitted API diagnostic retains canonical JSON bytes");
    let data = serde_json::json!({
        "identity": diagnostic.identity(),
        "diagnostic": complete,
    });
    Diagnostic {
        range,
        severity: Some(severity),
        code: Some(NumberOrString::String(diagnostic.code().to_owned())),
        source: Some("pale".to_owned()),
        message: diagnostic.summary().to_owned(),
        data: Some(data),
        ..Diagnostic::default()
    }
}

/// Convert API-owned UTF-8 coordinates into LSP's zero-based UTF-16 units.
///
/// Complete byte spans take precedence because they preserve exact end
/// positions. A line/column fallback interprets the API column as a one-based
/// Unicode-scalar coordinate, then counts the selected prefix in UTF-16 code
/// units. Invalid owner coordinates are omitted rather than saturated.
fn diagnostic_range(source: &pale_sdk::DiagnosticSource, text: &str) -> Option<Range> {
    match (source.byte_start(), source.byte_end()) {
        (Some(start), Some(end)) => {
            let start = usize::try_from(start).ok()?;
            let end = usize::try_from(end).ok()?;
            if start > end || !text.is_char_boundary(start) || !text.is_char_boundary(end) {
                return None;
            }
            Some(Range::new(
                utf16_position_at_byte(text, start)?,
                utf16_position_at_byte(text, end)?,
            ))
        }
        _ => {
            let line = source.line()?.checked_sub(1)?;
            let scalar = source.column()?.checked_sub(1)?;
            let byte = scalar_position_to_byte(text, line, scalar)?;
            let position = utf16_position_at_byte(text, byte)?;
            Some(Range::new(position, position))
        }
    }
}

fn scalar_position_to_byte(text: &str, line: u32, scalar: u32) -> Option<usize> {
    let mut line_start = 0_usize;
    for _ in 0..line {
        line_start += text.get(line_start..)?.find('\n')? + 1;
    }
    let line_text = text.get(line_start..)?.split('\n').next()?;
    if scalar == 0 {
        return Some(line_start);
    }
    let scalar = usize::try_from(scalar).ok()?;
    let offset = line_text.char_indices().nth(scalar).map_or_else(
        || (line_text.chars().count() == scalar).then_some(line_text.len()),
        |(byte, _)| Some(byte),
    )?;
    Some(line_start + offset)
}

fn utf16_position_at_byte(text: &str, byte: usize) -> Option<Position> {
    let prefix = text.get(..byte)?;
    let line = u32::try_from(prefix.bytes().filter(|byte| *byte == b'\n').count()).ok()?;
    let line_start = prefix.rfind('\n').map_or(0, |position| position + 1);
    let character = u32::try_from(prefix.get(line_start..)?.encode_utf16().count()).ok()?;
    Some(Position::new(line, character))
}

fn send_query_result(connection: &Connection, result: QueryResult) -> Result<(), ServerError> {
    connection
        .sender
        .send(Message::Response(query_response(result)))
        .map_err(|error| ServerError::Transport(error.to_string()))
}

fn query_response(result: QueryResult) -> Response {
    let QueryResult {
        request_id,
        outcome,
        ..
    } = result;
    match outcome {
        Ok(WorkspaceQueryResult::CompilationSummary(summary)) => {
            Response::new_ok(request_id, summary)
        }
        Ok(WorkspaceQueryResult::ComparisonSummary(summary)) => {
            Response::new_ok(request_id, summary)
        }
        Ok(WorkspaceQueryResult::ChangeRoots(page)) => Response::new_ok(request_id, page),
        Ok(WorkspaceQueryResult::Impacts(page)) => Response::new_ok(request_id, page),
        Ok(WorkspaceQueryResult::Explanation(explanation)) => {
            Response::new_ok(request_id, explanation)
        }
        Ok(WorkspaceQueryResult::ProfileContexts(response)) => {
            Response::new_ok(request_id, response)
        }
        Ok(WorkspaceQueryResult::EffectiveProfile(result)) => {
            Response::new_ok(request_id, result.response)
        }
        Ok(WorkspaceQueryResult::EffectiveProfileExplanation(explanation)) => {
            Response::new_ok(request_id, explanation)
        }
        Ok(WorkspaceQueryResult::ConfigurationImprovement(response)) => {
            Response::new_ok(request_id, response)
        }
        Ok(WorkspaceQueryResult::Authoring(result)) => match result {
            AuthoringQueryResult::SemanticTokens(result) => Response::new_ok(request_id, result),
            AuthoringQueryResult::Completion(result) => Response::new_ok(request_id, result),
            AuthoringQueryResult::Hover(result) => Response::new_ok(request_id, result),
            AuthoringQueryResult::DocumentSymbols(result) => Response::new_ok(request_id, result),
            AuthoringQueryResult::Definition(result) => Response::new_ok(request_id, result),
            AuthoringQueryResult::References(result) => Response::new_ok(request_id, result),
            AuthoringQueryResult::CodeActions(result) => Response::new_ok(request_id, result),
        },
        Err(QueryFailure::ProfileCoordinateRequired) => {
            Response::new_err(
                request_id,
                lsp_server::ErrorCode::InvalidParams as i32,
                "a continuation page requires the exact expectedCoordinate from its first page"
                    .to_owned(),
            )
        }
        Err(QueryFailure::ProfileCoordinateMismatch) => {
            Response::new_err(
                request_id,
                lsp_server::ErrorCode::ContentModified as i32,
                "the expected profile coordinate no longer matches the retained editor product"
                    .to_owned(),
            )
        }
        Err(QueryFailure::ImprovementCoordinateMismatch) => {
            Response::new_err(
                request_id,
                lsp_server::ErrorCode::ContentModified as i32,
                "the expected configuration-improvement coordinate no longer matches the retained review".to_owned(),
            )
        }
        Err(
            QueryFailure::AnalysisUnavailable
            | QueryFailure::ComparisonUnavailable
            | QueryFailure::ProfileExplanationUnavailable
            | QueryFailure::ImprovementPhaseUnavailable,
        ) => {
            Response::new_err(
                request_id,
                lsp_server::ErrorCode::RequestFailed as i32,
                "the requested admitted workspace coordinate is unavailable".to_owned(),
            )
        }
        Err(QueryFailure::ImprovementSource(error)) => {
            Response::new_err(
                request_id,
                lsp_server::ErrorCode::RequestFailed as i32,
                error.to_string(),
            )
        }
        Err(QueryFailure::Api(error)) => {
            let diagnostic = error.diagnostic();
            Response {
                id: request_id,
                response_result: Err(ResponseError {
                    code: lsp_server::ErrorCode::RequestFailed as i32,
                    message: diagnostic.summary().to_owned(),
                    data: Some(
                        serde_json::from_slice(diagnostic.canonical_bytes())
                            .expect("an admitted API diagnostic retains canonical JSON bytes"),
                    ),
                }),
            }
        }
        Err(QueryFailure::Authoring(AuthoringQueryError::InvalidPosition)) => {
            Response::new_err(
                request_id,
                lsp_server::ErrorCode::InvalidParams as i32,
                "authoring position is outside the retained editor document".to_owned(),
            )
        }
        Err(QueryFailure::Authoring(error)) => {
            if let Some(data) = authoring_limit_data(&error) {
                limit_error_response(request_id, &error.to_string(), data)
            } else {
                Response::new_err(
                    request_id,
                    lsp_server::ErrorCode::RequestFailed as i32,
                    error.to_string(),
                )
            }
        }
    }
}

fn authoring_limit_data(error: &AuthoringQueryError) -> Option<AuthoringLimitErrorData> {
    match error {
        AuthoringQueryError::ReferenceSetTooLarge => {
            Some(AuthoringLimitErrorData::direct_references())
        }
        AuthoringQueryError::ReferenceLocationBound => {
            Some(AuthoringLimitErrorData::reference_locations())
        }
        AuthoringQueryError::ReferencePageBound => Some(AuthoringLimitErrorData::reference_pages()),
        AuthoringQueryError::SyntaxFactLimitExceeded => Some(
            AuthoringLimitErrorData::syntax_facts(crate::syntax::MAX_AUTHORING_SYNTAX_FACTS),
        ),
        AuthoringQueryError::StandardResponseTooLarge => {
            Some(AuthoringLimitErrorData::standard_response(
                crate::authoring::MAX_STANDARD_AUTHORING_RESULTS,
            ))
        }
        AuthoringQueryError::InvalidPosition
        | AuthoringQueryError::Api(_)
        | AuthoringQueryError::SourceMappingUnavailable => None,
    }
}

fn limit_error_response(
    request_id: RequestId,
    summary: &str,
    data: AuthoringLimitErrorData,
) -> Response {
    Response {
        id: request_id,
        response_result: Err(ResponseError {
            code: lsp_server::ErrorCode::RequestFailed as i32,
            message: format!("{summary}. {}", data.recovery()),
            data: Some(serde_json::to_value(data).expect("LSP limit data is serializable")),
        }),
    }
}

fn send_limit_error(
    connection: &Connection,
    request_id: RequestId,
    summary: &str,
    data: AuthoringLimitErrorData,
) -> Result<(), ServerError> {
    connection
        .sender
        .send(Message::Response(limit_error_response(
            request_id, summary, data,
        )))
        .map_err(|error| ServerError::Transport(error.to_string()))
}

fn send_protocol_error(
    connection: &Connection,
    request_id: RequestId,
    error: ProtocolError,
) -> Result<(), ServerError> {
    send_request_error(
        connection,
        request_id,
        lsp_server::ErrorCode::InvalidParams,
        &error.to_string(),
    )
}

fn send_request_error(
    connection: &Connection,
    request_id: RequestId,
    code: lsp_server::ErrorCode,
    message: &str,
) -> Result<(), ServerError> {
    connection
        .sender
        .send(Message::Response(Response::new_err(
            request_id,
            code as i32,
            message.to_owned(),
        )))
        .map_err(|error| ServerError::Transport(error.to_string()))
}

#[cfg(test)]
#[path = "../tests/internal/server.rs"]
mod tests;

/// Describe only the private initialization options, not the standard LSP envelope.
#[cfg(feature = "protocol-codegen")]
pub(super) fn initialization_options_schema(
    generator: &mut schemars::SchemaGenerator,
) -> schemars::Schema {
    generator.subschema_for::<InitializationOptions>()
}
