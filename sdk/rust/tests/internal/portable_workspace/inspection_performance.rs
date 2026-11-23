//! Opt-in measurements of warmed queries on actual retained compiler fixtures.
//!
//! The application rows consume lazy dependency lookups without collecting them.
//! Parity projections and fixture construction occur outside measurements.

use super::{
    projection, PortableWorkspaceCausalStep, PortableWorkspaceCompilation,
    PortableWorkspaceDependencyCursor, PortableWorkspaceDependencyPage, PortableWorkspaceDocument,
    PortableWorkspaceRequest, PortableWorkspaceSubject, PortableWorkspaceSubjectInspection,
    PortableWorkspaceSubjectInspectionCursor, PortableWorkspaceSubjectSelector,
};
use crate::{
    AuthoringInstancePathSegment, AuthoringNavigationRequest, AuthoringOccurrence,
    AuthoringOccurrencePage, AuthoringOccurrencePageCursor, AuthoringSourceLocation, PaleApi,
    PortableWorkspaceSourceSubject, PortableWorkspaceSubjectOccurrences,
};
use pale_application::workspace as app;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{hint::black_box, sync::Arc};

#[path = "../../support/portable_workspace_inputs.rs"]
mod inputs;

#[path = "inspection_performance/direct_dependents.rs"]
mod direct_dependents;

struct Case {
    name: &'static str,
    input_digest: String,
    owner: Arc<PortableWorkspaceCompilation>,
    subject: PortableWorkspaceSubject,
    limits: (u16, u16, u16),
}

fn compile(request: PortableWorkspaceRequest) -> (Arc<PortableWorkspaceCompilation>, String) {
    let mut digest = Sha256::new();
    digest.update(b"pale.inspection-benchmark-inputs.v1\0");
    let mut part = |bytes: &[u8]| {
        digest.update(u64::try_from(bytes.len()).unwrap().to_be_bytes());
        digest.update(bytes);
    };
    part(request.root_manifest.as_bytes());
    for document in &request.documents {
        part(document.label.as_bytes());
        part(&document.bytes);
    }
    let identity = format!("{:x}", digest.finalize());
    let owner = PaleApi::new()
        .unwrap()
        .compile_portable_workspace(request)
        .unwrap();
    (Arc::new(owner), identity)
}

