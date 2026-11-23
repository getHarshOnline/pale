//! Journey and boundary tests for trusted ownership reconciliation.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::{reconcile_with, FilesystemEffects, MutationEffects};
use crate::apply::apply_admitted;
use crate::errors::FsError;
use crate::file_transaction::commit;
use crate::lock::AdvisoryLockEffect;
use crate::materialization_state::read_materialization_state;
use crate::remove::remove_admitted;
use pale_core::admission::AdmittedPlan;
use pale_core::managed::split_managed_block;
use pale_core::records::{
    AgentSurfacePlanPreimageV1, AgentSurfacePlanV1, BaselineEntry, BaselineStateV1, InstanceDigest,
    LogicalRepositoryIdV1, ManifestDigest, MaterializationPlanPreimageV1, MaterializationPlanV1,
    MaterializationStatePreimageV1, MaterializationStateTransitionV1, MaterializationStateV1,
    OneShotCompilationDigest, Operation, PackageSemanticIrDigest, PlanKind,
    PriorMaterializationStateV1, ProfileDigest, RecordSchemaIdentity, SchemaDigest, SchemaId,
    SchemaManifestDigest, SchemaSetDigest, SurfaceDigest, SurfaceOperationObservationV1,
    SurfaceOperationTransitionV1, SurfaceOwnership, UnchangedDispositionV1, UnixFileMode,
    AGENT_SURFACE_PLAN_V1_SCHEMA, MATERIALIZATION_PLAN_V1_SCHEMA,
    MATERIALIZATION_PLAN_V1_SCHEMA_VERSION, MATERIALIZATION_STATE_PATH,
    MATERIALIZATION_STATE_SCHEMA, MATERIALIZATION_STATE_SCHEMA_VERSION,
};

/// Logical owner identity shared by reconciliation fixtures.
const REPOSITORY_ID: &str = "repo";

fn repository_id() -> LogicalRepositoryIdV1 {
    LogicalRepositoryIdV1::parse(REPOSITORY_ID).expect("valid fixture repository")
}

fn schema_digest() -> SchemaDigest {
    SchemaDigest::parse("1".repeat(64)).unwrap()
}

fn schema_manifest_digest() -> SchemaManifestDigest {
    SchemaManifestDigest::parse("2".repeat(64)).unwrap()
}

fn schema_set_digest() -> SchemaSetDigest {
    SchemaSetDigest::parse("3".repeat(64)).unwrap()
}

fn one_shot_compilation_digest() -> OneShotCompilationDigest {
    OneShotCompilationDigest::parse("4".repeat(64)).unwrap()
}

fn lineage_schema_identity() -> RecordSchemaIdentity {
    RecordSchemaIdentity {
        schema: SchemaId::parse("pale.agent-surface.materialization-lineage.v1").unwrap(),
        schema_sha256: schema_digest(),
    }
}

fn contract_identity() -> pale_core::records::MaterializationContractIdentity {
    pale_core::records::MaterializationContractIdentity::new(
        RecordSchemaIdentity {
            schema: SchemaId::parse(MATERIALIZATION_STATE_SCHEMA).unwrap(),
            schema_sha256: schema_digest(),
        },
        lineage_schema_identity(),
        pale_core::records::SchemaRegistryIdentity {
            schema_manifest_sha256: schema_manifest_digest(),
            schema_set_sha256: schema_set_digest(),
        },
        one_shot_compilation_digest(),
    )
}

fn apply(
    effects: &dyn MutationEffects,
    root: &Path,
    lineage_root: &Path,
    plan: &AgentSurfacePlanV1,
    contents: &BTreeMap<String, String>,
) -> Result<crate::file_transaction::AppliedFileTransaction, FsError> {
    let admitted = admitted(plan);
    reconcile_with(
        effects,
        root,
        lineage_root,
        admitted.plan(),
        admitted.digest(),
        contents,
        &contract_identity(),
    )
}

