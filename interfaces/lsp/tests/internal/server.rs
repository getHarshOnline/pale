//! LSP server routing, cancellation, and retained-state laws.

use super::*;
use crate::syntax::AuthoringSyntaxSnapshot;
use pale_sdk::PortableWorkspaceRequest;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_SOURCE_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempSource {
    root: std::path::PathBuf,
    path: std::path::PathBuf,
    root_uri: Uri,
    uri: Uri,
}

impl TempSource {
    fn new(text: &str) -> Self {
        let sequence = TEMP_SOURCE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "pale-lsp-watched-source-{}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("pale-workspace.toml");
        std::fs::write(&path, text).unwrap();
        let root_uri = local_file_uri(&root);
        let uri = local_file_uri(&path);
        Self {
            root,
            path,
            root_uri,
            uri,
        }
    }
}

impl Drop for TempSource {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        let _ = std::fs::remove_dir(&self.root);
    }
}

fn local_file_uri(path: &std::path::Path) -> Uri {
    let path = path
        .to_string_lossy()
        .replace('%', "%25")
        .replace(' ', "%20");
    #[cfg(windows)]
    let path = path.replace('\\', "/");
    format!("file://{path}").parse().unwrap()
}

fn remote_file_uri(authority: &str, path: &std::path::Path) -> Uri {
    let path = path
        .to_string_lossy()
        .replace('%', "%25")
        .replace(' ', "%20");
    #[cfg(windows)]
    let path = path.replace('\\', "/");
    format!("vscode-remote://{authority}{path}")
        .parse()
        .unwrap()
}

fn worker_for(state: &ServerState) -> AnalysisWorker {
    AnalysisWorker::spawn(
        PaleApi::new().unwrap(),
        state.root_manifest.clone(),
        state.machine_id.clone(),
        state.documents.baseline_snapshot(),
    )
}

#[test]
fn authoring_capacity_failures_carry_bounds_and_safe_recovery() {
    let response = query_response(QueryResult {
        request_id: RequestId::from(7),
        generation: 1,
        document_revision: 1,
        outcome: Err(QueryFailure::Authoring(
            AuthoringQueryError::SyntaxFactLimitExceeded,
        )),
    });
    let error = response.response_result.unwrap_err();
    assert_eq!(error.code, lsp_server::ErrorCode::RequestFailed as i32);
    assert_eq!(
        error.data.unwrap(),
        serde_json::json!({
            "kind": "syntax-facts",
            "unit": "facts",
            "maximum": crate::syntax::MAX_AUTHORING_SYNTAX_FACTS.to_string(),
            "observedMinimum": (crate::syntax::MAX_AUTHORING_SYNTAX_FACTS + 1).to_string(),
            "recovery": "Split the configuration into smaller source documents, wait for analysis, and retry."
        })
    );
    assert!(error.message.contains("Split the configuration"));
}

#[test]
fn document_capacity_failures_preserve_exact_values_and_actionable_recovery() {
    let bytes = DocumentError::DocumentBytesExceeded {
        class: "workspace",
        observed: 1_025,
        maximum: 1_024,
    };
    assert_eq!(
        serde_json::to_value(document_limit_data(&bytes).unwrap()).unwrap(),
        serde_json::json!({
            "kind": "document-bytes",
            "unit": "bytes",
            "scope": "workspace",
            "maximum": "1024",
            "observed": "1025",
            "recovery": "Reduce this source document, then retry the editor change."
        })
    );
    assert_eq!(
        document_error_message(&bytes),
        "workspace document bytes exceed the authoring ingress limit: 1025 > 1024. Reduce this source document, then retry the editor change."
    );

    let count = DocumentError::DocumentCountExceeded {
        class: "private",
        observed: 65,
        maximum: 64,
    };
    assert!(document_error_message(&count).contains(
        "65 > 64. Remove or combine sources in this custody class, then restart the Pale language server."
    ));
}

#[test]
fn exhausted_reference_continuation_reports_restart_recovery() {
    let response = query_response(QueryResult {
        request_id: RequestId::from(8),
        generation: 1,
        document_revision: 1,
        outcome: Err(QueryFailure::Authoring(
            AuthoringQueryError::ReferencePageBound,
        )),
    });
    let error = response.response_result.unwrap_err();
    assert_eq!(
        error.data.unwrap(),
        serde_json::json!({
            "kind": "reference-pages",
            "unit": "pages",
            "maximum": u64::MAX.to_string(),
            "recovery": "Start a new reference request against the current editor generation."
        })
    );
    assert!(error
        .message
        .contains("Start a new reference request against the current editor generation."));
}

