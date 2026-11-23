//! Single-owner compilation, comparison, and proof-query worker.
//!
//! The worker serializes all calls into [`pale_sdk`]. It retains the opaque
//! API handles required for later proof queries, but neither inspects their
//! internals nor creates a second semantic cache.

use std::thread::{self, JoinHandle};

use lsp_server::RequestId;
use pale_sdk::{
    AuthoringSourceLocation, CompilationDocument, ConfigurationImprovementExplanation,
    ConfigurationImprovementPreview, ConfigurationImprovementProposalRequest, DiagnosticV1,
    EffectiveProfileCompilation, EffectiveProfileCompilationRequest, EffectiveProfileCoordinates,
    EffectiveProfilePageRequest, OneShotPrivateInputs, PackageDistributionRequest, PaleApi,
    PortableWorkspaceComparison, PortableWorkspaceCompilation, PortableWorkspaceDocument,
    PortableWorkspaceRequest,
};

use crate::document::{DocumentRole, Snapshot};
use crate::protocol::{
    ChangeRootPage, ConfigurationImprovementDetailQuery, ConfigurationImprovementPreviewProjection,
    ConfigurationImprovementQuery, ConfigurationImprovementResponse,
    ConfigurationImprovementSourceProjection, EditorCoordinate, EffectiveProfileExplanationQuery,
    EffectiveProfileExplanationResponse, EffectiveProfileQuery, EffectiveProfileResponse,
    EffectiveProfileResult, EffectiveProfileState, Explanation, ImpactPage,
    ImprovementPreviewCoordinate, ImprovementReviewDisposition, ProfileContextsResponse,
    ProfileInspectionContext, WorkspaceQuery, WorkspaceQueryResult,
};
use crate::syntax::{AuthoringSyntax, AuthoringSyntaxSnapshot};

/// Maximum rows read from either retained review partition in one LSP query.
const MAX_IMPROVEMENT_DETAIL_ROWS: u16 = 64;

/// One immutable current-snapshot compilation job.
#[derive(Debug)]
pub(super) struct AnalysisJob {
    /// Monotone editor generation represented by [`Self::snapshot`].
    pub(super) generation: u64,
    /// Exact input captured by [`crate::document::DocumentStore::snapshot`].
    pub(super) snapshot: Snapshot,
}

/// One analysis result bound to its exact editor generation.
pub(super) struct AnalysisResult {
    /// Generation compiled by the worker.
    pub(super) generation: u64,
    /// Snapshot used for diagnostics and stale-result checking.
    pub(super) snapshot: Snapshot,
    /// Tolerant source-local structure for exact diagnostic ranges.
    pub(super) syntax: AuthoringSyntaxSnapshot,
    /// Current compiler/comparison result. Successful opaque handles remain in
    /// the worker and are not copied into the transport state.
    pub(super) outcome: Result<(), AnalysisFailures>,
}

/// Deterministically ordered independent source-admission failures.
#[derive(Debug)]
pub(super) struct AnalysisFailures {
    failures: Vec<pale_sdk::ApiError>,
}

impl AnalysisFailures {
    /// Borrow every owner diagnostic without flattening their source families.
    pub(super) fn iter(&self) -> impl Iterator<Item = &pale_sdk::ApiError> {
        self.failures.iter()
    }
}

/// One typed query bound to an already analyzed editor generation.
#[derive(Debug)]
pub(super) struct QueryJob {
    /// JSON-RPC request identity used only for response correlation.
    pub(super) request_id: RequestId,
    /// Exact editor generation the caller observed.
    pub(super) generation: u64,
    /// Exact document-store revision observed when the query was queued.
    pub(super) document_revision: u64,
    /// Closed API operation selected at the protocol edge.
    pub(super) query: WorkspaceQuery,
    /// Client features that constrain the legal authoring response shape.
    pub(super) authoring_capabilities: crate::authoring::AuthoringClientCapabilities,
}

/// One typed query result from the single analysis owner.
pub(super) struct QueryResult {
    /// JSON-RPC request identity used only for response correlation.
    pub(super) request_id: RequestId,
    /// Editor generation queried by the worker.
    pub(super) generation: u64,
    /// Exact document-store revision represented by the result.
    pub(super) document_revision: u64,
    /// Typed result or a precise non-effect failure.
    pub(super) outcome: Result<WorkspaceQueryResult, QueryFailure>,
}

/// Result arriving from the worker thread.
pub(super) enum WorkerResult {
    /// Compiler/comparison publication.
    Analysis(AnalysisResult),
    /// Explicit custom-method response.
    Query(Box<QueryResult>),
    /// One reference page or an acknowledged release of its retained cursor.
    References(Box<ReferenceReply>),
}

/// Exact private reference-request/page correlation; no SDK cursor crosses it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ReferenceCoordinate {
    pub(super) request_id: RequestId,
    pub(super) generation: u64,
    pub(super) document_revision: u64,
    pub(super) page: u64,
}

pub(super) struct ReferenceReply {
    pub(super) coordinate: ReferenceCoordinate,
    pub(super) outcome: Result<ReferenceOutput, QueryFailure>,
}

pub(super) enum ReferenceOutput {
    Page(crate::authoring::references::ReferencePage),
    Discarded,
}

struct RetainedReferences {
    coordinate: ReferenceCoordinate,
    scan: crate::authoring::references::ReferenceScan,
}

