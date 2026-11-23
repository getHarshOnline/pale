//! Real worker/transport composition and exact rendezvous handoff races.

use super::*;

fn start(work_done: bool) -> (ServerState, AnalysisWorker) {
    let (store, mut params) = crate::authoring::references::tests::reference_store(300);
    if work_done {
        params.work_done_progress_params.work_done_token =
            Some(ProgressToken::String("work".to_owned()));
    }
    let mut state = ServerState::new(InitializationInput {
        root_uri: "file:///workspace".parse().unwrap(),
        capabilities: ClientCapabilities::default(),
        initialization_options: InitializationOptions {
            root_manifest: "pale-workspace.toml".to_owned(),
            machine_id: None,
            documents: Vec::new(),
        },
    })
    .unwrap();
    state.documents = store;
    let worker = AnalysisWorker::spawn(
        PaleApi::new().unwrap(),
        state.root_manifest.clone(),
        None,
        state.documents.baseline_snapshot(),
    );
    let now = Instant::now();
    state.schedule(now).unwrap();
    worker
        .analyze(
            state
                .claim_due(now + crate::scheduling::ANALYSIS_DEBOUNCE)
                .unwrap(),
        )
        .unwrap();
    let WorkerResult::Analysis(analysis) = worker
        .results
        .recv_timeout(Duration::from_secs(30))
        .unwrap()
    else {
        panic!("analysis");
    };
    assert!(analysis.outcome.is_ok());
    assert!(state.accept_analysis(&analysis));
    state
        .queue_query(
            RequestId::from(91),
            WorkspaceQuery::Authoring(AuthoringQuery::References(params)),
        )
        .unwrap();
    worker.query(state.claim_query().unwrap()).unwrap();
    (state, worker)
}

fn accept_page(state: &mut ServerState, worker: &AnalysisWorker) {
    let WorkerResult::References(reply) = worker
        .results
        .recv_timeout(Duration::from_secs(30))
        .unwrap()
    else {
        panic!("reference result");
    };
    assert!(reply.outcome.is_ok());
    state.accept_reference_reply(*reply);
}

#[test]
fn streamed_wire_is_complete_unique_and_ends_with_null_after_work_done() {
    let (mut state, worker) = start(true);
    let (connection, client) = Connection::memory();
    let mut wire = Vec::new();
    for _ in 0..500 {
        if matches!(
            state.references.as_ref().map(|delivery| &delivery.phase),
            Some(DeliveryPhase::Waiting)
        ) {
            accept_page(&mut state, &worker);
        }
        state.pump_references(&connection, &worker).unwrap();
        wire.extend(client.receiver.try_iter());
        if state.references.is_none() {
            break;
        }
    }
    assert!(state.references.is_none() && state.in_flight.is_none());
    let mut locations = Vec::new();
    let mut kinds = Vec::new();
    let mut responses = 0;
    for message in &wire {
        match message {
            Message::Notification(notification) => {
                assert_eq!(notification.method, "$/progress");
                if notification.params["token"] == "work" {
                    assert!(notification.params["value"].get("percentage").is_none());
                    kinds.push(notification.params["value"]["kind"].as_str().unwrap());
                } else {
                    assert_eq!(notification.params["token"], "references");
                    let page: Vec<Location> =
                        serde_json::from_value(notification.params["value"].clone()).unwrap();
                    assert!(!page.is_empty() && page.len() <= 256);
                    locations.extend(page);
                }
            }
            Message::Response(response) => {
                responses += 1;
                assert_eq!(response.id, RequestId::from(91));
                assert_eq!(
                    response.response_result.as_ref().unwrap(),
                    &serde_json::Value::Null
                );
            }
            _ => panic!("only progress and final response"),
        }
    }
    assert_eq!(kinds, ["begin", "end"]);
    assert!(matches!(wire.last(), Some(Message::Response(_))));
    assert_eq!(responses, 1);
    assert_eq!(locations.len(), 300);
    let unique = locations
        .iter()
        .map(|location| {
            (
                location.uri.as_str(),
                location.range.start.line,
                location.range.start.character,
                location.range.end.line,
                location.range.end.character,
            )
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), 300);
    worker.stop().unwrap();
}