fn cases() -> Vec<Case> {
    let (empty, empty_digest) = compile(inputs::empty_request());
    let (unadopted, unadopted_digest) = compile(PortableWorkspaceRequest::new(
        "pale-workspace.toml".into(),
        inputs::unadopted_documents("alpha.toml"),
    ));
    let source = unadopted
        .source_subject(AuthoringNavigationRequest::new(
            "alpha.toml".into(),
            vec![
                AuthoringInstancePathSegment::Field("declarations".into()),
                AuthoringInstancePathSegment::Index(0),
                AuthoringInstancePathSegment::Field("id".into()),
            ],
        ))
        .unwrap();
    let PortableWorkspaceSourceSubject::Unselected { subject, .. } = source.result() else {
        panic!("existing fixture contains an actually unadopted declaration")
    };
    let unadopted_subject = subject.clone();
    let long_utf8_label = format!("{}.toml", "é".repeat(1_900));
    let (long_utf8, long_utf8_digest) = compile(PortableWorkspaceRequest::new(
        "pale-workspace.toml".into(),
        inputs::unadopted_documents(&long_utf8_label),
    ));
    let long_utf8_source = long_utf8
        .source_subject(AuthoringNavigationRequest::new(
            long_utf8_label,
            vec![
                AuthoringInstancePathSegment::Field("declarations".into()),
                AuthoringInstancePathSegment::Index(0),
                AuthoringInstancePathSegment::Field("id".into()),
            ],
        ))
        .unwrap();
    let PortableWorkspaceSourceSubject::Unselected {
        subject: long_utf8_subject,
        ..
    } = long_utf8_source.result()
    else {
        panic!("long UTF-8 fixture contains an actually unadopted declaration")
    };
    let long_utf8_subject = long_utf8_subject.clone();
    let (wide, wide_digest) = compile(inputs::wide_dependency_request());
    let selected = |selector| PortableWorkspaceSubject::try_from(selector).unwrap();
    vec![
        Case {
            name: "mapped-root",
            input_digest: empty_digest.clone(),
            owner: empty.clone(),
            subject: selected(PortableWorkspaceSubjectSelector::Source {
                source_id: "workspace-manifest".into(),
            }),
            limits: (0, 0, 0),
        },
        Case {
            name: "lock-no-named-source",
            input_digest: empty_digest,
            owner: empty,
            subject: selected(PortableWorkspaceSubjectSelector::UniverseLock),
            limits: (0, 0, 0),
        },
        Case {
            name: "unadopted-declaration",
            input_digest: unadopted_digest.clone(),
            owner: unadopted.clone(),
            subject: unadopted_subject,
            limits: (0, 0, 0),
        },
        Case {
            name: "absent-subject",
            input_digest: unadopted_digest,
            owner: unadopted,
            subject: selected(PortableWorkspaceSubjectSelector::Source {
                source_id: "absent-source".into(),
            }),
            limits: (0, 0, 0),
        },
        Case {
            name: "long-utf8-source-label",
            input_digest: long_utf8_digest,
            owner: long_utf8,
            subject: long_utf8_subject,
            limits: (0, 0, 0),
        },
        Case {
            name: "wide-defaults",
            input_digest: wide_digest.clone(),
            owner: wide.clone(),
            subject: selected(PortableWorkspaceSubjectSelector::ResponsibilityClosure),
            limits: (0, 0, 0),
        },
        Case {
            name: "wide-sparse",
            input_digest: wide_digest,
            owner: wide,
            subject: selected(PortableWorkspaceSubjectSelector::ResponsibilityClosure),
            limits: (256, 16, 1),
        },
    ]
}

#[derive(Clone, Copy, Debug)]
enum Stage {
    ApplicationPrimitives,
    ApplicationComposition,
    SdkPrimitives,
    SdkComposition,
}

const STAGES: [Stage; 4] = [
    Stage::ApplicationPrimitives,
    Stage::ApplicationComposition,
    Stage::SdkPrimitives,
    Stage::SdkComposition,
];

// Independent positions deliberately use the real owners' opaque cursors. The
// consumed offset permits an application primitive root-only query at the tail.
#[derive(Clone, Default)]
struct Position {
    input_offset: usize,
    inputs_done: bool,
    sources_done: bool,
    app_source: Option<app::AuthoringOccurrencePageCursor>,
    app_composed: Option<app::PortableWorkspaceSubjectInspectionCursor>,
    sdk_input: Option<PortableWorkspaceDependencyCursor>,
    sdk_source: Option<AuthoringOccurrencePageCursor>,
    sdk_composed: Option<PortableWorkspaceSubjectInspectionCursor>,
}

struct ApplicationPair<'a> {
    dependencies: Option<app::PortableWorkspaceDependencyPage<'a>>,
    sources: Option<app::AuthoringOccurrencePage<'a>>,
    anchor: Option<&'a app::WorkspaceAuthoringLocation>,
}

struct SdkPair {
    dependencies: Option<PortableWorkspaceDependencyPage>,
    sources: Option<AuthoringOccurrencePage>,
}

enum Output<'a> {
    ApplicationPair(ApplicationPair<'a>),
    ApplicationComposition(app::PortableWorkspaceSubjectInspection<'a>),
    SdkPair(SdkPair),
    SdkComposition(PortableWorkspaceSubjectInspection),
}

fn app_sources(
    page: app::PortableWorkspaceSubjectOccurrences<'_>,
) -> app::AuthoringOccurrencePage<'_> {
    match page {
        app::PortableWorkspaceSubjectOccurrences::Selected(page)
        | app::PortableWorkspaceSubjectOccurrences::Unselected(page) => page,
    }
}

