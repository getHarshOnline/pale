//! End-to-end initial-v1 compilation over synthetic Package and workspace inputs.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pale_application::compile::{
    compile_captured_effective_profile_inputs, compile_captured_portable_inputs,
    compile_portable_inputs, reconstruct_removal_inputs, CapturedCompilationInputError,
    CapturedInputLimit, CapturedLifecycleGuidance, CapturedOneShotCompilationInputs,
    CapturedPackageDocument, CapturedPackageSource, CapturedPrivateDocument,
    CapturedPrivateWorkspaceInputs, Compilation, CompileError, CompositionError,
    EffectiveProfileFinding, EffectiveProfileNativeRealization, ProviderBindingViolation,
};
use pale_application::packages::inspect_distribution;
use pale_application::plan::{
    admit_saved_materialization_plan, bind_machine_local_effects, build_run_result,
    build_success_receipt_with_lineage, compare_reconciled_materialization,
    materialization_contract_identity, MaterializationDriftReason, MaterializationIntent,
    MaterializationLineageEvidence, PlanError, PlanKind, RepositoryMaterializationInput,
    SavedPlanAdmissionError,
};
use pale_application::workspace::{
    compile_portable_workspace, resolve_entry, CapturedWorkspaceDocument, CapturedWorkspaceSource,
    OneShotSourceCaptureError, WorkspaceError, WorkspaceInputSnapshot,
};
use pale_application::ApplicationFailureDisposition;
use pale_core::records::{
    ArtifactDigest, BaselineEntry, BaselineStateV1, DistributionLockDigest, DistributionLockFile,
    ExecutionWorkspaceRepositoryAccessV1, LogicalRepositoryIdV1, UnixFileMode,
    WorkspaceDerivationSubjectV1,
};
use pale_ports::{
    SourceFilesystemError, SourceFilesystemPort, SourceReadLimit, SourceTreeLimits,
    SourceTreeSnapshot,
};

#[path = "support/compilation_features.rs"]
mod compilation_features;
#[path = "support/compilation.rs"]
mod compilation_fixtures;
#[path = "support/composition_source.rs"]
mod composition_source_fixtures;
#[path = "support/organization.rs"]
mod organization_fixtures;
#[path = "support/packages.rs"]
mod package_fixtures;
#[path = "support/provider_binding.rs"]
mod provider_binding_fixtures;
#[path = "support/responsibility.rs"]
mod responsibility_fixtures;
#[path = "support/role_package.rs"]
mod role_package_fixture;
#[path = "support/workspace.rs"]
mod workspace_fixtures;

use compilation_fixtures::SyntheticWorkspace;

#[path = "compile/current_corpus.rs"]
mod current_corpus;

type SourceOnlyCompiler =
    fn(WorkspaceInputSnapshot, &dyn SourceFilesystemPort) -> Result<Compilation, CompileError>;

#[test]
fn authored_provider_identity_mismatch_is_not_a_compiler_invariant() {
    let fixture = SyntheticWorkspace::new(false);
    let path = fixture.manifest.parent().unwrap().join("composition.toml");
    let mut source: pale_core::records::WorkspaceCompositionSourceV1 =
        toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    source.declarations.provider_bindings[0]
        .artifact_coordinates
        .executable = "changed-executable".into();
    std::fs::write(&path, toml::to_string(&source).unwrap()).unwrap();
    let error = compile_portable_inputs(
        fixture.capture_inputs().unwrap(),
        &pale_adapter_fs::LocalSourceFilesystem,
    )
    .expect_err("an unchanged identity cannot admit a changed provider body");
    assert_eq!(
        error.failure_disposition(),
        ApplicationFailureDisposition::InvalidInput
    );
    assert!(matches!(
        error,
        CompileError::CompositionInput(source)
            if matches!(*source, CompositionError::ProviderBinding {
                violation: ProviderBindingViolation::BindingIdentityMismatch,
                ..
            })
    ));
}

#[test]
fn portable_and_removal_entrances_cannot_receive_machine_observation_ports() {
    let source_only_compilers: [SourceOnlyCompiler; 2] =
        [compile_portable_inputs, reconstruct_removal_inputs];

    assert_eq!(source_only_compilers.len(), 2);
}

fn read_bytes(path: impl AsRef<std::path::Path>) -> Vec<u8> {
    std::fs::read(path).expect("read captured fixture member")
}

/// Source port that changes the second observation of one regular file.
///
/// One-shot host capture reads every selected member once, then reobserves the
/// same bounded family before handing the stable bytes to semantic admission.
struct SecondReadChanges {
    target: PathBuf,
    target_reads: Cell<usize>,
    local: pale_adapter_fs::LocalSourceFilesystem,
}

/// Source port that returns one capability revision once, then a stable successor.
struct CapabilitiesChangeAfterFirstRead {
    target: PathBuf,
    target_reads: Cell<usize>,
    first: Vec<u8>,
    successor: Vec<u8>,
    local: pale_adapter_fs::LocalSourceFilesystem,
}

/// Source port that records every observation beneath one Package distribution.
struct PackageAccessSpy {
    distribution_root: PathBuf,
    package_accesses: Cell<usize>,
    local: pale_adapter_fs::LocalSourceFilesystem,
}

/// Count real reads and supply bounded virtual guidance without a large disk fixture.
struct CaptureReadSpy {
    reads: RefCell<Vec<PathBuf>>,
    guidance_sizes: BTreeMap<PathBuf, usize>,
}

impl SourceFilesystemPort for CaptureReadSpy {
    fn read_regular(
        &self,
        path: &Path,
        limit: SourceReadLimit,
    ) -> Result<Vec<u8>, SourceFilesystemError> {
        self.reads.borrow_mut().push(path.to_path_buf());
        if let Some(size) = self.guidance_sizes.get(path) {
            assert!(*size <= limit.max_bytes());
            return Ok(vec![b'x'; *size]);
        }
        pale_adapter_fs::LocalSourceFilesystem.read_regular(path, limit)
    }

    fn read_confined_regular(
        &self,
        root: &Path,
        relative: &Path,
        limit: SourceReadLimit,
    ) -> Result<Vec<u8>, SourceFilesystemError> {
        let path = root.join(relative);
        if self.guidance_sizes.contains_key(&path) {
            return self.read_regular(&path, limit);
        }
        self.reads.borrow_mut().push(path);
        pale_adapter_fs::LocalSourceFilesystem.read_confined_regular(root, relative, limit)
    }

    fn capture_tree(
        &self,
        root: &Path,
        limits: SourceTreeLimits,
    ) -> Result<SourceTreeSnapshot, SourceFilesystemError> {
        self.reads.borrow_mut().push(root.to_path_buf());
        pale_adapter_fs::LocalSourceFilesystem.capture_tree(root, limits)
    }
}

fn capture_guidance_fixture(workspace: &SyntheticWorkspace, count: usize) -> CaptureReadSpy {
    let root = workspace.manifest.parent().unwrap();
    let path = root.join("capabilities.toml");
    let mut capabilities = std::fs::read_to_string(&path).unwrap();
    capabilities.push_str("\n[lifecycle]\n");
    let mut guidance_sizes = BTreeMap::new();
    for index in 0..count {
        let label = format!("guidance/{index:04}.md");
        capabilities.push_str(&format!(
            "[[lifecycle.behaviour]]\nid = \"guide-{index}\"\ndescription = \"Bounded guidance\"\nguidance = \"{label}\"\nevent = \"pre-tool-use\"\npredicate_kind = \"first_touch\"\naction = \"guide\"\nfailure_posture = \"fail_open\"\nstate_scope = \"session\"\n"
        ));
        guidance_sizes.insert(root.join(label), 0);
    }
    std::fs::write(path, capabilities).unwrap();
    CaptureReadSpy {
        reads: RefCell::new(Vec::new()),
        guidance_sizes,
    }
}

impl SourceFilesystemPort for CapabilitiesChangeAfterFirstRead {
    fn read_regular(
        &self,
        path: &Path,
        limit: SourceReadLimit,
    ) -> Result<Vec<u8>, SourceFilesystemError> {
        if path != self.target {
            return self.local.read_regular(path, limit);
        }
        let read = self.target_reads.get();
        self.target_reads.set(read + 1);
        Ok(if read == 0 {
            self.first.clone()
        } else {
            self.successor.clone()
        })
    }

    fn capture_tree(
        &self,
        root: &Path,
        limits: SourceTreeLimits,
    ) -> Result<SourceTreeSnapshot, SourceFilesystemError> {
        self.local.capture_tree(root, limits)
    }

    fn read_confined_regular(
        &self,
        root: &Path,
        relative: &Path,
        limit: SourceReadLimit,
    ) -> Result<Vec<u8>, SourceFilesystemError> {
        let path = root.join(relative);
        if path != self.target {
            return self.local.read_confined_regular(root, relative, limit);
        }
        let read = self.target_reads.get();
        self.target_reads.set(read + 1);
        Ok(if read == 0 {
            self.first.clone()
        } else {
            self.successor.clone()
        })
    }
}

impl SourceFilesystemPort for SecondReadChanges {
    fn read_regular(
        &self,
        path: &Path,
        limit: SourceReadLimit,
    ) -> Result<Vec<u8>, SourceFilesystemError> {
        let mut bytes = self.local.read_regular(path, limit)?;
        if path == self.target {
            let read = self.target_reads.get() + 1;
            self.target_reads.set(read);
            if read == 2 {
                bytes.extend_from_slice(b"\n# changed during bounded capture\n");
            }
        }
        Ok(bytes)
    }

    fn capture_tree(
        &self,
        root: &Path,
        limits: SourceTreeLimits,
    ) -> Result<SourceTreeSnapshot, SourceFilesystemError> {
        self.local.capture_tree(root, limits)
    }