/// Query failure that does not fabricate compiler cancellation or semantics.
#[derive(Debug, thiserror::Error)]
pub(super) enum QueryFailure {
    /// No successful analysis owns the requested editor generation.
    #[error("no admitted workspace analysis is available for this editor generation")]
    AnalysisUnavailable,
    /// The immutable baseline could not support an admitted comparison.
    #[error("no admitted immutable-baseline comparison is available")]
    ComparisonUnavailable,
    /// The API rejected a retained causal proof query.
    #[error(transparent)]
    Api(#[from] pale_sdk::ApiError),
    /// Standard authoring projection rejected a protocol-local coordinate or bound.
    #[error(transparent)]
    Authoring(crate::authoring::AuthoringQueryError),
    /// A continuation page omitted the coordinate of its first admitted page.
    #[error("a noninitial effective-profile page requires expectedCoordinate")]
    ProfileCoordinateRequired,
    /// The continuation coordinate names another editor or profile product.
    #[error("the effective-profile page coordinate is stale or mismatched")]
    ProfileCoordinateMismatch,
    /// The handle does not belong to the sole retained profile at this coordinate.
    #[error("the effective-profile explanation is unavailable at this retained coordinate")]
    ProfileExplanationUnavailable,
    /// The requested improvement transition is not legal from the retained phase.
    #[error("the configuration-improvement transition is unavailable from the retained phase")]
    ImprovementPhaseUnavailable,
    /// The request names another improvement context or compiler frontier.
    #[error("the configuration-improvement coordinate is stale or mismatched")]
    ImprovementCoordinateMismatch,
    /// Full private candidate capture is unavailable for this editor snapshot.
    #[error("configuration-improvement candidate capture failed: {0}")]
    ImprovementSource(String),
}

impl From<crate::authoring::AuthoringQueryError> for QueryFailure {
    fn from(error: crate::authoring::AuthoringQueryError) -> Self {
        match error {
            crate::authoring::AuthoringQueryError::Api(error) => Self::Api(error),
            error => Self::Authoring(error),
        }
    }
}

/// Failure to communicate with or join the analysis worker.
#[derive(Debug, thiserror::Error)]
pub(super) enum AnalysisWorkerError {
    /// The worker channel closed before accepting a command.
    #[error("analysis worker channel closed")]
    ChannelClosed,
    /// The worker thread panicked.
    #[error("analysis worker panicked")]
    Panicked,
}

enum WorkerCommand {
    Analyze(AnalysisJob),
    Query(Box<QueryJob>),
    ContinueReferences(ReferenceCoordinate),
    DiscardReferences(ReferenceCoordinate),
    Stop,
}

/// Single thread that owns all opaque API compilation/comparison handles.
pub(super) struct AnalysisWorker {
    commands: crossbeam_channel::Sender<WorkerCommand>,
    /// Completed worker results consumed by the server event loop.
    pub(super) results: crossbeam_channel::Receiver<WorkerResult>,
    thread: JoinHandle<()>,
}

impl AnalysisWorker {
    /// Start one worker with the immutable initialization baseline `B`.
    pub(super) fn spawn(
        api: PaleApi,
        root_manifest: String,
        machine_id: Option<String>,
        baseline: Snapshot,
    ) -> Self {
        let (commands, command_receiver) = crossbeam_channel::bounded(1);
        let (result_sender, results) = crossbeam_channel::bounded(1);
        let thread = thread::spawn(move || {
            let baseline = compile_snapshot(&api, &root_manifest, &baseline).ok();
            let mut state = AnalysisState::new(baseline);
            while let Ok(command) = command_receiver.recv() {
                let result = match command {
                    WorkerCommand::Analyze(job) => {
                        WorkerResult::Analysis(state.analyze(&api, &root_manifest, job))
                    }
                    WorkerCommand::Query(job) => {
                        if let WorkspaceQuery::Authoring(
                            crate::protocol::AuthoringQuery::References(params),
                        ) = &job.query
                        {
                            if params.partial_result_params.partial_result_token.is_some() {
                                let coordinate = ReferenceCoordinate {
                                    request_id: job.request_id.clone(),
                                    generation: job.generation,
                                    document_revision: job.document_revision,
                                    page: 0,
                                };
                                WorkerResult::References(Box::new(
                                    state.start_references(&api, coordinate, params),
                                ))
                            } else {
                                WorkerResult::Query(Box::new(state.query(
                                    &api,
                                    &root_manifest,
                                    machine_id.as_deref(),
                                    *job,
                                )))
                            }
                        } else {
                            WorkerResult::Query(Box::new(state.query(
                                &api,
                                &root_manifest,
                                machine_id.as_deref(),
                                *job,
                            )))
                        }
                    }
                    WorkerCommand::ContinueReferences(coordinate) => {
                        WorkerResult::References(Box::new(state.reference_step(&api, coordinate)))
                    }
                    WorkerCommand::DiscardReferences(coordinate) => {
                        WorkerResult::References(Box::new(state.discard_references(coordinate)))
                    }
                    WorkerCommand::Stop => break,
                };
                if result_sender.send(result).is_err() {
                    break;
                }
            }
        });
        Self {
            commands,
            results,
            thread,
        }
    }

    /// Submit one analysis after the server has acquired its single-worker slot.
    pub(super) fn analyze(&self, job: AnalysisJob) -> Result<(), AnalysisWorkerError> {
        self.commands
            .send(WorkerCommand::Analyze(job))
            .map_err(|_| AnalysisWorkerError::ChannelClosed)
    }

    /// Submit one typed proof query after the server has acquired the worker.
    pub(super) fn query(&self, job: QueryJob) -> Result<(), AnalysisWorkerError> {
        self.commands
            .send(WorkerCommand::Query(Box::new(job)))
            .map_err(|_| AnalysisWorkerError::ChannelClosed)
    }

    /// Continue exactly one page after the prior page's transport handoff.
    pub(super) fn continue_references(
        &self,
        coordinate: ReferenceCoordinate,
    ) -> Result<(), AnalysisWorkerError> {
        self.commands
            .send(WorkerCommand::ContinueReferences(coordinate))
            .map_err(|_| AnalysisWorkerError::ChannelClosed)
    }

    /// Release a paused scan without another SDK call or compilation.
    pub(super) fn discard_references(
        &self,
        coordinate: ReferenceCoordinate,
    ) -> Result<(), AnalysisWorkerError> {
        self.commands
            .send(WorkerCommand::DiscardReferences(coordinate))
            .map_err(|_| AnalysisWorkerError::ChannelClosed)
    }

