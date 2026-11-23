//! Benchmark-only characterization of immutable Package-text mechanics.
//!
//! This feature-gated module consumes the private Package-text owner directly.
//! It does not publish text handles, reinterpret Package semantics, or place
//! measurement policy on the default application path.

use std::fmt::Write as _;
use std::fs;
use std::hint::black_box;
use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;

use pale_core::managed::{MANAGED_BEGIN, MANAGED_END};
use pale_core::records::{ArtifactDigest, PackageIdV1, PackageResourceIdV1, PackageResourceRefV1};

use super::text::{seal_texts, TextRecipe, TextRecipeBuilder, TextRef};

/// Full-run independent observations, matching the resident benchmark floor.
const FULL_SAMPLES: usize = 201;
/// Compile-and-execute observations that carry no characterization authority.
const SMOKE_SAMPLES: usize = 5;
/// Maximum Markdown members accepted by the bounded benchmark reader.
const MAX_DOCUMENTS: usize = 4_096;
/// Maximum filesystem members inspected below the benchmark root.
const MAX_TREE_ENTRIES: usize = 16_384;
/// Maximum actual Markdown bytes accepted before deterministic expansion.
///
/// The ceiling keeps every declared 100x case below the separate expanded-
/// source and contiguous-output bounds, including separators and transforms.
const MAX_ACTUAL_BYTES: usize = 2 * 1024 * 1024;
/// Maximum deterministic source bytes retained by one expanded fixture.
const MAX_EXPANDED_SOURCE_BYTES: usize = 256 * 1024 * 1024;
/// Maximum total UTF-8 bytes across repository outputs from one operation.
const MAX_LOGICAL_OUTPUT_BYTES: usize = 256 * 1024 * 1024;
/// Maximum explicit recipe parts constructed for one paired case.
const MAX_RECIPE_PARTS: usize = 2_000_000;
/// Maximum recursive directory depth below the caller-selected Package root.
const MAX_DIRECTORY_DEPTH: usize = 16;
/// Maximum UTF-8 bytes in one caller-supplied report coordinate.
const MAX_COORDINATE_BYTES: usize = 512;
/// Domain separator for the exact local Markdown-corpus digest.
const CORPUS_DIGEST_DOMAIN: &[u8] = b"pale.package-text-benchmark-corpus.v1\0";
/// Domain separator for one ordered set of repository output digests.
const OUTPUT_SET_DIGEST_DOMAIN: &[u8] = b"pale.package-text-benchmark-output-set.v1\0";
/// Number of timed operations emitted for every paired benchmark case.
const OPERATIONS_PER_CASE: usize = 7;
/// The harness executes every measured operation on its calling thread.
const BENCHMARK_THREADS: usize = 1;

/// Bounded, deliberately non-Cartesian workload matrix.
const CASES: [BenchmarkCase; 6] = [
    BenchmarkCase::new(
        "actual-baseline",
        CorpusScale::Actual,
        1,
        1,
        HeadingTransform::None,
        MaterializationBoundary::WholeFile,
    ),
    BenchmarkCase::new(
        "actual-ten-repositories",
        CorpusScale::Actual,
        10,
        1,
        HeadingTransform::Sparse,
        MaterializationBoundary::ManagedBlock,
    ),
    BenchmarkCase::new(
        "actual-thirty-two-consumers",
        CorpusScale::Actual,
        1,
        32,
        HeadingTransform::Dense,
        MaterializationBoundary::WholeFile,
    ),
    BenchmarkCase::new(
        "actual-hundred-repositories",
        CorpusScale::Actual,
        100,
        1,
        HeadingTransform::None,
        MaterializationBoundary::ManagedBlock,
    ),
    BenchmarkCase::new(
        "ten-x-four-consumers",
        CorpusScale::TenX,
        1,
        4,
        HeadingTransform::Sparse,
        MaterializationBoundary::WholeFile,
    ),
    BenchmarkCase::new(
        "hundred-x-dense",
        CorpusScale::HundredX,
        1,
        1,
        HeadingTransform::Dense,
        MaterializationBoundary::ManagedBlock,
    ),
];

/// Benchmark configuration resolved from one bounded environment snapshot.
struct BenchmarkConfiguration {
    /// Whether this invocation is compile-and-execute validation only.
    smoke: bool,
    /// Number of independent operation observations in each row.
    samples: usize,
    /// Local Package root whose Markdown bytes form the actual corpus.
    package_root: PathBuf,
    /// Exact caller-assigned source coordinate.
    source_coordinate: String,
    /// Exact caller-assigned run coordinate.
    run_coordinate: String,
    /// Stable caller description of the measured hardware.
    hardware: String,
    /// Exact caller-supplied operating-system kernel coordinate.
    kernel: String,
    /// Exact compiler version resolved once for the complete run.
    toolchain: String,
    /// Exact profile-guided-optimization coordinate.
    pgo: String,
    /// Exact caller-supplied CPU-target setting.
    cpu_target: String,
    /// Compiler host target resolved with [`Self::toolchain`].
    target: String,
}