    fn read_confined_regular(
        &self,
        root: &Path,
        relative: &Path,
        limit: SourceReadLimit,
    ) -> Result<Vec<u8>, SourceFilesystemError> {
        let path = root.join(relative);
        let mut bytes = self.local.read_confined_regular(root, relative, limit)?;
        if path == self.target {
            let read = self.target_reads.get() + 1;
            self.target_reads.set(read);
            if read == 2 {
                bytes.extend_from_slice(b"\n# changed during bounded capture\n");
            }
        }
        Ok(bytes)
    }
}

impl PackageAccessSpy {
    fn observe(&self, path: &Path) {
        if path.starts_with(&self.distribution_root) {
            self.package_accesses.set(self.package_accesses.get() + 1);
        }
    }
}

impl SourceFilesystemPort for PackageAccessSpy {
    fn read_regular(
        &self,
        path: &Path,
        limit: SourceReadLimit,
    ) -> Result<Vec<u8>, SourceFilesystemError> {
        self.observe(path);
        self.local.read_regular(path, limit)
    }

    fn capture_tree(
        &self,
        root: &Path,
        limits: SourceTreeLimits,
    ) -> Result<SourceTreeSnapshot, SourceFilesystemError> {
        self.observe(root);
        self.local.capture_tree(root, limits)
    }

    fn read_confined_regular(
        &self,
        root: &Path,
        relative: &Path,
        limit: SourceReadLimit,
    ) -> Result<Vec<u8>, SourceFilesystemError> {
        self.observe(&root.join(relative));
        self.local.read_confined_regular(root, relative, limit)
    }
}

fn captured_compilation_inputs(
    workspace: &SyntheticWorkspace,
    project_id: &str,
    machine_id: &str,
    capabilities_toml: Option<Vec<u8>>,
    guidance: Vec<CapturedLifecycleGuidance>,
) -> CapturedOneShotCompilationInputs {
    captured_compilation_inputs_with_capabilities_label(
        workspace,
        project_id,
        machine_id,
        "capabilities.toml",
        capabilities_toml,
        guidance,
    )
}

fn captured_compilation_inputs_with_capabilities_label(
    workspace: &SyntheticWorkspace,
    project_id: &str,
    machine_id: &str,
    capabilities_label: &str,
    capabilities_toml: Option<Vec<u8>>,
    guidance: Vec<CapturedLifecycleGuidance>,
) -> CapturedOneShotCompilationInputs {
    let workspace_root = workspace.root.path().join("workspace");
    let private = CapturedPrivateWorkspaceInputs::new(
        private_document(
            capabilities_label,
            capabilities_toml
                .unwrap_or_else(|| read_bytes(workspace_root.join(capabilities_label))),
        ),
        private_document(
            "machines/machine.toml",
            read_bytes(workspace_root.join("machines/machine.toml")),
        ),
        private_document(
            "distribution-lock.toml",
            read_bytes(workspace_root.join("distribution-lock.toml")),
        ),
        guidance,
    )
    .expect("bounded private fixture");
    let package_root = workspace.root.path().join("distribution/packages/engine");
    let packages = CapturedPackageSource::new(
        ["package.toml", "roles.toml", "role.md"]
            .into_iter()
            .map(|name| {
                CapturedPackageDocument::new(
                    format!("engine/{name}"),
                    read_bytes(package_root.join(name)),
                )
                .expect("bounded Package fixture")
            })
            .collect(),
    )
    .expect("bounded Package tree");
    CapturedOneShotCompilationInputs::new(
        project_id.to_owned(),
        "default".to_owned(),
        machine_id.to_owned(),
        private,
        packages,
    )
}

fn captured_workspace_universe(
    workspace: &SyntheticWorkspace,
) -> pale_application::workspace::PortableWorkspaceUniverse {
    let workspace_root = workspace.root.path().join("workspace");
    let root = CapturedWorkspaceSource::admit_root_manifest("pale-workspace.toml".into())
        .expect("confined root manifest");
    let documents = [
        "pale-workspace.toml",
        "composition.toml",
        "workspace-resolution.lock.toml",
        "workspace-universe.lock.toml",
    ]
    .into_iter()
    .map(|label| {
        CapturedWorkspaceDocument::new(label.to_owned(), read_bytes(workspace_root.join(label)))
            .expect("bounded workspace fixture")
    })
    .collect();
    compile_portable_workspace(
        CapturedWorkspaceSource::new(root, documents).expect("complete workspace fixture"),
    )
    .expect("admitted workspace fixture")
}

#[test]
fn portable_source_subjects_keep_named_occurrences_separate_from_proof_inputs() {
    use pale_application::workspace::{
        AuthoringInstancePathSegment::*, AuthoringOccurrenceRole, PortableWorkspaceSourceSubject,
        PortableWorkspaceSubjectOccurrences,
    };
    let workspace = SyntheticWorkspace::new(false);
    let universe = captured_workspace_universe(&workspace);
    let PortableWorkspaceSourceSubject::Selected {
        occurrence,
        subject,
    } = universe.source_subject(
        "pale-workspace.toml",
        &[Field("workspace".into()), Field("workspace_id".into())],
    )
    else {
        panic!("workspace definition denotes the selected workspace proof");
    };
    assert_eq!(occurrence.role(), AuthoringOccurrenceRole::Definition);
    assert!(matches!(
        subject,
        WorkspaceDerivationSubjectV1::Workspace { .. }
    ));
    let mut cursor = None;
    let mut rows = Vec::new();
    loop {
        let PortableWorkspaceSubjectOccurrences::Selected(page) =
            universe.subject_occurrences(subject, cursor.as_ref(), 2, 1)
        else {
            panic!("membership must not depend on the page");
        };
        assert!(page.inspected() <= 2);
        assert!(page.occurrences().len() <= 1);
        rows.extend(page.occurrences().iter().copied());
        cursor = page.continuation().cloned();
        if cursor.is_none() {
            break;
        }
    }
    assert!(rows.contains(&occurrence));
    assert!(rows
        .iter()
        .any(|row| row.role() == AuthoringOccurrenceRole::Reference));
    for row in &rows {
        let PortableWorkspaceSourceSubject::Selected {
            occurrence: located,
            subject: located_subject,
        } = universe.source_subject(row.source_label(), row.path())
        else {
            panic!("reverse rows must round-trip to the same selected subject");
        };
        assert_eq!(located, *row);
        assert_eq!(located_subject, subject);
    }
    assert!(matches!(
        universe.source_subject("pale-workspace.toml", &[]),
        PortableWorkspaceSourceSubject::Selected {
            subject: WorkspaceDerivationSubjectV1::Source { .. },
            ..
        }
    ));
    assert!(matches!(
        universe.source_subject("pale-workspace.toml", &[Field("not_indexed".into())]),
        PortableWorkspaceSourceSubject::NoOccurrence
    ));
    assert!(matches!(
        universe.source_subject(
            "pale-workspace.toml",
            &[
                Field("workspace".into()),
                Field("desired_topology".into()),
                Field("projects".into()),
                Index(0),
                Field("id".into()),
            ]
        ),
        PortableWorkspaceSourceSubject::NoDenotation { .. }
    ));

    // A selected closure has proof inputs, but its direct source names are not
    // fabricated by coercing the individual composition identities into it.
    let proof = universe
        .direct_dependencies(&WorkspaceDerivationSubjectV1::ExperienceComposition, 0, 64)
        .unwrap()
        .unwrap();
    assert!(proof.dependencies().len() > 0);
    let mut cursor = None;
    loop {
        let PortableWorkspaceSubjectOccurrences::Selected(page) =
            universe.subject_occurrences(&proof.root().subject, cursor.as_ref(), 64, 64)
        else {
            panic!("the compiler proof already establishes membership");
        };
        assert!(page.occurrences().is_empty());
        cursor = page.continuation().cloned();
        if cursor.is_none() {
            break;
        }
    }
}

fn package_document(label: impl Into<String>, byte_len: usize) -> CapturedPackageDocument {
    CapturedPackageDocument::new(label.into(), vec![b'x'; byte_len])
        .expect("test document is individually admissible")
}

#[path = "support/captured_one_shot.rs"]
mod captured_one_shot;
use captured_one_shot::{private_document, shared_captured_fixture_compilation};

#[test]
fn compiled_json_surfaces_are_readable_lf_documents() {
    let compilation = shared_captured_fixture_compilation();
    let json_surfaces = compilation.repositories()[0]
        .render()
        .surfaces
        .iter()
        .filter(|surface| surface.rel_path.ends_with(".json"))
        .collect::<Vec<_>>();

    assert!(
        !json_surfaces.is_empty(),
        "the compiled fixture must exercise generated JSON surfaces"
    );
    for surface in json_surfaces {
        let document = surface.content();
        assert!(
            document.ends_with('\n') && !document.ends_with("\n\n"),
            "{} must end in exactly one line feed",
            surface.rel_path
        );
        assert!(
            !document.contains(['\r', '\t']),
            "{} must use LF and spaces",
            surface.rel_path
        );
        let value = pale_core::canon::parse_strict(document.as_bytes()).unwrap_or_else(|error| {
            panic!("{} must remain strict JSON: {error}", surface.rel_path)
        });
        if matches!(&value, serde_json::Value::Array(items) if !items.is_empty())
            || matches!(&value, serde_json::Value::Object(entries) if !entries.is_empty())
        {
            assert!(
                document.lines().count() > 1,
                "{} must be a readable multiline document",
                surface.rel_path
            );
        }
    }
}