#[test]
fn saturated_query_queue_reports_its_capacity_and_retry_condition() {
    let (server_connection, client_connection) = Connection::memory();
    let input = InitializationInput {
        root_uri: "file:///repo".parse().unwrap(),
        capabilities: ClientCapabilities::default(),
        initialization_options: InitializationOptions {
            root_manifest: "pale-workspace.toml".to_owned(),
            machine_id: None,
            documents: Vec::new(),
        },
    };
    let mut state = ServerState::new(input).unwrap();
    for id in 0..MAX_PENDING_QUERIES {
        state
            .queue_query(
                RequestId::from(i32::try_from(id).unwrap()),
                WorkspaceQuery::CompilationSummary,
            )
            .unwrap();
    }
    handle_workspace_request(
        &server_connection,
        &mut state,
        Request::new(
            RequestId::from(100),
            crate::protocol::COMPILATION_SUMMARY_METHOD.to_owned(),
            (),
        ),
    )
    .unwrap();
    let Message::Response(response) = client_connection.receiver.recv().unwrap() else {
        panic!("queue saturation must return a response");
    };
    let error = response.response_result.unwrap_err();
    assert_eq!(
        error.data.unwrap(),
        serde_json::json!({
            "kind": "query-queue",
            "unit": "requests",
            "maximum": MAX_PENDING_QUERIES.to_string(),
            "observedMinimum": (MAX_PENDING_QUERIES + 1).to_string(),
            "recovery": "Wait for one Pale request to finish, then retry against the current editor generation."
        })
    );
    assert!(error
        .message
        .contains("retry against the current editor generation"));
}

#[test]
fn advertised_surface_includes_only_implemented_standard_authoring_features() {
    let capabilities = ServerState::capabilities();
    assert_eq!(
        capabilities.position_encoding,
        Some(PositionEncodingKind::UTF16)
    );
    let Some(TextDocumentSyncCapability::Options(sync)) = capabilities.text_document_sync else {
        panic!("server must advertise explicit open/close and incremental sync");
    };
    assert_eq!(sync.open_close, Some(true));
    assert_eq!(sync.change, Some(TextDocumentSyncKind::INCREMENTAL));
    assert_eq!(
        sync.save,
        Some(
            SaveOptions {
                include_text: Some(true)
            }
            .into()
        )
    );
    assert!(capabilities.definition_provider.is_some());
    assert!(capabilities.references_provider.is_some());
    assert!(capabilities.code_action_provider.is_some());
    assert!(capabilities.completion_provider.is_some());
    assert!(capabilities.hover_provider.is_some());
    assert!(capabilities.document_symbol_provider.is_some());
    assert!(capabilities.semantic_tokens_provider.is_some());
    assert!(capabilities.rename_provider.is_none());
    assert!(capabilities.document_formatting_provider.is_none());
    assert_eq!(
        capabilities.experimental,
        Some(crate::protocol::experimental_capabilities())
    );
}

#[test]
fn accepted_analysis_publishes_its_exact_editor_coordinate() {
    let (server_connection, client_connection) = Connection::memory();
    let input = InitializationInput {
        root_uri: "file:///repo".parse().unwrap(),
        capabilities: ClientCapabilities::default(),
        initialization_options: InitializationOptions {
            root_manifest: "pale-workspace.toml".to_owned(),
            machine_id: None,
            documents: Vec::new(),
        },
    };
    let state = ServerState::new(input).unwrap();
    let publication = AnalysisResult {
        generation: 17,
        snapshot: state.documents.snapshot(),
        syntax: AuthoringSyntaxSnapshot::default(),
        outcome: Ok(()),
    };

    publish_analysis_state(&server_connection, 23, &publication).unwrap();

    let Message::Notification(notification) = client_connection.receiver.recv().unwrap() else {
        panic!("analysis publication must be a notification");
    };
    assert_eq!(
        notification.method,
        crate::protocol::ANALYSIS_PUBLISHED_NOTIFICATION
    );
    assert_eq!(
        notification.params,
        serde_json::json!({
            "editor": {"generation": "17", "documentRevision": "23"},
            "admission": "admitted"
        })
    );
}