fn payload(path: impl Into<String>, content: impl Into<String>) -> BTreeMap<String, String> {
    BTreeMap::from([(path.into(), content.into())])
}

fn remove(
    effects: &dyn MutationEffects,
    root: &Path,
    lineage_root: &Path,
    plan: &AgentSurfacePlanV1,
) -> Result<crate::file_transaction::AppliedFileTransaction, FsError> {
    let admitted = admitted(plan);
    reconcile_with(
        effects,
        root,
        lineage_root,
        admitted.plan(),
        admitted.digest(),
        &BTreeMap::<String, String>::new(),
        &contract_identity(),
    )
}

/// Recover the semantic failure beneath a confirmed persistent lock-file effect.
///
/// Every reconciliation journey begins by acquiring the repository-scoped
/// advisory lock. A fresh fixture therefore creates confirmed coordination
/// state even when the requested materialization effect is rejected or fully
/// recovered.
fn confirmed_coordination_source(error: FsError) -> FsError {
    match error {
        FsError::Coordination {
            effect: AdvisoryLockEffect::Confirmed,
            source,
        } => *source,
        other => panic!("expected confirmed coordination effect, got {other:?}"),
    }
}

fn fixture_profile_digest() -> ProfileDigest {
    ProfileDigest::hash_canonical(&("adapter-test-profile", REPOSITORY_ID)).unwrap()
}

fn fixture_instance_digest() -> InstanceDigest {
    InstanceDigest::hash_canonical(&("adapter-test-instance", REPOSITORY_ID)).unwrap()
}

fn fixture_manifest_digest(variant: char) -> ManifestDigest {
    ManifestDigest::hash_canonical(&("adapter-test-manifest", REPOSITORY_ID, variant)).unwrap()
}

fn roots(tmp: &tempfile::TempDir) -> (PathBuf, PathBuf) {
    let target = tmp.path().join("target");
    let lineage = tmp.path().join("machine-state");
    fs::create_dir(&target).unwrap();
    fs::create_dir(&lineage).unwrap();
    (target, lineage)
}

#[test]
fn malformed_materialization_records_preserve_their_typed_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let (target, lineage) = roots(&tmp);
    let state_path = target.join(MATERIALIZATION_STATE_PATH);
    fs::create_dir_all(state_path.parent().unwrap()).unwrap();

    fs::write(&state_path, b"{").unwrap();
    let error =
        read_materialization_state(&target, &lineage, &repository_id(), &contract_identity())
            .unwrap_err();
    assert!(matches!(
        &error,
        FsError::Data { source, .. }
            if source.downcast_ref::<pale_core::canon::CanonError>().is_some()
    ));

    fs::write(&state_path, b"{}").unwrap();
    let error =
        read_materialization_state(&target, &lineage, &repository_id(), &contract_identity())
            .unwrap_err();
    assert!(matches!(
        &error,
        FsError::Data { source, .. }
            if source.downcast_ref::<serde_json::Error>().is_some()
    ));
}

#[test]
fn malformed_lineage_preserves_its_typed_parse_source() {
    let tmp = tempfile::tempdir().unwrap();
    let (target, lineage) = roots(&tmp);
    let state = state(Vec::new(), 'a');
    let state_path = target.join(MATERIALIZATION_STATE_PATH);
    fs::create_dir_all(state_path.parent().unwrap()).unwrap();
    fs::write(
        state_path,
        pale_core::canon::to_canonical_string(&state).unwrap(),
    )
    .unwrap();
    let target_canon = fs::canonicalize(&target).unwrap();
    let lineage_path = lineage.join(crate::materialization_state::lineage_relative(
        &target_canon,
    ));
    fs::create_dir_all(lineage_path.parent().unwrap()).unwrap();
    fs::write(lineage_path, b"{").unwrap();

    let error =
        read_materialization_state(&target, &lineage, &repository_id(), &contract_identity())
            .unwrap_err();
    assert!(matches!(
        &error,
        FsError::Data { source, .. }
            if source.downcast_ref::<pale_core::canon::CanonError>().is_some()
    ));
}