#[test]
fn composition_change_invalidates_profile_observation_coordinates() {
    use pale_application::observation::{
        ObservationCancellation, ObservationCausalSelector, ObservationCausalStep,
        ObservationDeltaOutcome, ObservationDeltaRequest, ObservationFilter,
        ObservationPageDisposition, ObservationPageRequest, ObservationResynchronizationReason,
        ObservationSnapshot, ObservationSubject,
    };
    use pale_core::records::{
        CompositionReasonStepV1, ExperiencePathV1, WorkspaceCompositionSourceV1,
    };

    let workspace = SyntheticWorkspace::new(false);
    let compile = || {
        compile_captured_effective_profile_inputs(
            captured_compilation_inputs(&workspace, "demo", "machine", None, Vec::new()),
            &captured_workspace_universe(&workspace),
        )
        .expect("captured fixture admits the effective-profile phase")
    };
    let baseline = compile();
    let before = ObservationSnapshot::from_effective_profile(&baseline);
    let selection = baseline.profile().execution_paths[0].clone();
    let old_selector = ObservationCausalSelector {
        generation: before.cursor().generation().clone(),
        subject: ObservationSubject::ExecutionPath(selection.id),
    };
    let old_path = before
        .causal_path(&old_selector)
        .expect("current selector is admitted")
        .expect("selected path has compiler-owned reasons");
    let ObservationPageDisposition::Ready(old_first_page) = before
        .page(ObservationPageRequest::new(ObservationFilter::all(), 1))
        .expect("first page is current")
    else {
        panic!("active page request was cancelled");
    };
    let old_page = old_first_page
        .continuation
        .expect("fixture has more than one observation fact");

    let source_path = workspace.root.path().join("workspace/composition.toml");
    let mut source: WorkspaceCompositionSourceV1 =
        toml::from_str(&std::fs::read_to_string(&source_path).expect("read composition fixture"))
            .expect("parse typed composition fixture");
    let path = source
        .declarations
        .selected_experience_paths
        .iter_mut()
        .find(|path| path.id == selection.experience_path_id)
        .expect("selection resolves to the fixture path");
    let old_continuity_id = path.continuity_contract_id.clone();
    let new_continuity_id = format!("{old_continuity_id}-reviewed");
    let mut path_body = path.to_preimage();
    path_body.continuity_contract_id = new_continuity_id.clone();
    *path = ExperiencePathV1::seal(path_body).expect("canonical path amendment reseals");
    source
        .declarations
        .continuity_contracts
        .iter_mut()
        .find(|contract| contract.id == old_continuity_id)
        .expect("path continuity reference resolves")
        .id = new_continuity_id.clone();
    std::fs::write(
        source_path,
        toml::to_string(&source).expect("serialize the amended fixture declaration"),
    )
    .expect("write only the temporary fixture");

    let candidate = compile();
    assert_eq!(baseline.profile(), candidate.profile());
    assert_eq!(baseline.instance(), candidate.instance());
    assert_ne!(
        baseline.experience_composition_plan().plan_digest,
        candidate.experience_composition_plan().plan_digest
    );
    let after = ObservationSnapshot::from_effective_profile(&candidate);
    let current_selector = ObservationCausalSelector {
        generation: after.cursor().generation().clone(),
        subject: old_selector.subject.clone(),
    };
    let current_path = after
        .causal_path(&current_selector)
        .expect("fresh candidate selector is admitted")
        .expect("candidate retains the selected path");
    assert_ne!(old_path.steps, current_path.steps);
    assert!(current_path
        .steps
        .contains(&ObservationCausalStep::Composition(
            CompositionReasonStepV1::ContinuityContract {
                contract_id: new_continuity_id,
            },
        )));

    let delta = after.delta_from(
        &before,
        ObservationDeltaRequest {
            resume: before.resume_coordinate(),
            maximum_changes: 0,
            cancellation: ObservationCancellation::Active,
        },
    );
    let stale_causal = after.causal_path(&old_selector);
    let stale_page =
        after.page(ObservationPageRequest::new(ObservationFilter::all(), 1).after(old_page));
    assert_eq!(
        (
            before.cursor().generation() != after.cursor().generation(),
            matches!(delta, ObservationDeltaOutcome::Resynchronize(ref rejection)
                if matches!(rejection.reason, ObservationResynchronizationReason::GenerationMismatch { .. })),
            matches!(stale_causal, Err(ref rejection)
                if matches!(rejection.reason, ObservationResynchronizationReason::GenerationMismatch { .. })),
            matches!(stale_page, Err(ref rejection)
                if matches!(rejection.reason, ObservationResynchronizationReason::ContinuationMismatch)),
        ),
        (true, true, true, true),
        "changed causal meaning must invalidate generation, delta resume, causal selector, and page continuation; delta: {delta:?}"
    );
}

