//! Configuration-review commands over the SDK's consuming stage handles.
//!
//! The caller retains each returned handle. The dispatcher stores no review
//! table and never reconstructs a handle from a summary. A source-based preview
//! starts a new review; it does not resume an earlier conversation or apply an
//! edit. Preview queries use the comparison and profile handles already held
//! by [`ConfigurationImprovementPreview`].

use pale_sdk::{
    ApiError, ConfigurationImprovementCandidateRequest, ConfigurationImprovementDiagnosis,
    ConfigurationImprovementDisposition, ConfigurationImprovementEvaluation,
    ConfigurationImprovementExplanation, ConfigurationImprovementInspection,
    ConfigurationImprovementPreview, ConfigurationImprovementPreviewSummary,
    ConfigurationImprovementProposal, ConfigurationImprovementProposalRequest,
    ConfigurationImprovementSourcePageState, ConfigurationImprovementSources,
    ConfigurationImprovementTerminal, EffectiveProfileCompilation, PaleApi,
    PortableWorkspaceCompilation, PortableWorkspaceExplanation, PortableWorkspaceImpactPage,
    PortableWorkspaceSubjectInspection, PortableWorkspaceSubjectSelector,
    PreparedConfigurationImprovementPreview,
};

use super::CommandDispatcher;

/// One review transition, carrying the exact opaque SDK owner it consumes.
///
/// No variant can be decoded from a protocol request by inventing a handle.
/// [`Self::PreviewSources`] is the source-first alternative for a new request.
pub enum ConfigurationImprovementCommand {
    /// Inspect an already compiled baseline without recompiling it.
    Inspect(Box<PortableWorkspaceCompilation>),
    /// Join an inspection to its matching effective profile.
    Explain {
        /// SDK inspection consumed by this transition.
        inspection: ConfigurationImprovementInspection,
        /// Profile compiled from the matching baseline.
        profile: Box<EffectiveProfileCompilation>,
    },
    /// Attach captured candidate source to one explanation.
    Propose {
        /// SDK explanation consumed by this transition.
        explanation: ConfigurationImprovementExplanation,
        /// Candidate with no independently supplied Package source.
        candidate: Box<ConfigurationImprovementProposalRequest>,
    },
    /// Compile the proposed candidate through canonical admission.
    Evaluate(Box<ConfigurationImprovementProposal>),
    /// Derive impact for the admitted candidate.
    Diagnose(Box<ConfigurationImprovementEvaluation>),
    /// Freeze the diagnosis for explicit review.
    Preview(Box<ConfigurationImprovementDiagnosis>),
    /// Commit a preparation using the exact explanation that prepared it.
    Commit {
        /// Original explanation, not an equal-coordinate replacement.
        explanation: ConfigurationImprovementExplanation,
        /// Value returned by the dispatcher's borrowing preparation method.
        prepared: Box<PreparedConfigurationImprovementPreview>,
    },
    /// Consume a preview with an explicit, effect-free review decision.
    Finish {
        /// The preview being decided, not a serialized summary.
        preview: Box<ConfigurationImprovementPreview>,
        /// Caller decision. `Stale` requires separately proved coordinate drift.
        disposition: ConfigurationImprovementDisposition,
    },
    /// Compile supplied sources once per role and return a preview with its
    /// first impact page at the SDK's default bound.
    ///
    /// No terminal decision is selected automatically.
    PreviewSources(Box<ConfigurationImprovementSources>),
    /// Rebuild a review, check its complete presented summary, and page its impacts.
    ///
    /// Every page checks the same review identity. This does not resume a
    /// handle, establish an editor revision, or select a review decision.
    ImpactsSources {
        /// Captured baseline and candidate used to rebuild the review.
        sources: Box<ConfigurationImprovementSources>,
        /// Complete original preview summary, including private-instance coordinates.
        presented_summary: Vec<u8>,
        /// Zero-based offset in canonical impact order.
        offset: u32,
        /// SDK page limit; zero selects its default and larger values are capped.
        limit: u16,
    },
    /// Rebuild a review and inspect one candidate subject after checking the summary.
    ///
    /// The source page is bounded and may report that either its inputs or
    /// source scan remains incomplete. This stateless command does not expose
    /// the SDK's owner-bound continuation cursor; retained SDK and editor
    /// clients own exact continuation. This query grants no disclosure or
    /// source-write authority.
    InspectSources {
        /// Captured baseline and candidate used to rebuild the review.
        sources: Box<ConfigurationImprovementSources>,
        /// Complete original preview summary, including private-instance coordinates.
        presented_summary: Vec<u8>,
        /// Closed compiler subject selected by the caller.
        subject: PortableWorkspaceSubjectSelector,
        /// Maximum direct proof inputs returned; zero selects the SDK default.
        input_limit: u16,
        /// Maximum source candidates examined; zero selects the SDK default.
        maximum_inspected: u16,
        /// Maximum matching source occurrences returned; zero selects the SDK default.
        maximum_returned: u16,
    },
    /// Compare a fresh source-based review with a previously presented summary.
    ///
    /// This starts a new review, not a continuation of an earlier handle.
    FinishSources {
        /// Captured baseline and candidate used to rebuild the review.
        sources: Box<ConfigurationImprovementSources>,
        /// Complete earlier summary, including private-instance coordinates.
        presented_summary: Vec<u8>,
        /// Explicit decision; neither this value nor summary equality permits a write.
        disposition: ConfigurationImprovementDisposition,
    },
}