#[test]
fn stdio_rendezvous_backpressure_allows_cancel_before_unsent_page_handoff() {
    let (mut state, worker) = start(false);
    accept_page(&mut state, &worker);
    // Same zero-capacity outgoing channel as pinned lsp-server's stdio owner.
    // This exercises the exact handoff, not a claim about OS stdout latency.
    let (sender, writer) = crossbeam_channel::bounded(0);
    let (_input, receiver) = crossbeam_channel::bounded(0);
    let connection = Connection { sender, receiver };
    state.pump_references(&connection, &worker).unwrap();
    assert!(matches!(
        state.references.as_ref().unwrap().phase,
        DeliveryPhase::Page {
            pending: Some(_),
            complete: false
        }
    ));
    assert!(
        worker.results.try_recv().is_err(),
        "no next page while prior output is blocked"
    );
    assert!(matches!(
        state.cancel_query(&RequestId::from(91)),
        CancellationDisposition::InFlight
    ));
    state.pump_references(&connection, &worker).unwrap();
    assert!(matches!(
        state.references.as_ref().unwrap().phase,
        DeliveryPhase::Discarding
    ));
    accept_page(&mut state, &worker);
    let writer_thread =
        std::thread::spawn(move || writer.recv_timeout(Duration::from_secs(10)).unwrap());
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        state.pump_references(&connection, &worker).unwrap();
        if state.references.is_none() {
            break;
        }
        std::thread::yield_now();
    }
    assert!(state.references.is_none());
    let Message::Response(response) = writer_thread.join().unwrap() else {
        panic!("unsent reference page must be discarded");
    };
    assert_eq!(
        response.response_result.unwrap_err().code,
        lsp_server::ErrorCode::RequestCanceled as i32
    );
    worker.stop().unwrap();
}

#[test]
fn handed_off_page_survives_but_cancel_stale_and_shutdown_stop_later_publication() {
    for interruption in ["cancel", "stale", "shutdown"] {
        let (mut state, worker) = start(false);
        let (connection, client) = Connection::memory();
        accept_page(&mut state, &worker);
        state.pump_references(&connection, &worker).unwrap();
        let Message::Notification(first) = client.receiver.recv().unwrap() else {
            panic!("actual first handoff");
        };
        assert!(!first.params["value"].as_array().unwrap().is_empty());
        // The next page may already be computing; its publication is fenced.
        match interruption {
            "cancel" => {
                state.cancel_query(&RequestId::from(91));
            }
            "stale" => {
                state.schedule(Instant::now()).unwrap();
            }
            "shutdown" => {
                assert!(state.cancel_pending().is_empty());
            }
            _ => unreachable!(),
        }
        state.pump_references(&connection, &worker).unwrap();
        assert!(matches!(
            state.references.as_ref().unwrap().phase,
            DeliveryPhase::Waiting
        ));
        assert!(
            client.receiver.try_recv().is_err(),
            "waiting continuation cannot publish after interruption"
        );
        accept_page(&mut state, &worker);
        state.pump_references(&connection, &worker).unwrap();
        if matches!(
            state.references.as_ref().unwrap().phase,
            DeliveryPhase::Discarding
        ) {
            accept_page(&mut state, &worker);
        }
        for _ in 0..5 {
            state.pump_references(&connection, &worker).unwrap();
        }
        let messages = client.receiver.try_iter().collect::<Vec<_>>();
        assert_eq!(messages.len(), 1);
        let Message::Response(response) = &messages[0] else {
            panic!("no later reference page");
        };
        assert_eq!(
            response.response_result.as_ref().unwrap_err().code,
            if interruption == "stale" {
                lsp_server::ErrorCode::ContentModified
            } else {
                lsp_server::ErrorCode::RequestCanceled
            } as i32
        );
        assert!(state.references.is_none());
        worker.stop().unwrap();
    }
}

#[test]
fn another_reference_request_waits_in_existing_queue_without_overwriting_scan() {
    let (mut state, worker) = start(false);
    let (_, mut params) = crate::authoring::references::tests::reference_store(1);
    params.partial_result_params.partial_result_token =
        Some(ProgressToken::String("second".to_owned()));
    state
        .queue_query(
            RequestId::from(92),
            WorkspaceQuery::Authoring(AuthoringQuery::References(params)),
        )
        .unwrap();
    assert!(state.claim_query().is_none());
    assert_eq!(state.pending_queries.len(), 1);
    assert_eq!(
        state.references.as_ref().unwrap().coordinate.request_id,
        RequestId::from(91)
    );
    state.cancel_query(&RequestId::from(91));
    accept_page(&mut state, &worker);
    let (connection, _client) = Connection::memory();
    state.pump_references(&connection, &worker).unwrap();
    accept_page(&mut state, &worker);
    for _ in 0..4 {
        state.pump_references(&connection, &worker).unwrap();
    }
    let second = state.claim_query().unwrap();
    assert_eq!(second.request_id, RequestId::from(92));
    assert_eq!(
        state.references.as_ref().unwrap().partial_token,
        ProgressToken::String("second".to_owned())
    );
    worker.stop().unwrap();
}

