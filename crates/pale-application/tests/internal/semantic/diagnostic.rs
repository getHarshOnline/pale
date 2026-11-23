//! Checks bounded semantic-diagnostic accumulation and its associative, commutative merge laws.

use super::*;

fn diagnostic(index: usize) -> SemanticDiagnostic {
    SemanticDiagnostic::new(SemanticCause::RepositoryProfileMismatch {
        profile: ProfileDigest::hash_canonical("profile").expect("canonical fixture profile"),
        repository: LogicalRepositoryIdV1::parse(format!("repository-{index:03}"))
            .expect("valid fixture repository"),
    })
}

#[test]
fn accumulation_is_bounded_and_stably_ordered() {
    let mut accumulator = DiagnosticAccumulator::default();
    for index in (0..MAX_SEMANTIC_DIAGNOSTICS + 7).rev() {
        accumulator.push(diagnostic(index));
    }
    let diagnostics = accumulator.finish(()).unwrap_err();
    assert_eq!(diagnostics.entries().len(), MAX_SEMANTIC_DIAGNOSTICS);
    assert_eq!(diagnostics.truncated(), 7);
    assert!(diagnostics
        .entries()
        .windows(2)
        .all(|pair| pair[0] <= pair[1]));
    assert_eq!(
        diagnostics.entries().last().unwrap().subject(),
        SemanticCoordinate::Repository(
            LogicalRepositoryIdV1::parse("repository-127").expect("valid fixture repository")
        )
    );
}

fn accumulated(indices: impl IntoIterator<Item = usize>) -> DiagnosticAccumulator {
    let mut accumulator = DiagnosticAccumulator::default();
    for index in indices {
        accumulator.push(diagnostic(index));
    }
    accumulator
}

fn summary(accumulator: DiagnosticAccumulator) -> SemanticDiagnostics {
    accumulator.finish(()).unwrap_err()
}

#[test]
fn merge_is_associative_commutative_and_has_an_empty_identity() {
    let a = || accumulated(0..70);
    let b = || accumulated(70..140);
    let c = || accumulated(140..210);

    let mut left = a();
    left.merge(b());
    left.merge(c());
    let mut right_partition = b();
    right_partition.merge(c());
    let mut right = a();
    right.merge(right_partition);
    assert_eq!(summary(left), summary(right));

    let mut ab = a();
    ab.merge(b());
    let mut ba = b();
    ba.merge(a());
    assert_eq!(summary(ab), summary(ba));

    let mut identity = a();
    identity.merge(DiagnosticAccumulator::default());
    assert_eq!(summary(identity), summary(a()));
}

#[test]
fn partition_and_permutation_preserve_prefix_and_exact_omitted_count() {
    let direct = summary(accumulated((0..210).rev()));
    let mut partitioned = accumulated((0..70).rev());
    partitioned.merge(accumulated((70..140).rev()));
    partitioned.merge(accumulated((140..210).rev()));
    let partitioned = summary(partitioned);

    assert_eq!(direct, partitioned);
    assert_eq!(direct.entries().len(), MAX_SEMANTIC_DIAGNOSTICS);
    assert_eq!(direct.truncated(), 210 - MAX_SEMANTIC_DIAGNOSTICS);
    assert_eq!(direct.observed_len(), 210);
}
