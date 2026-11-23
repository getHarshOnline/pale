//! Reference-only publication: one pending page, exact fences and handoff.

use crate::analysis::{ReferenceCoordinate, ReferenceOutput, ReferenceReply};
use crate::protocol::AuthoringQuery;

use super::*;

enum DeliveryPhase {
    Waiting,
    Page {
        pending: Option<Message>,
        complete: bool,
    },
    Discarding,
    Terminal(Response),
}

/// Transport state only; compilation handles and SDK cursors stay in the worker.
pub(super) struct ReferenceDelivery {
    coordinate: ReferenceCoordinate,
    partial_token: ProgressToken,
    work_token: Option<ProgressToken>,
    begun: bool,
    ended: bool,
    phase: DeliveryPhase,
    interruption: Option<Response>,
}

impl ReferenceDelivery {
    pub(super) fn for_job(job: &QueryJob) -> Option<Self> {
        let WorkspaceQuery::Authoring(AuthoringQuery::References(params)) = &job.query else {
            return None;
        };
        Some(Self {
            coordinate: ReferenceCoordinate {
                request_id: job.request_id.clone(),
                generation: job.generation,
                document_revision: job.document_revision,
                page: 0,
            },
            partial_token: params.partial_result_params.partial_result_token.clone()?,
            work_token: params.work_done_progress_params.work_done_token.clone(),
            begun: false,
            ended: false,
            phase: DeliveryPhase::Waiting,
            interruption: None,
        })
    }

    fn reply(&mut self, reply: ReferenceReply) {
        if reply.coordinate != self.coordinate {
            return;
        }
        match (&self.phase, reply.outcome) {
            (DeliveryPhase::Discarding, Ok(ReferenceOutput::Discarded)) => {
                self.phase = DeliveryPhase::Terminal(
                    self.interruption
                        .take()
                        .expect("discard follows interruption"),
                );
            }
            (DeliveryPhase::Waiting, Ok(ReferenceOutput::Page(page))) => {
                if !page.complete {
                    // The worker checked the same addition before retaining its scan.
                    self.coordinate.page = self
                        .coordinate
                        .page
                        .checked_add(1)
                        .expect("worker retained a representable page");
                }
                let pending = (!page.locations.is_empty())
                    .then(|| progress(&self.partial_token, page.locations));
                self.phase = DeliveryPhase::Page {
                    pending,
                    complete: page.complete,
                };
            }
            (DeliveryPhase::Waiting | DeliveryPhase::Discarding, Err(error)) => {
                self.phase = DeliveryPhase::Terminal(query_response(QueryResult {
                    request_id: self.coordinate.request_id.clone(),
                    generation: self.coordinate.generation,
                    document_revision: self.coordinate.document_revision,
                    outcome: Err(error),
                }));
            }
            _ => {}
        }
    }

    fn interrupt(&mut self, code: lsp_server::ErrorCode, message: &str) {
        if self.interruption.is_none() {
            self.interruption = Some(Response::new_err(
                self.coordinate.request_id.clone(),
                code as i32,
                message.to_owned(),
            ));
        }
    }

