//! Benchmark-only characterization of the complete local façade pipeline.
//!
//! All effects remain below disposable benchmark roots. The harness consumes
//! only public [`pale_sdk`] use cases and does not publish compiler records,
//! host paths, or adapter capabilities.

use std::fmt::Write as _;
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use pale_sdk::{
    LocalMaterializationAdmissionRequest, LocalMaterializationAdmittedRun,
    LocalMaterializationRunOutcome, LocalMaterializationTargetOutcome, LocalOneShotCompilation,
    LocalOneShotCompilationRequest, LocalOneShotPlan, LocalOneShotPlanRequest,
    LocalWorkspaceLocator, OneShotPlanKind, PaleApi,
};
use sha2::{Digest as _, Sha256};

/// Full-run independent observations, matching the repository benchmark floor.
const FULL_SAMPLES: usize = 201;
/// Compile-and-execute observations that carry no characterization authority.
const SMOKE_SAMPLES: usize = 5;
/// Maximum UTF-8 bytes in one caller-supplied report coordinate.
const MAX_COORDINATE_BYTES: usize = 512;
/// The harness executes every measured operation on its calling thread.
const BENCHMARK_THREADS: usize = 1;
/// Stable domain for one ordered digest set.
const SET_DIGEST_DOMAIN: &[u8] = b"pale.local-materialization-benchmark-set.v1\0";
/// Benchmark-owned Git exclusion observed by local planning.
const USER_EXCLUDE_BYTES: &[u8] = b"# benchmark-owned user exclusion\nlocal.cache\n";

/// Bounded, deliberately non-Cartesian repository scales.
const CASES: [BenchmarkCase; 3] = [
    BenchmarkCase::new("one-repository", 1),
    BenchmarkCase::new("ten-repositories", 10),
    BenchmarkCase::new("hundred-repositories", 100),
];

/// Real façade stages measured for every case.
const OPERATIONS: [Operation; 6] = [
    Operation::Compile,
    Operation::Plan,
    Operation::Publish,
    Operation::Admit,
    Operation::Settle,
    Operation::FreshPipeline,
];

macro_rules! fixture_bytes {
    ($member:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/cross-leaf.v1/one-shot/",
            $member
        ))
    };
}

/// One paired workload scale.
#[derive(Clone, Copy)]
struct BenchmarkCase {
    /// Stable report identity.
    id: &'static str,
    /// Number of repositories in the canonical graph and native run.
    repositories: usize,
}

impl BenchmarkCase {
    /// Construct one compile-time checked case.
    const fn new(id: &'static str, repositories: usize) -> Self {
        Self { id, repositories }
    }
}

/// Configuration captured once before fixtures or observations are created.
struct Configuration {
    /// Whether this is a compile-and-execute smoke run.
    smoke: bool,
    /// Number of independent timing observations per row.
    samples: usize,
    /// Exact caller-assigned source coordinate.
    source_coordinate: String,
    /// Exact caller-assigned run coordinate.
    run_coordinate: String,
    /// Stable caller description of the measured hardware.
    hardware: String,
    /// Exact caller-supplied operating-system kernel coordinate.
    kernel: String,
    /// Compiler version resolved once for the run.
    toolchain: String,
    /// Compiler host target resolved with [`Self::toolchain`].
    target: String,
    /// Exact profile-guided-optimization coordinate.
    pgo: String,
    /// Exact caller-supplied CPU-target setting.
    cpu_target: String,
    /// Exact executable identity bound into each synthetic machine.
    surface_executable: SurfaceExecutableCoordinate,
}

impl Configuration {
    /// Resolve one explicit run mode, or skip ordinary Cargo target discovery.
    fn from_environment(api: &PaleApi) -> Option<Self> {
        let smoke = match std::env::var_os("PALE_LOCAL_MATERIALIZATION_BENCH_SMOKE") {
            None => false,
            Some(value) if value == "1" => true,
            Some(_) => panic!("PALE_LOCAL_MATERIALIZATION_BENCH_SMOKE must be 1 when present"),
        };
        let characterize = match std::env::var_os("PALE_BENCH_RUN") {
            None => false,
            Some(value) if value == "1" => true,
            Some(_) => panic!("PALE_BENCH_RUN must be 1 when present"),
        };
        assert!(
            !(smoke && characterize),
            "select either smoke validation or full characterization"
        );
        if !smoke && !characterize {
            return None;
        }
        let (toolchain, target) = toolchain_coordinate();
        Some(Self {
            smoke,
            samples: if smoke { SMOKE_SAMPLES } else { FULL_SAMPLES },
            source_coordinate: required_coordinate(
                "PALE_LOCAL_MATERIALIZATION_BENCH_SOURCE_COORDINATE",
                smoke,
                "smoke-unsealed-worktree",
            ),
            run_coordinate: required_coordinate(
                "PALE_LOCAL_MATERIALIZATION_BENCH_RUN_COORDINATE",
                smoke,
                "smoke-validation",
            ),
            hardware: required_coordinate(
                "PALE_LOCAL_MATERIALIZATION_BENCH_HARDWARE",
                smoke,
                "unspecified-smoke-host",
            ),
            kernel: required_coordinate(
                "PALE_LOCAL_MATERIALIZATION_BENCH_KERNEL",
                smoke,
                "unspecified-smoke-kernel",
            ),
            toolchain,
            target,
            pgo: required_coordinate("PALE_LOCAL_MATERIALIZATION_BENCH_PGO", smoke, "none"),
            cpu_target: required_coordinate(
                "PALE_LOCAL_MATERIALIZATION_BENCH_CPU_TARGET",
                smoke,
                "unspecified-smoke-cpu-target",
            ),
            surface_executable: SurfaceExecutableCoordinate::capture(api),
        })
    }