#[test]
fn save_notification_advances_close_fallback_without_rescheduling_analysis() {
    let (server_connection, _client_connection) = Connection::memory();
    let input = InitializationInput {
        root_uri: "file:///repo".parse().unwrap(),
        capabilities: ClientCapabilities::default(),
        initialization_options: InitializationOptions {
            root_manifest: "pale-workspace.toml".to_owned(),
            machine_id: None,
            documents: vec![BaselineDocument {
                uri: "file:///repo/pale-workspace.toml".parse().unwrap(),
                language_id: "pale-toml".to_owned(),
                role: crate::document::DocumentRole::WorkspaceManifest,
                text: "startup".to_owned(),
            }],
        },
    };
    let mut state = ServerState::new(input).unwrap();
    let document_uri: Uri = "file:///repo/pale-workspace.toml".parse().unwrap();

    handle_editor_notification(
        &server_connection,
        &mut state,
        lsp_server::Notification::new(
            DidOpenTextDocument::METHOD.to_owned(),
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: document_uri.clone(),
                    language_id: "pale-toml".to_owned(),
                    version: 1,
                    text: "saved".to_owned(),
                },
            },
        ),
    )
    .unwrap();
    let analysis_generation = state.clock.generation;
    handle_editor_notification(
        &server_connection,
        &mut state,
        lsp_server::Notification::new(
            DidSaveTextDocument::METHOD.to_owned(),
            DidSaveTextDocumentParams {
                text_document: TextDocumentIdentifier {
                    uri: document_uri.clone(),
                },
                text: Some("saved".to_owned()),
            },
        ),
    )
    .unwrap();
    assert_eq!(state.clock.generation, analysis_generation);

    state
        .documents
        .change(
            &document_uri,
            2,
            vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "unsaved".to_owned(),
            }],
        )
        .unwrap();
    state.documents.close(&document_uri).unwrap();
    assert_eq!(state.documents.snapshot().documents()[0].text(), "saved");
    assert_eq!(
        state.documents.baseline_snapshot().documents()[0].text(),
        "startup"
    );
}

#[test]
fn watched_file_change_delete_recreate_and_stale_result_are_fenced() {
    let source = TempSource::new("startup");
    let (server_connection, client_connection) = Connection::memory();
    let input = InitializationInput {
        root_uri: source.root_uri.clone(),
        capabilities: ClientCapabilities::default(),
        initialization_options: InitializationOptions {
            root_manifest: "pale-workspace.toml".to_owned(),
            machine_id: None,
            documents: vec![BaselineDocument {
                uri: source.uri.clone(),
                language_id: "pale-toml".to_owned(),
                role: crate::document::DocumentRole::WorkspaceManifest,
                text: "startup".to_owned(),
            }],
        },
    };
    let start = Instant::now();
    let mut state = ServerState::new(input).unwrap();
    state.schedule(start).unwrap();
    let first = state
        .claim_due(start + crate::scheduling::ANALYSIS_DEBOUNCE)
        .unwrap();

    std::fs::write(&source.path, "external").unwrap();
    handle_editor_notification(
        &server_connection,
        &mut state,
        lsp_server::Notification::new(
            DidChangeWatchedFiles::METHOD.to_owned(),
            DidChangeWatchedFilesParams {
                changes: vec![FileEvent::new(source.uri.clone(), FileChangeType::CHANGED)],
            },
        ),
    )
    .unwrap();
    assert_eq!(state.documents.snapshot().documents()[0].text(), "external");
    let stale = AnalysisResult {
        generation: first.generation,
        snapshot: first.snapshot,
        syntax: AuthoringSyntaxSnapshot::default(),
        outcome: Ok(()),
    };
    assert!(!state.accept_analysis(&stale));
    let changed_generation = state.clock.generation;

    handle_editor_notification(
        &server_connection,
        &mut state,
        lsp_server::Notification::new(
            DidChangeWatchedFiles::METHOD.to_owned(),
            DidChangeWatchedFilesParams {
                changes: vec![
                    FileEvent::new(source.uri.clone(), FileChangeType::DELETED),
                    FileEvent::new(source.uri.clone(), FileChangeType::CREATED),
                ],
            },
        ),
    )
    .unwrap();
    assert_eq!(state.clock.generation, changed_generation);

    std::fs::remove_file(&source.path).unwrap();
    handle_editor_notification(
        &server_connection,
        &mut state,
        lsp_server::Notification::new(
            DidChangeWatchedFiles::METHOD.to_owned(),
            DidChangeWatchedFilesParams {
                changes: vec![FileEvent::new(source.uri.clone(), FileChangeType::DELETED)],
            },
        ),
    )
    .unwrap();
    assert!(state.documents.snapshot().documents().is_empty());
    let Message::Notification(failure) = client_connection.receiver.recv().unwrap() else {
        panic!("a missing configured source must be surfaced");
    };
    assert_eq!(failure.method, notification::LogMessage::METHOD);
    assert!(failure.params["message"]
        .as_str()
        .unwrap()
        .contains("configured Pale source is unavailable"));

    std::fs::write(&source.path, "recreated").unwrap();
    handle_editor_notification(
        &server_connection,
        &mut state,
        lsp_server::Notification::new(
            DidChangeWatchedFiles::METHOD.to_owned(),
            DidChangeWatchedFilesParams {
                changes: vec![FileEvent::new(source.uri.clone(), FileChangeType::CREATED)],
            },
        ),
    )
    .unwrap();
    assert_eq!(
        state.documents.snapshot().documents()[0].text(),
        "recreated"
    );

    std::fs::write(&source.path, [0xff]).unwrap();
    handle_editor_notification(
        &server_connection,
        &mut state,
        lsp_server::Notification::new(
            DidChangeWatchedFiles::METHOD.to_owned(),
            DidChangeWatchedFilesParams {
                changes: vec![FileEvent::new(source.uri.clone(), FileChangeType::CHANGED)],
            },
        ),
    )
    .unwrap();
    assert!(state.documents.snapshot().documents().is_empty());
    let Message::Notification(failure) = client_connection.receiver.recv().unwrap() else {
        panic!("an unreadable configured source must be surfaced");
    };
    assert_eq!(failure.method, notification::LogMessage::METHOD);
}