impl BenchmarkConfiguration {
    /// Resolve one explicit run mode, or skip ordinary Cargo target discovery.
    fn from_environment() -> Option<Self> {
        let smoke = match std::env::var_os("PALE_PACKAGE_TEXT_BENCH_SMOKE") {
            None => false,
            Some(value) if value == "1" => true,
            Some(_) => panic!("PALE_PACKAGE_TEXT_BENCH_SMOKE must be 1 when present"),
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
        let package_root = std::env::var_os("PALE_PACKAGE_TEXT_BENCH_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packages"));
        let (toolchain, target) = toolchain_coordinate();
        Some(Self {
            smoke,
            samples: if smoke { SMOKE_SAMPLES } else { FULL_SAMPLES },
            package_root,
            source_coordinate: required_coordinate(
                "PALE_PACKAGE_TEXT_BENCH_SOURCE_COORDINATE",
                smoke,
                "smoke-unsealed-worktree",
            ),
            run_coordinate: required_coordinate(
                "PALE_PACKAGE_TEXT_BENCH_RUN_COORDINATE",
                smoke,
                "smoke-validation",
            ),
            hardware: required_coordinate(
                "PALE_PACKAGE_TEXT_BENCH_HARDWARE",
                smoke,
                "unspecified-smoke-host",
            ),
            kernel: required_coordinate(
                "PALE_PACKAGE_TEXT_BENCH_KERNEL",
                smoke,
                "unspecified-smoke-kernel",
            ),
            toolchain,
            pgo: required_coordinate("PALE_PACKAGE_TEXT_BENCH_PGO", smoke, "none"),
            cpu_target: required_coordinate(
                "PALE_PACKAGE_TEXT_BENCH_CPU_TARGET",
                smoke,
                "unspecified-smoke-cpu-target",
            ),
            target,
        })
    }

    /// Exact evidence label whose ceiling is encoded in every row.
    const fn evidence_class(&self) -> &'static str {
        if self.smoke {
            "smoke_validation"
        } else {
            "local_characterization"
        }
    }
}

/// One immutable Markdown member from the caller-selected Package tree.
struct SourceDocument {
    /// Exact retained UTF-8 bytes.
    text: Arc<str>,
    /// Byte offsets of admitted-looking ATX headings in this measurement corpus.
    heading_offsets: Arc<[usize]>,
}

/// Exact current Markdown corpus before any deterministic scale expansion.
struct ActualCorpus {
    /// Documents in canonical relative-path order.
    documents: Vec<SourceDocument>,
    /// Sum of exact document byte lengths.
    bytes: usize,
    /// Digest of framed relative paths and exact bytes.
    digest: ArtifactDigest,
}

impl ActualCorpus {
    /// Read one bounded, non-symlink Markdown tree without interpreting Package meaning.
    fn capture(root: &Path) -> Self {
        let metadata = fs::symlink_metadata(root).expect("read Package benchmark root metadata");
        assert!(
            metadata.is_dir() && !metadata.file_type().is_symlink(),
            "Package benchmark root must be a regular non-symlink directory"
        );
        let mut paths = Vec::new();
        let mut tree_entries = 0;
        collect_markdown_paths(root, root, 0, &mut tree_entries, &mut paths);
        paths.sort_unstable();
        assert!(
            !paths.is_empty() && paths.len() <= MAX_DOCUMENTS,
            "Package benchmark corpus must contain 1..={MAX_DOCUMENTS} Markdown files"
        );

        let mut bytes = 0_usize;
        let mut preimage = Vec::new();
        preimage.extend_from_slice(CORPUS_DIGEST_DOMAIN);
        let documents = paths
            .into_iter()
            .map(|path| {
                let file = fs::File::open(&path).expect("open Package benchmark Markdown");
                let declared_bytes = usize::try_from(
                    file.metadata()
                        .expect("read Package benchmark Markdown metadata")
                        .len(),
                )
                .expect("Package benchmark Markdown length fits usize");
                let next_bytes = bytes
                    .checked_add(declared_bytes)
                    .expect("Package benchmark byte count overflow");
                assert!(
                    next_bytes <= MAX_ACTUAL_BYTES,
                    "Package benchmark Markdown exceeds {MAX_ACTUAL_BYTES} bytes"
                );
                let read_limit = u64::try_from(declared_bytes)
                    .expect("Package benchmark Markdown length fits u64")
                    .checked_add(1)
                    .expect("Package benchmark Markdown read limit overflow");
                let mut raw = Vec::with_capacity(declared_bytes);
                file.take(read_limit)
                    .read_to_end(&mut raw)
                    .expect("read Package benchmark Markdown");
                assert_eq!(
                    raw.len(),
                    declared_bytes,
                    "Package benchmark Markdown changed while being captured"
                );
                bytes = bytes
                    .checked_add(raw.len())
                    .expect("Package benchmark byte count overflow");
                let text = String::from_utf8(raw).expect("Package benchmark Markdown is UTF-8");
                let relative_path = confined_relative_path(root, &path);
                append_frame(&mut preimage, relative_path.as_bytes());
                append_frame(&mut preimage, text.as_bytes());
                let heading_offsets = atx_heading_offsets(&text).into();
                SourceDocument {
                    text: Arc::from(text),
                    heading_offsets,
                }
            })
            .collect();
        Self {
            documents,
            bytes,
            digest: ArtifactDigest::hash_bytes(&preimage),
        }
    }
}

/// One expanded source object with benchmark-only mechanical ownership.
struct ExpandedObject {
    /// Deterministic benchmark-local owner; never a Package admission claim.
    owner: PackageResourceRefV1,
    /// Exact immutable source bytes copied during untimed fixture preparation.
    text: Arc<str>,
    /// ATX heading offsets inherited from the corresponding actual member.
    heading_offsets: Arc<[usize]>,
}

/// Actual Package Markdown replicated to one deterministic scale.
struct ExpandedCorpus {
    /// Expanded objects in benchmark-owner order.
    objects: Vec<ExpandedObject>,
    /// Total logical source bytes represented by the expansion.
    bytes: usize,
}