    /// Evidence class encoded in every row.
    const fn evidence_class(&self) -> &'static str {
        if self.smoke {
            "smoke_validation"
        } else {
            "local_characterization"
        }
    }
}

/// Exact executable coordinate bound into synthetic private configuration.
struct SurfaceExecutableCoordinate {
    /// Canonical path of the executing benchmark.
    executable: String,
    /// Digest of the exact executable bytes.
    digest: String,
    /// Version projected by the public compiled interface identity.
    version: String,
    /// Protocol projected by the public compiled interface identity.
    protocol: String,
}

impl SurfaceExecutableCoordinate {
    /// Capture the current executable once, outside every timing interval.
    fn capture(api: &PaleApi) -> Self {
        let executable = std::env::current_exe()
            .expect("benchmark executable is available")
            .canonicalize()
            .expect("benchmark executable path is canonical");
        let digest =
            sha256(&fs::read(&executable).expect("benchmark executable bytes are readable"));
        let identity = api
            .compiled_interface_identity()
            .expect("public compiled interface identity is coherent");
        Self {
            executable: executable.display().to_string(),
            digest,
            version: identity.version().to_owned(),
            protocol: identity.protocol().to_owned(),
        }
    }
}

/// Disposable source and native-effect roots for one case observation.
struct Fixture {
    /// Retains every root until the observation is complete.
    _temporary: tempfile::TempDir,
    /// Canonical root used only by benchmark preparation and read-back.
    root: PathBuf,
    /// Absolute workspace-manifest spelling accepted by the façade.
    manifest: String,
    /// Repository identities in canonical materialization order.
    repositories: Vec<String>,
    /// Digest of exact benchmark source/configuration bytes.
    source_digest: String,
}

impl Fixture {
    /// Materialize one deterministic source-backed case below a fresh root.
    fn new(case: BenchmarkCase, surface_executable: &SurfaceExecutableCoordinate) -> Self {
        let temporary = tempfile::tempdir().expect("temporary benchmark root is created");
        let root = temporary
            .path()
            .canonicalize()
            .expect("temporary benchmark root is canonical");
        let distribution = root.join("distribution");
        let authority = root.join("authority");
        let lineage = root.join("lineage");
        fs::create_dir_all(&authority).expect("authority fixture root is created");
        fs::create_dir_all(&lineage).expect("lineage fixture root is created");

        let repositories = repository_ids(case.repositories);
        let workspace = workspace_manifest(&repositories);
        let machine = machine_binding(&root, &distribution, &repositories, surface_executable);
        let members: [(&str, &[u8]); 10] = [
            (
                "composition.toml",
                fixture_bytes!("workspace/composition.toml"),
            ),
            ("pale-workspace.toml", workspace.as_bytes()),
            (
                "workspace-resolution.lock.toml",
                fixture_bytes!("workspace/workspace-resolution.lock.toml"),
            ),
            (
                "workspace-universe.lock.toml",
                fixture_bytes!("workspace/workspace-universe.lock.toml"),
            ),
            (
                "capabilities.toml",
                fixture_bytes!("private/capabilities.toml"),
            ),
            (
                "distribution-lock.toml",
                fixture_bytes!("private/distribution-lock.toml"),
            ),
            ("machines/machine.toml", machine.as_bytes()),
            (
                "distribution/packages/engine/package.toml",
                fixture_bytes!("packages/engine/package.toml"),
            ),
            (
                "distribution/packages/engine/roles.toml",
                fixture_bytes!("packages/engine/roles.toml"),
            ),
            (
                "distribution/packages/engine/role.md",
                fixture_bytes!("packages/engine/role.md"),
            ),
        ];
        let mut source_preimage = Vec::new();
        source_preimage.extend_from_slice(b"pale.local-materialization-benchmark-source.v1\0");
        for (relative, bytes) in members {
            write(&root, relative, bytes);
            append_frame(&mut source_preimage, relative.as_bytes());
            append_frame(&mut source_preimage, bytes);
        }
        for repository in &repositories {
            let relative = format!("checkouts/{repository}/.git/info/exclude");
            write(&root, &relative, USER_EXCLUDE_BYTES);
            append_frame(&mut source_preimage, relative.as_bytes());
            append_frame(&mut source_preimage, USER_EXCLUDE_BYTES);
        }
        let manifest = root
            .join("pale-workspace.toml")
            .to_str()
            .expect("temporary benchmark path is UTF-8")
            .to_owned();
        Self {
            _temporary: temporary,
            root,
            manifest,
            repositories,
            source_digest: sha256(&source_preimage),
        }
    }