#[test]
fn watched_remote_source_maps_only_the_registered_extension_host() {
    let source = TempSource::new("remote");
    let authority = "ssh-remote+example";
    let root_uri = remote_file_uri(authority, &source.root);
    let uri = remote_file_uri(authority, &source.path);
    let store = DocumentStore::new(
        &root_uri,
        vec![BaselineDocument {
            uri: uri.clone(),
            language_id: "pale-toml".to_owned(),
            role: crate::document::DocumentRole::WorkspaceManifest,
            text: "remote".to_owned(),
        }],
    )
    .unwrap();

    assert_eq!(read_watched_source(&store, &uri).unwrap(), "remote");
    let unlike = remote_file_uri("ssh-remote+other", &source.path);
    assert!(registered_host_path(&store, &unlike)
        .unwrap_err()
        .contains("not registered"));
}

#[test]
fn byte_spans_convert_to_exact_utf16_ranges() {
    let text = "a😀b\nnext";
    assert_eq!(
        utf16_position_at_byte(text, "a😀".len()),
        Some(Position::new(0, 3))
    );
    assert_eq!(
        utf16_position_at_byte(text, "a😀b\n".len()),
        Some(Position::new(1, 0))
    );
}

#[test]
fn scalar_columns_convert_to_utf16_without_copying_units() {
    let text = "😀x";
    let byte = scalar_position_to_byte(text, 0, 1).unwrap();
    assert_eq!(byte, "😀".len());
    assert_eq!(
        utf16_position_at_byte(text, byte),
        Some(Position::new(0, 2))
    );
}

#[test]
fn lsp_data_preserves_the_complete_api_owned_diagnostic_with_opaque_identity() {
    let api = PaleApi::new().unwrap();
    let error = api
        .compile_portable_workspace(PortableWorkspaceRequest::new(
            "pale-workspace.toml".to_owned(),
            Vec::new(),
        ))
        .unwrap_err();
    let diagnostic = error.diagnostic();
    let projected = project_diagnostic(diagnostic, "", None);
    assert_eq!(
        projected.data,
        Some(serde_json::json!({
            "identity": diagnostic.identity(),
            "diagnostic": serde_json::from_slice::<serde_json::Value>(
                diagnostic.canonical_bytes()
            ).unwrap(),
        }))
    );
    assert_eq!(
        projected.code,
        Some(NumberOrString::String(diagnostic.code().to_owned()))
    );
    assert_eq!(projected.message, diagnostic.summary());
    assert!(diagnostic_targets_document(
        diagnostic,
        "pale-workspace.toml",
        "pale-workspace.toml"
    ));
    assert!(!diagnostic_targets_document(
        diagnostic,
        "pale-workspace.toml",
        "config/other.toml"
    ));
}