    /// Stop and join the worker without activating any external lifecycle.
    pub(super) fn stop(self) -> Result<(), AnalysisWorkerError> {
        let Self {
            commands,
            results,
            thread,
        } = self;
        drop(results);
        let _ = commands.send(WorkerCommand::Stop);
        thread.join().map_err(|_| AnalysisWorkerError::Panicked)
    }
}

struct AnalysisState {
    baseline: Option<PortableWorkspaceCompilation>,
    current: Option<PortableWorkspaceCompilation>,
    comparison: Option<PortableWorkspaceComparison>,
    current_snapshot: Option<Snapshot>,
    generation: Option<u64>,
    attempted_generation: Option<u64>,
    attempted_revision: Option<u64>,
    /// Exact current buffer snapshot, admitted or rejected.
    attempted_snapshot: Option<Snapshot>,
    /// Source-free tolerant structure for that exact attempted snapshot.
    attempted_syntax: AuthoringSyntaxSnapshot,
    /// Exact owner diagnostic for the latest rejected editor attempt.
    attempted_failure: Option<RetainedDiagnostic>,
    /// Deterministic source edits proved by that exact rejected attempt.
    attempted_corrections: Vec<pale_sdk::AuthoringCorrection>,
    /// Sole admitted effective profile retained across editor failures.
    profile: Option<RetainedProfile>,
    /// Sole human-governed review; API typestate never crosses the wire.
    improvement: Option<RetainedImprovement>,
    /// At most one SDK occurrence continuation; never overwritten by a query.
    references: Option<RetainedReferences>,
}

enum RetainedImprovement {
    Explained {
        context: ProfileInspectionContext,
        baseline: crate::protocol::ExpectedProfileCoordinate,
        explanation: ConfigurationImprovementExplanation,
    },
    Previewed {
        context: ProfileInspectionContext,
        coordinate: ImprovementPreviewCoordinate,
        preview: Box<ConfigurationImprovementPreview>,
    },
}

/// One API-owned profile retained with its exact editor and selection coordinate.
///
/// In the rendered law, `g` and `r` are the current editor generation and
/// document revision, while `χ` is the explicit project/execution-workspace
/// inspection context. Compilation maps the exact admitted intent to the
/// effective portable profile. Rejected current input may reveal only the last
/// admitted predecessor at `g_a` and `r_a` in the same `χ`, and the response
/// marks that evidence stale:
#[doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/effective-profile-retention.display.mathml"
))]
struct RetainedProfile {
    context: ProfileInspectionContext,
    editor: EditorCoordinate,
    compilation: EffectiveProfileCompilation,
    /// Immutable API projection captured once for paging and splice fences.
    coordinates: EffectiveProfileCoordinates,
}

/// API-owned diagnostic plus its non-wire exact authoring coordinate.
///
/// [`DiagnosticV1`] deliberately preserves portable display evidence, whereas
/// [`AuthoringSourceLocation`] preserves the decoded field/index path needed
/// to map a failure back into the current tolerant syntax tree. Keeping the
/// pair avoids reparsing either form at the LSP boundary.
#[derive(Clone)]
struct RetainedDiagnostic {
    diagnostic: DiagnosticV1,
    authoring_location: Option<AuthoringSourceLocation>,
}

impl RetainedDiagnostic {
    fn from_error(error: &pale_sdk::ApiError) -> Self {
        Self {
            diagnostic: error.diagnostic().clone(),
            authoring_location: error.authoring_location().cloned(),
        }
    }
}

impl AnalysisState {
    fn new(baseline: Option<PortableWorkspaceCompilation>) -> Self {
        Self {
            baseline,
            current: None,
            comparison: None,
            current_snapshot: None,
            generation: None,
            attempted_generation: None,
            attempted_revision: None,
            attempted_snapshot: None,
            attempted_syntax: AuthoringSyntaxSnapshot::default(),
            attempted_failure: None,
            attempted_corrections: Vec::new(),
            profile: None,
            improvement: None,
            references: None,
        }
    }

    fn analyze(&mut self, api: &PaleApi, root_manifest: &str, job: AnalysisJob) -> AnalysisResult {
        self.references = None;
        let AnalysisJob {
            generation,
            snapshot,
        } = job;
        let prepared = prepare_snapshot(root_manifest, &snapshot);
        let workspace_outcome =
            api.compile_portable_workspace(prepared.request)
                .and_then(|current| {
                    let comparison = self
                        .baseline
                        .as_ref()
                        .map(|baseline| api.compare_portable_workspaces(baseline, &current))
                        .transpose()?;
                    self.current = Some(current);
                    self.comparison = comparison;
                    self.current_snapshot = Some(snapshot.clone());
                    self.generation = Some(generation);
                    Ok(())
                });
        let mut failures = context_free_source_failures(api, &snapshot);
        if let Err(error) = workspace_outcome {
            failures.insert(0, error);
        }
        let outcome = if failures.is_empty() {
            Ok(())
        } else {
            Err(AnalysisFailures { failures })
        };
        self.attempted_generation = Some(generation);
        self.attempted_revision = Some(snapshot.revision());
        self.attempted_snapshot = Some(snapshot.clone());
        self.attempted_syntax = prepared.syntax;
        self.attempted_failure = outcome
            .as_ref()
            .err()
            .and_then(|errors| errors.failures.first())
            .map(RetainedDiagnostic::from_error);
        self.attempted_corrections = outcome
            .as_ref()
            .err()
            .map(|errors| {
                errors
                    .iter()
                    .flat_map(|error| error.authoring_corrections().iter().cloned())
                    .collect()
            })
            .unwrap_or_default();
        AnalysisResult {
            generation,
            snapshot,
            syntax: self.attempted_syntax.clone(),
            outcome,
        }
    }