impl ExpandedCorpus {
    /// Expand the exact actual corpus without claiming new semantic resources.
    fn new(actual: &ActualCorpus, scale: CorpusScale) -> Self {
        let factor = scale.factor();
        let object_count = actual
            .documents
            .len()
            .checked_mul(factor)
            .expect("expanded object count overflow");
        assert!(object_count <= u32::MAX as usize);
        let bytes = actual
            .bytes
            .checked_mul(factor)
            .expect("expanded corpus byte count overflow");
        assert!(
            bytes <= MAX_EXPANDED_SOURCE_BYTES,
            "expanded Package benchmark source exceeds {MAX_EXPANDED_SOURCE_BYTES} bytes"
        );
        let package_id = PackageIdV1::parse("package-text-benchmark")
            .expect("benchmark Package identity is valid");
        let mut objects = Vec::with_capacity(object_count);
        for replica in 0..factor {
            for (document_index, document) in actual.documents.iter().enumerate() {
                let object_index = replica * actual.documents.len() + document_index;
                let resource_id = PackageResourceIdV1::parse(format!("object-{object_index:08}"))
                    .expect("benchmark resource identity is valid");
                objects.push(ExpandedObject {
                    owner: PackageResourceRefV1 {
                        package_id: package_id.clone(),
                        resource_id,
                    },
                    text: Arc::from(document.text.as_ref()),
                    heading_offsets: Arc::clone(&document.heading_offsets),
                });
            }
        }
        Self { objects, bytes }
    }

    /// Prepare the owner/text vector consumed by the canonical private store.
    fn seal_input(&self) -> Vec<(PackageResourceRefV1, Arc<str>)> {
        self.objects
            .iter()
            .map(|object| (object.owner.clone(), Arc::clone(&object.text)))
            .collect()
    }

    /// Seal the current input and retain references in owner order.
    fn seal(&self) -> Vec<TextRef> {
        let sealed = seal_texts(self.seal_input());
        assert_eq!(sealed.len(), self.objects.len());
        sealed
            .into_iter()
            .zip(&self.objects)
            .map(|((owner, reference), object)| {
                assert_eq!(owner, object.owner);
                reference
            })
            .collect()
    }
}

/// Scale coordinate for one actual or deterministically replicated corpus.
#[derive(Clone, Copy)]
enum CorpusScale {
    /// Exact current Markdown member set.
    Actual,
    /// Ten deterministic copies of every current Markdown member.
    TenX,
    /// One hundred deterministic copies of every current Markdown member.
    HundredX,
}

impl CorpusScale {
    /// Replication factor represented by this scale.
    const fn factor(self) -> usize {
        match self {
            Self::Actual => 1,
            Self::TenX => 10,
            Self::HundredX => 100,
        }
    }

    /// Stable report label for this scale.
    const fn as_str(self) -> &'static str {
        match self {
            Self::Actual => "actual",
            Self::TenX => "10x",
            Self::HundredX => "100x",
        }
    }

    /// Honest source qualification for this scale.
    const fn corpus_class(self) -> &'static str {
        match self {
            Self::Actual => "current_package_markdown_tree",
            Self::TenX => "deterministic_package_markdown_replication_10x",
            Self::HundredX => "deterministic_package_markdown_replication_100x",
        }
    }
}

/// Density of measured ATX heading-marker insertion.
#[derive(Clone, Copy)]
enum HeadingTransform {
    /// Preserve every source byte unchanged.
    None,
    /// Insert one marker before every fourth measured ATX heading.
    Sparse,
    /// Insert one marker before every measured ATX heading.
    Dense,
}

impl HeadingTransform {
    /// Whether the heading at this ordinal receives one added marker.
    const fn selects(self, ordinal: usize) -> bool {
        match self {
            Self::None => false,
            Self::Sparse => ordinal.is_multiple_of(4),
            Self::Dense => true,
        }
    }

    /// Stable report label for this transformation density.
    const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Sparse => "sparse_every_fourth_atx_heading",
            Self::Dense => "dense_every_atx_heading",
        }
    }
}

/// Contiguous-output boundary exercised after immutable composition.
#[derive(Clone, Copy)]
enum MaterializationBoundary {
    /// Emit only the composed Package text.
    WholeFile,
    /// Wrap composed text in the canonical managed-block markers.
    ManagedBlock,
}

impl MaterializationBoundary {
    /// Stable report label for this boundary.
    const fn as_str(self) -> &'static str {
        match self {
            Self::WholeFile => "whole_file",
            Self::ManagedBlock => "managed_block",
        }
    }
}

/// One paired point in the bounded non-Cartesian workload matrix.
#[derive(Clone, Copy)]
struct BenchmarkCase {
    /// Stable benchmark-local case identity.
    id: &'static str,
    /// Source-corpus scale.
    scale: CorpusScale,
    /// Number of repository projections consuming the same closure.
    repositories: usize,
    /// Semantic consumer fan-out within every repository.
    fanout: usize,
    /// Heading transformation density.
    headings: HeadingTransform,
    /// Final contiguous-output boundary.
    boundary: MaterializationBoundary,
}

impl BenchmarkCase {
    /// Construct one statically bounded case.
    const fn new(
        id: &'static str,
        scale: CorpusScale,
        repositories: usize,
        fanout: usize,
        headings: HeadingTransform,
        boundary: MaterializationBoundary,
    ) -> Self {
        Self {
            id,
            scale,
            repositories,
            fanout,
            headings,
            boundary,
        }
    }

    /// Number of repeated consumers materialized by this case.
    fn consumers(self) -> usize {
        self.repositories
            .checked_mul(self.fanout)
            .expect("benchmark consumer count overflow")
    }
}

/// One fully prepared case plus its already-proved repository outputs.
struct CaseFixture {
    /// Static case coordinate.
    case: BenchmarkCase,
    /// Expanded immutable source objects.
    corpus: ExpandedCorpus,
    /// Generation-fenced references into the sealed store.
    references: Vec<TextRef>,
    /// Exact eager-reference outputs in deterministic repository order.
    reference_outputs: Vec<Arc<String>>,
    /// Total exact bytes across [`Self::reference_outputs`].
    logical_output_bytes: usize,
    /// Exact ordered output-set identity shared by both interpreters.
    output_digest: ArtifactDigest,
    /// Number of heading markers added across all consumers.
    transformed_headings: usize,
}

