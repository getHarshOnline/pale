//! Terminal projection for API diagnostics and process-local host failures.
//!
//! Render the [`DiagnosticV1`] carried by [`ApiError`] and map its disposition
//! to a process exit status. Do not infer application failure meaning from an
//! error's source chain. The smaller host fallback handles command arguments,
//! standard streams, output encoding, and mismatched command results.
//! Filesystem confinement, executable checks, and materialization remain SDK
//! operations and return their own diagnostics.

use std::fmt::{self, Write as _};
use std::process::ExitCode;

use anyhow::Error;
use pale_sdk::{
    ApiError, ApiInitializationError, DiagnosticDisposition, DiagnosticEffectCertainty,
    DiagnosticProjection, DiagnosticRetry, DiagnosticStage, DiagnosticV1,
    LocalMaterializationOwnershipObservation, LocalMaterializationReceiptObservation,
    LocalMaterializationReconciliation, PaleApi,
};

use crate::command::DiagnosticPresentation;

/// Maximum source links inspected before one unknown tail omission is recorded.
const MAX_CAUSE_SCAN: usize = 64;
/// Allocation guard for formatting a host error before façade projection.
///
/// This is not the portable diagnostic bound. [`PaleApi::project_diagnostic`]
/// owns the smaller initial-v1 character ceilings, redaction, and omission
/// accounting.
const MAX_CAPTURE_BYTES: usize = 16 << 10;

/// Stable process dispositions selected only at the CLI boundary.
///
/// Host failures use [`Self::InvalidInput`] only for command arguments.
/// [`Self::EffectUnknown`] requires explicit evidence of uncertainty from the
/// operation owner. Application failures use the API-owned disposition through
/// [`api_diagnostic_class`], never source-chain inference.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FailureClass {
    Operational,
    InvalidInput,
    /// API-owned admission denial; no host constructor selects this variant.
    Blocked,
    Internal,
    EffectUnknown,
}

impl FailureClass {
    const fn code(self) -> &'static str {
        match self {
            Self::Operational => "host_operation_failed",
            Self::InvalidInput => "host_input_rejected",
            Self::Blocked => "admission_denied",
            Self::Internal => "host_internal_failure",
            Self::EffectUnknown => "effect_outcome_unknown",
        }
    }

    const fn exit_status(self) -> u8 {
        match self {
            Self::Operational => 1,
            Self::InvalidInput => 4,
            Self::Blocked => 5,
            Self::Internal => 6,
            Self::EffectUnknown => 7,
        }
    }

    const fn disposition(self) -> DiagnosticDisposition {
        match self {
            Self::Operational => DiagnosticDisposition::OperationalFailure,
            Self::InvalidInput => DiagnosticDisposition::InvalidInput,
            Self::Blocked => DiagnosticDisposition::Blocked,
            Self::Internal => DiagnosticDisposition::InternalFailure,
            Self::EffectUnknown => DiagnosticDisposition::EffectOutcomeUnknown,
        }
    }

    const fn retry(self) -> DiagnosticRetry {
        match self {
            Self::InvalidInput => DiagnosticRetry::AfterCorrection,
            Self::Blocked => DiagnosticRetry::AfterStateChange,
            Self::EffectUnknown => DiagnosticRetry::ReconcileFirst,
            Self::Operational | Self::Internal => DiagnosticRetry::NotIndicated,
        }
    }

    const fn effect(self) -> DiagnosticEffectCertainty {
        match self {
            Self::EffectUnknown => DiagnosticEffectCertainty::OutcomeUnknown,
            Self::Operational | Self::InvalidInput | Self::Blocked | Self::Internal => {
                DiagnosticEffectCertainty::NotApplicable
            }
        }
    }

    const fn stage(self) -> DiagnosticStage {
        match self {
            Self::Operational => DiagnosticStage::HostObservation,
            Self::InvalidInput => DiagnosticStage::InputAdmission,
            Self::Blocked => DiagnosticStage::OperationAdmission,
            Self::Internal => DiagnosticStage::Internal,
            Self::EffectUnknown => DiagnosticStage::Materialization,
        }
    }
}