fn surface(
    rel_path: &str,
    kind: &str,
    ownership: SurfaceOwnership,
    rendered: &str,
) -> SurfaceDigest {
    let owned = match ownership {
        SurfaceOwnership::WholeFile => rendered,
        SurfaceOwnership::ManagedBlock => split_managed_block(rendered).unwrap().block(),
    };
    SurfaceDigest {
        rel_path: rel_path.into(),
        kind: kind.into(),
        ownership,
        execution_path_consumers: vec![],
        committable: ownership == SurfaceOwnership::WholeFile,
        sha256: pale_core::records::ArtifactDigest::hash_bytes(owned.as_bytes()),
    }
}

fn state(mut surfaces: Vec<SurfaceDigest>, manifest: char) -> MaterializationStateV1 {
    surfaces.sort_by(|left, right| left.rel_path.cmp(&right.rel_path));
    MaterializationStateV1::seal(MaterializationStatePreimageV1 {
        schema: SchemaId::parse(MATERIALIZATION_STATE_SCHEMA).unwrap(),
        schema_sha256: schema_digest(),
        schema_version: MATERIALIZATION_STATE_SCHEMA_VERSION,
        schema_manifest_sha256: schema_manifest_digest(),
        schema_set_sha256: schema_set_digest(),
        one_shot_compilation_digest: one_shot_compilation_digest(),
        repository_id: repository_id(),
        profile_digest: fixture_profile_digest(),
        instance_digest: fixture_instance_digest(),
        manifest_digest: fixture_manifest_digest(manifest),
        surfaces,
    })
    .unwrap()
}