fn sdk_sources(page: PortableWorkspaceSubjectOccurrences) -> AuthoringOccurrencePage {
    match page {
        PortableWorkspaceSubjectOccurrences::Selected(page)
        | PortableWorkspaceSubjectOccurrences::Unselected(page) => page,
    }
}

fn consume(page: Option<&app::PortableWorkspaceDependencyPage<'_>>) {
    if let Some(page) = page {
        for node in page.dependencies() {
            black_box(node);
        }
    }
}

fn query<'a>(case: &'a Case, stage: Stage, before: &Position) -> (Output<'a>, Position) {
    let (d, i, s) = case.limits;
    let owner = case.owner.application_universe();
    let subject = case.subject.application_subject();
    let mut next = Position::default();
    let output = match stage {
        Stage::ApplicationPrimitives => {
            // Once done, offset is the exact consumed tail, not a restart.
            let dependencies = owner
                .direct_dependencies(subject, before.input_offset, d)
                .unwrap();
            consume(dependencies.as_ref());
            next.input_offset =
                before.input_offset + dependencies.as_ref().map_or(0, |p| p.dependencies().len());
            next.inputs_done = dependencies
                .as_ref()
                .is_none_or(|p| p.next_offset().is_none());
            let sources = (!before.sources_done).then(|| {
                app_sources(owner.subject_occurrences(
                    subject,
                    before.app_source.as_ref(),
                    usize::from(i),
                    usize::from(s),
                ))
            });
            next.app_source = sources.as_ref().and_then(|p| p.continuation()).cloned();
            next.sources_done = next.app_source.is_none();
            Output::ApplicationPair(ApplicationPair {
                dependencies,
                sources,
                anchor: owner.subject_record_anchor(subject),
            })
        }
        Stage::ApplicationComposition => {
            let page = owner
                .inspect_subject(
                    subject,
                    before.app_composed.as_ref(),
                    d,
                    usize::from(i),
                    usize::from(s),
                )
                .unwrap();
            consume(page.dependency_page());
            next.inputs_done = page.inputs_complete();
            next.sources_done = page.sources_complete();
            next.app_composed = page.continuation().cloned();
            Output::ApplicationComposition(page)
        }
        Stage::SdkPrimitives => {
            let dependencies = (!before.inputs_done).then(|| {
                case.owner
                    .direct_dependencies(&case.subject, before.sdk_input.as_ref(), d)
                    .unwrap()
            });
            next.sdk_input = dependencies
                .as_ref()
                .and_then(|p| p.continuation())
                .cloned();
            next.inputs_done = next.sdk_input.is_none();
            let sources = (!before.sources_done).then(|| {
                sdk_sources(
                    case.owner
                        .subject_occurrences(&case.subject, before.sdk_source.as_ref(), i, s)
                        .unwrap(),
                )
            });
            next.sdk_source = sources.as_ref().and_then(|p| p.continuation()).cloned();
            next.sources_done = next.sdk_source.is_none();
            Output::SdkPair(SdkPair {
                dependencies,
                sources,
            })
        }
        Stage::SdkComposition => {
            let page = case
                .owner
                .inspect_subject(&case.subject, before.sdk_composed.as_ref(), d, i, s)
                .unwrap();
            next.inputs_done = page.inputs_complete();
            next.sources_done = page.sources_complete();
            next.sdk_composed = page.continuation().cloned();
            Output::SdkComposition(page)
        }
    };
    (output, next)
}

#[derive(Debug, PartialEq, Eq)]
struct Evidence {
    root: Option<PortableWorkspaceCausalStep>,
    inputs: Vec<PortableWorkspaceCausalStep>,
    sources: Vec<AuthoringOccurrence>,
    inspected: usize,
    anchor: Option<AuthoringSourceLocation>,
    inputs_done: bool,
    sources_done: bool,
}