/// CLI-owned classification attached at the exact host boundary that observed it.
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
struct ClassifiedHostFailure {
    class: FailureClass,
    message: String,
    #[source]
    source: Error,
}

/// Classify invalid command arguments before dispatching an SDK request.
pub(crate) fn invalid_input(message: impl Into<String>, source: Error) -> Error {
    ClassifiedHostFailure {
        class: FailureClass::InvalidInput,
        message: message.into(),
        source,
    }
    .into()
}

/// Classify a contradiction in CLI-owned dispatch or projection mechanics.
///
/// This constructor is not an escape hatch for application failures. Callers
/// use it only after a closed local branch proves that the process shell
/// received an outcome incompatible with the request it just dispatched.
pub(crate) fn internal_failure(message: impl Into<String>, source: Error) -> Error {
    ClassifiedHostFailure {
        class: FailureClass::Internal,
        message: message.into(),
        source,
    }
    .into()
}

/// One post-parse terminal failure with an admitted or initialization-fallback document.
pub(crate) struct Diagnostic {
    document: DiagnosticDocument,
    class: FailureClass,
}

/// The sole exceptional fallback when the façade itself cannot initialize.
enum DiagnosticDocument {
    Admitted {
        presentation: Box<DiagnosticPresentation>,
        reconciliation: Option<MaterializationReconciliationPresentation>,
    },
    InitializationFallback {
        code: String,
        summary: String,
    },
}

/// CLI projection of the read-back retained after an uncertain materialization.
///
/// This is supplementary evidence, not a settlement, retry permit, or new
/// portable contract. The nested admitted diagnostic remains authoritative.
struct MaterializationReconciliationPresentation {
    repository_id: String,
    plan_digest: String,
    receipt: &'static str,
    ownership: &'static str,
}

impl MaterializationReconciliationPresentation {
    fn from_observation(observation: &LocalMaterializationReconciliation) -> Self {
        Self {
            repository_id: observation.repository_id().to_owned(),
            plan_digest: observation.plan_digest().to_owned(),
            receipt: receipt_label(observation.receipt()),
            ownership: ownership_label(observation.ownership()),
        }
    }

    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "repositoryId": self.repository_id,
            "planDigest": self.plan_digest,
            "receipt": self.receipt,
            "ownership": self.ownership,
        })
    }
}

impl Diagnostic {
    /// Wrap one already-admitted API diagnostic for terminal presentation.
    ///
    /// This path performs no error-chain search, classification or diagnostic
    /// admission. It copies the SDK's authoring evidence for the requesting
    /// caller. Process exit status follows the API-owned disposition.
    pub(crate) fn from_api_error(error: &ApiError) -> Self {
        Self {
            document: DiagnosticDocument::Admitted {
                presentation: Box::new(DiagnosticPresentation::from_api_error(error)),
                reconciliation: None,
            },
            class: api_diagnostic_class(error.diagnostic()),
        }
    }

    /// Retain one uncertain materialization read-back beside its original error.
    ///
    /// This does not change diagnostic identity, classification, or retry law.
    pub(crate) fn from_api_error_with_reconciliation(
        error: &ApiError,
        reconciliation: &LocalMaterializationReconciliation,
    ) -> Self {
        Self {
            document: DiagnosticDocument::Admitted {
                presentation: Box::new(DiagnosticPresentation::from_api_error(error)),
                reconciliation: Some(MaterializationReconciliationPresentation::from_observation(
                    reconciliation,
                )),
            },
            class: api_diagnostic_class(error.diagnostic()),
        }
    }

    /// Wrap one admitted diagnostic returned inside a successful API result.
    ///
    /// Repository-scoped materialization settlement can return a closed
    /// failed-no-effect result whose target row carries the exact admitted
    /// diagnostic. The CLI renders that document directly instead of inventing
    /// a second failure classification.
    pub(crate) fn from_api_diagnostic(diagnostic: &DiagnosticV1) -> Self {
        Self {
            document: DiagnosticDocument::Admitted {
                presentation: Box::new(DiagnosticPresentation::from_diagnostic(diagnostic)),
                reconciliation: None,
            },
            class: api_diagnostic_class(diagnostic),
        }
    }