    /// Construct the complete local compile request without host access.
    fn request(&self) -> LocalOneShotCompilationRequest {
        LocalOneShotCompilationRequest::new(
            LocalWorkspaceLocator::new(self.manifest.clone()),
            "demo".to_owned(),
            "default".to_owned(),
            "machine".to_owned(),
        )
    }

    /// Caller-selected durable-plan root for this fixture.
    fn plans(&self) -> String {
        self.root.join("plans").display().to_string()
    }

    /// Caller-selected immutable-evidence root for this fixture.
    fn evidence(&self) -> String {
        self.root.join("evidence").display().to_string()
    }
}

/// Pre-timing law evidence retained in every case row.
struct CaseEvidence {
    /// Digest of exact fixture source/configuration bytes.
    source_digest: String,
    /// Stable one-shot compilation identity.
    compilation_digest: String,
    /// Ordered set digest of per-repository render identities.
    render_set_digest: String,
    /// Ordered set digest of complete canonical plans.
    plan_set_digest: String,
    /// Number of rendered materialization targets proved against plan counts.
    materialized_surfaces: u64,
    /// Number of planned surface operations proved by the law gate.
    planned_operations: u64,
    /// Number of changed surfaces in the successful settlement.
    changed_surfaces: u64,
    /// Exact run-result byte length read back after settlement.
    run_result_bytes: usize,
    /// Lower bound over regular repository files visible after settlement.
    visible_repository_bytes: u64,
}

/// One measured façade operation.
#[derive(Clone, Copy)]
enum Operation {
    /// Resolve, capture, admit, compile, and render selected local source.
    Compile,
    /// Construct complete plans for all repositories.
    Plan,
    /// Durably publish all complete plans.
    Publish,
    /// Rebuild and admit every saved plan without mutation.
    Admit,
    /// Settle an already admitted run.
    Settle,
    /// Run compilation through settlement against one fresh fixture.
    FreshPipeline,
}

impl Operation {
    /// Stable report spelling.
    const fn as_str(self) -> &'static str {
        match self {
            Self::Compile => "compile_local_one_shot",
            Self::Plan => "plan_all_repositories",
            Self::Publish => "publish_all_plans",
            Self::Admit => "admit_materialization_run",
            Self::Settle => "settle_materialization_run",
            Self::FreshPipeline => "fresh_compile_to_settlement",
        }
    }

    /// Collect independent observations with all prerequisites outside timing.
    fn observe(
        self,
        api: &PaleApi,
        case: BenchmarkCase,
        configuration: &Configuration,
    ) -> Distribution {
        let mut observations = Vec::with_capacity(configuration.samples);
        match self {
            Self::Compile => {
                let fixture = Fixture::new(case, &configuration.surface_executable);
                for _ in 0..configuration.samples {
                    observations.push(time(|| {
                        api.compile_local_one_shot(black_box(fixture.request()))
                            .expect("timed local compilation succeeds")
                    }));
                }
            }
            Self::Plan => {
                let fixture = Fixture::new(case, &configuration.surface_executable);
                let compilation = compile(api, &fixture);
                for _ in 0..configuration.samples {
                    observations.push(time(|| plan_all(api, &compilation, &fixture.repositories)));
                }
            }
            Self::Publish => {
                let fixture = Fixture::new(case, &configuration.surface_executable);
                let compilation = compile(api, &fixture);
                let plans = plan_all(api, &compilation, &fixture.repositories);
                for _ in 0..configuration.samples {
                    let destination =
                        tempfile::tempdir_in(&fixture.root).expect("publication parent is created");
                    let output = destination.path().join("plans").display().to_string();
                    observations.push(time(|| publish_all(api, &plans, &output)));
                }
            }
            Self::Admit => {
                let fixture = Fixture::new(case, &configuration.surface_executable);
                let compilation = compile(api, &fixture);
                let plans = plan_all(api, &compilation, &fixture.repositories);
                publish_all(api, &plans, &fixture.plans());
                for sample in 0..configuration.samples {
                    let evidence = fixture
                        .root
                        .join(format!("admission-evidence-{sample}"))
                        .display()
                        .to_string();
                    observations.push(time(|| {
                        api.admit_local_materialization_run(
                            black_box(&compilation),
                            LocalMaterializationAdmissionRequest::new(
                                OneShotPlanKind::Apply,
                                fixture.plans(),
                                evidence,
                            ),
                        )
                        .expect("timed aggregate admission succeeds")
                    }));
                }
            }
            Self::Settle => {
                for _ in 0..configuration.samples {
                    let (fixture, compilation, mut admitted) =
                        prepare_admitted(api, case, &configuration.surface_executable);
                    observations.push(time(|| {
                        api.settle_local_materialization_run(black_box(&mut admitted))
                            .expect("timed settlement succeeds")
                    }));
                    black_box((fixture, compilation));
                }
            }
            Self::FreshPipeline => {
                for _ in 0..configuration.samples {
                    let fixture = Fixture::new(case, &configuration.surface_executable);
                    observations.push(time(|| complete_pipeline(api, &fixture)));
                }
            }
        }
        Distribution::new(observations, configuration.samples)
    }
}