impl std::fmt::Debug for ConfigurationImprovementCommand {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let transition = match self {
            Self::Inspect(_) => "Inspect",
            Self::Explain { .. } => "Explain",
            Self::Propose { .. } => "Propose",
            Self::Evaluate(_) => "Evaluate",
            Self::Diagnose(_) => "Diagnose",
            Self::Preview(_) => "Preview",
            Self::Commit { .. } => "Commit",
            Self::Finish { .. } => "Finish",
            Self::PreviewSources(_) => "PreviewSources",
            Self::ImpactsSources { .. } => "ImpactsSources",
            Self::InspectSources { .. } => "InspectSources",
            Self::FinishSources { .. } => "FinishSources",
        };
        formatter
            .debug_tuple("ConfigurationImprovementCommand")
            .field(&transition)
            .finish()
    }
}

/// Exact SDK result of a review transition, with no copied lifecycle state.
pub enum ConfigurationImprovementOutcome {
    /// An admitted baseline available for explanation.
    Inspected(ConfigurationImprovementInspection),
    /// A baseline joined to its effective-profile evidence.
    Explained(Box<ConfigurationImprovementExplanation>),
    /// Captured candidate awaiting evaluation.
    Proposed(Box<ConfigurationImprovementProposal>),
    /// Admitted candidate awaiting impact derivation.
    Evaluated(Box<ConfigurationImprovementEvaluation>),
    /// Impact proof awaiting preview projection.
    Diagnosed(Box<ConfigurationImprovementDiagnosis>),
    /// Reviewable proof retaining its comparison and both profiles.
    Previewed(Box<ConfigurationImprovementPreview>),
    /// Source-based review with its initial impact query already completed.
    SourcePreviewed {
        /// Retained review, available for later explicit decisions.
        preview: Box<ConfigurationImprovementPreview>,
        /// First page in SDK order, with the SDK's default limit and cursor.
        impacts: PortableWorkspaceImpactPage,
    },
    /// One SDK-owned impact page after checking the presented review summary.
    Impacts(PortableWorkspaceImpactPage),
    /// Causal and candidate-source evidence for one checked review subject.
    SourceDetails {
        /// Complete checked preview summary binding this evidence.
        summary: Box<ConfigurationImprovementPreviewSummary>,
        /// Existing comparison-owner explanation for the selected subject.
        explanation: PortableWorkspaceExplanation,
        /// Bounded source evidence from the exact candidate compilation.
        source: Box<PortableWorkspaceSubjectInspection>,
        /// Whether this stateless page is complete or needs retained review.
        source_page_state: ConfigurationImprovementSourcePageState,
    },
    /// Explicit decision that neither writes source nor authorizes an effect.
    Finished(Box<ConfigurationImprovementTerminal>),
}

impl std::fmt::Debug for ConfigurationImprovementOutcome {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let stage = match self {
            Self::Inspected(_) => "Inspected",
            Self::Explained(_) => "Explained",
            Self::Proposed(_) => "Proposed",
            Self::Evaluated(_) => "Evaluated",
            Self::Diagnosed(_) => "Diagnosed",
            Self::Previewed(_) => "Previewed",
            Self::SourcePreviewed { .. } => "SourcePreviewed",
            Self::Impacts(_) => "Impacts",
            Self::SourceDetails { .. } => "SourceDetails",
            Self::Finished(_) => "Finished",
        };
        formatter
            .debug_tuple("ConfigurationImprovementOutcome")
            .field(&stage)
            .finish()
    }
}