    /// Returns true only after the terminal response's actual transport handoff.
    /// A full stdio rendezvous leaves this one message pending and lets the
    /// server read cancellation/editor input. Already handed-off bytes cannot
    /// be withdrawn; no bounded stdout-write latency is claimed.
    fn pump(
        &mut self,
        connection: &Connection,
        worker: &AnalysisWorker,
    ) -> Result<bool, ServerError> {
        if self.interruption.is_some() {
            match self.phase {
                DeliveryPhase::Page {
                    complete: false, ..
                } => {
                    worker.discard_references(self.coordinate.clone())?;
                    self.phase = DeliveryPhase::Discarding;
                }
                DeliveryPhase::Page { complete: true, .. } | DeliveryPhase::Terminal(_) => {
                    self.phase = DeliveryPhase::Terminal(
                        self.interruption.take().expect("interruption checked"),
                    );
                }
                DeliveryPhase::Waiting | DeliveryPhase::Discarding => {}
            }
        }
        if !self.begun
            && self.interruption.is_none()
            && !matches!(
                self.phase,
                DeliveryPhase::Terminal(_) | DeliveryPhase::Discarding
            )
        {
            if let Some(token) = &self.work_token {
                let mut message = Some(progress(
                    token,
                    WorkDoneProgress::Begin(WorkDoneProgressBegin {
                        title: "Finding references".to_owned(),
                        cancellable: Some(true),
                        message: None,
                        percentage: None,
                    }),
                ));
                if !try_handoff(connection, &mut message)? {
                    return Ok(false);
                }
            }
            self.begun = true;
        }
        match &mut self.phase {
            DeliveryPhase::Page { pending, complete } => {
                if !try_handoff(connection, pending)? {
                    return Ok(false);
                }
                if *complete {
                    self.phase = DeliveryPhase::Terminal(Response::new_ok(
                        self.coordinate.request_id.clone(),
                        Option::<Vec<Location>>::None,
                    ));
                } else {
                    worker.continue_references(self.coordinate.clone())?;
                    self.phase = DeliveryPhase::Waiting;
                }
                Ok(false)
            }
            DeliveryPhase::Terminal(_) => {
                if self.begun && !self.ended {
                    if let Some(token) = &self.work_token {
                        let mut message = Some(progress(
                            token,
                            WorkDoneProgress::End(WorkDoneProgressEnd { message: None }),
                        ));
                        if !try_handoff(connection, &mut message)? {
                            return Ok(false);
                        }
                    }
                    self.ended = true;
                }
                // Move, rather than clone, the error/diagnostic payload on backpressure.
                let DeliveryPhase::Terminal(response) =
                    std::mem::replace(&mut self.phase, DeliveryPhase::Waiting)
                else {
                    unreachable!()
                };
                let mut pending = Some(Message::Response(response));
                if try_handoff(connection, &mut pending)? {
                    Ok(true)
                } else {
                    let Some(Message::Response(response)) = pending else {
                        unreachable!()
                    };
                    self.phase = DeliveryPhase::Terminal(response);
                    Ok(false)
                }
            }
            DeliveryPhase::Waiting | DeliveryPhase::Discarding => Ok(false),
        }
    }
}

impl ServerState {
    pub(super) fn accept_reference_reply(&mut self, reply: ReferenceReply) {
        if let Some(delivery) = &mut self.references {
            delivery.reply(reply);
        }
    }

    pub(super) fn pump_references(
        &mut self,
        connection: &Connection,
        worker: &AnalysisWorker,
    ) -> Result<(), ServerError> {
        let Some(delivery) = &mut self.references else {
            return Ok(());
        };
        if let Some(InFlight::Query { cancelled, .. }) = self.in_flight.as_ref() {
            if *cancelled {
                delivery.interrupt(
                    lsp_server::ErrorCode::RequestCanceled,
                    "reference request cancelled; handed-off partial results may be incomplete",
                );
            } else if delivery.coordinate.generation != self.clock.generation
                || delivery.coordinate.document_revision != self.documents.revision()
                || self.analyzed_generation != Some(delivery.coordinate.generation)
            {
                delivery.interrupt(
                    lsp_server::ErrorCode::ContentModified,
                    "editor state changed before reference publication",
                );
            }
        }
        if delivery.pump(connection, worker)? {
            self.references = None;
            self.in_flight = None;
        }
        Ok(())
    }
}

fn progress(token: &ProgressToken, value: impl serde::Serialize) -> Message {
    // lsp-types 0.97's ProgressParamsValue contains only work-done progress.
    // This is the standard generic progress envelope, not a custom wire family.
    Message::Notification(lsp_server::Notification::new(
        "$/progress".to_owned(),
        serde_json::json!({ "token": token, "value": value }),
    ))
}

fn try_handoff(
    connection: &Connection,
    pending: &mut Option<Message>,
) -> Result<bool, ServerError> {
    let Some(message) = pending.take() else {
        return Ok(true);
    };
    match connection.sender.try_send(message) {
        Ok(()) => Ok(true),
        Err(crossbeam_channel::TrySendError::Full(message)) => {
            *pending = Some(message);
            Ok(false)
        }
        Err(crossbeam_channel::TrySendError::Disconnected(_)) => Err(ServerError::Transport(
            "LSP output channel closed".to_owned(),
        )),
    }
}

#[cfg(test)]
#[path = "../../tests/internal/reference_transport.rs"]
mod tests;