#[test]
fn worker_keeps_only_one_compile_in_flight() {
    let input = InitializationInput {
        root_uri: "file:///repo".parse().unwrap(),
        capabilities: ClientCapabilities::default(),
        initialization_options: InitializationOptions {
            root_manifest: "pale-workspace.toml".to_owned(),
            machine_id: None,
            documents: Vec::new(),
        },
    };
    let mut state = ServerState::new(input).unwrap();
    state.schedule(Instant::now()).unwrap();
    let first = state.pending_analysis.take().unwrap();
    state.in_flight = Some(InFlight::Analysis {
        generation: first.generation,
    });
    state.schedule(Instant::now()).unwrap();
    assert!(state
        .claim_due(Instant::now() + crate::scheduling::ANALYSIS_DEBOUNCE)
        .is_none());
    assert!(state.pending_analysis.is_some());
}

#[test]
fn stale_result_is_suppressed_before_latest_pending_work_runs() {
    let input = InitializationInput {
        root_uri: "file:///repo".parse().unwrap(),
        capabilities: ClientCapabilities::default(),
        initialization_options: InitializationOptions {
            root_manifest: "pale-workspace.toml".to_owned(),
            machine_id: None,
            documents: Vec::new(),
        },
    };
    let start = Instant::now();
    let mut state = ServerState::new(input).unwrap();
    state.schedule(start).unwrap();
    let first = state
        .claim_due(start + crate::scheduling::ANALYSIS_DEBOUNCE)
        .unwrap();
    state
        .schedule(start + crate::scheduling::ANALYSIS_DEBOUNCE)
        .unwrap();
    let stale = AnalysisResult {
        generation: first.generation,
        snapshot: first.snapshot,
        syntax: AuthoringSyntaxSnapshot::default(),
        outcome: Ok(()),
    };
    assert!(!state.accept_analysis(&stale));
    let latest = state
        .claim_due(
            start + crate::scheduling::ANALYSIS_DEBOUNCE + crate::scheduling::ANALYSIS_DEBOUNCE,
        )
        .unwrap();
    assert!(latest.generation > stale.generation);
}

#[test]
fn pending_and_in_flight_queries_have_honest_cancellation() {
    let input = InitializationInput {
        root_uri: "file:///repo".parse().unwrap(),
        capabilities: ClientCapabilities::default(),
        initialization_options: InitializationOptions {
            root_manifest: "pale-workspace.toml".to_owned(),
            machine_id: None,
            documents: Vec::new(),
        },
    };
    let mut state = ServerState::new(input).unwrap();
    state.clock.generation = 1;
    state.analyzed_generation = Some(1);

    let pending = RequestId::from(7);
    state
        .queue_query(pending.clone(), WorkspaceQuery::CompilationSummary)
        .unwrap();
    assert!(matches!(
        state.cancel_query(&pending),
        CancellationDisposition::Pending
    ));

    let in_flight = RequestId::from(8);
    state
        .queue_query(in_flight.clone(), WorkspaceQuery::CompilationSummary)
        .unwrap();
    let job = state.claim_query().unwrap();
    assert!(matches!(
        state.cancel_query(&in_flight),
        CancellationDisposition::InFlight
    ));
    let result = QueryResult {
        request_id: job.request_id,
        generation: job.generation,
        document_revision: job.document_revision,
        outcome: Err(QueryFailure::AnalysisUnavailable),
    };
    assert!(matches!(
        state.accept_query(&result),
        QueryAcceptance::Cancelled
    ));
}

#[test]
fn editor_generation_supersedes_only_queued_query_results() {
    let input = InitializationInput {
        root_uri: "file:///repo".parse().unwrap(),
        capabilities: ClientCapabilities::default(),
        initialization_options: InitializationOptions {
            root_manifest: "pale-workspace.toml".to_owned(),
            machine_id: None,
            documents: Vec::new(),
        },
    };
    let mut state = ServerState::new(input).unwrap();
    state.clock.generation = 1;
    state.analyzed_generation = Some(1);
    let request_id = RequestId::from(9);
    state
        .queue_query(request_id.clone(), WorkspaceQuery::CompilationSummary)
        .unwrap();
    assert_eq!(state.schedule(Instant::now()).unwrap(), vec![request_id]);
}

