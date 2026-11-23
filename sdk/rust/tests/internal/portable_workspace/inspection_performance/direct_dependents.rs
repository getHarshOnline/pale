//! Paired direct-consumer measurements on the existing portable input fixtures.

use super::{
    app, black_box, compile, inputs, json, measure, report, run_header, Arc,
    AuthoringNavigationRequest, PaleApi, PortableWorkspaceCausalStep, PortableWorkspaceCompilation,
    PortableWorkspaceSourceSubject, PortableWorkspaceSubject, PortableWorkspaceSubjectSelector,
};
use crate::{PortableWorkspaceDependentCursor, PortableWorkspaceDependentPage};

struct Case {
    name: &'static str,
    owner: Arc<PortableWorkspaceCompilation>,
    input_digest: String,
    subject: PortableWorkspaceSubject,
    limits: (u16, u16),
    expected_consumers: usize,
    wide: bool,
}

fn selected_source(owner: &PortableWorkspaceCompilation, label: &str) -> PortableWorkspaceSubject {
    let inspection = owner
        .source_subject(AuthoringNavigationRequest::new(label.into(), Vec::new()))
        .unwrap();
    let PortableWorkspaceSourceSubject::Selected { subject, .. } = inspection.result() else {
        panic!("the existing captured source must be selected")
    };
    subject.clone()
}

fn cases() -> Vec<Case> {
    let (minimal, minimal_digest) = compile(inputs::empty_request());
    let minimal_source = selected_source(&minimal, "pale-workspace.toml");
    let (wide, wide_digest) = compile(inputs::wide_dependency_request());
    let sparse_source = selected_source(&wide, "responsibility-0.toml");
    let workspace =
        PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Workspace {
            workspace_id: wide.workspace_id().into(),
        })
        .unwrap();
    vec![
        Case {
            name: "minimal-source-consumer",
            owner: minimal,
            input_digest: minimal_digest,
            subject: minimal_source,
            limits: (1, 1),
            expected_consumers: 4,
            wide: false,
        },
        Case {
            name: "wide-terminal-defaults",
            owner: wide.clone(),
            input_digest: wide_digest.clone(),
            subject: workspace.clone(),
            limits: (0, 0),
            expected_consumers: 0,
            wide: true,
        },
        Case {
            name: "wide-terminal-capped",
            owner: wide.clone(),
            input_digest: wide_digest.clone(),
            subject: workspace,
            limits: (u16::MAX, u16::MAX),
            expected_consumers: 0,
            wide: true,
        },
        Case {
            name: "wide-sparse-source",
            owner: wide.clone(),
            input_digest: wide_digest.clone(),
            subject: sparse_source,
            limits: (16, 1),
            expected_consumers: 1,
            wide: true,
        },
        Case {
            name: "wide-missing-subject",
            owner: wide,
            input_digest: wide_digest,
            subject: PortableWorkspaceSubject::try_from(PortableWorkspaceSubjectSelector::Source {
                source_id: "absent-source".into(),
            })
            .unwrap(),
            limits: (1, 1),
            expected_consumers: 0,
            wide: true,
        },
    ]
}

#[derive(Clone, Copy, Debug)]
enum Stage {
    Application,
    Sdk,
}

const STAGES: [Stage; 2] = [Stage::Application, Stage::Sdk];

#[derive(Clone, Default)]
struct Position {
    offset: usize,
    cursor: Option<PortableWorkspaceDependentCursor>,
    complete: bool,
}

fn app_query<'a>(
    case: &'a Case,
    before: &Position,
) -> (Option<app::PortableWorkspaceDependentPage<'a>>, Position) {
    let page = case
        .owner
        .application_universe()
        .direct_dependents(
            case.subject.application_subject(),
            before.offset,
            case.limits.0,
            case.limits.1,
        )
        .unwrap();
    if let Some(page) = &page {
        black_box(page.root());
        for node in page.dependents() {
            black_box(node);
        }
    }
    let next = Position {
        offset: before.offset + page.as_ref().map_or(0, |page| page.inspected()),
        complete: page
            .as_ref()
            .is_none_or(|page| page.next_offset().is_none()),
        cursor: None,
    };
    (page, next)
}