#[test]
fn shared_captured_fixture_and_observation_target_law_remain_admitted() {
    let compilation = shared_captured_fixture_compilation();
    let repository_id = compilation.repositories()[0].id();
    let input = RepositoryMaterializationInput::for_repository(&compilation, repository_id)
        .expect("shared repository planning input");
    let apply_targets = input
        .required_observation_targets(PlanKind::Apply, None)
        .expect("desired observation targets");
    assert!(!apply_targets.is_empty());
    assert!(apply_targets.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(apply_targets.iter().all(|target| !target.starts_with('{')));
    assert!(input
        .required_observation_targets(PlanKind::Remove, None)
        .expect("unowned removal targets")
        .is_empty());

    let baseline = apply_targets
        .iter()
        .map(|target| BaselineEntry {
            rel_path: target.clone(),
            observation: BaselineStateV1::Absent {},
        })
        .collect();
    let plan = input
        .build(MaterializationIntent::Apply, None, baseline)
        .expect("the projected target set is an exact planning baseline")
        .into_parts()
        .0;
    let prior = plan
        .transition
        .next_state()
        .expect("initial apply publishes ownership state");
    let prior_targets = prior
        .surfaces
        .iter()
        .map(|surface| surface.rel_path.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        input
            .required_observation_targets(PlanKind::Apply, Some(prior))
            .expect("apply union targets"),
        apply_targets
    );
    assert_eq!(
        input
            .required_observation_targets(PlanKind::Remove, Some(prior))
            .expect("remove prior targets"),
        prior_targets
    );
}

#[test]
fn repository_intent_and_saved_plan_admission_share_one_application_owner() {
    let compilation = shared_captured_fixture_compilation();
    let repository_id = compilation.repositories()[0].id();
    let input = RepositoryMaterializationInput::for_repository(&compilation, repository_id)
        .expect("shared repository planning input");
    let targets = input
        .required_observation_targets(PlanKind::Apply, None)
        .expect("apply observation targets");
    let absent = targets
        .iter()
        .map(|target| BaselineEntry {
            rel_path: target.clone(),
            observation: BaselineStateV1::Absent {},
        })
        .collect::<Vec<_>>();
    let planned = input
        .build(MaterializationIntent::Apply, None, absent)
        .expect("application-owned apply plan");
    assert_eq!(
        planned.payloads().keys().collect::<Vec<_>>(),
        targets.iter().collect::<Vec<_>>()
    );
    let (surface, _) = planned.into_parts();
    let rebuilt = bind_machine_local_effects(surface, Vec::new()).expect("complete rebuilt plan");
    let saved = rebuilt.canonical_bytes().to_vec();
    let admitted = admit_saved_materialization_plan(&saved, &rebuilt, PlanKind::Apply)
        .expect("exact saved plan admission");
    assert_eq!(admitted.digest(), &rebuilt.plan_digest);

    let present = targets
        .iter()
        .map(|target| BaselineEntry {
            rel_path: target.clone(),
            observation: BaselineStateV1::Present {
                sha256: ArtifactDigest::hash_bytes(target.as_bytes()),
                mode: UnixFileMode::DEFAULT_FILE,
            },
        })
        .collect::<Vec<_>>();
    let adoption = input
        .build(MaterializationIntent::AdoptUnowned, None, present)
        .expect("application-owned adoption plan");
    let (surface, _) = adoption.into_parts();
    let adoption = bind_machine_local_effects(surface, Vec::new()).expect("complete adoption plan");
    let stale = admit_saved_materialization_plan(&saved, &adoption, PlanKind::Apply)
        .expect_err("different current plan must be stale");
    assert!(matches!(
        stale,
        SavedPlanAdmissionError::Admission(pale_core::admission::AdmissionError::Stale)
    ));
    assert_eq!(stale.disposition(), ApplicationFailureDisposition::Blocked);

    let malformed = admit_saved_materialization_plan(b"[", &rebuilt, PlanKind::Apply)
        .expect_err("malformed saved plan must fail before admission");
    assert!(matches!(malformed, SavedPlanAdmissionError::Syntax(_)));
    assert_eq!(
        malformed.disposition(),
        ApplicationFailureDisposition::InvalidInput
    );
}

#[test]
fn captured_and_source_backed_ingress_share_one_compiler_identity() {
    let workspace = SyntheticWorkspace::new(false);
    let source_backed = workspace.compile().expect("source-backed compilation");
    let captured = compile_captured_portable_inputs(
        captured_compilation_inputs(&workspace, "demo", "machine", None, Vec::new()),
        captured_workspace_universe(&workspace),
    )
    .expect("captured compilation");

    assert_eq!(
        source_backed.compilation_input_digest(),
        captured.compilation_input_digest()
    );
    assert_eq!(
        source_backed.one_shot_compilation_digest(),
        captured.one_shot_compilation_digest()
    );
    assert_eq!(source_backed.profile(), captured.profile());
    assert_eq!(
        captured.profile().repositories[0].access,
        ExecutionWorkspaceRepositoryAccessV1::Mutate
    );
    assert_eq!(source_backed.instance(), captured.instance());
    assert!(captured.executable_identity_digest().is_none());
}

#[test]
fn captured_capabilities_paths_preserve_the_confined_workspace_grammar() {
    let workspace = SyntheticWorkspace::new(false);
    let workspace_root = workspace.root.path().join("workspace");
    let nested_label = "project-sets/demo/capabilities.toml";
    let nested_path = workspace_root.join(nested_label);
    std::fs::create_dir_all(nested_path.parent().unwrap()).unwrap();
    std::fs::rename(workspace_root.join("capabilities.toml"), &nested_path).unwrap();
    let manifest = std::fs::read_to_string(&workspace.manifest)
        .unwrap()
        .replace(
            "capabilities_file = \"capabilities.toml\"",
            &format!("capabilities_file = {nested_label:?}"),
        );
    std::fs::write(&workspace.manifest, manifest).unwrap();

    let source_backed = workspace
        .compile()
        .expect("nested source-backed capabilities");
    let captured = compile_captured_portable_inputs(
        captured_compilation_inputs_with_capabilities_label(
            &workspace,
            "demo",
            "machine",
            nested_label,
            None,
            Vec::new(),
        ),
        captured_workspace_universe(&workspace),
    )
    .expect("nested captured capabilities");
    assert_eq!(
        source_backed.one_shot_compilation_digest(),
        captured.one_shot_compilation_digest()
    );

    for escaped in ["../capabilities.toml", "/capabilities.toml"] {
        assert!(CapturedPrivateDocument::new(escaped.to_owned(), Vec::new()).is_err());
    }
}

#[test]
fn retained_effective_profile_phase_shares_workspace_authority_and_full_compiler_semantics() {
    let workspace = SyntheticWorkspace::new(false);
    let portable = captured_workspace_universe(&workspace);
    let portable_principal_digest = portable.principal_universe_digest().to_owned();
    let effective = compile_captured_effective_profile_inputs(
        captured_compilation_inputs(&workspace, "demo", "machine", None, Vec::new()),
        &portable,
    )
    .expect("effective profile compiles without consuming its portable owner");

    assert_eq!(
        effective.principal_admitted_universe_digest().as_str(),
        portable_principal_digest
    );
    let inspection = effective.inspection();
    assert_eq!(inspection.workspace_id(), "compile-tests");
    assert_eq!(inspection.project_id(), "demo");
    assert_eq!(inspection.execution_workspace_id(), "default");
    assert_eq!(
        inspection.package_closure(),
        effective.profile().package_closure
    );
    assert_eq!(
        inspection.normative_pack_closure(),
        effective.profile().normative_pack_closure
    );
    assert_eq!(
        inspection.declared_semantic_selection(),
        &effective.profile().semantic
    );
    assert_eq!(
        inspection.governing_order(),
        &effective.profile().governing_order
    );
    assert_eq!(
        inspection.capability_narrowing(),
        effective.profile().capability_narrowing.as_ref()
    );
    assert_eq!(inspection.run_policy(), &effective.profile().run_policy);
    assert_eq!(inspection.session(), effective.profile().session.as_ref());
    assert!(matches!(
        inspection.native_realization(),
        EffectiveProfileNativeRealization::Unknown {
            machine_id: "machine"
        }
    ));
    assert!(inspection.findings().iter().any(|finding| matches!(
        finding,
        EffectiveProfileFinding::NativeRealizationUnknown {
            machine_id: "machine"
        }
    )));
    let repositories = inspection.repositories().collect::<Vec<_>>();
    assert_eq!(repositories.len(), 1);
    assert_eq!(repositories[0].repository_id().as_str(), "engine");
    assert_eq!(repositories[0].materialization_order(), 0);
    assert_eq!(
        repositories[0].access_ceiling(),
        ExecutionWorkspaceRepositoryAccessV1::Mutate
    );
    assert_eq!(
        repositories[0].role(),
        &effective.profile().repositories[0].role
    );
    assert_eq!(
        repositories[0].stack_overlays(),
        effective.profile().repositories[0].stacks
    );
    assert_eq!(
        repositories[0].policy_families(),
        effective.profile().repositories[0].families
    );
    let paths = inspection.execution_paths().collect::<Vec<_>>();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0].selection_id().as_str(), "direct-claude");
    assert_eq!(paths[0].experience_path_id(), "direct-claude-code");
    assert!(paths[0].terminal_provider_binding().is_some());
    assert!(!paths[0].responsibility_contract_ids().is_empty());
    let selected_contracts = paths[0].responsibility_contracts().collect::<Vec<_>>();
    assert_eq!(
        selected_contracts
            .iter()
            .map(|contract| contract.contract_id())
            .collect::<Vec<_>>(),
        paths[0]
            .responsibility_contract_ids()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
    );
    assert!(selected_contracts
        .iter()
        .all(|contract| !contract.responsibilities().is_empty()));
    assert!(!paths[0].reason_steps().is_empty());

    let full = compile_captured_portable_inputs(
        captured_compilation_inputs(&workspace, "demo", "machine", None, Vec::new()),
        captured_workspace_universe(&workspace),
    )
    .expect("full compiler consumes the same effective-profile phase");
    assert_eq!(effective.profile(), full.profile());
    assert_eq!(effective.instance(), full.instance());
    assert_eq!(
        effective.experience_composition_plan(),
        full.experience_composition_plan()
    );
    assert_eq!(
        effective.repository_semantic_plans(),
        full.repositories()
            .iter()
            .map(|repository| repository.semantic_plan())
            .cloned()
            .collect::<Vec<_>>()
    );
}

#[test]
fn compilation_inspection_is_the_single_repository_observation_preimage() {
    let workspace = SyntheticWorkspace::new(false);
    let source = std::fs::read_to_string(&workspace.manifest).unwrap();
    std::fs::write(
        &workspace.manifest,
        source.replace("access = \"mutate\"", "access = \"read\""),
    )
    .unwrap();
    let compilation = workspace.compile().expect("source-backed compilation");
    let inspection = compilation.inspection();

    assert_eq!(
        inspection.repository_count(),
        compilation.repositories().len()
    );
    assert_eq!(
        inspection.execution_path_ids().collect::<Vec<_>>(),
        compilation
            .profile()
            .execution_paths
            .iter()
            .map(pale_core::records::ExecutionPathSelectionV1::id)
            .collect::<Vec<_>>()
    );
    for (observed, compiled) in inspection.repositories().zip(compilation.repositories()) {
        assert_eq!(observed.repository_id(), compiled.id());
        assert_eq!(
            observed.manifest_digest(),
            &compiled.manifest().manifest_digest
        );
        assert_eq!(
            observed.render_digest(),
            compilation.repository_render_digest(compiled.id())
        );
        assert_eq!(observed.surface_count(), compiled.render().surfaces.len());
        assert_eq!(
            observed.materialized_surface_count(),
            compiled
                .render()
                .surfaces
                .iter()
                .filter(|surface| {
                    !surface.deferred
                        && pale_application::plan::is_filesystem_target(&surface.rel_path)
                })
                .count()
        );
        assert_eq!(
            observed.deferred_surface_count(),
            compiled
                .render()
                .surfaces
                .iter()
                .filter(|surface| surface.deferred)
                .count()
        );
        assert_eq!(
            observed.committable_surface_count(),
            compiled
                .render()
                .surfaces
                .iter()
                .filter(|surface| surface.committable)
                .count()
        );

        let mut expected_paths = compiled
            .render()
            .surfaces
            .iter()
            .map(|surface| surface.rel_path.as_str())
            .collect::<Vec<_>>();
        expected_paths.sort_unstable();
        assert_eq!(observed.surface_paths(), expected_paths);

        let support = &compiled.render().support;
        assert_eq!(observed.execution_paths().len(), support.len());
        assert_eq!(observed.execution_paths().as_ptr(), support.as_ptr());
        assert_eq!(
            observed.activation_selected_execution_path_ids(),
            support
                .iter()
                .filter(|row| row.activation_selected)
                .map(|row| row.execution_path_witness.selection.id())
                .collect::<Vec<_>>()
        );
        assert!(
            support
                .iter()
                .any(|row| !row.hook_requirements.events().is_empty()),
            "the fixture must distinguish retained hook requirements from activation wiring"
        );
        assert!(support.iter().all(|row| !row.activation_selected));
        assert!(observed.activation_selected_hook_events().is_empty());

        let semantics = compiled.semantic_plan();
        assert_eq!(observed.operation_count(), semantics.operations.len());
        assert_eq!(observed.role_count(), semantics.roles.len());
        assert_eq!(observed.doctrine_count(), semantics.doctrine.len());
        assert_eq!(observed.run_mode_count(), semantics.run_modes.len());
        assert_eq!(observed.stack_count(), semantics.stacks.len());
        assert_eq!(
            observed.policy_family_count(),
            semantics.policy_families.len()
        );
        assert_eq!(observed.recipe_count(), semantics.recipes.len());
    }
}

#[test]
fn materialization_contract_identity_is_bound_by_the_compilation_owner() {
    let compilation = shared_captured_fixture_compilation();
    let identity = materialization_contract_identity(&compilation)
        .expect("embedded materialization contracts are selected");

    assert_eq!(
        identity.state_schema,
        pale_contracts::record_schema_identity("materialization-state.v1").unwrap()
    );
    assert_eq!(
        identity.lineage_schema,
        pale_contracts::record_schema_identity("materialization-lineage.v1").unwrap()
    );
    assert_eq!(
        &identity.schema_registry,
        compilation.schema_registry_identity()
    );
    assert_eq!(
        &identity.one_shot_compilation_digest,
        compilation.one_shot_compilation_digest()
    );
}