    fn query(
        &mut self,
        api: &PaleApi,
        root_manifest: &str,
        machine_id: Option<&str>,
        job: QueryJob,
    ) -> QueryResult {
        if self.references.is_some() {
            return QueryResult {
                request_id: job.request_id,
                generation: job.generation,
                document_revision: job.document_revision,
                outcome: Err(QueryFailure::AnalysisUnavailable),
            };
        }
        let QueryJob {
            request_id,
            generation,
            document_revision,
            query,
            authoring_capabilities,
        } = job;
        let editor = EditorCoordinate {
            generation,
            document_revision,
        };
        let outcome = match query {
            WorkspaceQuery::EffectiveProfile(query) => self
                .effective_profile(api, machine_id, editor, query)
                .map(Box::new)
                .map(WorkspaceQueryResult::EffectiveProfile),
            WorkspaceQuery::EffectiveProfileExplanation(query) => self
                .effective_profile_explanation(editor, query)
                .map(Box::new)
                .map(WorkspaceQueryResult::EffectiveProfileExplanation),
            WorkspaceQuery::ConfigurationImprovement(query) => self
                .configuration_improvement(api, root_manifest, machine_id, editor, query)
                .map(Box::new)
                .map(WorkspaceQueryResult::ConfigurationImprovement),
            WorkspaceQuery::Authoring(query)
                if self.attempted_generation == Some(generation)
                    && self.attempted_revision == Some(document_revision) =>
            {
                self.attempted_snapshot.as_ref().map_or(
                    Err(QueryFailure::AnalysisUnavailable),
                    |snapshot| {
                        crate::authoring::execute(
                            query,
                            crate::authoring::AuthoringExecution {
                                snapshot,
                                syntax: &self.attempted_syntax,
                                semantics: crate::authoring::AuthoringSemanticContext {
                                    portable: self
                                        .current
                                        .as_ref()
                                        .filter(|_| self.generation == Some(generation)),
                                    effective: self
                                        .profile
                                        .as_ref()
                                        .filter(|profile| profile.editor == editor)
                                        .map(|profile| &profile.compilation),
                                    api,
                                },
                                stale: crate::authoring::StaleAuthoringCompilation {
                                    portable: self
                                        .current
                                        .as_ref()
                                        .filter(|_| self.generation != Some(generation)),
                                },
                                corrections: &self.attempted_corrections,
                                client: authoring_capabilities,
                            },
                        )
                        .map(WorkspaceQueryResult::Authoring)
                        .map_err(QueryFailure::Authoring)
                    },
                )
            }
            query if self.generation == Some(generation) => self.execute(editor, query),
            _ => Err(QueryFailure::AnalysisUnavailable),
        };
        QueryResult {
            request_id,
            generation,
            document_revision,
            outcome,
        }
    }

    fn start_references(
        &mut self,
        api: &PaleApi,
        coordinate: ReferenceCoordinate,
        params: &lsp_types::ReferenceParams,
    ) -> ReferenceReply {
        let outcome = (|| {
            if self.references.is_some() || coordinate.page != 0 {
                return Err(QueryFailure::AnalysisUnavailable);
            }
            let (snapshot, semantics) = self.reference_context(api, &coordinate)?;
            let scan = crate::authoring::references::ReferenceScan::start(
                params,
                snapshot,
                &self.attempted_syntax,
                semantics,
            )?;
            let Some(scan) = scan else {
                return Ok(ReferenceOutput::Page(
                    crate::authoring::references::ReferencePage {
                        locations: Vec::new(),
                        complete: true,
                    },
                ));
            };
            self.references = Some(RetainedReferences {
                coordinate: coordinate.clone(),
                scan,
            });
            self.reference_step(api, coordinate.clone()).outcome
        })();
        ReferenceReply {
            coordinate,
            outcome,
        }
    }