impl CaseFixture {
    /// Build one case and fail before timing unless both interpreters agree.
    fn new(actual: &ActualCorpus, case: BenchmarkCase) -> Self {
        let corpus = ExpandedCorpus::new(actual, case.scale);
        validate_case_bounds(&corpus, case);
        let references = corpus.seal();
        let recipes = build_recipes(&corpus, &references, case);
        let recipe_outputs = materialize_recipes(&recipes);
        let reference_outputs = materialize_references(&corpus, case)
            .into_iter()
            .map(Arc::new)
            .collect::<Vec<_>>();
        assert_eq!(
            recipe_outputs.len(),
            reference_outputs.len(),
            "recipe and eager reference differ for {}",
            case.id
        );
        for (recipe, reference) in recipe_outputs.iter().zip(&reference_outputs) {
            assert_eq!(
                recipe.as_bytes(),
                reference.as_bytes(),
                "recipe and eager repository output differ for {}",
                case.id
            );
        }
        let output_digest = output_set_digest(reference_outputs.iter().map(|text| text.as_str()));
        assert_eq!(
            output_set_digest(recipe_outputs.iter().map(|text| text.as_str())),
            output_digest,
            "recipe and eager digest differ for {}",
            case.id
        );
        let logical_output_bytes = logical_output_bytes(&corpus, case);
        assert_eq!(
            reference_outputs
                .iter()
                .map(|output| output.len())
                .sum::<usize>(),
            logical_output_bytes
        );
        let transformed_headings = transformed_heading_count(&corpus, case)
            .checked_mul(case.consumers())
            .expect("transformed heading count overflow");
        Self {
            case,
            corpus,
            references,
            reference_outputs,
            logical_output_bytes,
            output_digest,
            transformed_headings,
        }
    }

    /// Time one operation repeatedly with untimed prerequisite preparation.
    fn observe(&self, operation: Operation, samples: usize) -> Distribution {
        let mut elapsed = Vec::with_capacity(samples);
        without_allocation_counting(|| {
            for _ in 0..samples {
                elapsed.push(operation.time_once(self));
            }
        });
        Distribution::new(elapsed, samples)
    }

    /// Measure current-thread Rust allocations for one operation invocation.
    fn allocation_evidence(&self, operation: Operation) -> Option<AllocationEvidence> {
        operation.allocation_evidence(self)
    }
}

/// Measured operation over the same prepared semantic workload.
#[derive(Clone, Copy)]
enum Operation {
    /// Seal owner-qualified immutable text objects.
    SealStore,
    /// Build one explicit-part composition recipe per repository.
    BuildRecipe,
    /// Construct repository outputs with the independent eager reference.
    EagerReference,
    /// Materialize every newly built repository recipe.
    ColdMaterialize,
    /// Clone every memoized repository materialization.
    WarmMaterialize,
    /// Hash the ordered set of already materialized outputs.
    DigestOutput,
    /// Copy every materialized output into its owned boundary buffer.
    BoundaryCopy,
}

impl Operation {
    /// Operations emitted once for every case in stable order.
    const ALL: [Self; OPERATIONS_PER_CASE] = [
        Self::SealStore,
        Self::BuildRecipe,
        Self::EagerReference,
        Self::ColdMaterialize,
        Self::WarmMaterialize,
        Self::DigestOutput,
        Self::BoundaryCopy,
    ];

    /// Stable row label for this operation.
    const fn as_str(self) -> &'static str {
        match self {
            Self::SealStore => "seal_text_store",
            Self::BuildRecipe => "build_text_recipes",
            Self::EagerReference => "materialize_eager_reference_outputs",
            Self::ColdMaterialize => "materialize_recipes_cold",
            Self::WarmMaterialize => "materialize_recipes_warm",
            Self::DigestOutput => "digest_materialized_output_set",
            Self::BoundaryCopy => "copy_contiguous_boundary_outputs",
        }
    }

    /// Static lower bound for explicit source/output text copies.
    ///
    /// `Q` is the exact total repository-output byte count. Fixture preparation, metadata
    /// framing, hashing reads, allocator internals, and operating-system copies
    /// are outside this bound. The mutation-amplification ratio is deliberately
    /// undefined because immutable composition has no logical mutation.
    fn explicit_text_copy_lower_bound(self, fixture: &CaseFixture) -> usize {
        match self {
            Self::EagerReference | Self::ColdMaterialize | Self::BoundaryCopy => {
                fixture.logical_output_bytes
            }
            Self::SealStore | Self::BuildRecipe | Self::WarmMaterialize | Self::DigestOutput => 0,
        }
    }

    /// Measure one invocation while keeping operation prerequisites outside the scope.
    fn allocation_evidence(self, fixture: &CaseFixture) -> Option<AllocationEvidence> {
        match self {
            Self::SealStore => {
                let input = fixture.corpus.seal_input();
                measure_allocations(|| {
                    black_box(seal_texts(input));
                })
            }
            Self::BuildRecipe => measure_allocations(|| {
                black_box(build_recipes(
                    &fixture.corpus,
                    &fixture.references,
                    fixture.case,
                ));
            }),
            Self::EagerReference => measure_allocations(|| {
                black_box(materialize_references(&fixture.corpus, fixture.case));
            }),
            Self::ColdMaterialize => {
                let recipes = build_recipes(&fixture.corpus, &fixture.references, fixture.case);
                measure_allocations(|| {
                    black_box(materialize_recipes(&recipes));
                })
            }
            Self::WarmMaterialize => {
                let recipes = build_recipes(&fixture.corpus, &fixture.references, fixture.case);
                black_box(materialize_recipes(&recipes));
                measure_allocations(|| {
                    for recipe in &recipes {
                        black_box(recipe.materialize());
                    }
                })
            }
            Self::DigestOutput => measure_allocations(|| {
                black_box(output_set_digest(
                    fixture.reference_outputs.iter().map(|text| text.as_str()),
                ));
            }),
            Self::BoundaryCopy => measure_allocations(|| {
                black_box(
                    fixture
                        .reference_outputs
                        .iter()
                        .map(|output| output.as_bytes().to_vec())
                        .collect::<Vec<_>>(),
                );
            }),
        }
    }

    /// Time one invocation while keeping operation prerequisites outside the interval.
    fn time_once(self, fixture: &CaseFixture) -> u128 {
        match self {
            Self::SealStore => {
                let input = fixture.corpus.seal_input();
                let started = Instant::now();
                let output = seal_texts(input);
                let elapsed = started.elapsed().as_nanos();
                black_box(&output);
                elapsed
            }
            Self::BuildRecipe => {
                let started = Instant::now();
                let output = build_recipes(&fixture.corpus, &fixture.references, fixture.case);
                let elapsed = started.elapsed().as_nanos();
                black_box(&output);
                elapsed
            }
            Self::EagerReference => {
                let started = Instant::now();
                let output = materialize_references(&fixture.corpus, fixture.case);
                let elapsed = started.elapsed().as_nanos();
                black_box(&output);
                elapsed
            }
            Self::ColdMaterialize => {
                let recipes = build_recipes(&fixture.corpus, &fixture.references, fixture.case);
                let started = Instant::now();
                let output = materialize_recipes(&recipes);
                let elapsed = started.elapsed().as_nanos();
                black_box(&output);
                elapsed
            }
            Self::WarmMaterialize => {
                let recipes = build_recipes(&fixture.corpus, &fixture.references, fixture.case);
                black_box(materialize_recipes(&recipes));
                let started = Instant::now();
                for recipe in &recipes {
                    black_box(recipe.materialize());
                }
                started.elapsed().as_nanos()
            }
            Self::DigestOutput => {
                let started = Instant::now();
                let output =
                    output_set_digest(fixture.reference_outputs.iter().map(|text| text.as_str()));
                let elapsed = started.elapsed().as_nanos();
                black_box(&output);
                elapsed
            }
            Self::BoundaryCopy => {
                let started = Instant::now();
                let output = fixture
                    .reference_outputs
                    .iter()
                    .map(|output| output.as_bytes().to_vec())
                    .collect::<Vec<_>>();
                let elapsed = started.elapsed().as_nanos();
                black_box(&output);
                elapsed
            }
        }
    }
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
    /// Build nearest-rank percentiles from independent observations.
    fn new(mut values: Vec<u128>, independent_samples: usize) -> Self {
        assert_eq!(values.len(), independent_samples);
        assert!(!values.is_empty());
        values.sort_unstable();
        Self {
            p50: percentile(&values, 50),
            p95: percentile(&values, 95),
            p99: (independent_samples >= 101).then(|| percentile(&values, 99)),
            max: *values.last().expect("distribution is nonempty"),
        }
    }
}