#[test]
fn admitted_receipt_and_run_result_retain_their_exact_canonical_bytes() {
    let compilation = shared_captured_fixture_compilation();
    let repository_id = compilation.repositories()[0].id();
    let input = RepositoryMaterializationInput::for_repository(&compilation, repository_id)
        .expect("shared repository planning input");
    let planned = input
        .build(MaterializationIntent::Remove, None, Vec::new())
        .expect("unowned removal plan");
    let (surface_plan, payloads) = planned.into_parts();
    assert!(payloads.is_empty());
    let plan = bind_machine_local_effects(surface_plan, Vec::new())
        .expect("removal plan has no machine-local effects");
    let admitted_plan =
        admit_saved_materialization_plan(plan.canonical_bytes(), &plan, PlanKind::Remove)
            .expect("exact rebuilt removal plan is admitted");
    let receipt = build_success_receipt_with_lineage(
        &admitted_plan,
        Vec::new(),
        Vec::new(),
        MaterializationLineageEvidence::NoOwnedState,
    )
    .expect("no-owned-state removal receipt is admitted");

    let expected_receipt = pale_core::canon::to_canonical_string(receipt.record()).unwrap();
    assert_eq!(receipt.canonical_bytes(), expected_receipt.as_bytes());
    let receipt_value = pale_core::canon::parse_strict(receipt.canonical_bytes()).unwrap();
    let decoded_receipt: pale_core::records::MaterializationReceiptV1 =
        serde_json::from_value(receipt_value).unwrap();
    assert_eq!(&decoded_receipt, receipt.record());
    assert_eq!(
        decoded_receipt.recompute_digest().unwrap(),
        decoded_receipt.receipt_digest
    );

    let result = build_run_result(
        &compilation,
        PlanKind::Remove,
        vec![receipt.into_run_target()],
    )
    .expect("receipt-backed complete run result is admitted");
    let expected_result = pale_core::canon::to_canonical_string(result.record()).unwrap();
    assert_eq!(result.canonical_bytes(), expected_result.as_bytes());
    let result_value = pale_core::canon::parse_strict(result.canonical_bytes()).unwrap();
    let decoded_result: pale_core::records::RunResultV1 =
        serde_json::from_value(result_value).unwrap();
    assert_eq!(&decoded_result, result.record());
    assert_eq!(
        decoded_result.recompute_digest().unwrap(),
        decoded_result.run_digest
    );
}

#[test]
fn execution_workspace_repository_access_changes_portable_profile_identity() {
    let mutate = SyntheticWorkspace::new(false);
    let mutate_compilation = mutate.compile().expect("mutating workspace compilation");

    let read = SyntheticWorkspace::new(false);
    let source = std::fs::read_to_string(&read.manifest).unwrap();
    assert!(source.contains("access = \"mutate\""));
    std::fs::write(
        &read.manifest,
        source.replace("access = \"mutate\"", "access = \"read\""),
    )
    .unwrap();
    let read_compilation = read.compile().expect("read-only workspace compilation");

    assert_eq!(
        read_compilation.profile().repositories[0].access,
        ExecutionWorkspaceRepositoryAccessV1::Read
    );
    assert_eq!(
        mutate_compilation.profile().repositories[0].access,
        ExecutionWorkspaceRepositoryAccessV1::Mutate
    );
    assert_ne!(
        read_compilation.profile().profile_digest,
        mutate_compilation.profile().profile_digest,
        "the execution-workspace access ceiling is portable semantic identity"
    );
}

#[test]
fn read_repository_membership_observes_but_cannot_construct_mutation_plans() {
    let workspace = SyntheticWorkspace::new(false);
    let source = std::fs::read_to_string(&workspace.manifest).unwrap();
    assert!(source.contains("access = \"mutate\""));
    std::fs::write(
        &workspace.manifest,
        source.replace("access = \"mutate\"", "access = \"read\""),
    )
    .unwrap();
    let compilation = workspace
        .compile()
        .expect("read-only workspace compilation");
    let support = &compilation.repositories()[0].render().support;
    assert!(!support.is_empty());
    assert!(support.iter().all(|support| support
        .hook_requirements
        .events()
        .contains(&pale_application::hooks::HookEvent::PreToolUse)));
    let hook_context = compilation.repositories()[0]
        .render()
        .surfaces
        .iter()
        .find(|surface| surface.rel_path.ends_with("/hook-context.json"))
        .expect("the selected route renders one hook context");
    let hook_context: serde_json::Value =
        serde_json::from_str(hook_context.content()).expect("hook context is JSON");
    assert_eq!(
        hook_context["read_only_dirs"],
        serde_json::json!([compilation
            .instance()
            .repositories
            .get(compilation.repositories()[0].id())
            .expect("engine checkout")
            .as_str()])
    );
    let input = RepositoryMaterializationInput::for_repository(
        &compilation,
        compilation.repositories()[0].id(),
    )
    .expect("read-only repository is still an observable profile member");
    let targets = input
        .required_observation_targets(PlanKind::Apply, None)
        .expect("read access permits effect-free target observation");
    let baseline = targets
        .into_iter()
        .map(|rel_path| BaselineEntry {
            rel_path,
            observation: BaselineStateV1::Absent {},
        })
        .collect::<Vec<_>>();

    let status = compare_reconciled_materialization(input, None, baseline.clone())
        .expect("read access permits semantic comparison")
        .complete(Vec::new(), &Default::default())
        .expect("empty machine-local observation set is valid");
    assert!(!status.is_clean());
    assert!(status
        .reasons()
        .iter()
        .any(|reason| matches!(reason, MaterializationDriftReason::OwnershipState { .. })));

    for error in [
        input
            .build(MaterializationIntent::Apply, None, baseline.clone())
            .expect_err("read access cannot produce an apply plan"),
        input
            .build(MaterializationIntent::AdoptUnowned, None, baseline)
            .expect_err("read access cannot produce an adoption plan"),
        input
            .build(MaterializationIntent::Remove, None, Vec::new())
            .expect_err("read access cannot produce a removal plan"),
    ] {
        assert!(matches!(
            error,
            PlanError::RepositoryMutationDenied { repository }
                if repository.as_str() == "engine"
        ));
    }
}

#[test]
fn captured_selectors_are_verified_against_admitted_topology_and_machine_records() {
    let workspace = SyntheticWorkspace::new(false);
    let error = compile_captured_portable_inputs(
        captured_compilation_inputs(&workspace, "another-set", "machine", None, Vec::new()),
        captured_workspace_universe(&workspace),
    )
    .expect_err("the selected project must exist in the admitted topology");
    assert!(matches!(
        error,
        CompileError::Workspace(source)
            if matches!(source.as_ref(), pale_application::workspace::WorkspaceError::UnknownProject(id)
                if id == "another-set")
    ));

    let error = compile_captured_portable_inputs(
        captured_compilation_inputs(&workspace, "demo", "another-machine", None, Vec::new()),
        captured_workspace_universe(&workspace),
    )
    .expect_err("declared machine identity must match the selector");
    assert!(matches!(
        error,
        CompileError::Workspace(source)
            if matches!(source.as_ref(), pale_application::workspace::WorkspaceError::IdentityMismatch {
                kind,
                declared,
                requested,
            } if kind == "machine" && declared == "machine" && requested == "another-machine")
    ));
}

#[test]
fn captured_package_tree_matches_source_capture_limits() {
    let exact_depth = (0..24)
        .map(|index| format!("d{index}"))
        .chain(std::iter::once("body.md".to_owned()))
        .collect::<Vec<_>>()
        .join("/");
    CapturedPackageSource::new(vec![package_document(exact_depth, 0)])
        .expect("a file below a depth-24 parent is admitted");
    let excessive_depth = (0..25)
        .map(|index| format!("d{index}"))
        .chain(std::iter::once("body.md".to_owned()))
        .collect::<Vec<_>>()
        .join("/");
    assert!(matches!(
        CapturedPackageSource::new(vec![package_document(excessive_depth, 0)]),
        Err(CapturedCompilationInputError::Limit {
            limit: CapturedInputLimit::PackageDepth,
            observed: 25,
            maximum: 24,
        })
    ));

    let exact_entries = (0..4_096)
        .map(|index| package_document(format!("d{index}/body"), 0))
        .collect();
    CapturedPackageSource::new(exact_entries).expect("8,192 inferred descendants are admitted");
    let excessive_entries = (0..4_097)
        .map(|index| package_document(format!("d{index}/body"), 0))
        .collect();
    assert!(matches!(
        CapturedPackageSource::new(excessive_entries),
        Err(CapturedCompilationInputError::Limit {
            limit: CapturedInputLimit::PackageEntries,
            observed: 8_194,
            maximum: 8_192,
        })
    ));

    CapturedPackageDocument::new("exact.bin".into(), vec![0; 4 * 1024 * 1024])
        .expect("four MiB is admitted");
    assert!(matches!(
        CapturedPackageDocument::new("over.bin".into(), vec![0; 4 * 1024 * 1024 + 1]),
        Err(CapturedCompilationInputError::Limit {
            limit: CapturedInputLimit::PackageFileBytes,
            observed: 4_194_305,
            maximum: 4_194_304,
        })
    ));

    let exact_total = (0..16)
        .map(|index| package_document(format!("exact-{index}.bin"), 4 * 1024 * 1024))
        .collect();
    CapturedPackageSource::new(exact_total).expect("64 MiB aggregate is admitted");
    let excessive_total = (0..16)
        .map(|index| package_document(format!("over-{index}.bin"), 4 * 1024 * 1024))
        .chain(std::iter::once(package_document("one-more-byte", 1)))
        .collect();
    assert!(matches!(
        CapturedPackageSource::new(excessive_total),
        Err(CapturedCompilationInputError::Limit {
            limit: CapturedInputLimit::PackageTotalBytes,
            observed: 67_108_865,
            maximum: 67_108_864,
        })
    ));
}