/// Run all case law gates and timed observations, then emit one sealed report.
#[must_use]
pub(crate) fn run() -> ExitCode {
    let api = PaleApi::new().expect("embedded API dependencies are coherent");
    let Some(configuration) = Configuration::from_environment(&api) else {
        eprintln!(
            "local-materialization benchmark skipped; set PALE_BENCH_RUN=1 or PALE_LOCAL_MATERIALIZATION_BENCH_SMOKE=1"
        );
        return ExitCode::SUCCESS;
    };
    let cases = if configuration.smoke {
        &CASES[..1]
    } else {
        &CASES[..]
    };
    let expected_rows = cases.len() * OPERATIONS.len();
    let mut payload = String::from(
        "source_coordinate\tfixture_sha256\trun_coordinate\tevidence_class\tcase_id\trepositories\toperation\tlaw_gate_passed\tcompilation_sha256\trender_set_sha256\tplan_set_sha256\tmaterialized_surfaces\tplanned_operations\tchanged_surfaces\trun_result_bytes\tvisible_repository_bytes\tsamples\tp50_ns\tp95_ns\tp99_ns\tmax_ns\thardware\tos\tkernel\tarch\ttoolchain\tprofile\tpgo\tcpu_target\ttarget\tthreads\tpolicy_selected\trelease_evidence\n",
    );
    let mut row_count = 0_usize;
    for &case in cases {
        let evidence = prove_case_laws(&api, case, &configuration.surface_executable);
        for operation in OPERATIONS {
            let distribution = operation.observe(&api, case, &configuration);
            render_row(
                &mut payload,
                &configuration,
                case,
                operation,
                &evidence,
                &distribution,
            );
            row_count += 1;
        }
    }
    assert_eq!(row_count, expected_rows, "benchmark row count drift");
    let payload_digest = sha256(payload.as_bytes());
    writeln!(
        payload,
        "# pale-local-materialization-benchmark-seal\trows={row_count}\texpected_rows={expected_rows}\tpayload_sha256={}\tsource_coordinate={}\trun_coordinate={}",
        payload_digest,
        configuration.source_coordinate,
        configuration.run_coordinate,
    )
    .expect("benchmark seal is written");
    print!("{payload}");
    ExitCode::SUCCESS
}