// This owned comparison is never inside a measurement. The primitive SDK pair
// has no anchor/header query or repeated root after input exhaustion: use the
// prepared root only for semantic equality, not for cost or projection counts.
fn evidence(
    output: &Output<'_>,
    next: &Position,
    prior_root: Option<&PortableWorkspaceCausalStep>,
) -> Evidence {
    let mut result = Evidence {
        root: None,
        inputs: vec![],
        sources: vec![],
        inspected: 0,
        anchor: None,
        inputs_done: next.inputs_done,
        sources_done: next.sources_done,
    };
    match output {
        Output::ApplicationPair(pair) => {
            if let Some(page) = &pair.dependencies {
                result.root = Some(PortableWorkspaceCausalStep::from_internal(page.root()));
                result.inputs = page
                    .dependencies()
                    .map(PortableWorkspaceCausalStep::from_internal)
                    .collect();
            }
            if let Some(page) = &pair.sources {
                result.sources = page
                    .occurrences()
                    .iter()
                    .map(|p| AuthoringOccurrence::from_application(p))
                    .collect();
                result.inspected = page.inspected();
            }
            result.anchor = pair.anchor.map(AuthoringSourceLocation::from_application);
        }
        Output::ApplicationComposition(page) => {
            result.root = page.root().map(PortableWorkspaceCausalStep::from_internal);
            if let Some(inputs) = page.dependency_page() {
                result.inputs = inputs
                    .dependencies()
                    .map(PortableWorkspaceCausalStep::from_internal)
                    .collect();
            }
            if let Some(sources) = page.source_page() {
                result.sources = sources
                    .occurrences()
                    .iter()
                    .map(|p| AuthoringOccurrence::from_application(p))
                    .collect();
                result.inspected = sources.inspected();
            }
            result.anchor = page
                .record_anchor()
                .map(AuthoringSourceLocation::from_application);
        }
        Output::SdkPair(pair) => {
            result.root = pair
                .dependencies
                .as_ref()
                .and_then(|p| p.root())
                .or(prior_root)
                .cloned();
            if let Some(page) = &pair.dependencies {
                result.inputs = page.dependencies().to_vec();
            }
            if let Some(page) = &pair.sources {
                result.sources = page.occurrences().to_vec();
                result.inspected = usize::from(page.inspected());
            }
        }
        Output::SdkComposition(page) => {
            result.root = page.root().cloned();
            result.inputs = page.dependencies().to_vec();
            result.sources = page.occurrences().to_vec();
            result.inspected = usize::from(page.inspected());
            result.anchor = page.record_anchor().cloned();
        }
    }
    result
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Counts {
    pages: usize,
    roots: usize,
    inputs: usize,
    inspected: usize,
    sources: usize,
    dependency_copied_string_bytes: usize,
    source_copied_string_bytes: usize,
    canonical_bytes: usize,
}

fn counts(output: &Output<'_>) -> Counts {
    let mut c = Counts {
        pages: 1,
        ..Counts::default()
    };
    match output {
        Output::ApplicationPair(pair) => {
            if let Some(p) = &pair.dependencies {
                c.roots = 1;
                c.inputs = p.dependencies().len();
                c.dependency_copied_string_bytes = p.copied_string_bytes();
            }
            if let Some(p) = &pair.sources {
                c.inspected = p.inspected();
                c.sources = p.occurrences().len();
                c.source_copied_string_bytes = p.copied_string_bytes();
            }
            if let Some(anchor) = pair.anchor {
                c.source_copied_string_bytes += anchor.source_label().len()
                    + anchor
                        .path()
                        .iter()
                        .filter_map(|segment| match segment {
                            app::AuthoringInstancePathSegment::Field(field) => Some(field.len()),
                            app::AuthoringInstancePathSegment::Index(_) => None,
                        })
                        .sum::<usize>();
            }
        }
        Output::ApplicationComposition(p) => {
            c.roots = usize::from(p.root().is_some());
            c.inputs = p.dependency_page().map_or(0, |p| p.dependencies().len());
            c.dependency_copied_string_bytes = p.dependency_copied_string_bytes();
            c.source_copied_string_bytes = p.source_copied_string_bytes();
            if let Some(p) = p.source_page() {
                c.inspected = p.inspected();
                c.sources = p.occurrences().len();
            }
        }
        Output::SdkPair(pair) => {
            if let Some(p) = &pair.dependencies {
                c.roots = usize::from(p.root().is_some());
                c.inputs = p.dependencies().len();
                c.canonical_bytes = p.canonical_bytes().len();
                c.dependency_copied_string_bytes = p.copied_string_bytes();
            }
            if let Some(p) = &pair.sources {
                c.inspected = usize::from(p.inspected());
                c.sources = p.occurrences().len();
                c.source_copied_string_bytes = usize::try_from(p.copied_string_bytes()).unwrap();
            }
        }
        Output::SdkComposition(p) => {
            c.roots = usize::from(p.root().is_some());
            c.inputs = p.dependencies().len();
            c.inspected = usize::from(p.inspected());
            c.sources = p.occurrences().len();
            c.dependency_copied_string_bytes =
                usize::try_from(p.dependency_copied_string_bytes()).unwrap();
            c.source_copied_string_bytes = usize::try_from(p.source_copied_string_bytes()).unwrap();
            c.canonical_bytes = p.canonical_bytes().len();
        }
    }
    c
}

impl Counts {
    fn add(&mut self, other: Self) {
        self.pages += other.pages;
        self.roots += other.roots;
        self.inputs += other.inputs;
        self.inspected += other.inspected;
        self.sources += other.sources;
        self.dependency_copied_string_bytes += other.dependency_copied_string_bytes;
        self.source_copied_string_bytes += other.source_copied_string_bytes;
        self.canonical_bytes += other.canonical_bytes;
    }
}

struct Prepared {
    positions: Vec<(usize, [Position; 4])>,
    totals: [Counts; 4],
}

fn prepare(case: &Case) -> Prepared {
    let mut positions: [Position; 4] = std::array::from_fn(|_| Position::default());
    let mut selected = Vec::new();
    let mut totals = [Counts::default(); 4];
    let mut root = None;
    for index in 0..100_000 {
        let before = positions.clone();
        let mut expected = None;
        for (slot, stage) in STAGES.into_iter().enumerate() {
            let (output, next) = query(case, stage, &positions[slot]);
            let actual = evidence(&output, &next, root.as_ref());
            if slot == 0 {
                root = actual.root.clone();
            }
            if let Some(expected) = &expected {
                assert_eq!(&actual, expected, "{} page {index} {stage:?}", case.name);
            } else {
                expected = Some(actual);
            }
            totals[slot].add(counts(&output));
            positions[slot] = next;
        }
        let terminal = positions[0].inputs_done && positions[0].sources_done;
        // Retain only first, next and terminal incoming positions; no journey-sized
        // fixture allocation is needed for the single-call cases.
        if index < 2 || terminal {
            selected.push((index, before));
        }
        if terminal {
            for total in &totals[1..] {
                assert_eq!(
                    (
                        total.pages,
                        total.inputs,
                        total.inspected,
                        total.sources,
                        total.dependency_copied_string_bytes,
                        total.source_copied_string_bytes,
                    ),
                    (
                        totals[0].pages,
                        totals[0].inputs,
                        totals[0].inspected,
                        totals[0].sources,
                        totals[0].dependency_copied_string_bytes,
                        totals[0].source_copied_string_bytes,
                    )
                );
            }
            return Prepared {
                positions: selected,
                totals,
            };
        }
    }
    panic!("fixed fixture journey did not terminate");
}

fn journey(case: &Case, stage: Stage) -> Counts {
    let mut position = Position::default();
    let mut total = Counts::default();
    loop {
        let (output, next) = query(case, stage, &position);
        total.add(counts(&output));
        position = next;
        black_box(&output);
        drop(output);
        if position.inputs_done && position.sources_done {
            return total;
        }
    }
}

fn observations() -> usize {
    if std::env::var_os("PALE_INSPECTION_BENCH_SMOKE").is_some() {
        5
    } else {
        201
    }
}

fn run_header() {
    let executable = std::env::current_exe().unwrap();
    let executable_hash = format!("{:x}", Sha256::digest(std::fs::read(executable).unwrap()));
    let metadata: Vec<_> = [
        "SOURCE",
        "HARDWARE",
        "KERNEL",
        "TOOLCHAIN",
        "CPU_TARGET",
        "PGO",
        "RUN",
    ]
    .into_iter()
    .map(|key| {
        let name = format!("PALE_INSPECTION_BENCH_{key}");
        (
            key,
            std::env::var(name).expect("explicit benchmark provenance is required"),
        )
    })
    .collect();
    report(
        json!({"metadata":metadata,"executableSha256":executable_hash,
        "observations":observations(),"allocationFeature":cfg!(feature="bench-memory"),
        "scope":"warmed query harness including outgoing cursor copies and Position; no authority or workload-generalization claim"}),
    );
}

fn report(value: serde_json::Value) {
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    writeln!(out, "INSPECTION_BENCH_BEGIN").unwrap();
    serde_json::to_writer_pretty(&mut out, &value).unwrap();
    writeln!(out, "\nINSPECTION_BENCH_END").unwrap();
}

fn row(case: &Case, stage: &str, page: &str, counts: Option<Counts>, result: serde_json::Value) {
    report(json!({"case":case.name,"inputSha256":case.input_digest,
        "subject":format!("{:?}",case.subject),"rawLimits":case.limits,"stage":stage,"page":page,
        "counts":counts,"observations":result}));
}

fn attribution(
    case: &Case,
    stage: &str,
    prepared_bytes: usize,
    encoded_bytes: Option<usize>,
    mut result: serde_json::Value,
) {
    result["preparedCanonicalBytes"] = json!(prepared_bytes);
    result["encodedOutputBytes"] = json!(encoded_bytes);
    row(case, stage, "prepared-first-answer", None, result);
}

#[cfg(not(feature = "bench-memory"))]
fn measure<T>(mut run: impl FnMut() -> T) -> serde_json::Value {
    for _ in 0..3 {
        black_box(run());
    }
    let mut samples = Vec::with_capacity(observations());
    for _ in 0..observations() {
        let start = std::time::Instant::now();
        let output = run();
        samples.push(start.elapsed().as_nanos());
        black_box(output); // output destruction is outside the recorded interval
    }
    let mut sorted = samples.clone();
    sorted.sort_unstable();
    let percentile = |percent: usize| sorted[(sorted.len() * percent).div_ceil(100) - 1];
    json!({"unit":"nanoseconds","raw":samples,"p50":percentile(50),"p95":percentile(95),
        "p99":percentile(99),"max":sorted.last()})
}

#[cfg(feature = "bench-memory")]
fn measure<T>(mut run: impl FnMut() -> T) -> serde_json::Value {
    for _ in 0..3 {
        black_box(run());
    }
    let mut samples = Vec::with_capacity(observations());
    let mut retained_samples = Vec::with_capacity(observations());
    let mut dropped_samples = Vec::with_capacity(observations());
    for _ in 0..observations() {
        let mut retained = None;
        let live = allocation_counter::measure(|| {
            retained = Some(black_box(run()));
        });
        drop(retained);
        let dropped = allocation_counter::measure(|| {
            drop(black_box(run()));
        });
        let fields = |v: allocation_counter::AllocationInfo| {
            json!({
            "allocations":v.count_total,"bytes":v.bytes_total,"peakAllocations":v.count_max,
            "peakBytes":v.bytes_max,"netAllocations":v.count_current,"netBytes":v.bytes_current})
        };
        samples.push(json!({"retained":fields(live),"createAndDrop":fields(dropped)}));
        retained_samples.push(live);
        dropped_samples.push(dropped);
    }
    let percentiles = |values: Vec<u64>| {
        let mut values = values;
        values.sort_unstable();
        let percentile = |percent: usize| values[(values.len() * percent).div_ceil(100) - 1];
        json!({"p50":percentile(50),"p95":percentile(95),"p99":percentile(99)})
    };
    let summary = |values: &[allocation_counter::AllocationInfo]| {
        json!({
            "allocations":percentiles(values.iter().map(|sample| sample.count_total).collect()),
            "allocatedBytes":percentiles(values.iter().map(|sample| sample.bytes_total).collect()),
            "peakAllocations":percentiles(values.iter().map(|sample| sample.count_max).collect()),
            "peakBytes":percentiles(values.iter().map(|sample| sample.bytes_max).collect()),
        })
    };
    json!({"scope":"current thread allocations; relative peaks, not RSS or total process memory",
        "summary":{"retained":summary(&retained_samples),"createAndDrop":summary(&dropped_samples)},
        "raw":samples})
}

fn run() {
    run_header();
    for case in cases() {
        let prepared = prepare(&case);
        for (slot, stage) in STAGES.into_iter().enumerate() {
            for (index, positions) in &prepared.positions {
                let position = positions[slot].clone(); // outside every measured call
                let (output, next) = query(&case, stage, &position);
                let expected_counts = counts(&output);
                let anchor_present = evidence(&output, &next, None).anchor.is_some();
                drop(output);
                let mut result = measure(|| query(&case, stage, &position));
                result["inputsComplete"] = json!(next.inputs_done);
                result["sourcesComplete"] = json!(next.sources_done);
                result["anchorPresent"] = json!(anchor_present);
                row(
                    &case,
                    &format!("{stage:?}"),
                    &format!("page-{index}"),
                    Some(expected_counts),
                    result,
                );
            }
            assert_eq!(journey(&case, stage), prepared.totals[slot]);
            let result = measure(|| journey(&case, stage));
            row(
                &case,
                &format!("{stage:?}"),
                "whole-journey-consume-and-drop",
                Some(prepared.totals[slot]),
                result,
            );
        }
        let page = case
            .owner
            .inspect_subject(
                &case.subject,
                None,
                case.limits.0,
                case.limits.1,
                case.limits.2,
            )
            .unwrap();
        let value = pale_core::canon::parse_strict(page.canonical_bytes()).unwrap();
        assert_eq!(
            projection::subject_inspection(&page),
            page.canonical_bytes()
        );
        assert_eq!(
            pale_core::canon::to_canonical_string(&value)
                .unwrap()
                .as_bytes(),
            page.canonical_bytes()
        );
        let canonical_bytes = page.canonical_bytes().len();
        attribution(
            &case,
            "whole-sdk-projection",
            canonical_bytes,
            Some(canonical_bytes),
            measure(|| projection::subject_inspection(&page)),
        );
        attribution(
            &case,
            "prepared-value-canonicalize-and-validate",
            canonical_bytes,
            Some(canonical_bytes),
            measure(|| pale_core::canon::to_canonical_string(&value).unwrap()),
        );
        attribution(
            &case,
            "prepared-bytes-strict-parse",
            canonical_bytes,
            None,
            measure(|| pale_core::canon::parse_strict(page.canonical_bytes()).unwrap()),
        );
        let borrowed = case
            .owner
            .application_universe()
            .inspect_subject(
                case.subject.application_subject(),
                None,
                case.limits.0,
                usize::from(case.limits.1),
                usize::from(case.limits.2),
            )
            .unwrap();
        if let Some(root) = borrowed.root() {
            assert_eq!(
                Some(PortableWorkspaceCausalStep::from_internal(root)),
                page.root().cloned()
            );
            let mut result = measure(|| PortableWorkspaceCausalStep::from_internal(root));
            result["mappedRoots"] = json!(1);
            row(
                &case,
                "one-prepared-root-mapping",
                "prepared-first-answer",
                None,
                result,
            );
        }
    }
}

#[test]
fn inspection_workloads_preserve_query_parity() {
    for case in cases() {
        let prepared = prepare(&case);
        for (slot, stage) in STAGES.into_iter().enumerate() {
            assert_eq!(journey(&case, stage), prepared.totals[slot]);
        }
    }
}

#[cfg(not(feature = "bench-memory"))]
#[test]
#[ignore = "explicit optimized inspection latency characterization"]
fn inspection_latency() {
    run();
}

#[cfg(feature = "bench-memory")]
#[test]
#[ignore = "explicit optimized current-thread allocation characterization"]
fn inspection_allocations() {
    run();
}