#[test]
fn duplicate_package_label_precedes_aggregate_byte_accounting() {
    let documents = (0..16)
        .map(|index| package_document(format!("large-{index}.bin"), 4 * 1024 * 1024))
        .chain(std::iter::once(package_document(
            "large-0.bin",
            4 * 1024 * 1024,
        )))
        .collect();
    assert!(matches!(
        CapturedPackageSource::new(documents),
        Err(CapturedCompilationInputError::DuplicateLogicalLabel { label, .. })
            if label == "large-0.bin"
    ));

    for labels in [
        ["z", "z", "a", "a"],
        ["a", "z", "a", "z"],
        ["z", "a", "z", "a"],
    ] {
        assert!(matches!(
            CapturedPackageSource::new(
                labels
                    .into_iter()
                    .map(|label| package_document(label, 0))
                    .collect()
            ),
            Err(CapturedCompilationInputError::DuplicateLogicalLabel { label, .. })
                if label == "a"
        ));
    }
}

#[test]
fn captured_private_documents_enforce_the_existing_per_file_ceiling() {
    CapturedPrivateDocument::new("capabilities.toml".into(), vec![0; 4 * 1024 * 1024])
        .expect("four MiB private document is admitted at ingress");
    assert!(matches!(
        CapturedPrivateDocument::new("capabilities.toml".into(), vec![0; 4 * 1024 * 1024 + 1],),
        Err(CapturedCompilationInputError::Limit {
            limit: CapturedInputLimit::PrivateFileBytes,
            observed: 4_194_305,
            maximum: 4_194_304,
        })
    ));
}

#[test]
fn lifecycle_guidance_is_an_exact_unique_label_set_and_may_be_shared() {
    let workspace = SyntheticWorkspace::new(false);
    let capabilities = br#"[[run_policies]]
id = "standard"
description = "Portable compilation without an action gate."
session_inject = false
child_propagation = "propagate"
action_gate = false
gated_tool_matchers = []

[lifecycle]
[[lifecycle.behaviour]]
id = "shared-a"
description = "First shared guidance consumer."
guidance = "shared.md"
event = "pre-tool-use"
predicate_kind = "first_touch"
action = "guide"
failure_posture = "fail_open"
state_scope = "session"

[[lifecycle.behaviour]]
id = "shared-b"
description = "Second shared guidance consumer."
guidance = "shared.md"
event = "pre-tool-use"
predicate_kind = "first_touch"
action = "guide"
failure_posture = "fail_open"
state_scope = "session"
"#
    .to_vec();
    let guidance = CapturedLifecycleGuidance::new(
        "shared.md".into(),
        b"Shared lifecycle guidance.\n".to_vec(),
    )
    .expect("bounded shared guidance");
    let compilation = compile_captured_portable_inputs(
        captured_compilation_inputs(
            &workspace,
            "demo",
            "machine",
            Some(capabilities.clone()),
            vec![guidance],
        ),
        captured_workspace_universe(&workspace),
    )
    .expect("one document satisfies two references to the same unique label");
    assert_eq!(compilation.profile().lifecycle.len(), 2);

    let error = compile_captured_portable_inputs(
        captured_compilation_inputs(
            &workspace,
            "demo",
            "machine",
            Some(capabilities),
            Vec::new(),
        ),
        captured_workspace_universe(&workspace),
    )
    .expect_err("a referenced guidance label must be present");
    assert!(matches!(
        error,
        CompileError::CapturedInput(source)
            if matches!(source.as_ref(), CapturedCompilationInputError::LifecycleGuidanceMismatch {
                missing,
                unexpected,
            } if missing.len() == 1
                && missing.first().is_some_and(|label| label == "shared.md")
                && unexpected.is_empty())
    ));
}