/// Prove deterministic compilation/planning and exact settlement read-back.
fn prove_case_laws(
    api: &PaleApi,
    case: BenchmarkCase,
    surface_executable: &SurfaceExecutableCoordinate,
) -> CaseEvidence {
    let fixture = Fixture::new(case, surface_executable);
    let first = compile(api, &fixture);
    let second = compile(api, &fixture);
    let first_summary = first.summary();
    let second_summary = second.summary();
    assert_eq!(
        first_summary.one_shot_compilation_digest(),
        second_summary.one_shot_compilation_digest(),
        "repeated compilation must preserve identity"
    );
    assert_eq!(
        first_summary.repositories(),
        second_summary.repositories(),
        "repeated compilation must preserve repository summaries"
    );
    assert_eq!(
        usize::try_from(first_summary.repository_count()).expect("repository count fits usize"),
        case.repositories
    );

    let first_plans = plan_all(api, &first, &fixture.repositories);
    let second_plans = plan_all(api, &second, &fixture.repositories);
    assert_eq!(first_plans.len(), second_plans.len());
    let mut materialized_surfaces = 0_u64;
    let mut planned_operations = 0_u64;
    for ((repository, first_plan), second_plan) in first_summary
        .repositories()
        .iter()
        .zip(&first_plans)
        .zip(&second_plans)
    {
        assert_eq!(
            first_plan.document().canonical_bytes(),
            second_plan.document().canonical_bytes(),
            "repeated planning must preserve canonical bytes"
        );
        let counts = first_plan.counts();
        assert_eq!(counts.create(), repository.materialized_surface_count());
        assert_eq!(counts.operations(), repository.materialized_surface_count());
        assert_eq!(
            counts.total_operations(),
            counts.operations() + counts.machine_local_effects()
        );
        materialized_surfaces += u64::from(repository.materialized_surface_count());
        planned_operations += u64::from(counts.total_operations());
    }

    let publications = publish_all(api, &first_plans, &fixture.plans());
    let first_saved = publications
        .iter()
        .map(|publication| {
            fs::read(
                fixture
                    .root
                    .join("plans")
                    .join(publication.relative_target()),
            )
            .expect("published plan is readable")
        })
        .collect::<Vec<_>>();
    let repeated = publish_all(api, &first_plans, &fixture.plans());
    assert_eq!(
        publications, repeated,
        "publication identities are idempotent"
    );
    for (publication, expected) in repeated.iter().zip(&first_saved) {
        let observed = fs::read(
            fixture
                .root
                .join("plans")
                .join(publication.relative_target()),
        )
        .expect("repeated plan publication is readable");
        assert_eq!(&observed, expected, "publication is exact-byte idempotent");
        assert!(observed.windows(3).any(|window| window == b"\n  "));
        assert!(observed.ends_with(b"\n"));
    }

    let mut admitted = api
        .admit_local_materialization_run(
            &first,
            LocalMaterializationAdmissionRequest::new(
                OneShotPlanKind::Apply,
                fixture.plans(),
                fixture.evidence(),
            ),
        )
        .expect("law-gate aggregate admission succeeds");
    assert_eq!(admitted.targets().len(), case.repositories);
    for (target, plan) in admitted.targets().iter().zip(&first_plans) {
        assert_eq!(
            target.repository_id(),
            plan.document().repository_id(),
            "admission must preserve repository identity"
        );
        assert_eq!(
            target.plan_digest(),
            public_plan_digest(plan),
            "admission must preserve exact plan identity"
        );
    }
    let result = api
        .settle_local_materialization_run(&mut admitted)
        .expect("law-gate settlement succeeds");
    assert_eq!(result.outcome(), LocalMaterializationRunOutcome::Succeeded);
    assert_eq!(result.targets().len(), case.repositories);
    assert!(result
        .targets()
        .iter()
        .all(|target| target.outcome() == LocalMaterializationTargetOutcome::Succeeded));
    let changed_surfaces = result
        .targets()
        .iter()
        .map(|target| u64::from(target.changed_surface_count()))
        .sum();
    let run_result = fs::read(fixture.root.join("evidence/run-result.apply.json"))
        .expect("settled run-result read-back is available");
    let observed: serde_json::Value =
        serde_json::from_slice(&run_result).expect("read-back is strict JSON");
    let expected: serde_json::Value =
        serde_json::from_slice(result.canonical_bytes()).expect("result projection is strict JSON");
    assert_eq!(observed, expected, "read-back preserves the result value");
    assert!(run_result.windows(3).any(|window| window == b"\n  "));
    assert!(run_result.ends_with(b"\n"));
    for repository in &fixture.repositories {
        assert!(api
            .inspect_local_repository_status(&first, repository.clone())
            .expect("post-settlement status is readable")
            .is_clean());
    }

    let post_compilation = compile(api, &fixture);
    assert_eq!(
        first_summary.one_shot_compilation_digest(),
        post_compilation.summary().one_shot_compilation_digest(),
        "native materialization cannot change source compilation"
    );
    let post_first = plan_all(api, &post_compilation, &fixture.repositories);
    let post_second = plan_all(api, &post_compilation, &fixture.repositories);
    assert!(post_first.iter().zip(&post_second).all(|(left, right)| {
        left.document().canonical_bytes() == right.document().canonical_bytes()
            && left.counts().create() == 0
            && left.counts().update() == 0
            && left.counts().splice() == 0
    }));

    CaseEvidence {
        source_digest: fixture.source_digest,
        compilation_digest: first_summary.one_shot_compilation_digest().to_owned(),
        render_set_digest: digest_set(
            first_summary
                .repositories()
                .iter()
                .map(|repository| repository.render_digest().as_bytes()),
        ),
        plan_set_digest: digest_set(
            first_plans
                .iter()
                .map(|plan| plan.document().canonical_bytes()),
        ),
        materialized_surfaces,
        planned_operations,
        changed_surfaces,
        run_result_bytes: run_result.len(),
        visible_repository_bytes: fixture
            .repositories
            .iter()
            .map(|repository| visible_bytes(&fixture.root.join("checkouts").join(repository)))
            .sum(),
    }
}