    fn reference_context<'a>(
        &'a self,
        api: &'a PaleApi,
        coordinate: &ReferenceCoordinate,
    ) -> Result<(&'a Snapshot, crate::authoring::AuthoringSemanticContext<'a>), QueryFailure> {
        if self.attempted_generation != Some(coordinate.generation)
            || self.attempted_revision != Some(coordinate.document_revision)
        {
            return Err(QueryFailure::AnalysisUnavailable);
        }
        let snapshot = self
            .attempted_snapshot
            .as_ref()
            .ok_or(QueryFailure::AnalysisUnavailable)?;
        let editor = EditorCoordinate {
            generation: coordinate.generation,
            document_revision: coordinate.document_revision,
        };
        Ok((
            snapshot,
            crate::authoring::AuthoringSemanticContext {
                portable: self
                    .current
                    .as_ref()
                    .filter(|_| self.generation == Some(coordinate.generation)),
                effective: self
                    .profile
                    .as_ref()
                    .filter(|profile| profile.editor == editor)
                    .map(|profile| &profile.compilation),
                api,
            },
        ))
    }

    fn reference_step(&mut self, api: &PaleApi, coordinate: ReferenceCoordinate) -> ReferenceReply {
        if !self
            .references
            .as_ref()
            .is_some_and(|retained| retained.coordinate == coordinate)
        {
            return ReferenceReply {
                coordinate,
                outcome: Err(QueryFailure::AnalysisUnavailable),
            };
        }
        let mut retained = self.references.take().expect("the exact scan was checked");
        let outcome = (|| {
            let (snapshot, semantics) = self.reference_context(api, &coordinate)?;
            let page = retained
                .scan
                .next(snapshot, &self.attempted_syntax, semantics)?;
            if !page.complete {
                retained.coordinate.page = retained
                    .coordinate
                    .page
                    .checked_add(1)
                    .ok_or(crate::authoring::AuthoringQueryError::ReferencePageBound)?;
                self.references = Some(retained);
            }
            Ok(ReferenceOutput::Page(page))
        })();
        ReferenceReply {
            coordinate,
            outcome,
        }
    }

    fn discard_references(&mut self, coordinate: ReferenceCoordinate) -> ReferenceReply {
        let outcome = if self
            .references
            .as_ref()
            .is_some_and(|retained| retained.coordinate == coordinate)
        {
            self.references = None;
            Ok(ReferenceOutput::Discarded)
        } else {
            Err(QueryFailure::AnalysisUnavailable)
        };
        ReferenceReply {
            coordinate,
            outcome,
        }
    }

    fn execute(
        &self,
        editor: EditorCoordinate,
        query: WorkspaceQuery,
    ) -> Result<WorkspaceQueryResult, QueryFailure> {
        match query {
            WorkspaceQuery::CompilationSummary => self
                .current
                .as_ref()
                .map(|current| WorkspaceQueryResult::CompilationSummary(current.summary()))
                .ok_or(QueryFailure::AnalysisUnavailable),
            WorkspaceQuery::ComparisonSummary => self
                .comparison
                .as_ref()
                .map(|comparison| WorkspaceQueryResult::ComparisonSummary(comparison.summary()))
                .ok_or(QueryFailure::ComparisonUnavailable),
            WorkspaceQuery::ChangeRoots(params) => self
                .comparison
                .as_ref()
                .map(|comparison| {
                    let page = comparison.change_roots(params.offset, params.limit);
                    WorkspaceQueryResult::ChangeRoots(ChangeRootPage::from(&page))
                })
                .ok_or(QueryFailure::ComparisonUnavailable),
            WorkspaceQuery::Impacts(params) => self
                .comparison
                .as_ref()
                .map(|comparison| {
                    let page = comparison.impacts(params.offset, params.limit);
                    WorkspaceQueryResult::Impacts(ImpactPage::from(&page))
                })
                .ok_or(QueryFailure::ComparisonUnavailable),
            WorkspaceQuery::Explanation(subject) => {
                let comparison = self
                    .comparison
                    .as_ref()
                    .ok_or(QueryFailure::ComparisonUnavailable)?;
                comparison
                    .explain(&subject)
                    .map(|explanation| {
                        WorkspaceQueryResult::Explanation(Box::new(Explanation::from(&explanation)))
                    })
                    .map_err(QueryFailure::Api)
            }
            WorkspaceQuery::ProfileContexts => self
                .current
                .as_ref()
                .map(|current| {
                    WorkspaceQueryResult::ProfileContexts(ProfileContextsResponse {
                        editor,
                        contexts: current.profile_contexts(),
                    })
                })
                .ok_or(QueryFailure::AnalysisUnavailable),
            WorkspaceQuery::EffectiveProfile(_)
            | WorkspaceQuery::EffectiveProfileExplanation(_)
            | WorkspaceQuery::ConfigurationImprovement(_)
            | WorkspaceQuery::Authoring(_) => unreachable!(
                "effective-profile and authoring queries require coordinate-specific state"
            ),
        }
    }

    fn effective_profile(
        &mut self,
        api: &PaleApi,
        machine_id: Option<&str>,
        editor: EditorCoordinate,
        query: EffectiveProfileQuery,
    ) -> Result<EffectiveProfileResult, QueryFailure> {
        if self.attempted_generation != Some(editor.generation)
            || self.attempted_revision != Some(editor.document_revision)
        {
            return Err(QueryFailure::AnalysisUnavailable);
        }

        if self.generation == Some(editor.generation) {
            let same_current_profile = self.profile.as_ref().is_some_and(|profile| {
                profile.context == query.context && profile.editor == editor
            });
            if !same_current_profile {
                let snapshot = self
                    .current_snapshot
                    .as_ref()
                    .ok_or(QueryFailure::AnalysisUnavailable)?;
                let admission = capture_profile_request(snapshot, machine_id, &query.context)
                    .map_err(ProfileAdmissionFailure::Source)
                    .and_then(|request| {
                        api.compile_effective_profile(
                            self.current
                                .as_ref()
                                .expect("the admitted generation retains its workspace"),
                            request,
                        )
                        .map_err(|error| {
                            ProfileAdmissionFailure::Diagnostic(Box::new(
                                RetainedDiagnostic::from_error(&error),
                            ))
                        })
                    });
                match admission {
                    Ok(compilation) => {
                        let coordinates = compilation.coordinates();
                        self.profile = Some(RetainedProfile {
                            context: query.context.clone(),
                            editor,
                            compilation,
                            coordinates,
                        });
                    }
                    Err(failure) => return self.profile_failure(editor, &query, failure),
                }
            }
        } else {
            let diagnostic = self
                .attempted_failure
                .clone()
                .expect("a rejected current generation retains its admitted API diagnostic");
            return self.profile_failure(
                editor,
                &query,
                ProfileAdmissionFailure::Diagnostic(Box::new(diagnostic)),
            );
        }

        let profile = self
            .profile
            .as_ref()
            .expect("a successful current profile admission is retained");
        validate_page_coordinate(editor, profile, &query)?;
        Ok(EffectiveProfileResult::new(
            EffectiveProfileResponse {
                state: EffectiveProfileState::Current,
                editor,
                context: query.context,
                coordinates: Some(profile.coordinates.clone()),
                page: Some(
                    profile
                        .compilation
                        .inspect(EffectiveProfilePageRequest::new(
                            query.section,
                            query.offset,
                            query.limit,
                        )),
                ),
                diagnostic_identity: None,
            },
            None,
            None,
        ))
    }

    fn profile_failure(
        &self,
        editor: EditorCoordinate,
        query: &EffectiveProfileQuery,
        failure: ProfileAdmissionFailure,
    ) -> Result<EffectiveProfileResult, QueryFailure> {
        let summary = failure.summary().to_owned();
        let diagnostic = failure.into_diagnostic();
        let diagnostic_identity = diagnostic
            .as_ref()
            .map(|retained| retained.diagnostic.identity().clone());
        let authoring_location = diagnostic
            .as_ref()
            .and_then(|retained| retained.authoring_location.clone());
        let diagnostic = diagnostic.map(|retained| retained.diagnostic);
        if let Some(profile) = self
            .profile
            .as_ref()
            .filter(|profile| profile.context == query.context)
        {
            validate_page_coordinate(editor, profile, query)?;
            Ok(EffectiveProfileResult::new(
                EffectiveProfileResponse {
                    state: EffectiveProfileState::Stale {
                        admitted_editor: profile.editor,
                        admitted_profile_digest: profile.coordinates.profile_digest.clone(),
                        summary,
                    },
                    editor,
                    context: query.context.clone(),
                    coordinates: Some(profile.coordinates.clone()),
                    page: Some(
                        profile
                            .compilation
                            .inspect(EffectiveProfilePageRequest::new(
                                query.section,
                                query.offset,
                                query.limit,
                            )),
                    ),
                    diagnostic_identity,
                },
                diagnostic,
                authoring_location,
            ))
        } else {
            Ok(EffectiveProfileResult::new(
                EffectiveProfileResponse {
                    state: EffectiveProfileState::NotAdmitted { summary },
                    editor,
                    context: query.context.clone(),
                    coordinates: None,
                    page: None,
                    diagnostic_identity,
                },
                diagnostic,
                authoring_location,
            ))
        }
    }

    fn effective_profile_explanation(
        &self,
        editor: EditorCoordinate,
        query: EffectiveProfileExplanationQuery,
    ) -> Result<EffectiveProfileExplanationResponse, QueryFailure> {
        if self.attempted_generation != Some(editor.generation)
            || self.attempted_revision != Some(editor.document_revision)
        {
            return Err(QueryFailure::AnalysisUnavailable);
        }
        let profile = self
            .profile
            .as_ref()
            .filter(|profile| profile.context == query.context)
            .ok_or(QueryFailure::ProfileExplanationUnavailable)?;
        validate_expected_profile_coordinate(editor, profile, &query.expected_coordinate)?;
        let explanation = profile
            .compilation
            .explain(&query.handle)
            .ok_or(QueryFailure::ProfileExplanationUnavailable)?;
        Ok(EffectiveProfileExplanationResponse {
            editor,
            context: query.context,
            coordinates: profile.coordinates.clone(),
            explanation,
        })
    }

    fn configuration_improvement(
        &mut self,
        api: &PaleApi,
        root_manifest: &str,
        machine_id: Option<&str>,
        editor: EditorCoordinate,
        query: ConfigurationImprovementQuery,
    ) -> Result<ConfigurationImprovementResponse, QueryFailure> {
        match query {
            ConfigurationImprovementQuery::Begin {
                context,
                expected_profile,
            } => {
                if let Some(RetainedImprovement::Previewed { preview, .. }) =
                    self.improvement.take()
                {
                    let _terminal = (*preview).mark_stale();
                }
                let profile = self
                    .profile
                    .as_ref()
                    .filter(|profile| profile.context == context && profile.editor == editor)
                    .ok_or(QueryFailure::ProfileExplanationUnavailable)?;
                validate_expected_profile_coordinate(editor, profile, &expected_profile)?;
                let current = self
                    .current
                    .as_ref()
                    .filter(|_| self.generation == Some(editor.generation))
                    .ok_or(QueryFailure::AnalysisUnavailable)?;
                let inspection = api.inspect_configuration_improvement(current);
                let explanation =
                    api.explain_configuration_improvement(inspection, &profile.compilation)?;
                let baseline = crate::protocol::ExpectedProfileCoordinate::from_profile(
                    editor,
                    &profile.coordinates,
                );
                self.improvement = Some(RetainedImprovement::Explained {
                    context: context.clone(),
                    baseline: baseline.clone(),
                    explanation,
                });
                Ok(ConfigurationImprovementResponse::Explained { context, baseline })
            }
            ConfigurationImprovementQuery::Evaluate {
                context,
                expected_baseline,
            } => {
                if self.attempted_generation != Some(editor.generation)
                    || self.attempted_revision != Some(editor.document_revision)
                    || self.generation != Some(editor.generation)
                    || self.attempted_failure.is_some()
                {
                    return Err(QueryFailure::AnalysisUnavailable);
                }
                let retained_matches = matches!(
                    self.improvement.as_ref(),
                    Some(RetainedImprovement::Explained {
                        context: retained_context,
                        baseline,
                        ..
                    }) if retained_context == &context && baseline == &expected_baseline
                );
                if !retained_matches {
                    return Err(QueryFailure::ImprovementCoordinateMismatch);
                }
                let snapshot = self
                    .current_snapshot
                    .as_ref()
                    .ok_or(QueryFailure::AnalysisUnavailable)?;
                let request =
                    capture_improvement_request(root_manifest, snapshot, machine_id, &context)
                        .map_err(|error| {
                            QueryFailure::ImprovementSource(error.summary().to_owned())
                        })?;
                let Some(RetainedImprovement::Explained {
                    context: _,
                    baseline,
                    explanation,
                }) = self.improvement.take()
                else {
                    return Err(QueryFailure::ImprovementPhaseUnavailable);
                };
                let proposal = api.propose_configuration_improvement(explanation, request)?;
                let evaluation = api.evaluate_configuration_improvement(proposal)?;
                let diagnosis = api.diagnose_configuration_improvement(evaluation)?;
                let preview = api.preview_configuration_improvement(diagnosis)?;
                let summary = preview.summary().clone();
                let projection = ConfigurationImprovementPreviewProjection::from(&summary);
                let coordinate = ImprovementPreviewCoordinate {
                    baseline,
                    candidate_editor: editor,
                    candidate_selected_topology_digest: summary
                        .candidate_selected_topology_digest()
                        .to_owned(),
                    candidate_profile_digest: summary.candidate_profile_digest().to_owned(),
                    candidate_instance_digest: summary.candidate_instance_digest().to_owned(),
                    derivation_receipt_digest: summary.derivation_receipt_digest().to_owned(),
                };
                self.improvement = Some(RetainedImprovement::Previewed {
                    context: context.clone(),
                    coordinate: coordinate.clone(),
                    preview: Box::new(preview),
                });
                Ok(ConfigurationImprovementResponse::Previewed {
                    context,
                    coordinate,
                    preview: projection,
                })
            }
            ConfigurationImprovementQuery::Inspect {
                context,
                expected_preview,
                detail,
            } => {
                let Some(RetainedImprovement::Previewed {
                    context: retained_context,
                    coordinate,
                    preview,
                }) = self.improvement.as_ref()
                else {
                    return Err(QueryFailure::ImprovementPhaseUnavailable);
                };
                if retained_context != &context || coordinate != &expected_preview {
                    return Err(QueryFailure::ImprovementCoordinateMismatch);
                }
                if editor != coordinate.candidate_editor {
                    return Err(QueryFailure::ImprovementCoordinateMismatch);
                }
                match *detail {
                    ConfigurationImprovementDetailQuery::Overview {
                        roots_offset,
                        impacts_offset,
                        limit,
                    } => {
                        let limit = bounded_improvement_detail_limit(limit);
                        let roots = preview.change_roots(roots_offset, limit);
                        let impacts = preview.impacts(impacts_offset, limit);
                        Ok(ConfigurationImprovementResponse::Details {
                            context,
                            coordinate: coordinate.clone(),
                            change_roots: ChangeRootPage::from(&roots),
                            impacts: ImpactPage::from(&impacts),
                        })
                    }
                    ConfigurationImprovementDetailQuery::Subject {
                        subject,
                        input_limit,
                        maximum_inspected,
                        maximum_returned,
                    } => {
                        let explanation = preview.explain_impact(&subject)?;
                        let source = preview.inspect_candidate_subject(
                            &subject,
                            None,
                            bounded_improvement_detail_limit(input_limit),
                            bounded_improvement_detail_limit(maximum_inspected),
                            bounded_improvement_detail_limit(maximum_returned),
                        )?;
                        Ok(ConfigurationImprovementResponse::SubjectDetails {
                            context,
                            coordinate: coordinate.clone(),
                            explanation: Explanation::from(&explanation),
                            source: ConfigurationImprovementSourceProjection::from(&source),
                        })
                    }
                }
            }
            ConfigurationImprovementQuery::Finish {
                context,
                expected_preview,
                disposition,
            } => {
                let retained_matches = matches!(
                    self.improvement.as_ref(),
                    Some(RetainedImprovement::Previewed {
                        context: retained_context,
                        coordinate,
                        ..
                    }) if retained_context == &context && coordinate == &expected_preview
                );
                if !retained_matches {
                    return Err(QueryFailure::ImprovementCoordinateMismatch);
                }
                let Some(RetainedImprovement::Previewed {
                    context: _,
                    coordinate,
                    preview,
                }) = self.improvement.take()
                else {
                    return Err(QueryFailure::ImprovementPhaseUnavailable);
                };
                let terminal = if editor == coordinate.candidate_editor {
                    match disposition {
                        ImprovementReviewDisposition::Accept => (*preview).accept(),
                        ImprovementReviewDisposition::Reject => (*preview).reject(),
                        ImprovementReviewDisposition::Cancel => (*preview).cancel(),
                    }
                } else {
                    (*preview).mark_stale()
                };
                let projection =
                    ConfigurationImprovementPreviewProjection::from(terminal.preview());
                Ok(ConfigurationImprovementResponse::Terminal {
                    context,
                    disposition: terminal.disposition(),
                    coordinate,
                    preview: projection,
                })
            }
        }
    }
}