#[test]
fn queued_cancellation_returns_a_json_rpc_terminal_response() {
    let (server_connection, client_connection) = Connection::memory();
    let input = InitializationInput {
        root_uri: "file:///repo".parse().unwrap(),
        capabilities: ClientCapabilities::default(),
        initialization_options: InitializationOptions {
            root_manifest: "pale-workspace.toml".to_owned(),
            machine_id: None,
            documents: Vec::new(),
        },
    };
    let mut state = ServerState::new(input).unwrap();
    let worker = worker_for(&state);
    client_connection
        .sender
        .send(Message::Request(Request::new(
            RequestId::from(41),
            crate::protocol::COMPILATION_SUMMARY_METHOD.to_owned(),
            (),
        )))
        .unwrap();
    client_connection
        .sender
        .send(Message::Notification(lsp_server::Notification::new(
            Cancel::METHOD.to_owned(),
            CancelParams {
                id: NumberOrString::Number(41),
            },
        )))
        .unwrap();
    client_connection
        .sender
        .send(Message::Request(Request::new(
            RequestId::from(42),
            Shutdown::METHOD.to_owned(),
            (),
        )))
        .unwrap();
    client_connection
        .sender
        .send(Message::Notification(lsp_server::Notification::new(
            notification::Exit::METHOD.to_owned(),
            (),
        )))
        .unwrap();

    serve(&server_connection, &worker, &mut state).unwrap();
    let responses = [
        client_connection.receiver.recv().unwrap(),
        client_connection.receiver.recv().unwrap(),
    ];
    let cancelled = responses.into_iter().find_map(|message| match message {
        Message::Response(response) if response.id == RequestId::from(41) => Some(response),
        _ => None,
    });
    let response = cancelled.expect("cancelled request must receive one terminal response");
    assert_eq!(
        response.response_result.unwrap_err().code,
        lsp_server::ErrorCode::RequestCanceled as i32
    );
    worker.stop().unwrap();
}

#[test]
fn initialization_options_are_typed_and_closed() {
    let input: InitializationInput = serde_json::from_value(serde_json::json!({
        "rootUri": "file:///repo",
        "initializationOptions": {
            "rootManifest": "pale-workspace.toml",
            "documents": [{
                "uri": "file:///repo/pale-workspace.toml",
                "languageId": "pale-toml",
                "role": "workspace-manifest",
                "text": "[workspace]"
            }]
        }
    }))
    .unwrap();
    assert_eq!(input.initialization_options.documents.len(), 1);

    assert!(
        serde_json::from_value::<InitializationInput>(serde_json::json!({
            "rootUri": "file:///repo",
            "initializationOptions": {
                "rootManifest": "pale-workspace.toml",
                "documents": [],
                "filesystemRoot": "/repo"
            }
        }))
        .is_err()
    );
}

#[test]
fn shutdown_requires_and_then_accepts_exit() {
    let (server_connection, client_connection) = Connection::memory();
    let input = InitializationInput {
        root_uri: "file:///repo".parse().unwrap(),
        capabilities: ClientCapabilities::default(),
        initialization_options: InitializationOptions {
            root_manifest: "pale-workspace.toml".to_owned(),
            machine_id: None,
            documents: Vec::new(),
        },
    };
    let mut state = ServerState::new(input).unwrap();
    let worker = worker_for(&state);
    client_connection
        .sender
        .send(Message::Request(Request::new(
            lsp_server::RequestId::from(1),
            "shutdown".to_owned(),
            (),
        )))
        .unwrap();
    client_connection
        .sender
        .send(Message::Notification(lsp_server::Notification::new(
            notification::Exit::METHOD.to_owned(),
            (),
        )))
        .unwrap();
    serve(&server_connection, &worker, &mut state).unwrap();
    let Message::Response(response) = client_connection.receiver.recv().unwrap() else {
        panic!("shutdown must return a response");
    };
    assert!(response.response_result.is_ok());
    worker.stop().unwrap();
}

#[test]
fn exit_before_shutdown_is_rejected() {
    let (server_connection, client_connection) = Connection::memory();
    let input = InitializationInput {
        root_uri: "file:///repo".parse().unwrap(),
        capabilities: ClientCapabilities::default(),
        initialization_options: InitializationOptions {
            root_manifest: "pale-workspace.toml".to_owned(),
            machine_id: None,
            documents: Vec::new(),
        },
    };
    let mut state = ServerState::new(input).unwrap();
    let worker = worker_for(&state);
    client_connection
        .sender
        .send(Message::Notification(lsp_server::Notification::new(
            notification::Exit::METHOD.to_owned(),
            (),
        )))
        .unwrap();
    assert!(matches!(
        serve(&server_connection, &worker, &mut state),
        Err(ServerError::ExitBeforeShutdown)
    ));
    worker.stop().unwrap();
}