/// Read the admitted identity already carried by one public canonical plan.
fn public_plan_digest(plan: &LocalOneShotPlan) -> String {
    let value: serde_json::Value = serde_json::from_slice(plan.document().canonical_bytes())
        .expect("public plan bytes are strict JSON");
    value["plan_digest"]
        .as_str()
        .expect("public plan carries its admitted identity")
        .to_owned()
}

/// Complete one fresh compile, plan-publication, admission, and settlement path.
fn complete_pipeline(api: &PaleApi, fixture: &Fixture) -> impl std::fmt::Debug {
    let compilation = compile(api, fixture);
    let plans = plan_all(api, &compilation, &fixture.repositories);
    publish_all(api, &plans, &fixture.plans());
    let mut admitted = api
        .admit_local_materialization_run(
            &compilation,
            LocalMaterializationAdmissionRequest::new(
                OneShotPlanKind::Apply,
                fixture.plans(),
                fixture.evidence(),
            ),
        )
        .expect("fresh-pipeline admission succeeds");
    api.settle_local_materialization_run(&mut admitted)
        .expect("fresh-pipeline settlement succeeds")
}

/// Prepare one admitted run entirely outside its settlement interval.
fn prepare_admitted(
    api: &PaleApi,
    case: BenchmarkCase,
    surface_executable: &SurfaceExecutableCoordinate,
) -> (
    Fixture,
    LocalOneShotCompilation,
    LocalMaterializationAdmittedRun,
) {
    let fixture = Fixture::new(case, surface_executable);
    let compilation = compile(api, &fixture);
    let plans = plan_all(api, &compilation, &fixture.repositories);
    publish_all(api, &plans, &fixture.plans());
    let admitted = api
        .admit_local_materialization_run(
            &compilation,
            LocalMaterializationAdmissionRequest::new(
                OneShotPlanKind::Apply,
                fixture.plans(),
                fixture.evidence(),
            ),
        )
        .expect("settlement prerequisite admission succeeds");
    (fixture, compilation, admitted)
}

/// Resolve and compile one disposable source fixture through the public façade.
fn compile(api: &PaleApi, fixture: &Fixture) -> LocalOneShotCompilation {
    api.compile_local_one_shot(fixture.request())
        .expect("benchmark fixture compiles")
}

/// Construct all repository plans in canonical materialization order.
fn plan_all(
    api: &PaleApi,
    compilation: &LocalOneShotCompilation,
    repositories: &[String],
) -> Vec<LocalOneShotPlan> {
    repositories
        .iter()
        .map(|repository| {
            api.plan_local_one_shot(
                compilation,
                LocalOneShotPlanRequest::new(repository.clone(), OneShotPlanKind::Apply),
            )
            .expect("benchmark repository plan succeeds")
        })
        .collect()
}

/// Publish every repository plan into one confined caller-selected root.
fn publish_all(
    api: &PaleApi,
    plans: &[LocalOneShotPlan],
    output: &str,
) -> Vec<pale_sdk::LocalOneShotPlanPublication> {
    plans
        .iter()
        .map(|plan| {
            api.publish_local_one_shot_plan(plan, output.to_owned())
                .expect("benchmark plan publication succeeds")
        })
        .collect()
}

/// Measure one closure while excluding output destruction from elapsed time.
fn time<T>(run: impl FnOnce() -> T) -> u128 {
    let started = Instant::now();
    let output = run();
    let elapsed = started.elapsed().as_nanos();
    black_box(&output);
    elapsed
}

/// Sorted elapsed-time distribution for one operation and case.
struct Distribution {
    /// Median elapsed nanoseconds.
    p50: u128,
    /// Ninety-fifth percentile elapsed nanoseconds.
    p95: u128,
    /// Ninety-ninth percentile when at least 101 samples exist.
    p99: Option<u128>,
    /// Maximum elapsed nanoseconds.
    max: u128,
}

impl Distribution {
    /// Construct nearest-rank percentiles from independent observations.
    fn new(mut values: Vec<u128>, samples: usize) -> Self {
        assert_eq!(values.len(), samples);
        assert!(!values.is_empty());
        values.sort_unstable();
        Self {
            p50: percentile(&values, 50),
            p95: percentile(&values, 95),
            p99: (samples >= 101).then(|| percentile(&values, 99)),
            max: *values.last().expect("distribution is nonempty"),
        }
    }
}