    /// Project one process-local host failure without inferring semantic policy.
    ///
    /// Wrapped [`ApiError`] values retain their exact admitted bytes. Otherwise
    /// only [`ClassifiedHostFailure`] and [`ApiInitializationError`] are
    /// recognized. Every other source chain is a generic operational host
    /// failure; application use cases must cross the shared command façade and
    /// arrive through [`Self::from_api_error`].
    pub(crate) fn from_host_error(error: &Error) -> Self {
        if let Some(api_error) = find_in_chain::<ApiError>(error) {
            return Self::from_api_error(api_error);
        }

        let (class, code) = if let Some(classified) = find_in_chain::<ClassifiedHostFailure>(error)
        {
            (classified.class, classified.class.code())
        } else if let Some(initialization) = find_in_chain::<ApiInitializationError>(error) {
            (FailureClass::Internal, initialization.code())
        } else {
            (FailureClass::Operational, FailureClass::Operational.code())
        };
        let chain = captured_chain(error);
        let summary = chain
            .entries
            .first()
            .cloned()
            .unwrap_or_else(|| "Pale host operation failed".to_string());
        let mut projection = DiagnosticProjection::new(
            code.to_string(),
            class.stage(),
            class.disposition(),
            summary,
            class.retry(),
            class.effect(),
        )
        .with_causes(chain.entries.into_iter().skip(1).collect(), chain.omitted);
        if class == FailureClass::EffectUnknown {
            projection = projection.with_help(vec![
                "Do not repeat the operation until its retained evidence has been reconciled."
                    .to_string(),
            ]);
        }
        match PaleApi::new() {
            Ok(api) => match api.project_diagnostic(projection) {
                Ok(admitted) => Self {
                    document: DiagnosticDocument::Admitted {
                        presentation: Box::new(DiagnosticPresentation::from_diagnostic(&admitted)),
                        reconciliation: None,
                    },
                    class,
                },
                Err(projection_error) => Self::from_api_error(&projection_error),
            },
            Err(initialization) => Self {
                document: DiagnosticDocument::InitializationFallback {
                    code: initialization.code().to_string(),
                    summary: initialization.to_string(),
                },
                class: FailureClass::Internal,
            },
        }
    }

    /// Return the stable process exit selected for this diagnostic.
    pub(crate) fn exit_code(&self) -> ExitCode {
        ExitCode::from(self.class.exit_status())
    }

    fn render_base_human(&self) -> String {
        match &self.document {
            DiagnosticDocument::Admitted { presentation, .. } => {
                render_api_human(presentation.diagnostic())
            }
            DiagnosticDocument::InitializationFallback { code, summary } => {
                format!("error[{code}]: {summary}")
            }
        }
    }

    fn render_human(&self) -> Result<String, serde_json::Error> {
        let mut output = self.render_base_human();
        if let DiagnosticDocument::Admitted {
            presentation,
            reconciliation,
        } = &self.document
        {
            if let Some(location) = presentation.authoring_location() {
                output.push_str("\n  Source location:");
                for line in serde_json::to_string_pretty(location)?.lines() {
                    output.push_str("\n    ");
                    output.push_str(line);
                }
            }
            if !presentation.authoring_corrections().is_empty() {
                output.push_str("\n  Suggested edits (not applied):");
                for line in
                    serde_json::to_string_pretty(presentation.authoring_corrections())?.lines()
                {
                    output.push_str("\n    ");
                    output.push_str(line);
                }
                output
                    .push_str("\n  Match the current source and review the change before writing.");
            }
            if let Some(observation) = reconciliation {
                output.push_str(&format!(
                    "\n  reconciliation: repository {}; plan {}; receipt {}; ownership {}",
                    observation.repository_id,
                    observation.plan_digest,
                    observation.receipt,
                    observation.ownership,
                ));
            }
        }
        Ok(output)
    }