/// Current-thread Rust allocation observations for one operation invocation.
///
/// Balances begin at zero, not at the prepared fixture's live heap size.
/// Dropping prepared input can make balances negative and suppress later peaks;
/// relative peaks therefore cannot establish absolute operation or process memory.
struct AllocationEvidence {
    /// Total allocation calls.
    total_count: u64,
    /// Total bytes allocated, including bytes later freed.
    total_bytes: u64,
    /// Maximum nonnegative relative allocation-count balance.
    peak_relative_count: u64,
    /// Maximum nonnegative relative allocated-byte balance.
    peak_relative_bytes: u64,
    /// Net allocation count after the invocation.
    retained_count: i64,
    /// Net allocated bytes after the invocation.
    retained_bytes: i64,
}

/// Run the complete opt-in characterization and write one sealed TSV payload.
///
/// The result is benchmark evidence only. It neither selects a representation
/// nor admits Package, workspace, runtime, release, or native-effect state.
#[must_use]
pub fn run_package_text_benchmark() -> ExitCode {
    let Some(configuration) = BenchmarkConfiguration::from_environment() else {
        eprintln!(
            "Package-text benchmark skipped; set PALE_BENCH_RUN=1 or PALE_PACKAGE_TEXT_BENCH_SMOKE=1"
        );
        return ExitCode::SUCCESS;
    };
    let actual = ActualCorpus::capture(&configuration.package_root);
    let expected_rows = CASES.len() * OPERATIONS_PER_CASE;
    let mut payload = String::from(
        "source_coordinate\tcorpus_sha256\trun_coordinate\tevidence_class\tcorpus_class\towner_qualified_actual\tpolicy_selected\tacceptance_budget_selected\trelease_evidence\tcase_id\toperation\tscale\tdocuments\tlogical_source_bytes\trepositories\tfanout\theading_transform\ttransformed_headings\tmaterialization_boundary\tlogical_change_bytes\tlogical_output_bytes\texplicit_text_copy_lower_bound_bytes\tmutation_amplification\tpaired_output_set_sha256\tpaired_interpreters_equivalent\tsamples\tp50_ns\tp95_ns\tp99_ns\tmax_ns\tallocation_scope\tallocation_total_count\tallocation_total_bytes\tallocation_peak_relative_count\tallocation_peak_relative_bytes\tallocation_retained_count\tallocation_retained_bytes\trss_scope\tprocess_rss_high_water_bytes\thardware\tos\tkernel\tarch\ttoolchain\tprofile\tallocator\ttiming_allocation_counter\tpgo\tcpu_target\ttarget\tthreads\n",
    );
    let mut row_count = 0_usize;
    for case in CASES {
        let fixture = CaseFixture::new(&actual, case);
        for operation in Operation::ALL {
            let distribution = fixture.observe(operation, configuration.samples);
            let allocations = fixture.allocation_evidence(operation);
            let rss = resident_set_high_water();
            render_row(
                &mut payload,
                &configuration,
                &actual,
                &fixture,
                operation,
                &distribution,
                allocations.as_ref(),
                rss,
            );
            row_count += 1;
        }
    }
    assert_eq!(row_count, expected_rows, "benchmark row count drift");
    let payload_digest = ArtifactDigest::hash_bytes(payload.as_bytes());
    writeln!(
        payload,
        "# pale-package-text-benchmark-seal\trows={row_count}\texpected_rows={expected_rows}\tpayload_sha256={}\tsource_coordinate={}\tcorpus_sha256={}\trun_coordinate={}",
        payload_digest.as_str(),
        configuration.source_coordinate,
        actual.digest.as_str(),
        configuration.run_coordinate,
    )
    .expect("write benchmark seal");
    print!("{payload}");
    ExitCode::SUCCESS
}