fn sdk_query(case: &Case, before: &Position) -> (PortableWorkspaceDependentPage, Position) {
    let page = case
        .owner
        .direct_dependents(
            &case.subject,
            before.cursor.as_ref(),
            case.limits.0,
            case.limits.1,
        )
        .unwrap();
    black_box(page.root());
    for node in page.dependents() {
        black_box(node);
    }
    black_box(page.canonical_bytes());
    let next = Position {
        offset: before.offset + page.inspected(),
        cursor: page.continuation().cloned(),
        complete: page.is_complete(),
    };
    (page, next)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Counts {
    pages: usize,
    roots: usize,
    dependents: usize,
    inspected: usize,
    input_digest_comparisons: usize,
    canonical_bytes: Option<usize>,
}

impl Counts {
    fn add(&mut self, other: Self) {
        self.pages += other.pages;
        self.roots += other.roots;
        self.dependents += other.dependents;
        self.inspected += other.inspected;
        self.input_digest_comparisons += other.input_digest_comparisons;
        if let Some(bytes) = other.canonical_bytes {
            *self.canonical_bytes.get_or_insert(0) += bytes;
        }
    }

    fn without_bytes(mut self) -> Self {
        self.canonical_bytes = None;
        self
    }
}

fn app_counts(page: Option<&app::PortableWorkspaceDependentPage<'_>>) -> Counts {
    Counts {
        pages: 1,
        roots: usize::from(page.is_some()),
        dependents: page.map_or(0, |page| page.dependents().len()),
        inspected: page.map_or(0, |page| page.inspected()),
        input_digest_comparisons: page.map_or(0, |page| page.input_digest_comparisons()),
        canonical_bytes: None,
    }
}

fn sdk_counts(page: &PortableWorkspaceDependentPage) -> Counts {
    Counts {
        pages: 1,
        roots: usize::from(page.root().is_some()),
        dependents: page.dependents().len(),
        inspected: page.inspected(),
        input_digest_comparisons: page.input_digest_comparisons(),
        canonical_bytes: Some(page.canonical_bytes().len()),
    }
}

struct ForwardNode {
    root: PortableWorkspaceCausalStep,
    inputs: Vec<PortableWorkspaceCausalStep>,
    wire: serde_json::Value,
}

// The full forward reference is deliberately built and dropped outside every
// measurement. It is not an index retained by the production reverse query.
fn forward_reference(owner: &PortableWorkspaceCompilation) -> Vec<ForwardNode> {
    let comparison = PaleApi::new()
        .unwrap()
        .compare_portable_workspaces(owner, owner)
        .unwrap();
    let mut nodes = Vec::new();
    let mut offset = 0;
    loop {
        let subjects = comparison.impacts(offset, 256);
        for entry in subjects.entries() {
            let mut inputs = Vec::new();
            let mut after = None;
            loop {
                let page = owner
                    .direct_dependencies(entry.subject(), after.as_ref(), 256)
                    .unwrap();
                inputs.extend_from_slice(page.dependencies());
                after = page.continuation().cloned();
                if after.is_none() {
                    let wire: serde_json::Value =
                        serde_json::from_slice(page.canonical_bytes()).unwrap();
                    nodes.push(ForwardNode {
                        root: page.root().unwrap().clone(),
                        inputs,
                        wire: wire["root"].clone(),
                    });
                    break;
                }
            }
        }
        let Some(next) = subjects.next_offset() else {
            break;
        };
        offset = next;
    }
    nodes.sort_by(|a, b| a.root.node_digest().cmp(b.root.node_digest()));
    assert!(nodes
        .windows(2)
        .all(|pair| pair[0].root.node_digest() < pair[1].root.node_digest()));
    nodes
}

struct Prepared {
    positions: Vec<(usize, [Position; 2])>,
    totals: [Counts; 2],
    candidates: usize,
    source_documents: u32,
    matched_consumers: usize,
    empty_incomplete: bool,
}

fn check_projection(
    case: &Case,
    page: &PortableWorkspaceDependentPage,
    target: &serde_json::Value,
    reference: &[ForwardNode],
) {
    let value: serde_json::Value = serde_json::from_slice(page.canonical_bytes()).unwrap();
    let mut keys = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "admissionEvidenceDigest",
            "dependents",
            "inputDigestComparisons",
            "inspected",
            "isComplete",
            "principalUniverseDigest",
            "root",
            "semanticIrDigest",
            "subject",
            "workspaceId"
        ]
    );
    assert_eq!(value["workspaceId"], case.owner.workspace_id());
    assert_eq!(
        value["principalUniverseDigest"],
        case.owner.principal_universe_digest()
    );
    assert_eq!(value["semanticIrDigest"], case.owner.semantic_ir_digest());
    assert_eq!(
        value["admissionEvidenceDigest"],
        case.owner.admission_evidence_digest()
    );
    assert_eq!(value["subject"], target["subject"]);
    assert_eq!(value["root"], target["root"]);
    assert_eq!(value["inspected"], page.inspected());
    assert_eq!(
        value["inputDigestComparisons"],
        page.input_digest_comparisons()
    );
    assert_eq!(value["isComplete"], page.is_complete());
    assert_eq!(page.is_complete(), page.continuation().is_none());
    assert!(page.belongs_to(&case.owner));
    let rows = value["dependents"].as_array().unwrap();
    assert_eq!(rows.len(), page.dependents().len());
    for (wire, node) in rows.iter().zip(page.dependents()) {
        let index = reference
            .binary_search_by(|entry| entry.root.node_digest().cmp(node.node_digest()))
            .unwrap();
        assert_eq!(node, &reference[index].root);
        assert_eq!(wire, &reference[index].wire);
    }
    assert_eq!(
        pale_core::canon::to_canonical_string(&value)
            .unwrap()
            .as_bytes(),
        page.canonical_bytes()
    );
}