#[test]
fn retained_package_source_compiles_into_a_source_free_content_projection() {
    let workspace = SyntheticWorkspace::new(false);
    let compilation = workspace.compile().expect("synthetic workspace compiles");
    let retained_lock: DistributionLockFile = toml::from_str(
        &std::fs::read_to_string(
            workspace
                .root
                .path()
                .join("workspace/distribution-lock.toml"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        compilation.distribution_lock_digest(),
        &DistributionLockDigest::from_lock(&retained_lock).unwrap()
    );
    compilation
        .package_semantic_ir()
        .verify()
        .expect("sealed Package graph re-verifies");
    assert_eq!(compilation.package_semantic_ir_record_count(), 1);
    assert!(compilation.executable_identity_digest().is_none());
    assert_eq!(compilation.repositories().len(), 1);
    let repository = &compilation.repositories()[0];
    assert_eq!(
        compilation.repository_manifest_digest(repository.id()),
        Some(&repository.manifest().manifest_digest)
    );
    assert!(compilation
        .repository_render_digest(repository.id())
        .is_some());
    let unselected = LogicalRepositoryIdV1::parse("not-selected").expect("repository id");
    assert!(compilation
        .repository_manifest_digest(&unselected)
        .is_none());
    assert_eq!(repository.semantic_plan().roles.len(), 1);
    assert!(repository
        .render()
        .surfaces
        .iter()
        .any(|surface| surface.content().contains("Exact retained role body.")));

    std::fs::write(
        workspace
            .root
            .path()
            .join("distribution/packages/engine/role.md"),
        "# Changed after compilation\n",
    )
    .expect("change authored source after the compilation has sealed");
    assert!(repository
        .render()
        .surfaces
        .iter()
        .any(|surface| surface.content().contains("Exact retained role body.")));
}

#[test]
fn unselected_package_changes_do_not_perturb_the_selected_compilation() {
    let workspace = SyntheticWorkspace::new(true);
    let first = workspace.compile().expect("first compilation");
    let first_digest = first.one_shot_compilation_digest().clone();
    let first_render = first.repositories()[0]
        .render()
        .surfaces
        .iter()
        .map(|surface| (&surface.rel_path, surface.content()))
        .collect::<Vec<_>>();

    std::fs::write(
        workspace
            .root
            .path()
            .join("distribution/packages/unused/body.md"),
        "# Changed but still unselected\n",
    )
    .expect("change unselected Package source");
    let second = workspace
        .compile()
        .expect("unselected release drift does not contaminate the selected closure");
    assert_eq!(&first_digest, second.one_shot_compilation_digest());
    assert_eq!(
        first_render,
        second.repositories()[0]
            .render()
            .surfaces
            .iter()
            .map(|surface| (&surface.rel_path, surface.content()))
            .collect::<Vec<_>>()
    );
}

#[test]
fn selected_package_byte_drift_fails_its_exact_lock_coordinate() {
    let workspace = SyntheticWorkspace::new(false);
    std::fs::write(
        workspace
            .root
            .path()
            .join("distribution/packages/engine/role.md"),
        "# Changed selected body\n",
    )
    .expect("change selected Package source");
    assert!(matches!(
        workspace.compile(),
        Err(CompileError::PackageLoad(error))
            if matches!(*error, pale_application::packages::PackageError::ReleaseMismatch { .. })
    ));
}

#[test]
fn sovereign_workspace_identity_is_bound_into_the_compilation_identity() {
    let workspace = SyntheticWorkspace::new(false);
    let first = workspace.compile().expect("first sovereign compilation");

    workspace.retarget_workspace_identity("compile-tests-retargeted");
    let second = workspace
        .compile()
        .expect("retargeted sovereign compilation");

    assert_ne!(first.profile(), second.profile());
    assert_ne!(first.instance(), second.instance());
    assert_eq!(
        first.machine_overlay_digest(),
        second.machine_overlay_digest()
    );
    assert_eq!(
        first.package_semantic_ir().digest(),
        second.package_semantic_ir().digest()
    );
    assert_ne!(
        first.workspace_semantic_ir_digest(),
        second.workspace_semantic_ir_digest()
    );
    assert_ne!(
        first.principal_admitted_universe_digest(),
        second.principal_admitted_universe_digest()
    );
    assert_ne!(
        first.compilation_input_digest(),
        second.compilation_input_digest()
    );
    assert_ne!(
        first.one_shot_compilation_digest(),
        second.one_shot_compilation_digest()
    );
}

#[test]
fn selected_semantics_and_machine_overlay_move_only_their_owned_coordinates() {
    let workspace = SyntheticWorkspace::new(false);
    let baseline = workspace.compile().expect("baseline compilation");

    workspace.select_responsibility_derivation();
    let responsible = workspace
        .compile()
        .expect("explicit responsibility source compiles");

    assert_eq!(baseline.profile(), responsible.profile());
    assert_eq!(baseline.instance(), responsible.instance());
    assert_eq!(
        baseline.package_semantic_ir().digest(),
        responsible.package_semantic_ir().digest()
    );
    assert_ne!(
        baseline.responsibility_derivation_closure_digest(),
        responsible.responsibility_derivation_closure_digest()
    );
    assert_eq!(
        baseline.organizational_translation_closure_digest(),
        responsible.organizational_translation_closure_digest()
    );
    assert_eq!(
        baseline.experience_composition_source_digest(),
        responsible.experience_composition_source_digest()
    );
    assert_eq!(
        baseline.experience_composition_plan().plan_digest,
        responsible.experience_composition_plan().plan_digest
    );
    assert!(responsible
        .workspace_derivation_sources()
        .iter()
        .any(|source| source.source_path.as_str() == "responsibility.toml"));
    assert!(responsible
        .workspace_derivation_nodes()
        .iter()
        .any(|node| { node.subject == WorkspaceDerivationSubjectV1::ResponsibilityClosure }));
    assert_ne!(
        baseline.compilation_input_digest(),
        responsible.compilation_input_digest()
    );

    workspace.select_organizational_translation();
    let organized = workspace
        .compile()
        .expect("explicit organizational source compiles");

    assert_eq!(responsible.profile(), organized.profile());
    assert_eq!(responsible.instance(), organized.instance());
    assert_eq!(
        responsible.responsibility_derivation_closure_digest(),
        organized.responsibility_derivation_closure_digest()
    );
    assert_ne!(
        responsible.organizational_translation_closure_digest(),
        organized.organizational_translation_closure_digest()
    );
    assert_eq!(
        responsible.experience_composition_source_digest(),
        organized.experience_composition_source_digest()
    );
    assert!(organized
        .workspace_derivation_sources()
        .iter()
        .any(|source| source.source_path.as_str() == "organization.toml"));
    assert!(organized.workspace_derivation_nodes().iter().any(|node| {
        node.subject == WorkspaceDerivationSubjectV1::OrganizationalTranslationClosure
    }));
    assert_ne!(
        responsible.compilation_input_digest(),
        organized.compilation_input_digest()
    );

    workspace.select_composition_source();
    let composed = workspace
        .compile()
        .expect("explicit composition source compiles");

    assert_eq!(organized.profile(), composed.profile());
    assert_eq!(organized.instance(), composed.instance());
    assert_eq!(
        organized.package_semantic_ir().digest(),
        composed.package_semantic_ir().digest()
    );
    assert_ne!(
        organized.experience_composition_source_digest(),
        composed.experience_composition_source_digest()
    );
    assert_ne!(
        organized.experience_composition_plan().plan_digest,
        composed.experience_composition_plan().plan_digest
    );
    assert_eq!(
        organized.responsibility_derivation_closure_digest(),
        composed.responsibility_derivation_closure_digest()
    );
    assert_eq!(
        organized.organizational_translation_closure_digest(),
        composed.organizational_translation_closure_digest()
    );
    assert_eq!(
        composed
            .experience_composition_plan()
            .declarations
            .participant_provider_manifests
            .len(),
        organized
            .experience_composition_plan()
            .declarations
            .participant_provider_manifests
            .len()
            + 1
    );
    assert!(composed
        .experience_composition_plan()
        .declarations
        .participant_provider_manifests
        .iter()
        .any(|manifest| manifest.id == "manifest.reviewable-capability"));
    assert!(composed
        .workspace_derivation_sources()
        .iter()
        .any(|source| source.source_path.as_str() == "composition.toml"));
    assert!(composed
        .workspace_derivation_nodes()
        .iter()
        .any(|node| { node.subject == WorkspaceDerivationSubjectV1::ExperienceComposition }));
    assert_ne!(
        baseline.compilation_input_digest(),
        composed.compilation_input_digest()
    );

    let composition_source_digest = composed.experience_composition_source_digest().clone();
    let composition_plan_digest = composed.experience_composition_plan().plan_digest.clone();
    let provider_closure_digest = composed.provider_binding_closure_digest().clone();
    let workspace_semantic_ir_digest = composed.workspace_semantic_ir_digest().clone();
    let profile = composed.profile().clone();

    workspace.retarget_machine_repository();
    let retargeted = workspace
        .compile()
        .expect("retargeted machine overlay compiles");

    assert_eq!(&profile, retargeted.profile());
    assert_eq!(
        &workspace_semantic_ir_digest,
        retargeted.workspace_semantic_ir_digest()
    );
    assert_eq!(
        &composition_source_digest,
        retargeted.experience_composition_source_digest()
    );
    assert_eq!(
        &composition_plan_digest,
        &retargeted.experience_composition_plan().plan_digest
    );
    assert_eq!(
        &provider_closure_digest,
        retargeted.provider_binding_closure_digest()
    );
    assert_ne!(composed.instance(), retargeted.instance());
    assert_ne!(
        composed.machine_overlay_digest(),
        retargeted.machine_overlay_digest()
    );
    assert_ne!(
        composed.compilation_input_digest(),
        retargeted.compilation_input_digest()
    );
}

#[test]
fn host_capture_preserves_exact_roles_labels_bytes_and_lexical_order() {
    let workspace = SyntheticWorkspace::new(false);
    let workspace_root = workspace.root.path().join("workspace");
    let capabilities_path = workspace_root.join("capabilities.toml");
    let mut capabilities = std::fs::read(&capabilities_path).expect("read capabilities fixture");
    capabilities.extend_from_slice(b"\n# Exact private source comment.\n");
    std::fs::write(&capabilities_path, &capabilities).expect("revise capabilities fixture");

    let sources = pale_adapter_fs::LocalSourceFilesystem;
    let entry = resolve_entry(&sources, &workspace.manifest, "demo", "default", "machine")
        .expect("resolve canonical fixture entry");
    let parts = entry
        .capture_one_shot_sources(&sources)
        .expect("capture exact source roles")
        .into_parts();

    assert_eq!(parts.project_id, "demo");
    assert_eq!(parts.execution_workspace_id, "default");
    assert_eq!(parts.machine_id, "machine");
    assert_eq!(parts.workspace_root_manifest, "pale-workspace.toml");
    let (capabilities_label, captured_capabilities) = parts.capabilities.into_parts();
    assert_eq!(capabilities_label, "capabilities.toml");
    assert_eq!(captured_capabilities, capabilities);
    let (machine_label, captured_machine) = parts.machine_binding.into_parts();
    assert_eq!(machine_label, "machines/machine.toml");
    assert_eq!(
        captured_machine,
        std::fs::read(&workspace.machine_binding).expect("read machine fixture")
    );
    let (lock_label, captured_lock) = parts.distribution_lock.into_parts();
    assert_eq!(lock_label, "distribution-lock.toml");
    assert_eq!(
        captured_lock,
        std::fs::read(workspace_root.join("distribution-lock.toml")).expect("read lock fixture")
    );
    assert!(parts.lifecycle_guidance.is_empty());

    let workspace_documents = parts
        .workspace_documents
        .into_iter()
        .map(|document| document.into_parts())
        .collect::<Vec<_>>();
    assert_eq!(
        workspace_documents
            .iter()
            .map(|(label, _)| label.as_str())
            .collect::<Vec<_>>(),
        [
            "composition.toml",
            "pale-workspace.toml",
            "workspace-resolution.lock.toml",
            "workspace-universe.lock.toml",
        ]
    );
    for (label, bytes) in workspace_documents {
        assert_eq!(
            bytes,
            std::fs::read(workspace_root.join(label)).expect("read workspace source fixture")
        );
    }

    let package_documents = parts
        .package_documents
        .into_iter()
        .map(|document| document.into_parts())
        .collect::<Vec<_>>();
    assert_eq!(
        package_documents
            .iter()
            .map(|(label, _)| label.as_str())
            .collect::<Vec<_>>(),
        ["engine/package.toml", "engine/role.md", "engine/roles.toml"]
    );
    for (label, bytes) in package_documents {
        assert_eq!(
            bytes,
            std::fs::read(
                workspace
                    .root
                    .path()
                    .join("distribution/packages")
                    .join(label)
            )
            .expect("read Package source fixture")
        );
    }
}

#[test]
fn host_capture_rejects_a_source_change_between_bounded_observations() {
    let workspace = SyntheticWorkspace::new(false);
    let local = pale_adapter_fs::LocalSourceFilesystem;
    let entry = resolve_entry(&local, &workspace.manifest, "demo", "default", "machine")
        .expect("resolve canonical fixture entry");
    let source = SecondReadChanges {
        target: workspace.manifest.clone(),
        target_reads: Cell::new(0),
        local,
    };

    let error = entry
        .capture_one_shot_sources(&source)
        .expect_err("source changed during bounded capture must fail closed");
    assert!(matches!(
        error,
        OneShotSourceCaptureError::Workspace(WorkspaceError::InputChanged(_))
    ));
}

#[test]
fn candidate_capture_preserves_roles_without_observing_package_source() {
    let workspace = SyntheticWorkspace::new(false);
    let local = pale_adapter_fs::LocalSourceFilesystem;
    let entry = resolve_entry(&local, &workspace.manifest, "demo", "default", "machine")
        .expect("resolve canonical candidate entry");
    let sources = PackageAccessSpy {
        distribution_root: workspace.root.path().join("distribution"),
        package_accesses: Cell::new(0),
        local,
    };

    let parts = entry
        .capture_configuration_improvement_candidate_sources(&sources)
        .expect("capture candidate workspace and private roles")
        .into_parts();

    assert_eq!(sources.package_accesses.get(), 0);
    assert_eq!(parts.project_id, "demo");
    assert_eq!(parts.execution_workspace_id, "default");
    assert_eq!(parts.machine_id, "machine");
    assert_eq!(parts.workspace_root_manifest, "pale-workspace.toml");
    assert_eq!(parts.workspace_documents.len(), 4);
    assert_eq!(parts.capabilities.into_parts().0, "capabilities.toml");
    assert!(parts.lifecycle_guidance.is_empty());
    assert_eq!(
        parts.machine_binding.into_parts().0,
        "machines/machine.toml"
    );
    assert_eq!(
        parts.distribution_lock.into_parts().0,
        "distribution-lock.toml"
    );
}

#[test]
fn host_capture_rejects_manifest_role_retargeting_after_resolution_before_role_reads() {
    for (old, new) in [
        ("distribution-lock.toml", "different-lock.toml"),
        (
            "machine_binding_root = \"machines\"",
            "machine_binding_root = \"different-machines\"",
        ),
    ] {
        for candidate in [false, true] {
            let workspace = SyntheticWorkspace::new(false);
            let entry = resolve_entry(
                &pale_adapter_fs::LocalSourceFilesystem,
                &workspace.manifest,
                "demo",
                "default",
                "machine",
            )
            .unwrap();
            let original = std::fs::read_to_string(&workspace.manifest).unwrap();
            assert!(original.contains(old));
            std::fs::write(&workspace.manifest, original.replace(old, new)).unwrap();
            let source = CaptureReadSpy {
                reads: RefCell::new(Vec::new()),
                guidance_sizes: BTreeMap::new(),
            };
            let error = if candidate {
                entry
                    .capture_configuration_improvement_candidate_sources(&source)
                    .map(|_| ())
                    .unwrap_err()
            } else {
                match entry.capture_one_shot_sources(&source).unwrap_err() {
                    OneShotSourceCaptureError::Workspace(error) => error,
                    other => panic!("expected manifest mismatch, got {other}"),
                }
            };
            assert!(matches!(error, WorkspaceError::InputChanged(_)));
            assert_eq!(*source.reads.borrow(), [workspace.manifest]);
        }
    }
}

#[test]
fn host_capture_enforces_private_document_count_before_guidance_reads() {
    let guidance_limit = CapturedPrivateWorkspaceInputs::MAX_DOCUMENTS - 3;
    for count in [guidance_limit, guidance_limit + 1] {
        let workspace = SyntheticWorkspace::new(false);
        let source = capture_guidance_fixture(&workspace, count);
        let entry = resolve_entry(
            &pale_adapter_fs::LocalSourceFilesystem,
            &workspace.manifest,
            "demo",
            "default",
            "machine",
        )
        .unwrap();
        let result = entry.capture_configuration_improvement_candidate_sources(&source);
        if count == guidance_limit {
            assert_eq!(result.unwrap().into_parts().lifecycle_guidance.len(), count);
        } else {
            let error = result.unwrap_err();
            assert_eq!(
                error.failure_disposition(),
                ApplicationFailureDisposition::InvalidInput
            );
            assert!(matches!(error, WorkspaceError::CapturedPrivateInput(
                CapturedCompilationInputError::Limit {
                    limit: CapturedInputLimit::PrivateDocuments, observed, maximum,
                }
            ) if observed == (count + 3) as u64 && maximum == (guidance_limit + 3) as u64));
            assert!(!source
                .reads
                .borrow()
                .iter()
                .any(|path| source.guidance_sizes.contains_key(path)));
        }
    }
}

#[test]
fn host_capture_enforces_exact_private_byte_limit_and_one_more_byte() {
    for extra in [0, 1] {
        let workspace = SyntheticWorkspace::new(false);
        let mut source = capture_guidance_fixture(&workspace, 16);
        let root = workspace.manifest.parent().unwrap();
        let fixed_bytes = [
            "capabilities.toml",
            "machines/machine.toml",
            "distribution-lock.toml",
        ]
        .into_iter()
        .map(|label| std::fs::metadata(root.join(label)).unwrap().len())
        .sum::<u64>();
        let mut remaining = CapturedPrivateWorkspaceInputs::MAX_TOTAL_BYTES - fixed_bytes + extra;
        for size in source.guidance_sizes.values_mut() {
            *size = remaining.min(CapturedLifecycleGuidance::MAX_BYTES as u64) as usize;
            remaining -= *size as u64;
        }
        assert_eq!(remaining, 0);
        let entry = resolve_entry(
            &pale_adapter_fs::LocalSourceFilesystem,
            &workspace.manifest,
            "demo",
            "default",
            "machine",
        )
        .unwrap();
        let result = entry.capture_configuration_improvement_candidate_sources(&source);
        if extra == 0 {
            assert_eq!(result.unwrap().into_parts().lifecycle_guidance.len(), 16);
        } else {
            assert!(
                matches!(result.unwrap_err(), WorkspaceError::CapturedPrivateInput(
                CapturedCompilationInputError::Limit {
                    limit: CapturedInputLimit::PrivateTotalBytes, observed, maximum,
                }
            ) if observed == maximum + 1 && maximum == CapturedPrivateWorkspaceInputs::MAX_TOTAL_BYTES)
            );
        }
    }
}

#[test]
fn host_capture_stops_reading_guidance_at_private_byte_overflow() {
    let workspace = SyntheticWorkspace::new(false);
    let mut source = capture_guidance_fixture(&workspace, 20);
    for size in source.guidance_sizes.values_mut() {
        *size = CapturedLifecycleGuidance::MAX_BYTES;
    }
    let entry = resolve_entry(
        &pale_adapter_fs::LocalSourceFilesystem,
        &workspace.manifest,
        "demo",
        "default",
        "machine",
    )
    .unwrap();
    assert!(matches!(
        entry
            .capture_configuration_improvement_candidate_sources(&source)
            .unwrap_err(),
        WorkspaceError::CapturedPrivateInput(CapturedCompilationInputError::Limit {
            limit: CapturedInputLimit::PrivateTotalBytes,
            ..
        })
    ));
    let reads = source.reads.borrow();
    let guidance_reads = reads
        .iter()
        .filter(|path| source.guidance_sizes.contains_key(*path))
        .count();
    assert_eq!(guidance_reads, 16);
    assert!(!reads.contains(&workspace.machine_binding));
    assert!(!reads.contains(
        &workspace
            .manifest
            .parent()
            .unwrap()
            .join("distribution-lock.toml")
    ));
}

#[test]
fn candidate_capture_rejects_a_relevant_source_change_between_observations() {
    let workspace = SyntheticWorkspace::new(false);
    let local = pale_adapter_fs::LocalSourceFilesystem;
    let entry = resolve_entry(&local, &workspace.manifest, "demo", "default", "machine")
        .expect("resolve canonical candidate entry");
    let sources = SecondReadChanges {
        target: workspace.manifest.clone(),
        target_reads: Cell::new(0),
        local,
    };

    let error = entry
        .capture_configuration_improvement_candidate_sources(&sources)
        .expect_err("candidate source changed during bounded capture must fail closed");
    assert!(matches!(error, WorkspaceError::InputChanged(_)));
}

#[test]
fn host_capture_derives_guidance_from_the_exact_capability_revision() {
    let workspace = SyntheticWorkspace::new(false);
    let workspace_root = workspace.root.path().join("workspace");
    let capabilities_path = workspace_root.join("capabilities.toml");
    std::fs::write(workspace_root.join("first.md"), b"First guidance.\n").unwrap();
    std::fs::write(
        workspace_root.join("successor.md"),
        b"Successor guidance.\n",
    )
    .unwrap();
    let policy = r#"[[run_policies]]
id = "standard"
description = "Portable compilation without an action gate."
session_inject = false
child_propagation = "propagate"
action_gate = false
gated_tool_matchers = []

[lifecycle]
[[lifecycle.behaviour]]
id = "revision"
description = "Revision-specific guidance."
guidance = "{guidance}"
event = "pre-tool-use"
predicate_kind = "first_touch"
action = "guide"
failure_posture = "fail_open"
state_scope = "session"
"#;
    let first = policy.replace("{guidance}", "first.md").into_bytes();
    let successor = policy.replace("{guidance}", "successor.md").into_bytes();
    let local = pale_adapter_fs::LocalSourceFilesystem;
    let entry = resolve_entry(&local, &workspace.manifest, "demo", "default", "machine")
        .expect("resolve canonical fixture entry");
    let changing = CapabilitiesChangeAfterFirstRead {
        target: capabilities_path,
        target_reads: Cell::new(0),
        first,
        successor,
        local,
    };

    let error = entry
        .capture_one_shot_sources(&changing)
        .expect_err("capability revision changed during bounded capture");
    assert!(matches!(
        error,
        OneShotSourceCaptureError::Workspace(WorkspaceError::InputChanged(_))
    ));
}

#[test]
fn host_capture_deduplicates_shared_nested_guidance_from_retained_capabilities() {
    let workspace = SyntheticWorkspace::new(false);
    let workspace_root = workspace.root.path().join("workspace");
    let capabilities = br#"[[run_policies]]
id = "standard"
description = "Portable compilation without an action gate."
session_inject = false
child_propagation = "propagate"
action_gate = false
gated_tool_matchers = []

[lifecycle]
[[lifecycle.behaviour]]
id = "shared-a"
description = "First shared guidance consumer."
guidance = "guidance/shared.md"
event = "pre-tool-use"
predicate_kind = "first_touch"
action = "guide"
failure_posture = "fail_open"
state_scope = "session"

[[lifecycle.behaviour]]
id = "shared-b"
description = "Second shared guidance consumer."
guidance = "guidance/shared.md"
event = "pre-tool-use"
predicate_kind = "first_touch"
action = "guide"
failure_posture = "fail_open"
state_scope = "session"
"#;
    std::fs::write(workspace_root.join("capabilities.toml"), capabilities).unwrap();
    std::fs::create_dir(workspace_root.join("guidance")).unwrap();
    std::fs::write(
        workspace_root.join("guidance/shared.md"),
        b"Shared lifecycle guidance.\n",
    )
    .unwrap();
    let sources = pale_adapter_fs::LocalSourceFilesystem;
    let entry = resolve_entry(&sources, &workspace.manifest, "demo", "default", "machine")
        .expect("resolve canonical fixture entry");

    let parts = entry
        .capture_one_shot_sources(&sources)
        .expect("shared nested guidance is captured once")
        .into_parts();
    let guidance = parts
        .lifecycle_guidance
        .into_iter()
        .map(|document| document.into_parts())
        .collect::<Vec<_>>();
    assert_eq!(
        guidance,
        vec![(
            "guidance/shared.md".to_string(),
            b"Shared lifecycle guidance.\n".to_vec(),
        )]
    );
}

#[path = "compile/representation.rs"]
mod representation;