const fn bounded_improvement_detail_limit(requested: u16) -> u16 {
    if requested == 0 || requested > MAX_IMPROVEMENT_DETAIL_ROWS {
        MAX_IMPROVEMENT_DETAIL_ROWS
    } else {
        requested
    }
}

/// Failure before a profile becomes the retained API-owned inspection product.
enum ProfileAdmissionFailure {
    /// LSP source-role capture was incomplete or inconsistent.
    Source(ProfileSourceError),
    /// The API rejected the complete typed request.
    Diagnostic(Box<RetainedDiagnostic>),
}

impl ProfileAdmissionFailure {
    fn summary(&self) -> &str {
        match self {
            Self::Source(error) => error.summary(),
            Self::Diagnostic(diagnostic) => diagnostic.diagnostic.summary(),
        }
    }

    fn into_diagnostic(self) -> Option<RetainedDiagnostic> {
        match self {
            Self::Source(_) => None,
            Self::Diagnostic(diagnostic) => Some(*diagnostic),
        }
    }
}

/// Transport-owned failure to assemble already selected source roles.
enum ProfileSourceError {
    /// Initialization selected no machine identity.
    MissingMachine,
    /// A singular role has no current document.
    MissingRole(DocumentRole),
    /// A singular role has more than one current document.
    DuplicateRole(DocumentRole),
}