/// Write one complete operation row into the in-memory sealed payload.
#[allow(clippy::too_many_arguments)]
fn render_row(
    payload: &mut String,
    configuration: &BenchmarkConfiguration,
    actual: &ActualCorpus,
    fixture: &CaseFixture,
    operation: Operation,
    distribution: &Distribution,
    allocations: Option<&AllocationEvidence>,
    rss: Option<u64>,
) {
    let case = fixture.case;
    let p99 = distribution
        .p99
        .map(|value| value.to_string())
        .unwrap_or_default();
    let allocation_fields = allocations.map_or_else(
        || vec![String::new(); 6],
        |value| {
            vec![
                value.total_count.to_string(),
                value.total_bytes.to_string(),
                value.peak_relative_count.to_string(),
                value.peak_relative_bytes.to_string(),
                value.retained_count.to_string(),
                value.retained_bytes.to_string(),
            ]
        },
    );
    let fields = [
        configuration.source_coordinate.as_str().to_owned(),
        actual.digest.as_str().to_owned(),
        configuration.run_coordinate.as_str().to_owned(),
        configuration.evidence_class().to_owned(),
        case.scale.corpus_class().to_owned(),
        "false".to_owned(),
        "false".to_owned(),
        "false".to_owned(),
        "false".to_owned(),
        case.id.to_owned(),
        operation.as_str().to_owned(),
        case.scale.as_str().to_owned(),
        fixture.corpus.objects.len().to_string(),
        fixture.corpus.bytes.to_string(),
        case.repositories.to_string(),
        case.fanout.to_string(),
        case.headings.as_str().to_owned(),
        fixture.transformed_headings.to_string(),
        case.boundary.as_str().to_owned(),
        "0".to_owned(),
        fixture.logical_output_bytes.to_string(),
        operation
            .explicit_text_copy_lower_bound(fixture)
            .to_string(),
        "not_applicable_immutable".to_owned(),
        fixture.output_digest.as_str().to_owned(),
        "true".to_owned(),
        configuration.samples.to_string(),
        distribution.p50.to_string(),
        distribution.p95.to_string(),
        p99,
        distribution.max.to_string(),
        allocation_scope().to_owned(),
        allocation_fields[0].clone(),
        allocation_fields[1].clone(),
        allocation_fields[2].clone(),
        allocation_fields[3].clone(),
        allocation_fields[4].clone(),
        allocation_fields[5].clone(),
        rss_scope(rss).to_owned(),
        rss.map(|value| value.to_string()).unwrap_or_default(),
        configuration.hardware.clone(),
        std::env::consts::OS.to_owned(),
        configuration.kernel.clone(),
        std::env::consts::ARCH.to_owned(),
        configuration.toolchain.clone(),
        benchmark_profile().to_owned(),
        allocator_coordinate().to_owned(),
        "disabled".to_owned(),
        configuration.pgo.clone(),
        configuration.cpu_target.clone(),
        configuration.target.clone(),
        BENCHMARK_THREADS.to_string(),
    ];
    for field in &fields {
        validate_tsv_field(field);
    }
    writeln!(payload, "{}", fields.join("\t")).expect("write benchmark row");
}

/// Build one recipe per repository in deterministic repository order.
fn build_recipes(
    corpus: &ExpandedCorpus,
    references: &[TextRef],
    case: BenchmarkCase,
) -> Vec<TextRecipe> {
    (0..case.repositories)
        .map(|_| build_repository_recipe(corpus, references, case))
        .collect()
}

/// Build one repository output with the selected semantic fan-out.
fn build_repository_recipe(
    corpus: &ExpandedCorpus,
    references: &[TextRef],
    case: BenchmarkCase,
) -> TextRecipe {
    assert_eq!(corpus.objects.len(), references.len());
    let mut recipe = TextRecipeBuilder::default();
    if matches!(case.boundary, MaterializationBoundary::ManagedBlock) {
        recipe.push_static(MANAGED_BEGIN);
        recipe.push_static("\n");
    }
    for consumer in 0..case.fanout {
        if consumer > 0 {
            recipe.push_static("\n\n");
        }
        for (index, (object, reference)) in corpus.objects.iter().zip(references).enumerate() {
            if index > 0 {
                recipe.push_static("\n\n");
            }
            push_transformed_reference(&mut recipe, reference, object, case.headings);
        }
    }
    if matches!(case.boundary, MaterializationBoundary::ManagedBlock) {
        recipe.push_static("\n");
        recipe.push_static(MANAGED_END);
    }
    recipe.finish()
}

/// Materialize every repository recipe while preserving repository boundaries.
fn materialize_recipes(recipes: &[TextRecipe]) -> Vec<Arc<String>> {
    recipes.iter().map(TextRecipe::materialize).collect()
}

/// Append one source reference while representing heading shifts as explicit parts.
fn push_transformed_reference(
    recipe: &mut TextRecipeBuilder,
    reference: &TextRef,
    object: &ExpandedObject,
    transform: HeadingTransform,
) {
    let mut cursor = 0;
    for (ordinal, &offset) in object.heading_offsets.iter().enumerate() {
        if transform.selects(ordinal) {
            recipe.push_ref(
                reference
                    .slice(cursor..offset)
                    .expect("measured heading offset is a UTF-8 boundary"),
            );
            recipe.push_hashes(1);
            cursor = offset;
        }
    }
    recipe.push_ref(
        reference
            .slice(cursor..reference.len())
            .expect("measured tail is a UTF-8 span"),
    );
}