/// Render one complete benchmark row into the checksum-covered payload.
fn render_row(
    payload: &mut String,
    configuration: &Configuration,
    case: BenchmarkCase,
    operation: Operation,
    evidence: &CaseEvidence,
    distribution: &Distribution,
) {
    let fields = [
        configuration.source_coordinate.clone(),
        evidence.source_digest.clone(),
        configuration.run_coordinate.clone(),
        configuration.evidence_class().to_owned(),
        case.id.to_owned(),
        case.repositories.to_string(),
        operation.as_str().to_owned(),
        "true".to_owned(),
        evidence.compilation_digest.clone(),
        evidence.render_set_digest.clone(),
        evidence.plan_set_digest.clone(),
        evidence.materialized_surfaces.to_string(),
        evidence.planned_operations.to_string(),
        evidence.changed_surfaces.to_string(),
        evidence.run_result_bytes.to_string(),
        evidence.visible_repository_bytes.to_string(),
        configuration.samples.to_string(),
        distribution.p50.to_string(),
        distribution.p95.to_string(),
        distribution
            .p99
            .map(|value| value.to_string())
            .unwrap_or_default(),
        distribution.max.to_string(),
        configuration.hardware.clone(),
        std::env::consts::OS.to_owned(),
        configuration.kernel.clone(),
        std::env::consts::ARCH.to_owned(),
        configuration.toolchain.clone(),
        benchmark_profile().to_owned(),
        configuration.pgo.clone(),
        configuration.cpu_target.clone(),
        configuration.target.clone(),
        BENCHMARK_THREADS.to_string(),
        "false".to_owned(),
        "false".to_owned(),
    ];
    for field in &fields {
        validate_tsv_field(field);
    }
    writeln!(payload, "{}", fields.join("\t")).expect("benchmark row is written");
}

/// Generate stable logical repository identities for one case.
fn repository_ids(count: usize) -> Vec<String> {
    (0..count)
        .map(|index| {
            if index == 0 {
                "engine".to_owned()
            } else {
                format!("engine-{index:03}")
            }
        })
        .collect()
}