impl CommandDispatcher {
    /// Prepare a candidate while retaining the explanation for retry.
    ///
    /// This borrowing route is intentionally separate from consuming dispatch.
    /// On error, the caller still owns both input handles. On success, commit
    /// must consume the same explanation through
    /// [`ConfigurationImprovementCommand::Commit`].
    ///
    /// # Errors
    ///
    /// Returns the SDK diagnostic unchanged for rejected source, admission,
    /// derivation, or projection. No stage advances on failure.
    pub fn prepare_configuration_improvement_preview(
        &self,
        explanation: &ConfigurationImprovementExplanation,
        candidate: &PortableWorkspaceCompilation,
        request: ConfigurationImprovementCandidateRequest,
    ) -> Result<PreparedConfigurationImprovementPreview, ApiError> {
        self.api
            .prepare_configuration_improvement_preview(explanation, candidate, request)
    }
}

/// Forward each transition without interpreting the SDK's state or errors.
pub(super) fn dispatch(
    api: &PaleApi,
    command: ConfigurationImprovementCommand,
) -> Result<ConfigurationImprovementOutcome, ApiError> {
    use ConfigurationImprovementCommand as Command;
    use ConfigurationImprovementOutcome as Outcome;

    match command {
        Command::Inspect(baseline) => Ok(Outcome::Inspected(
            api.inspect_configuration_improvement(&baseline),
        )),
        Command::Explain {
            inspection,
            profile,
        } => api
            .explain_configuration_improvement(inspection, &profile)
            .map(Box::new)
            .map(Outcome::Explained),
        Command::Propose {
            explanation,
            candidate,
        } => api
            .propose_configuration_improvement(explanation, *candidate)
            .map(Box::new)
            .map(Outcome::Proposed),
        Command::Evaluate(proposal) => api
            .evaluate_configuration_improvement(*proposal)
            .map(Box::new)
            .map(Outcome::Evaluated),
        Command::Diagnose(evaluation) => api
            .diagnose_configuration_improvement(*evaluation)
            .map(Box::new)
            .map(Outcome::Diagnosed),
        Command::Preview(diagnosis) => api
            .preview_configuration_improvement(*diagnosis)
            .map(Box::new)
            .map(Outcome::Previewed),
        Command::Commit {
            explanation,
            prepared,
        } => api
            .commit_configuration_improvement_preview(explanation, *prepared)
            .map(Box::new)
            .map(Outcome::Previewed),
        Command::Finish {
            preview,
            disposition,
        } => Ok(Outcome::Finished(Box::new(match disposition {
            ConfigurationImprovementDisposition::Accepted => preview.accept(),
            ConfigurationImprovementDisposition::Rejected => preview.reject(),
            ConfigurationImprovementDisposition::Cancelled => preview.cancel(),
            ConfigurationImprovementDisposition::Stale => preview.mark_stale(),
        }))),
        Command::PreviewSources(sources) => api
            .preview_configuration_improvement_sources(*sources)
            .map(|preview| Outcome::SourcePreviewed {
                impacts: preview.comparison().impacts(0, 0),
                preview: Box::new(preview),
            }),
        Command::ImpactsSources {
            sources,
            presented_summary,
            offset,
            limit,
        } => {
            let preview = api.preview_configuration_improvement_sources(*sources)?;
            api.validate_presented_configuration_improvement(&preview, &presented_summary)?;
            Ok(Outcome::Impacts(
                preview.comparison().impacts(offset, limit),
            ))
        }
        Command::InspectSources {
            sources,
            presented_summary,
            subject,
            input_limit,
            maximum_inspected,
            maximum_returned,
        } => {
            let preview = api.preview_configuration_improvement_sources(*sources)?;
            api.validate_presented_configuration_improvement(&preview, &presented_summary)?;
            let explanation = preview.comparison().explain_selector(subject)?;
            let source = preview.inspect_candidate_subject(
                explanation.subject(),
                None,
                input_limit,
                maximum_inspected,
                maximum_returned,
            )?;
            let source_page_state = ConfigurationImprovementSourcePageState::from(&source);
            Ok(Outcome::SourceDetails {
                summary: Box::new(preview.summary().clone()),
                explanation,
                source: Box::new(source),
                source_page_state,
            })
        }
        Command::FinishSources {
            sources,
            presented_summary,
            disposition,
        } => api
            .finish_presented_configuration_improvement(
                api.preview_configuration_improvement_sources(*sources)?,
                &presented_summary,
                disposition,
            )
            .map(Box::new)
            .map(Outcome::Finished),
    }
}