/// Independently flatten every repository without using [`TextRecipe`].
fn materialize_references(corpus: &ExpandedCorpus, case: BenchmarkCase) -> Vec<String> {
    (0..case.repositories)
        .map(|_| materialize_repository_reference(corpus, case))
        .collect()
}

/// Independently flatten one repository and its selected semantic fan-out.
fn materialize_repository_reference(corpus: &ExpandedCorpus, case: BenchmarkCase) -> String {
    let mut output = String::with_capacity(logical_repository_output_bytes(corpus, case));
    if matches!(case.boundary, MaterializationBoundary::ManagedBlock) {
        output.push_str(MANAGED_BEGIN);
        output.push('\n');
    }
    for consumer in 0..case.fanout {
        if consumer > 0 {
            output.push_str("\n\n");
        }
        for (index, object) in corpus.objects.iter().enumerate() {
            if index > 0 {
                output.push_str("\n\n");
            }
            let mut cursor = 0;
            for (ordinal, &offset) in object.heading_offsets.iter().enumerate() {
                if case.headings.selects(ordinal) {
                    output.push_str(&object.text[cursor..offset]);
                    output.push('#');
                    cursor = offset;
                }
            }
            output.push_str(&object.text[cursor..]);
        }
    }
    if matches!(case.boundary, MaterializationBoundary::ManagedBlock) {
        output.push('\n');
        output.push_str(MANAGED_END);
    }
    assert_eq!(output.len(), logical_repository_output_bytes(corpus, case));
    output
}

/// Compute one repository's output bytes without materializing its text.
fn logical_repository_output_bytes(corpus: &ExpandedCorpus, case: BenchmarkCase) -> usize {
    let within_closure_separators = corpus
        .objects
        .len()
        .saturating_sub(1)
        .checked_mul(2)
        .expect("closure separator byte count overflow");
    let heading_bytes = transformed_heading_count(corpus, case);
    let closure_bytes = corpus
        .bytes
        .checked_add(within_closure_separators)
        .and_then(|value| value.checked_add(heading_bytes))
        .expect("logical closure byte count overflow");
    let consumers = case.fanout;
    let repeated = closure_bytes
        .checked_mul(consumers)
        .and_then(|value| {
            consumers
                .saturating_sub(1)
                .checked_mul(2)
                .and_then(|separators| value.checked_add(separators))
        })
        .expect("logical repeated output byte count overflow");
    match case.boundary {
        MaterializationBoundary::WholeFile => repeated,
        MaterializationBoundary::ManagedBlock => repeated
            .checked_add(MANAGED_BEGIN.len() + 1 + 1 + MANAGED_END.len())
            .expect("managed output byte count overflow"),
    }
}

/// Compute total output bytes across all repository boundaries.
fn logical_output_bytes(corpus: &ExpandedCorpus, case: BenchmarkCase) -> usize {
    let bytes = logical_repository_output_bytes(corpus, case)
        .checked_mul(case.repositories)
        .expect("logical repository-output byte count overflow");
    assert!(
        bytes <= MAX_LOGICAL_OUTPUT_BYTES,
        "Package benchmark output exceeds {MAX_LOGICAL_OUTPUT_BYTES} bytes"
    );
    bytes
}

/// Count inserted heading markers in one source closure.
fn transformed_heading_count(corpus: &ExpandedCorpus, case: BenchmarkCase) -> usize {
    corpus
        .objects
        .iter()
        .map(|object| {
            object
                .heading_offsets
                .iter()
                .enumerate()
                .filter(|(ordinal, _)| case.headings.selects(*ordinal))
                .count()
        })
        .try_fold(0_usize, |total, count| total.checked_add(count))
        .expect("transformed heading count overflow")
}

/// Reject a case before it can construct an unbounded recipe or output.
fn validate_case_bounds(corpus: &ExpandedCorpus, case: BenchmarkCase) {
    assert!(
        case.repositories > 0 && case.fanout > 0,
        "Package benchmark cases require repositories and consumers"
    );
    let transformed_headings = transformed_heading_count(corpus, case);
    let object_parts = corpus
        .objects
        .len()
        .checked_mul(2)
        .expect("recipe part bound overflow");
    let per_consumer_parts = transformed_headings
        .checked_mul(2)
        .and_then(|value| value.checked_add(object_parts))
        .expect("recipe part bound overflow");
    let boundary_parts = if matches!(case.boundary, MaterializationBoundary::ManagedBlock) {
        4
    } else {
        0
    };
    let repository_parts = per_consumer_parts
        .checked_mul(case.fanout)
        .and_then(|value| value.checked_add(case.fanout.saturating_sub(1)))
        .and_then(|value| value.checked_add(boundary_parts))
        .expect("recipe part bound overflow");
    let recipe_parts = repository_parts
        .checked_mul(case.repositories)
        .expect("recipe part bound overflow");
    assert!(
        recipe_parts <= MAX_RECIPE_PARTS,
        "Package benchmark recipe exceeds {MAX_RECIPE_PARTS} parts"
    );
    let _ = logical_output_bytes(corpus, case);
}

/// Digest an ordered repository-output set without copying its text bytes.
fn output_set_digest<'a>(outputs: impl IntoIterator<Item = &'a str>) -> ArtifactDigest {
    let mut preimage = Vec::new();
    preimage.extend_from_slice(OUTPUT_SET_DIGEST_DOMAIN);
    let mut count = 0_u64;
    for output in outputs {
        let digest = ArtifactDigest::hash_bytes(output.as_bytes());
        append_frame(&mut preimage, digest.as_str().as_bytes());
        count = count.checked_add(1).expect("output count overflow");
    }
    preimage.extend_from_slice(&count.to_be_bytes());
    ArtifactDigest::hash_bytes(&preimage)
}