/// Construct the single canonical workspace graph varied by repository count.
fn workspace_manifest(repositories: &[String]) -> String {
    let repositories_value = repositories
        .iter()
        .map(|id| format!("{{ id = {id:?}, title = {id:?} }}"))
        .collect::<Vec<_>>()
        .join(", ");
    let project_repositories = repositories
        .iter()
        .enumerate()
        .map(|(order, id)| {
            format!(
                "{{ project_id = \"demo\", repository_id = {id:?}, role = {{ package_id = \"example.engine\", resource_id = \"engine\" }}, stacks = [], families = [], materialization_order = {order} }}"
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let execution_repositories = repositories
        .iter()
        .map(|id| {
            format!(
                "{{ execution_workspace_id = \"default\", repository_id = {id:?}, access = \"mutate\" }}"
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "[workspace]\nschema_version = 1\nworkspace_id = \"act-ws\"\ndistribution_lock = \"distribution-lock.toml\"\nmachine_binding_root = \"machines\"\ndesired_topology = {{ projects = [{{ id = \"demo\", title = \"Synthetic compile\", run_policy = \"standard\", package_seeds = [\"example.engine\"], authority_root_ids = [\"authority\"], capabilities_file = \"capabilities.toml\" }}], execution_workspaces = [{{ id = \"default\", title = \"Default execution workspace\", execution_path_ids = [\"direct-claude\"] }}], repositories = [{repositories_value}], execution_paths = [{{ id = \"direct-claude\", experience_path_id = \"direct-claude-code\" }}], project_repositories = [{project_repositories}], project_execution_workspaces = [{{ project_id = \"demo\", execution_workspace_id = \"default\" }}], execution_workspace_repositories = [{execution_repositories}] }}\nmodule_manifests = []\nsource_components = []\nresponsibility_derivations = []\norganizational_translations = []\nexperience_compositions = [\"composition.toml\"]\nresolution_lock = \"workspace-resolution.lock.toml\"\nuniverse_lock = \"workspace-universe.lock.toml\"\n"
    )
}

/// Construct one private machine binding over all disposable checkout roots.
fn machine_binding(
    root: &Path,
    distribution: &Path,
    repositories: &[String],
    surface_executable: &SurfaceExecutableCoordinate,
) -> String {
    let mut output = format!(
        "[machine]\nid = \"machine\"\nschema_version = 1\ndistribution_root = {:?}\n",
        distribution.display().to_string(),
    );
    for repository in repositories {
        writeln!(
            output,
            "\n[[repository_checkouts]]\nrepository_id = {repository:?}\ncheckout_root = {:?}",
            root.join("checkouts")
                .join(repository)
                .display()
                .to_string(),
        )
        .expect("machine binding is written");
    }
    write!(
        output,
        "\n[authority_roots]\nauthority = {:?}\n\n[materialization_lineage]\nroot = {:?}\nmax_route_records_per_collection = 4096\n\n[surface_executable]\nexec = {:?}\ndigest = {:?}\nversion = {:?}\nprotocol = {:?}\n",
        root.join("authority").display().to_string(),
        root.join("lineage").display().to_string(),
        surface_executable.executable,
        surface_executable.digest,
        surface_executable.version,
        surface_executable.protocol,
    )
    .expect("machine binding is written");
    output
}

/// Write one exact fixture member, creating only its required parents.
fn write(root: &Path, relative: &str, bytes: &[u8]) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("fixture member has a parent"))
        .expect("fixture parent is created");
    fs::write(path, bytes).expect("fixture member is written");
}

/// Sum regular-file bytes below one checkout while excluding `.git` mechanics.
fn visible_bytes(root: &Path) -> u64 {
    fs::read_dir(root)
        .expect("settled repository is readable")
        .map(|entry| entry.expect("repository member is readable"))
        .filter(|entry| entry.file_name() != ".git")
        .map(|entry| {
            let metadata = entry.metadata().expect("repository metadata is readable");
            if metadata.is_dir() {
                visible_bytes(&entry.path())
            } else if metadata.is_file() {
                metadata.len()
            } else {
                panic!("benchmark repository contains a non-regular member")
            }
        })
        .sum()
}

/// Hash an ordered sequence of already-bounded byte strings without flattening it.
fn digest_set<'a>(values: impl IntoIterator<Item = &'a [u8]>) -> String {
    let mut preimage = Vec::new();
    preimage.extend_from_slice(SET_DIGEST_DOMAIN);
    let mut count = 0_u64;
    for value in values {
        let digest = sha256(value);
        append_frame(&mut preimage, digest.as_bytes());
        count = count.checked_add(1).expect("digest-set count overflow");
    }
    preimage.extend_from_slice(&count.to_be_bytes());
    sha256(&preimage)
}

/// Return the lowercase SHA-256 digest of one exact benchmark byte string.
fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Append one unambiguous length-delimited field to a digest preimage.
fn append_frame(target: &mut Vec<u8>, bytes: &[u8]) {
    let length = u64::try_from(bytes.len()).expect("benchmark digest frame fits u64");
    target.extend_from_slice(&length.to_be_bytes());
    target.extend_from_slice(bytes);
}

/// Resolve a caller coordinate, permitting explicit smoke-only defaults.
fn required_coordinate(name: &str, smoke: bool, smoke_default: &str) -> String {
    let value = match std::env::var(name) {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) if smoke => smoke_default.to_owned(),
        Err(std::env::VarError::NotPresent) => {
            panic!("full local-materialization characterization requires {name}")
        }
        Err(std::env::VarError::NotUnicode(_)) => panic!("{name} must be UTF-8"),
    };
    assert!(!value.is_empty(), "{name} must not be empty");
    validate_tsv_field(&value);
    value
}

/// Reject a field that could escape its bounded TSV column.
fn validate_tsv_field(value: &str) {
    assert!(
        value.len() <= MAX_COORDINATE_BYTES && !value.contains(['\t', '\r', '\n', '\0']),
        "benchmark report fields must be bounded single-line UTF-8"
    );
}

/// Return one nearest-rank percentile from sorted nonempty observations.
fn percentile(sorted: &[u128], percentile: usize) -> u128 {
    assert!(!sorted.is_empty() && (1..=100).contains(&percentile));
    let rank = (percentile * sorted.len()).div_ceil(100);
    sorted[rank.saturating_sub(1)]
}

/// Return the compiler and host coordinates once for this benchmark run.
fn toolchain_coordinate() -> (String, String) {
    let output = std::process::Command::new("rustc")
        .arg("-vV")
        .output()
        .expect("rustc is available to identify the benchmark run");
    assert!(output.status.success(), "rustc -vV succeeds");
    let output = String::from_utf8(output.stdout).expect("rustc output is UTF-8");
    let toolchain = output
        .lines()
        .next()
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown-rustc")
        .to_owned();
    let target = output
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown-target")
        .to_owned();
    validate_tsv_field(&toolchain);
    validate_tsv_field(&target);
    (toolchain, target)
}

/// Cargo profile that produced the executing benchmark binary.
const fn benchmark_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}