fn prepare(case: &Case) -> Prepared {
    let reference = forward_reference(&case.owner);
    let target = case
        .owner
        .direct_dependencies(&case.subject, None, 256)
        .unwrap();
    let target_wire: serde_json::Value = serde_json::from_slice(target.canonical_bytes()).unwrap();
    let expected = reference
        .iter()
        .filter(|node| {
            target.root().is_some_and(|root| {
                node.inputs
                    .iter()
                    .any(|input| input.node_digest() == root.node_digest())
            })
        })
        .map(|node| node.root.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        expected.len(),
        case.expected_consumers,
        "{} actual fixture shape; target={:#?}; forward consumers={:#?}",
        case.name,
        target.root(),
        expected
    );
    if case.wide {
        assert_eq!(case.owner.source_document_count(), 303);
        assert!(
            reference.len() > 300,
            "wide candidates, not 300 positive consumers"
        );
    } else {
        assert_eq!(case.owner.source_document_count(), 3);
    }
    let mut positions = [Position::default(), Position::default()];
    let mut selected = Vec::new();
    let mut totals = [Counts::default(); 2];
    let mut actual = Vec::new();
    let mut first_positive = false;
    let mut empty_incomplete = false;
    for ordinal in 0..100_000 {
        let before = positions.clone();
        let (application, app_next) = app_query(case, &before[0]);
        let (sdk, sdk_next) = sdk_query(case, &before[1]);
        let app_root = application
            .as_ref()
            .map(|page| PortableWorkspaceCausalStep::from_internal(page.root()));
        let app_rows = application.as_ref().map_or_else(Vec::new, |page| {
            page.dependents()
                .map(PortableWorkspaceCausalStep::from_internal)
                .collect()
        });
        assert_eq!(app_root.as_ref(), target.root());
        assert_eq!(sdk.root(), target.root());
        assert_eq!(app_rows, sdk.dependents());
        check_projection(case, &sdk, &target_wire, &reference);
        let app_count = app_counts(application.as_ref());
        let sdk_count = sdk_counts(&sdk);
        assert_eq!(app_count, sdk_count.without_bytes());
        assert_eq!(app_next.offset, sdk_next.offset);
        assert_eq!(app_next.complete, sdk_next.complete);
        assert!(
            sdk_count.inspected
                <= usize::from(if case.limits.0 == 0 {
                    64
                } else {
                    case.limits.0.min(256)
                })
        );
        assert!(
            sdk_count.dependents
                <= usize::from(if case.limits.1 == 0 {
                    64
                } else {
                    case.limits.1.min(256)
                })
        );
        let mut comparisons = 0;
        if let Some(root) = target.root() {
            for node in &reference[before[1].offset..sdk_next.offset] {
                let _ = node.inputs.binary_search_by(|input| {
                    comparisons += 1;
                    input.node_digest().cmp(root.node_digest())
                });
            }
        }
        assert_eq!(sdk.input_digest_comparisons(), comparisons);
        empty_incomplete |= sdk.dependents().is_empty() && !sdk.is_complete();
        let positive = !sdk.dependents().is_empty();
        if ordinal < 2 || sdk.is_complete() || (positive && !first_positive) {
            selected.push((ordinal, before));
        }
        first_positive |= positive;
        actual.extend_from_slice(sdk.dependents());
        totals[0].add(app_count);
        totals[1].add(sdk_count);
        positions = [app_next, sdk_next];
        if sdk.is_complete() {
            assert_eq!(actual, expected);
            assert_eq!(totals[0], totals[1].without_bytes());
            assert_eq!(
                totals[0].inspected,
                if target.root().is_some() {
                    reference.len()
                } else {
                    0
                }
            );
            assert!(selected.len() <= 4);
            if case.name == "wide-terminal-defaults" || case.name == "wide-sparse-source" {
                assert!(empty_incomplete);
            }
            // Only positions and scalar counts escape; the full oracle, mapped
            // rows and canonical reference values are dropped before measurement.
            return Prepared {
                positions: selected,
                totals,
                candidates: reference.len(),
                source_documents: case.owner.source_document_count(),
                matched_consumers: expected.len(),
                empty_incomplete,
            };
        }
        assert!(sdk.inspected() > 0);
    }
    panic!("fixed direct-consumer fixture did not terminate");
}