fn transition_plan(
    kind: PlanKind,
    prior: Option<&MaterializationStateV1>,
    next: Option<MaterializationStateV1>,
) -> AgentSurfacePlanV1 {
    let prior_index = prior
        .map(|state| {
            state
                .surfaces
                .iter()
                .map(|surface| (surface.rel_path.as_str(), surface))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let next_index = next
        .as_ref()
        .map(|state| {
            state
                .surfaces
                .iter()
                .map(|surface| (surface.rel_path.as_str(), surface))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let paths = prior_index
        .keys()
        .chain(next_index.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    let operations = paths
        .into_iter()
        .map(|rel_path| {
            let previous = prior_index.get(rel_path).copied();
            let desired = next_index.get(rel_path).copied();
            let identity = desired.or(previous).unwrap();
            let transition = match (previous, desired) {
                (None, Some(new)) => SurfaceOperationTransitionV1::Create {
                    ownership: new.ownership,
                    desired_sha256: new.sha256.clone(),
                },
                (Some(old), Some(new)) if old.sha256 == new.sha256 => {
                    SurfaceOperationTransitionV1::Unchanged {
                        ownership: new.ownership,
                        owned_sha256: new.sha256.clone(),
                    }
                }
                (Some(_), Some(new)) => SurfaceOperationTransitionV1::Update {
                    ownership: new.ownership,
                    desired_sha256: new.sha256.clone(),
                },
                (Some(old), None) => match old.ownership {
                    SurfaceOwnership::ManagedBlock => SurfaceOperationTransitionV1::Splice {
                        prior_owned_sha256: old.sha256.clone(),
                    },
                    SurfaceOwnership::WholeFile => SurfaceOperationTransitionV1::Delete {
                        prior_owned_sha256: old.sha256.clone(),
                    },
                },
                (None, None) => unreachable!(),
            };
            Operation {
                rel_path: rel_path.into(),
                kind: identity.kind.clone(),
                transition,
                mode: UnixFileMode::DEFAULT_FILE,
                committable: identity.committable,
            }
        })
        .collect();
    let identity = next.as_ref().or(prior);
    let profile_digest = identity
        .map(|state| state.profile_digest.clone())
        .unwrap_or_else(fixture_profile_digest);
    let instance_digest = identity
        .map(|state| state.instance_digest.clone())
        .unwrap_or_else(fixture_instance_digest);
    let manifest_digest = identity
        .map(|state| state.manifest_digest.clone())
        .unwrap_or_else(|| fixture_manifest_digest('3'));
    let prior_identity = || {
        prior.map(|state| PriorMaterializationStateV1 {
            state_digest: state.state_digest.clone(),
            one_shot_compilation_digest: state.one_shot_compilation_digest.clone(),
        })
    };
    let transition = match (kind, prior_identity(), next) {
        (PlanKind::Apply, None, Some(next_state)) => {
            MaterializationStateTransitionV1::Initialize { next_state }
        }
        (PlanKind::Apply, Some(prior), Some(next_state)) => {
            MaterializationStateTransitionV1::Advance { prior, next_state }
        }
        (PlanKind::Remove, Some(prior), None) => MaterializationStateTransitionV1::Remove { prior },
        (PlanKind::Remove, None, None) => MaterializationStateTransitionV1::RemoveUnowned {},
        _ => panic!("test fixture requested an impossible materialization transition"),
    };
    AgentSurfacePlanV1::seal(AgentSurfacePlanPreimageV1 {
        schema: SchemaId::parse(AGENT_SURFACE_PLAN_V1_SCHEMA).unwrap(),
        schema_sha256: schema_digest(),
        schema_manifest_sha256: schema_manifest_digest(),
        schema_set_sha256: schema_set_digest(),
        one_shot_compilation_digest: one_shot_compilation_digest(),
        transition,
        operation_marker: "local_explicit_operator".into(),
        admission: "unavailable".into(),
        compiler_version: "test".into(),
        renderer_version: "test".into(),
        package_closure: Vec::new(),
        normative_pack_closure: Vec::new(),
        profile_digest,
        instance_digest,
        manifest_digest,
        package_semantic_ir_digest: PackageSemanticIrDigest::hash_canonical(
            &"reconciliation-test-package-semantic-ir",
        )
        .unwrap(),
        repository_id: repository_id(),
        target_baseline: vec![],
        operations,
        deferred_effects: vec![],
    })
    .unwrap()
}

fn admitted(plan: &AgentSurfacePlanV1) -> AdmittedPlan {
    let envelope = MaterializationPlanV1::seal(MaterializationPlanPreimageV1 {
        schema: SchemaId::parse(MATERIALIZATION_PLAN_V1_SCHEMA).unwrap(),
        schema_sha256: schema_digest(),
        schema_version: MATERIALIZATION_PLAN_V1_SCHEMA_VERSION,
        schema_manifest_sha256: plan.schema_manifest_sha256.clone(),
        schema_set_sha256: plan.schema_set_sha256.clone(),
        one_shot_compilation_digest: plan.one_shot_compilation_digest.clone(),
        surface_plan: plan.clone(),
        machine_local_effects: Vec::new(),
    })
    .unwrap();
    pale_core::admission::admit_plan(&envelope, &envelope, plan.transition.plan_kind()).unwrap()
}

fn adopt(operation: &mut Operation) {
    operation.transition = SurfaceOperationTransitionV1::Adopt {
        ownership: operation.transition.ownership(),
        desired_sha256: operation.transition.content_sha256().clone(),
    };
}

fn present_baseline(rel_path: &str, bytes: &[u8], mode: u32) -> BaselineEntry {
    BaselineEntry {
        rel_path: rel_path.into(),
        observation: BaselineStateV1::Present {
            sha256: pale_core::records::ArtifactDigest::hash_bytes(bytes),
            mode: UnixFileMode::new(mode).unwrap(),
        },
    }
}

#[test]
fn admitted_plan_kind_gate_rejects_cross_class_use() {
    let tmp = tempfile::tempdir().unwrap();
    let (target, lineage) = roots(&tmp);
    let remove_plan = transition_plan(PlanKind::Remove, None, None);
    let remove_token = admitted(&remove_plan);
    assert!(matches!(
        apply_admitted(
            &target,
            &lineage,
            &remove_token,
            &BTreeMap::<String, String>::new(),
            &contract_identity(),
        ),
        Err(FsError::WrongPlanKind {
            actual: PlanKind::Remove,
            expected: PlanKind::Apply,
        })
    ));

    let apply_plan = transition_plan(PlanKind::Apply, None, Some(state(vec![], '3')));
    let apply_token = admitted(&apply_plan);
    assert!(matches!(
        remove_admitted(&target, &lineage, &apply_token, &contract_identity(),),
        Err(FsError::WrongPlanKind {
            actual: PlanKind::Apply,
            expected: PlanKind::Remove,
        })
    ));
}

#[test]
fn invalid_surface_mode_is_rejected_at_the_wire_boundary_before_any_effect() {
    let tmp = tempfile::tempdir().unwrap();
    let (target, _lineage) = roots(&tmp);
    let desired = state(
        vec![
            surface(
                ".claude/a.txt",
                "artifact",
                SurfaceOwnership::WholeFile,
                "a\n",
            ),
            surface(
                ".claude/b.txt",
                "artifact",
                SurfaceOwnership::WholeFile,
                "b\n",
            ),
        ],
        '3',
    );
    let plan = transition_plan(PlanKind::Apply, None, Some(desired));
    let mut wire = serde_json::to_value(plan).unwrap();
    wire["operations"][1]["mode"] = serde_json::json!(0o10_000);

    assert!(serde_json::from_value::<AgentSurfacePlanV1>(wire).is_err());
    assert!(!target.join(".claude/a.txt").exists());
    assert!(!target.join(".claude/b.txt").exists());
    assert!(!target.join(MATERIALIZATION_STATE_PATH).exists());
}

#[test]
fn forged_repo_local_state_cannot_adopt_or_delete_a_user_file() {
    let tmp = tempfile::tempdir().unwrap();
    let (target, lineage) = roots(&tmp);
    let user_path = target.join("notes.txt");
    fs::write(&user_path, "private notes\n").unwrap();
    let forged = state(
        vec![surface(
            "notes.txt",
            "operation-skill",
            SurfaceOwnership::WholeFile,
            "private notes\n",
        )],
        '3',
    );
    fs::create_dir(target.join(".pale")).unwrap();
    fs::write(
        target.join(MATERIALIZATION_STATE_PATH),
        pale_core::canon::to_canonical_string(&forged).unwrap(),
    )
    .unwrap();

    let remove_plan = transition_plan(PlanKind::Remove, Some(&forged), None);
    let error = remove(&FilesystemEffects, &target, &lineage, &remove_plan)
        .expect_err("forged state must not authorize removal");
    assert!(matches!(
        confirmed_coordination_source(error),
        FsError::Collision(_, _)
    ));
    assert_eq!(fs::read_to_string(&user_path).unwrap(), "private notes\n");
}

#[test]
fn adoption_rechecks_exact_baseline_before_the_first_target_effect() {
    use std::os::unix::fs::PermissionsExt;

    for drift in ["content", "mode"] {
        let tmp = tempfile::tempdir().unwrap();
        let (target, lineage) = roots(&tmp);
        let rel_path = "existing.txt";
        let original = "pre-existing user bytes\n";
        let rendered = "generated Pale bytes\n";
        let path = target.join(rel_path);
        fs::write(&path, original).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();

        let desired = state(
            vec![surface(
                rel_path,
                "activation",
                SurfaceOwnership::WholeFile,
                rendered,
            )],
            '3',
        );
        let mut plan = transition_plan(PlanKind::Apply, None, Some(desired));
        adopt(&mut plan.operations[0]);
        plan.target_baseline = vec![present_baseline(rel_path, original.as_bytes(), 0o640)];
        plan = AgentSurfacePlanV1::seal(plan.to_preimage()).unwrap();

        match drift {
            "content" => fs::write(&path, "changed after planning\n").unwrap(),
            "mode" => {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
            }
            _ => unreachable!(),
        }
        let before_bytes = fs::read(&path).unwrap();
        let before_mode = fs::metadata(&path).unwrap().permissions().mode() & 0o7777;

        let error = apply(
            &FilesystemEffects,
            &target,
            &lineage,
            &plan,
            &payload(rel_path, rendered),
        )
        .expect_err("a changed adoption baseline must fail closed");
        assert!(matches!(
            confirmed_coordination_source(error),
            FsError::Collision(ref path, ref reason)
                if path == rel_path && reason.contains("admitted baseline")
        ));
        assert_eq!(fs::read(&path).unwrap(), before_bytes);
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o7777,
            before_mode
        );
        assert!(!target.join(MATERIALIZATION_STATE_PATH).exists());
        assert!(fs::read_dir(&lineage).unwrap().next().is_none());
    }
}

#[test]
fn managed_block_adoption_replaces_only_the_exact_preexisting_block() {
    let tmp = tempfile::tempdir().unwrap();
    let (target, lineage) = roots(&tmp);
    let rel_path = "AGENTS.md";
    let old_block = format!(
        "{}\nlegacy projection\n{}",
        pale_core::MANAGED_BEGIN,
        pale_core::MANAGED_END
    );
    let desired_block = format!(
        "{}\nnormalized projection\n{}",
        pale_core::MANAGED_BEGIN,
        pale_core::MANAGED_END
    );
    let original = format!("user prefix\n{old_block}\nuser suffix\n");
    let expected = format!("user prefix\n{desired_block}\nuser suffix\n");
    fs::write(target.join(rel_path), &original).unwrap();

    let desired = state(
        vec![surface(
            rel_path,
            "activation",
            SurfaceOwnership::ManagedBlock,
            &desired_block,
        )],
        '3',
    );
    let mut plan = transition_plan(PlanKind::Apply, None, Some(desired));
    adopt(&mut plan.operations[0]);
    plan.target_baseline = vec![present_baseline(rel_path, original.as_bytes(), 0o644)];
    plan = AgentSurfacePlanV1::seal(plan.to_preimage()).unwrap();

    let applied = apply(
        &FilesystemEffects,
        &target,
        &lineage,
        &plan,
        &payload(rel_path, desired_block),
    )
    .unwrap();
    assert!(applied.observation().operations.iter().any(|operation| {
        operation.rel_path == rel_path
            && matches!(
                operation.observation,
                SurfaceOperationObservationV1::Adopt { .. }
            )
    }));
    commit(applied);

    assert_eq!(fs::read_to_string(target.join(rel_path)).unwrap(), expected);
    assert!(
        read_materialization_state(&target, &lineage, &repository_id(), &contract_identity())
            .unwrap()
            .is_some()
    );
}

#[test]
fn equal_managed_block_adoption_completes_without_a_redundant_surface_write() {
    let tmp = tempfile::tempdir().unwrap();
    let (target, lineage) = roots(&tmp);
    let rel_path = "AGENTS.md";
    let desired_block = format!(
        "{}\nnormalized projection\n{}",
        pale_core::MANAGED_BEGIN,
        pale_core::MANAGED_END
    );
    let original = format!("user prefix\n{desired_block}\nuser suffix\n");
    fs::write(target.join(rel_path), &original).unwrap();

    let desired = state(
        vec![surface(
            rel_path,
            "activation",
            SurfaceOwnership::ManagedBlock,
            &desired_block,
        )],
        '3',
    );
    let mut plan = transition_plan(PlanKind::Apply, None, Some(desired));
    adopt(&mut plan.operations[0]);
    plan.target_baseline = vec![present_baseline(rel_path, original.as_bytes(), 0o644)];
    plan = AgentSurfacePlanV1::seal(plan.to_preimage()).unwrap();

    let applied = apply(
        &FilesystemEffects,
        &target,
        &lineage,
        &plan,
        &payload(rel_path, desired_block),
    )
    .unwrap();
    let observation = applied
        .observation()
        .operations
        .iter()
        .find(|operation| operation.rel_path == rel_path)
        .unwrap();
    assert!(matches!(
        &observation.observation,
        SurfaceOperationObservationV1::Adopt {
            pre_sha256,
            post_sha256,
        } if pre_sha256 == post_sha256
    ));
    commit(applied);

    assert_eq!(fs::read_to_string(target.join(rel_path)).unwrap(), original);
    assert!(
        read_materialization_state(&target, &lineage, &repository_id(), &contract_identity())
            .unwrap()
            .is_some()
    );
}

#[test]
fn managed_block_adoption_rejects_drift_and_missing_markers_without_effects() {
    for defect in ["drift", "missing-markers"] {
        let tmp = tempfile::tempdir().unwrap();
        let (target, lineage) = roots(&tmp);
        let rel_path = "CLAUDE.md";
        let old_block = format!(
            "{}\nlegacy projection\n{}",
            pale_core::MANAGED_BEGIN,
            pale_core::MANAGED_END
        );
        let desired_block = format!(
            "{}\nnormalized projection\n{}",
            pale_core::MANAGED_BEGIN,
            pale_core::MANAGED_END
        );
        let original = match defect {
            "drift" => format!("user prefix\n{old_block}\nuser suffix\n"),
            "missing-markers" => "user content only\n".into(),
            _ => unreachable!(),
        };
        fs::write(target.join(rel_path), &original).unwrap();
        let desired = state(
            vec![surface(
                rel_path,
                "activation",
                SurfaceOwnership::ManagedBlock,
                &desired_block,
            )],
            '3',
        );
        let mut plan = transition_plan(PlanKind::Apply, None, Some(desired));
        adopt(&mut plan.operations[0]);
        plan.target_baseline = vec![present_baseline(rel_path, original.as_bytes(), 0o644)];
        plan = AgentSurfacePlanV1::seal(plan.to_preimage()).unwrap();

        if defect == "drift" {
            fs::write(
                target.join(rel_path),
                format!("user prefix\n{old_block}\ntampered suffix\n"),
            )
            .unwrap();
        }
        let before = fs::read(target.join(rel_path)).unwrap();
        let error = apply(
            &FilesystemEffects,
            &target,
            &lineage,
            &plan,
            &payload(rel_path, desired_block),
        )
        .expect_err("managed-block adoption drift must fail closed");
        assert!(matches!(
            confirmed_coordination_source(error),
            FsError::Collision(_, _)
        ));
        assert_eq!(fs::read(target.join(rel_path)).unwrap(), before);
        assert!(!target.join(MATERIALIZATION_STATE_PATH).exists());
        assert!(fs::read_dir(&lineage).unwrap().next().is_none());
    }
}

#[test]
fn user_modified_prior_owned_file_collides_without_marker_heuristics() {
    let tmp = tempfile::tempdir().unwrap();
    let (target, lineage) = roots(&tmp);
    let path = ".claude/skills/pale-review/SKILL.md";
    let original = "---\nname: pale-review\n---\nReview independently.\n";
    let desired = state(
        vec![surface(
            path,
            "operation-skill",
            SurfaceOwnership::WholeFile,
            original,
        )],
        '3',
    );
    let apply_plan = transition_plan(PlanKind::Apply, None, Some(desired));
    commit(
        apply(
            &FilesystemEffects,
            &target,
            &lineage,
            &apply_plan,
            &payload(path, original),
        )
        .unwrap(),
    );
    let installed =
        read_materialization_state(&target, &lineage, &repository_id(), &contract_identity())
            .unwrap()
            .unwrap();
    fs::write(target.join(path), "user replacement\n").unwrap();

    let remove_plan = transition_plan(PlanKind::Remove, Some(&installed), None);
    assert!(matches!(
        remove(&FilesystemEffects, &target, &lineage, &remove_plan),
        Err(FsError::Collision(_, _))
    ));
    assert_eq!(
        fs::read_to_string(target.join(path)).unwrap(),
        "user replacement\n"
    );
    assert!(
        read_materialization_state(&target, &lineage, &repository_id(), &contract_identity())
            .unwrap()
            .is_some()
    );
}

#[cfg(unix)]
#[test]
fn mode_only_drift_is_repaired_for_managed_surfaces_state_and_lineage() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let tmp = tempfile::tempdir().unwrap();
    let (target, lineage) = roots(&tmp);
    let path = ".claude/settings.json";
    let managed = format!(
        "{}\nowned\n{}",
        pale_core::MANAGED_BEGIN,
        pale_core::MANAGED_END
    );
    fs::create_dir_all(target.join(".claude")).unwrap();
    fs::write(target.join(path), "user\n").unwrap();
    let desired = state(
        vec![surface(
            path,
            "activation",
            SurfaceOwnership::ManagedBlock,
            &managed,
        )],
        '3',
    );
    let initial_plan = transition_plan(PlanKind::Apply, None, Some(desired));
    commit(
        apply(
            &FilesystemEffects,
            &target,
            &lineage,
            &initial_plan,
            &payload(path, managed.clone()),
        )
        .unwrap(),
    );
    let installed =
        read_materialization_state(&target, &lineage, &repository_id(), &contract_identity())
            .unwrap()
            .unwrap();
    let steady_plan = transition_plan(PlanKind::Apply, Some(&installed), Some(installed.clone()));
    commit(
        apply(
            &FilesystemEffects,
            &target,
            &lineage,
            &steady_plan,
            &payload(path, managed.clone()),
        )
        .unwrap(),
    );

    let state_path = target.join(MATERIALIZATION_STATE_PATH);
    let lineage_path = lineage.join(crate::materialization_state::lineage_relative(
        &fs::canonicalize(&target).unwrap(),
    ));
    for drifted in [target.join(path), state_path.clone(), lineage_path.clone()] {
        fs::set_permissions(&drifted, fs::Permissions::from_mode(0o640)).unwrap();
    }

    let repaired = apply(
        &FilesystemEffects,
        &target,
        &lineage,
        &steady_plan,
        &payload(path, managed.clone()),
    )
    .unwrap();
    assert!(repaired.observation().operations.iter().any(|operation| {
        operation.rel_path == path
            && matches!(
                operation.observation,
                SurfaceOperationObservationV1::Unchanged {
                    disposition: UnchangedDispositionV1::ModeRepaired,
                    ..
                }
            )
    }));
    commit(repaired);
    assert_eq!(
        fs::metadata(target.join(path)).unwrap().mode() & 0o7777,
        0o644
    );
    assert_eq!(fs::metadata(&state_path).unwrap().mode() & 0o7777, 0o600);
    assert_eq!(fs::metadata(&lineage_path).unwrap().mode() & 0o7777, 0o600);

    fs::set_permissions(target.join(path), fs::Permissions::from_mode(0o640)).unwrap();
    let remove_plan = transition_plan(PlanKind::Remove, Some(&installed), None);
    commit(remove(&FilesystemEffects, &target, &lineage, &remove_plan).unwrap());
    assert_eq!(fs::read_to_string(target.join(path)).unwrap(), "user\n");
    assert_eq!(
        fs::metadata(target.join(path)).unwrap().mode() & 0o7777,
        0o644
    );
}

#[path = "reconcile/confinement.rs"]
mod confinement;
#[path = "reconcile/recovery.rs"]
mod recovery;
#[path = "reconcile/representation.rs"]
mod representation;