impl ProfileSourceError {
    fn summary(&self) -> &str {
        match self {
            Self::MissingMachine => "effective-profile inspection has no selected machine identity",
            Self::MissingRole(DocumentRole::Capabilities) => {
                "effective-profile inspection has no capabilities source"
            }
            Self::MissingRole(DocumentRole::MachineBinding) => {
                "effective-profile inspection has no machine-binding source"
            }
            Self::MissingRole(DocumentRole::DistributionLock) => {
                "effective-profile inspection has no distribution-lock source"
            }
            Self::DuplicateRole(DocumentRole::Capabilities) => {
                "effective-profile inspection has multiple capabilities sources"
            }
            Self::DuplicateRole(DocumentRole::MachineBinding) => {
                "effective-profile inspection has multiple machine-binding sources"
            }
            Self::DuplicateRole(DocumentRole::DistributionLock) => {
                "effective-profile inspection has multiple distribution-lock sources"
            }
            Self::MissingRole(_) | Self::DuplicateRole(_) => {
                "effective-profile source family is inconsistent"
            }
        }
    }
}

/// Reject a continuation that could splice unlike editor/profile products.
fn validate_page_coordinate(
    editor: EditorCoordinate,
    profile: &RetainedProfile,
    query: &EffectiveProfileQuery,
) -> Result<(), QueryFailure> {
    let Some(expected) = &query.expected_coordinate else {
        return if query.offset == 0 {
            Ok(())
        } else {
            Err(QueryFailure::ProfileCoordinateRequired)
        };
    };
    validate_expected_profile_coordinate(editor, profile, expected)
}

