//! Compose read-only diagnosis from existing compiler and observation owners.

use std::path::Path;

use pale_application::doctor::{BuildSize, Check, CheckState, Report};

use super::LocalOneShotCompilationRequest;
use crate::{ApiError, PaleApi};

/// Explicit selection for one diagnostic pass; construction performs no I/O.
#[derive(Debug, Default)]
pub struct LocalDoctorRequest {
    /// Existing workspace selection; absence is reported rather than guessed.
    pub workspace: Option<LocalOneShotCompilationRequest>,
    /// Up to 32 absolute physical build directories selected by the caller.
    pub build_roots: Vec<String>,
    /// Optional maximum logical regular-file bytes in each selected tree.
    pub build_limit: Option<u64>,
}

/// Owned diagnostic output. Success concerns checks, never runtime authority.
#[derive(Debug)]
pub struct DoctorInspection {
    canonical_bytes: Vec<u8>,
    checks_passed: bool,
}

impl DoctorInspection {
    /// Borrow the complete JSON report, including unavailable and failed checks.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Whether all selected checks passed; this does not grant permission to act.
    #[must_use]
    pub const fn checks_passed(&self) -> bool {
        self.checks_passed
    }
}

impl PaleApi {
    /// Diagnose declared inputs and local observations without installing or repairing.
    ///
    /// Filesystem scans and exact executable identity probes use existing local
    /// adapters. The report keeps missing release/toolchain declarations and the
    /// absent authenticated resident diagnostic path explicitly unavailable.
    /// It never reads credentials, starts a resident or changes runtime roots.
    ///
    /// # Errors
    ///
    /// Returns [`ApiError`] only if an internal report cannot be encoded. Invalid
    /// selection and failed observations are retained as report entries so other
    /// independent checks can finish.
    pub fn inspect_local_doctor(
        &self,
        request: LocalDoctorRequest,
    ) -> Result<DoctorInspection, ApiError> {
        let mut report = Report::default();
        let platform = if matches!(std::env::consts::OS, "macos" | "linux") {
            CheckState::Passed
        } else {
            CheckState::Failed
        };
        report.checks.push(Check::new(
            "platform",
            "compiled target",
            platform,
            format!("{} / {}", std::env::consts::OS, std::env::consts::ARCH),
            "Use a supported macOS or Linux build for native observations.",
        ));
        let contracts = self.contract_bundle_identity();
        report.checks.push(Check::new(
            "embedded_contracts",
            "compiled bundle",
            CheckState::Passed,
            format!(
                "{} schemas; manifest {}; schema set {}. Embedded registry admission passed.",
                contracts.member_count(),
                contracts.schema_manifest_sha256(),
                contracts.schema_set_sha256()
            ),
            "",
        ));
        for (code, message, help) in [
            (
                "toolchains",
                "The current machine declaration has no exact toolchain pins.",
                "Complete the owner-defined toolchain declarations before claiming this deployment's prerequisites are verified.",
            ),
            (
                "distribution_release",
                "The Package lock is not an immutable executable release, notice inventory or component-source attestation.",
                "Supply the distribution owner's release evidence, including exact component references and notices; do not infer release identity from a checkout or version string.",
            ),
            (
                "core_component_selection",
                "The current diagnostic inputs have no owner-defined component-loading profile for the core/MONAD exclusion check.",
                "Supply the distribution's component selection and compatibility inspection before claiming the core excludes MONAD at runtime.",
            ),
            (
                "owner_runtime_packets",
                "Owner-native component compatibility packets are not exposed by the current diagnostic path.",
                "Use the selected owner's declared compatibility inspection; Gas Town retains ownership of its Beads and Dolt binaries.",
            ),
            (
                "resident_reachability",
                "This executable has no authenticated resident diagnostic route or selected recovery-readiness evidence.",
                "Complete and bind the authenticated resident inspection path. A socket or running process cannot establish recovery, active generation or readiness.",
            ),
        ] {
            report.checks.push(Check::new(
                code,
                "deployment",
                CheckState::Unavailable,
                message,
                help,
            ));
        }
        if let Some(workspace) = request.workspace {
            match self.compile_local_one_shot(workspace) {
                Ok(compilation) => {
                    report.checks.push(Check::new("workspace_compilation", "selected workspace", CheckState::Passed,
                        "Workspace declarations compiled through the ordinary source admission and rendering path.", ""));
                    report.checks.push(Check::new("package_contract_lock", "selected distribution", CheckState::Passed,
                        format!("Package bytes, compiler and contract pins admitted; distribution lock {}.",
                            compilation.summary().distribution_lock_digest()), ""));
                    report
                        .checks
                        .extend(pale_application::doctor::inspect_machine(
                            compilation.compilation.application(),
                            &pale_adapter_fs::LocalMachineFilesystem,
                            &pale_adapter_process::LocalProcessProbe,
                        ));
                    for repository in compilation.summary().repositories() {
                        let id = repository.repository_id().to_owned();
                        match self.inspect_local_repository_status(&compilation, id.clone()) {
                            Ok(status) => {
                                report.checks.push(Check::new("installed_surfaces", id,
                                    if status.is_clean() { CheckState::Passed } else { CheckState::Failed },
                                    if status.is_clean() { "Installed repository files match the admitted projection.".to_owned() }
                                    else { format!("Installed repository files differ: {}.",
                                        status.drift().iter().map(|item| item.status_label()).collect::<Vec<_>>().join(", ")) },
                                    "Review the repository guidance plan and its ownership before applying changes."));
                            }
                            Err(error) => {
                                report
                                    .checks
                                    .push(diagnostic("installed_surfaces", &id, &error))
                            }
                        }
                    }
                    report.checks.push(Check::new("hook_delivery", "selected hosts", CheckState::Unavailable,
                        "File comparison does not establish host callback delivery, current sessions or resident enforcement.",
                        "Complete the selected host's resident-delivery qualification before ordinary work resumes."));
                }
                Err(error) => {
                    report.checks.push(diagnostic(
                        "workspace_compilation",
                        "selected workspace",
                        &error,
                    ));
                    unavailable_dependents(&mut report);
                }
            }
        } else {
            report.checks.push(Check::new(
                "workspace_compilation",
                "workspace selection",
                CheckState::Unavailable,
                "No complete workspace selection was supplied.",
                "Supply --workspace, --project, --execution-workspace and --machine together.",
            ));
            unavailable_dependents(&mut report);
        }
        if request.build_roots.len() > 32 {
            report.checks.push(Check::new(
                "build_roots",
                "selection",
                CheckState::Failed,
                "More than 32 build roots were supplied; no build trees were scanned.",
                "Select at most 32 physical build directories for one invocation.",
            ));
        } else {
            for root in request.build_roots {
                let observed = pale_adapter_fs::measure_build_tree(Path::new(&root));
                report.add_build(
                    BuildSize {
                        root,
                        logical_bytes: observed.logical_bytes,
                        file_count: observed.file_count,
                        skipped_symlinks: observed.skipped_symlinks,
                        complete: observed.errors.is_empty(),
                        errors: observed.errors,
                    },
                    request.build_limit,
                );
            }
        }
        let checks_passed = report.checks_passed();
        let value = serde_json::json!({"checks_passed": checks_passed, "checks": report.checks, "builds": report.builds});
        let canonical_bytes = pale_core::canon::to_canonical_string(&value)
            .map(String::into_bytes)
            .map_err(|error| {
                super::local_host_failure(
                    &self.diagnostic_identity,
                    error,
                    pale_application::ApplicationFailureDisposition::Internal,
                    pale_core::records::DiagnosticStageV1::Internal,
                    "doctor_report_encoding_failed",
                    "Pale could not encode its diagnostic report",
                )
            })?;
        Ok(DoctorInspection {
            canonical_bytes,
            checks_passed,
        })
    }
}

fn diagnostic(code: &str, subject: &str, error: &ApiError) -> Check {
    let diagnostic = error.diagnostic();
    let mut details = vec![format!("{}: {}", diagnostic.code(), diagnostic.summary())];
    details.extend(diagnostic.causes().iter().cloned());
    details.extend(diagnostic.notes().iter().cloned());
    details.extend(
        diagnostic
            .facts()
            .iter()
            .map(|fact| format!("{}={}", fact.key(), fact.value())),
    );
    Check::new(
        code,
        subject,
        CheckState::Failed,
        details.join("; "),
        if diagnostic.help().is_empty() {
            "Correct the selected source identified by this diagnostic, then repeat doctor."
                .to_owned()
        } else {
            diagnostic.help().join(" ")
        },
    )
}

fn unavailable_dependents(report: &mut Report) {
    for code in [
        "package_contract_lock",
        "machine_bindings",
        "agent_executables",
        "installed_surfaces",
        "hook_delivery",
    ] {
        report.checks.push(Check::new(
            code,
            "selected workspace",
            CheckState::Unavailable,
            "This check requires a successfully admitted workspace selection.",
            "Resolve the workspace diagnostic before checking its dependent bindings.",
        ));
    }
}