    fn render_json(&self) -> Result<String, serde_json::Error> {
        match &self.document {
            DiagnosticDocument::Admitted {
                presentation,
                reconciliation,
            } => {
                let mut value = presentation.to_json_value()?;
                if let Some(observation) = reconciliation {
                    if let serde_json::Value::Object(envelope) = &mut value {
                        envelope.insert("reconciliation".to_owned(), observation.to_json());
                    }
                }
                serde_json::to_string_pretty(&value)
            }
            DiagnosticDocument::InitializationFallback { code, summary } => {
                serde_json::to_string_pretty(&serde_json::json!({
                    "severity": "error",
                    "code": code,
                    "summary": summary,
                }))
            }
        }
    }
}

const fn receipt_label(observation: LocalMaterializationReceiptObservation) -> &'static str {
    match observation {
        LocalMaterializationReceiptObservation::NotConstructed => "not-constructed",
        LocalMaterializationReceiptObservation::Exact => "exact",
        LocalMaterializationReceiptObservation::Absent => "absent",
        LocalMaterializationReceiptObservation::Different => "different",
    }
}

const fn ownership_label(observation: LocalMaterializationOwnershipObservation) -> &'static str {
    match observation {
        LocalMaterializationOwnershipObservation::Unowned => "unowned",
        LocalMaterializationOwnershipObservation::MatchesPrior => "matches-prior",
        LocalMaterializationOwnershipObservation::MatchesPost => "matches-post",
        LocalMaterializationOwnershipObservation::Other => "other",
    }
}

/// Render one admitted portable diagnostic without reclassifying or re-redacting it.
fn render_api_human(diagnostic: &DiagnosticV1) -> String {
    let mut output = format!("error[{}]: {}", diagnostic.code(), diagnostic.summary());
    output.push_str(&format!("\n  stage: {}", diagnostic.stage()));
    output.push_str(&format!("\n  disposition: {}", diagnostic.disposition()));
    output.push_str(&format!("\n  retry: {}", diagnostic.retry()));
    output.push_str(&format!("\n  effect: {}", diagnostic.effect()));

    if let Some((kind, identity)) = diagnostic.subject() {
        output.push_str(&format!("\n  subject: {kind} `{identity}`"));
    }
    if let Some(source) = diagnostic.source() {
        output.push_str(&format!("\n  --> {}", source.label()));
        if let Some(line) = source.line() {
            output.push_str(&format!(":{line}"));
            if let Some(column) = source.column() {
                output.push_str(&format!(":{column}"));
            }
        }
        if let (Some(start), Some(end)) = (source.byte_start(), source.byte_end()) {
            output.push_str(&format!("\n  byte range: {start}..{end}"));
        }
        if let Some(path) = source.value_path() {
            output.push_str(&format!("\n  value path: {path}"));
        }
    }
    if let Some(schema) = diagnostic.schema_coordinate() {
        output.push_str(&format!("\n  schema: {}", schema.registry_id()));
        if let Some(wire) = schema.wire_schema_id() {
            output.push_str(&format!(" ({wire})"));
        }
        output.push_str(&format!("\n  schema digest: {}", schema.schema_sha256()));
        if let Some(path) = schema.schema_path() {
            output.push_str(&format!("\n  schema path: {path}"));
        }
    }
    push_api_correlation(&mut output, "request", diagnostic.request_correlation());
    push_api_correlation(
        &mut output,
        "generation",
        diagnostic.generation_correlation(),
    );
    push_api_correlation(&mut output, "receipt", diagnostic.receipt_correlation());
    for fact in diagnostic.facts() {
        output.push_str(&format!("\n  {}: {}", fact.key(), fact.value()));
    }
    for cause in diagnostic.causes() {
        output.push_str(&format!("\n  caused by: {cause}"));
    }
    for note in diagnostic.notes() {
        output.push_str(&format!("\n  note: {note}"));
    }
    let truncation = diagnostic.truncation();
    if !truncation.is_empty() {
        output.push_str(&format!(
            "\n  note: diagnostic truncation: shortened_fields={}, omitted_causes={}, omitted_notes={}, omitted_help={}, omitted_facts={}",
            truncation.shortened_fields(),
            truncation.omitted_causes(),
            truncation.omitted_notes(),
            truncation.omitted_help(),
            truncation.omitted_facts(),
        ));
    }
    for help in diagnostic.help() {
        output.push_str(&format!("\n  help: {help}"));
    }
    output
}