#[test]
fn foreign_page_cannot_complete_or_advance_the_active_transport_request() {
    let (mut state, worker) = start(false);
    let expected = state.references.as_ref().unwrap().coordinate.clone();
    for coordinate in [
        ReferenceCoordinate {
            request_id: RequestId::from(999),
            ..expected.clone()
        },
        ReferenceCoordinate {
            generation: expected.generation + 1,
            ..expected.clone()
        },
        ReferenceCoordinate {
            document_revision: expected.document_revision + 1,
            ..expected.clone()
        },
        ReferenceCoordinate {
            page: expected.page + 1,
            ..expected.clone()
        },
    ] {
        state.accept_reference_reply(ReferenceReply {
            coordinate,
            outcome: Ok(ReferenceOutput::Page(
                crate::authoring::references::ReferencePage {
                    locations: Vec::new(),
                    complete: true,
                },
            )),
        });
        assert_eq!(state.references.as_ref().unwrap().coordinate, expected);
        assert!(matches!(
            state.references.as_ref().unwrap().phase,
            DeliveryPhase::Waiting
        ));
    }
    state.cancel_query(&expected.request_id);
    accept_page(&mut state, &worker);
    let (connection, _client) = Connection::memory();
    state.pump_references(&connection, &worker).unwrap();
    accept_page(&mut state, &worker);
    for _ in 0..4 {
        state.pump_references(&connection, &worker).unwrap();
    }
    assert!(state.references.is_none());
    worker.stop().unwrap();
}

#[test]
fn cancellation_while_waiting_for_first_page_publishes_no_progress() {
    let (mut state, worker) = start(true);
    let (connection, client) = Connection::memory();
    state.cancel_query(&RequestId::from(91));
    state.pump_references(&connection, &worker).unwrap();
    assert!(matches!(
        state.references.as_ref().unwrap().phase,
        DeliveryPhase::Waiting
    ));
    assert!(client.receiver.try_recv().is_err());
    accept_page(&mut state, &worker);
    state.pump_references(&connection, &worker).unwrap();
    assert!(matches!(
        state.references.as_ref().unwrap().phase,
        DeliveryPhase::Discarding
    ));
    accept_page(&mut state, &worker);
    for _ in 0..4 {
        state.pump_references(&connection, &worker).unwrap();
    }
    let messages = client.receiver.try_iter().collect::<Vec<_>>();
    assert_eq!(messages.len(), 1);
    let Message::Response(response) = &messages[0] else {
        panic!("cancel must not begin work progress");
    };
    assert_eq!(
        response.response_result.as_ref().unwrap_err().code,
        lsp_server::ErrorCode::RequestCanceled as i32
    );
    assert!(state.references.is_none());
    worker.stop().unwrap();
}

#[test]
fn cancellation_after_final_page_before_terminal_handoff_replaces_null_success() {
    let (mut state, worker) = start(true);
    let (connection, client) = Connection::memory();
    let mut reached_final = false;
    for _ in 0..100 {
        accept_page(&mut state, &worker);
        let final_page = matches!(
            state.references.as_ref().unwrap().phase,
            DeliveryPhase::Page { complete: true, .. }
        );
        state.pump_references(&connection, &worker).unwrap();
        if final_page {
            reached_final = true;
            break;
        }
    }
    assert!(reached_final);
    assert!(matches!(
        state.references.as_ref().unwrap().phase,
        DeliveryPhase::Terminal(_)
    ));
    let published = client.receiver.try_iter().collect::<Vec<_>>();
    assert!(published
        .iter()
        .all(|message| matches!(message, Message::Notification(_))));
    assert_eq!(
        published
            .iter()
            .filter_map(|message| match message {
                Message::Notification(value) if value.params["token"] == "references" =>
                    Some(value.params["value"].as_array().unwrap().len()),
                _ => None,
            })
            .sum::<usize>(),
        300
    );
    state.cancel_query(&RequestId::from(91));
    state.pump_references(&connection, &worker).unwrap();
    let remaining = client.receiver.try_iter().collect::<Vec<_>>();
    assert_eq!(remaining.len(), 2);
    let Message::Notification(end) = &remaining[0] else {
        panic!("work-done end first");
    };
    assert_eq!(end.params["value"]["kind"], "end");
    let Message::Response(response) = &remaining[1] else {
        panic!("one cancellation terminal");
    };
    assert_eq!(
        response.response_result.as_ref().unwrap_err().code,
        lsp_server::ErrorCode::RequestCanceled as i32
    );
    assert!(state.references.is_none());
    worker.stop().unwrap();
}