fn journey(case: &Case, stage: Stage) -> Counts {
    let mut before = Position::default();
    let mut total = Counts::default();
    loop {
        let (counts, next) = match stage {
            Stage::Application => {
                let (output, next) = app_query(case, &before);
                let counts = app_counts(output.as_ref());
                black_box(&output);
                drop(output);
                (counts, next)
            }
            Stage::Sdk => {
                let (output, next) = sdk_query(case, &before);
                let counts = sdk_counts(&output);
                black_box(&output);
                drop(output);
                (counts, next)
            }
        };
        total.add(counts);
        before = next;
        if before.complete {
            return total;
        }
    }
}

fn row(
    case: &Case,
    prepared: &Prepared,
    stage: Stage,
    page: &str,
    counts: Counts,
    complete: bool,
    observations: serde_json::Value,
) {
    report(json!({
        "suite": "direct-dependents", "case": case.name, "inputSha256": case.input_digest,
        "subject": format!("{:?}", case.subject), "rawLimits": case.limits,
        "stage": format!("{stage:?}"), "page": page, "counts": counts,
        "isComplete": complete, "observations": observations,
        "fixture": { "candidates": prepared.candidates, "sourceDocuments": prepared.source_documents,
            "matchedConsumers": prepared.matched_consumers, "hasEmptyIncomplete": prepared.empty_incomplete },
    }));
}

fn run() {
    run_header();
    for case in cases() {
        let prepared = prepare(&case);
        for (slot, stage) in STAGES.into_iter().enumerate() {
            for (ordinal, positions) in &prepared.positions {
                let before = positions[slot].clone();
                // Dispatch is outside single-call measurement. Concrete outputs
                // avoid adding a box just to unify borrowed and owned pages.
                let (counts, complete, observations) = match stage {
                    Stage::Application => {
                        let (output, next) = app_query(&case, &before);
                        let counts = app_counts(output.as_ref());
                        drop(output);
                        (counts, next.complete, measure(|| app_query(&case, &before)))
                    }
                    Stage::Sdk => {
                        let (output, next) = sdk_query(&case, &before);
                        let counts = sdk_counts(&output);
                        drop(output);
                        (counts, next.complete, measure(|| sdk_query(&case, &before)))
                    }
                };
                row(
                    &case,
                    &prepared,
                    stage,
                    &format!("page-{ordinal}"),
                    counts,
                    complete,
                    observations,
                );
            }
            assert_eq!(journey(&case, stage), prepared.totals[slot]);
            let observations = measure(|| journey(&case, stage));
            row(
                &case,
                &prepared,
                stage,
                "whole-journey-consume-and-drop",
                prepared.totals[slot],
                true,
                observations,
            );
        }
    }
}

#[test]
fn direct_dependent_workloads_preserve_query_parity() {
    for case in cases() {
        let prepared = prepare(&case);
        for (slot, stage) in STAGES.into_iter().enumerate() {
            assert_eq!(journey(&case, stage), prepared.totals[slot]);
        }
    }
}

#[cfg(not(feature = "bench-memory"))]
#[test]
#[ignore = "explicit optimized direct-consumer latency characterization"]
fn direct_dependent_latency() {
    run();
}

#[cfg(feature = "bench-memory")]
#[test]
#[ignore = "explicit optimized current-thread direct-consumer allocations"]
fn direct_dependent_allocations() {
    run();
}