/// Identify CommonMark-style ATX heading starts without reparsing semantic meaning.
fn atx_heading_offsets(text: &str) -> Vec<usize> {
    let mut offsets = Vec::new();
    let mut line_start = 0;
    for line in text.split_inclusive('\n') {
        let bytes = line.as_bytes();
        let markers = bytes.iter().take_while(|&&byte| byte == b'#').count();
        if (1..=6).contains(&markers) && bytes.get(markers) == Some(&b' ') {
            offsets.push(line_start);
        }
        line_start += line.len();
    }
    offsets
}

/// Recursively collect regular Markdown members without following symlinks.
fn collect_markdown_paths(
    root: &Path,
    current: &Path,
    depth: usize,
    tree_entries: &mut usize,
    paths: &mut Vec<PathBuf>,
) {
    assert!(
        depth <= MAX_DIRECTORY_DEPTH,
        "Package benchmark tree is too deep"
    );
    let mut entries = Vec::new();
    for entry in fs::read_dir(current).expect("read Package benchmark directory") {
        *tree_entries = tree_entries
            .checked_add(1)
            .expect("Package benchmark tree-entry count overflow");
        assert!(
            *tree_entries <= MAX_TREE_ENTRIES,
            "Package benchmark tree exceeds {MAX_TREE_ENTRIES} entries"
        );
        entries.push(entry.expect("read Package benchmark directory entry"));
    }
    entries.sort_unstable_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).expect("read Package benchmark member metadata");
        assert!(
            !metadata.file_type().is_symlink(),
            "Package benchmark refuses symlink member {}",
            confined_relative_path(root, &path)
        );
        if metadata.is_dir() {
            collect_markdown_paths(root, &path, depth + 1, tree_entries, paths);
        } else if metadata.is_file() && path.extension().is_some_and(|value| value == "md") {
            paths.push(path);
            assert!(
                paths.len() <= MAX_DOCUMENTS,
                "Package benchmark has too many documents"
            );
        }
    }
}

/// Convert one descendant path into a stable slash-separated report member.
fn confined_relative_path(root: &Path, path: &Path) -> String {
    let relative = path
        .strip_prefix(root)
        .expect("Package benchmark member remains below its root");
    relative
        .components()
        .map(|component| {
            component
                .as_os_str()
                .to_str()
                .expect("Package benchmark paths must be UTF-8")
        })
        .collect::<Vec<_>>()
        .join("/")
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
            panic!("full Package-text characterization requires {name}")
        }
        Err(std::env::VarError::NotUnicode(_)) => panic!("{name} must be UTF-8"),
    };
    assert!(!value.is_empty(), "{name} must not be empty");
    validate_tsv_field(&value);
    value
}

/// Reject report fields that could escape their bounded TSV column.
///
/// Empty fields represent deliberately unavailable optional evidence such as
/// p99 in a smoke run; required coordinates reject emptiness at their source.
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
        .expect("rustc must be available to identify a benchmark run");
    assert!(output.status.success(), "rustc -vV must succeed");
    let output = String::from_utf8(output.stdout).expect("rustc -vV output must be UTF-8");
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

/// Exact allocation-instrument scope carried by every row.
const fn allocation_scope() -> &'static str {
    if cfg!(feature = "bench-memory") {
        "current_thread_single_operation_rust_allocator_only"
    } else {
        "unavailable_feature_disabled"
    }
}

/// Exact allocator coordinate carried by every row.
const fn allocator_coordinate() -> &'static str {
    if cfg!(feature = "bench-memory") {
        "system_with_allocation-counter-0.8.1"
    } else {
        "system_uninstrumented"
    }
}

/// Measure one current-thread operation when allocation instrumentation is enabled.
#[cfg(feature = "bench-memory")]
fn measure_allocations(run: impl FnOnce()) -> Option<AllocationEvidence> {
    let observed = allocation_counter::measure(run);
    Some(AllocationEvidence {
        total_count: observed.count_total,
        total_bytes: observed.bytes_total,
        peak_relative_count: observed.count_max,
        peak_relative_bytes: observed.bytes_max,
        retained_count: observed.count_current,
        retained_bytes: observed.bytes_current,
    })
}

/// Represent unavailable allocation evidence without fabricating zeros.
#[cfg(not(feature = "bench-memory"))]
fn measure_allocations(_run: impl FnOnce()) -> Option<AllocationEvidence> {
    None
}

/// Time without counter bookkeeping; the benchmark-only allocator wrapper remains.
fn without_allocation_counting(run: impl FnOnce()) {
    #[cfg(feature = "bench-memory")]
    allocation_counter::opt_out(run);
    #[cfg(not(feature = "bench-memory"))]
    run();
}

/// Exact process RSS scope carried by every row.
const fn rss_scope(observed: Option<u64>) -> &'static str {
    if observed.is_some() {
        "process_lifetime_high_water_not_operation_local"
    } else {
        "unavailable_feature_platform_or_observation"
    }
}

/// Read the process-lifetime RSS high-water mark through the safe native wrapper.
#[cfg(all(
    feature = "bench-memory",
    any(target_os = "macos", target_os = "linux")
))]
fn resident_set_high_water() -> Option<u64> {
    use nix::sys::resource::{getrusage, UsageWho};

    rss_bytes(getrusage(UsageWho::RUSAGE_SELF).ok()?.max_rss())
}

/// Normalize verified macOS bytes or Linux KiB; invalid observations are unavailable.
#[cfg(all(
    feature = "bench-memory",
    any(target_os = "macos", target_os = "linux")
))]
fn rss_bytes(value: std::ffi::c_long) -> Option<u64> {
    let value = u64::try_from(value).ok()?;
    if cfg!(target_os = "macos") {
        Some(value)
    } else {
        value.checked_mul(1024)
    }
}

/// Represent unavailable RSS evidence without fabricating a zero measurement.
#[cfg(not(all(
    feature = "bench-memory",
    any(target_os = "macos", target_os = "linux")
)))]
fn resident_set_high_water() -> Option<u64> {
    None
}

#[cfg(test)]
#[path = "../../tests/internal/packages/text_benchmark.rs"]
mod tests;