/// Append one typed correlation without assigning it authority or effect meaning.
fn push_api_correlation(output: &mut String, label: &str, correlation: Option<(&str, &str)>) {
    if let Some((kind, identity)) = correlation {
        output.push_str(&format!("\n  {label}: {kind} `{identity}`"));
    }
}

struct CapturedChain {
    entries: Vec<String>,
    omitted: u32,
}

fn captured_chain(error: &Error) -> CapturedChain {
    let mut entries = Vec::new();
    let mut omitted = 0_u32;
    for (index, cause) in error.chain().enumerate() {
        if index == MAX_CAUSE_SCAN {
            omitted = omitted.saturating_add(1);
            break;
        }
        let rendered = captured_display(cause);
        if entries.last() == Some(&rendered) {
            continue;
        }
        entries.push(rendered);
    }
    CapturedChain { entries, omitted }
}

/// Format a foreign display value under an allocation guard.
///
/// Redaction and initial-v1 character bounds remain exclusively owned by
/// [`PaleApi::project_diagnostic`].
fn captured_display(value: &dyn fmt::Display) -> String {
    let mut prefix = PrefixWriter::new(MAX_CAPTURE_BYTES);
    let _ = write!(&mut prefix, "{value}");
    if !prefix.truncated {
        return prefix.output;
    }
    let mut end = MAX_CAPTURE_BYTES
        .saturating_sub('…'.len_utf8())
        .min(prefix.output.len());
    while !prefix.output.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    prefix.output.truncate(end);
    prefix.output.push('…');
    prefix.output
}

struct PrefixWriter {
    output: String,
    max_bytes: usize,
    truncated: bool,
}

impl PrefixWriter {
    fn new(max_bytes: usize) -> Self {
        Self {
            output: String::with_capacity(max_bytes),
            max_bytes,
            truncated: false,
        }
    }
}

impl fmt::Write for PrefixWriter {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        let remaining = self.max_bytes.saturating_sub(self.output.len());
        if remaining == 0 {
            self.truncated |= !value.is_empty();
            return Ok(());
        }
        let mut end = value.len().min(remaining);
        while !value.is_char_boundary(end) {
            end = end.saturating_sub(1);
        }
        self.output.push_str(&value[..end]);
        self.truncated |= end != value.len();
        Ok(())
    }
}

fn find_in_chain<T>(error: &Error) -> Option<&T>
where
    T: std::error::Error + Send + Sync + 'static,
{
    error.chain().find_map(|cause| cause.downcast_ref::<T>())
}

fn api_diagnostic_class(diagnostic: &DiagnosticV1) -> FailureClass {
    match diagnostic.disposition() {
        DiagnosticDisposition::OperationalFailure => FailureClass::Operational,
        DiagnosticDisposition::InvalidInput => FailureClass::InvalidInput,
        DiagnosticDisposition::Blocked => FailureClass::Blocked,
        DiagnosticDisposition::InternalFailure => FailureClass::Internal,
        DiagnosticDisposition::EffectOutcomeUnknown => FailureClass::EffectUnknown,
    }
}

/// Render one post-parse failure to stderr without changing its established
/// process or effect disposition when reporting itself fails.
pub(crate) fn report(diagnostic: &Diagnostic, format: crate::cli::ErrorFormatArg) {
    let rendered = match format {
        crate::cli::ErrorFormatArg::Human => diagnostic.render_human(),
        crate::cli::ErrorFormatArg::Json => diagnostic.render_json(),
    }
    .unwrap_or_else(|_| diagnostic.render_base_human());
    crate::terminal::report_stderr(format_args!("{rendered}"));
}

#[cfg(test)]
#[path = "../tests/internal/diagnostic.rs"]
mod tests;