fn validate_expected_profile_coordinate(
    editor: EditorCoordinate,
    profile: &RetainedProfile,
    expected: &crate::protocol::ExpectedProfileCoordinate,
) -> Result<(), QueryFailure> {
    if expected.generation == editor.generation
        && expected.document_revision == editor.document_revision
        && expected.profile_digest == profile.coordinates.profile_digest
        && expected.instance_digest == profile.coordinates.instance_digest
        && expected.selected_topology_digest == profile.coordinates.selected_topology_digest
        && expected.workspace_semantic_ir_digest == profile.coordinates.workspace_semantic_ir_digest
        && expected.workspace_admission_evidence_digest
            == profile.coordinates.workspace_admission_evidence_digest
        && expected.package_semantic_ir_digest == profile.coordinates.package_semantic_ir_digest
        && expected.composition_plan_digest == profile.coordinates.composition_plan_digest
    {
        Ok(())
    } else {
        Err(QueryFailure::ProfileCoordinateMismatch)
    }
}

/// Bind explicit document roles to the existing API request without parsing.
fn capture_profile_request(
    snapshot: &Snapshot,
    machine_id: Option<&str>,
    context: &ProfileInspectionContext,
) -> Result<EffectiveProfileCompilationRequest, ProfileSourceError> {
    let machine_id = machine_id.ok_or(ProfileSourceError::MissingMachine)?;
    let private = capture_private_inputs(snapshot)?;
    let packages = snapshot
        .package_documents()
        .map(|document| {
            CompilationDocument::new(document.logical_label().to_owned(), document.bytes())
        })
        .collect();
    Ok(EffectiveProfileCompilationRequest::new(
        context.project_id.clone(),
        context.execution_workspace_id.clone(),
        machine_id.to_owned(),
        private,
        packages,
    ))
}

/// Capture one complete effect-free candidate without retaining Package source.
fn capture_improvement_request(
    root_manifest: &str,
    snapshot: &Snapshot,
    machine_id: Option<&str>,
    context: &ProfileInspectionContext,
) -> Result<ConfigurationImprovementProposalRequest, ProfileSourceError> {
    let machine_id = machine_id.ok_or(ProfileSourceError::MissingMachine)?;
    Ok(ConfigurationImprovementProposalRequest::new(
        context.project_id.clone(),
        context.execution_workspace_id.clone(),
        machine_id.to_owned(),
        prepare_snapshot(root_manifest, snapshot).request,
        capture_private_inputs(snapshot)?,
    ))
}

fn capture_private_inputs(snapshot: &Snapshot) -> Result<OneShotPrivateInputs, ProfileSourceError> {
    let capabilities = singular_source(snapshot, DocumentRole::Capabilities)?;
    let machine_binding = singular_source(snapshot, DocumentRole::MachineBinding)?;
    let distribution_lock = singular_source(snapshot, DocumentRole::DistributionLock)?;
    let lifecycle_guidance = repeated_sources(snapshot, DocumentRole::LifecycleGuidance);
    Ok(OneShotPrivateInputs::new(
        capabilities,
        machine_binding,
        distribution_lock,
        lifecycle_guidance,
    ))
}

/// Capture exactly one source body for a singular compiler role.
fn singular_source(
    snapshot: &Snapshot,
    role: DocumentRole,
) -> Result<CompilationDocument, ProfileSourceError> {
    let mut documents = snapshot.documents_with_role(role);
    let document = documents
        .next()
        .ok_or(ProfileSourceError::MissingRole(role))?;
    if documents.next().is_some() {
        return Err(ProfileSourceError::DuplicateRole(role));
    }
    Ok(CompilationDocument::new(
        document.logical_label().to_owned(),
        document.bytes(),
    ))
}

/// Capture a repeated source family in deterministic snapshot order.
fn repeated_sources(snapshot: &Snapshot, role: DocumentRole) -> Vec<CompilationDocument> {
    snapshot
        .documents_with_role(role)
        .map(|document| {
            CompilationDocument::new(document.logical_label().to_owned(), document.bytes())
        })
        .collect()
}

fn compile_snapshot(
    api: &PaleApi,
    root_manifest: &str,
    snapshot: &Snapshot,
) -> Result<PortableWorkspaceCompilation, pale_sdk::ApiError> {
    api.compile_portable_workspace(prepare_snapshot(root_manifest, snapshot).request)
}

struct PreparedSnapshot {
    request: PortableWorkspaceRequest,
    syntax: AuthoringSyntaxSnapshot,
}

/// Flatten each mutable TOML buffer once for both syntax indexing and API capture.
fn prepare_snapshot(root_manifest: &str, snapshot: &Snapshot) -> PreparedSnapshot {
    let mut documents = Vec::new();
    let mut syntax = AuthoringSyntaxSnapshot::default();
    for document in snapshot.documents() {
        let text = document.text();
        if let Some(index) = crate::authoring::standalone_role(document.role())
            .and_then(|role| role.declaration_range(&text))
            .and_then(|range| AuthoringSyntax::parse_region(&text, range))
        {
            syntax.insert(document.logical_label().to_owned(), index);
        }
        if document.role().is_workspace() {
            documents.push(PortableWorkspaceDocument::new(
                document.logical_label().to_owned(),
                text.as_bytes().to_vec(),
            ));
        }
    }
    PreparedSnapshot {
        request: PortableWorkspaceRequest::new(root_manifest.to_owned(), documents),
        syntax,
    }
}

fn context_free_source_failures(api: &PaleApi, snapshot: &Snapshot) -> Vec<pale_sdk::ApiError> {
    let mut failures = snapshot
        .documents()
        .iter()
        .filter(|document| !document.role().is_workspace() && !document.role().is_package())
        .filter_map(|document| {
            let role = crate::authoring::standalone_role(document.role())?;
            api.validate_authoring_document(
                role,
                document.logical_label().to_owned(),
                document.bytes(),
            )
            .err()
        })
        .collect::<Vec<_>>();
    let package_documents = snapshot
        .package_documents()
        .map(|document| {
            CompilationDocument::new(document.logical_label().to_owned(), document.bytes())
        })
        .collect::<Vec<_>>();
    if !package_documents.is_empty() {
        if let Err(error) =
            api.inspect_package_distribution(PackageDistributionRequest::new(package_documents))
        {
            failures.push(error);
        }
    }
    failures
}

#[cfg(test)]
#[path = "../tests/internal/analysis.rs"]
mod tests;
