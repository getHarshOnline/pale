//! Behavioral tests for Package-text benchmark custody and equivalence.

use super::*;

#[cfg(feature = "bench-memory")]
#[test]
fn prepared_input_drops_can_suppress_relative_peaks() {
    let input = vec![0_u8; 4096];
    let observed = measure_allocations(|| {
        drop(black_box(input));
        drop(black_box(vec![0_u8; 1024]));
    })
    .unwrap();
    assert_eq!(observed.total_count, 1);
    assert_eq!(observed.total_bytes, 1024);
    assert_eq!(observed.peak_relative_count, 0);
    assert_eq!(observed.peak_relative_bytes, 0);
    assert_eq!(observed.retained_count, -1);
    assert_eq!(observed.retained_bytes, -4096);
}

#[cfg(feature = "bench-memory")]
#[test]
fn timing_pass_disables_bookkeeping_without_skipping_operations() {
    let mut ran = false;
    let observed = allocation_counter::measure(|| {
        without_allocation_counting(|| {
            drop(black_box(vec![0_u8; 1024]));
            ran = true;
        });
    });
    assert!(ran);
    assert_eq!(observed.count_total, 0);
    assert_eq!(observed.bytes_total, 0);
    let fixture = CaseFixture::new(&corpus(), CASES[0]);
    let observed = allocation_counter::measure(|| {
        black_box(fixture.observe(Operation::EagerReference, 5));
    });
    // Only the timing-vector allocation occurs outside opt-out, not the outputs.
    assert_eq!(observed.count_total, 1);
    assert_eq!(observed.bytes_total, 5 * size_of::<u128>() as u64);
}

#[cfg(not(feature = "bench-memory"))]
#[test]
fn disabled_memory_sample_is_absent_and_does_not_run() {
    assert!(measure_allocations(|| panic!("disabled sample must not run")).is_none());
    assert!(resident_set_high_water().is_none());
    assert_eq!(
        rss_scope(None),
        "unavailable_feature_platform_or_observation"
    );
}

#[cfg(all(
    feature = "bench-memory",
    any(target_os = "macos", target_os = "linux")
))]
#[test]
fn rss_normalization_preserves_units_and_unavailability() {
    assert_eq!(rss_bytes(-1), None);
    assert_eq!(rss_bytes(0), Some(0));
    assert_eq!(
        rss_bytes(2),
        Some(if cfg!(target_os = "macos") { 2 } else { 2048 })
    );
    #[cfg(all(target_os = "linux", target_pointer_width = "64"))]
    assert_eq!(rss_bytes(std::ffi::c_long::MAX), None);
    assert_eq!(
        rss_scope(None),
        "unavailable_feature_platform_or_observation"
    );
}

/// Construct one tiny source corpus without filesystem ownership.
fn corpus() -> ActualCorpus {
    let documents = vec![
        SourceDocument {
            text: Arc::from("# Alpha\nbody β\n"),
            heading_offsets: vec![0].into(),
        },
        SourceDocument {
            text: Arc::from("plain\n## Beta\n"),
            heading_offsets: vec![6].into(),
        },
    ];
    let bytes = documents.iter().map(|document| document.text.len()).sum();
    ActualCorpus {
        documents,
        bytes,
        digest: ArtifactDigest::hash_bytes(b"test-corpus"),
    }
}

#[test]
fn paired_matrix_covers_required_dimensions_without_a_cartesian_product() {
    assert_eq!(CASES.len(), 6);
    assert!(CASES
        .iter()
        .any(|case| matches!(case.scale, CorpusScale::Actual)));
    assert!(CASES
        .iter()
        .any(|case| matches!(case.scale, CorpusScale::TenX)));
    assert!(CASES
        .iter()
        .any(|case| matches!(case.scale, CorpusScale::HundredX)));
    assert!(CASES.iter().any(|case| case.repositories == 100));
    assert!(CASES.iter().any(|case| case.fanout == 32));
    assert!(CASES
        .iter()
        .any(|case| matches!(case.headings, HeadingTransform::None)));
    assert!(CASES
        .iter()
        .any(|case| matches!(case.headings, HeadingTransform::Sparse)));
    assert!(CASES
        .iter()
        .any(|case| matches!(case.headings, HeadingTransform::Dense)));
    assert!(CASES
        .iter()
        .any(|case| matches!(case.boundary, MaterializationBoundary::WholeFile)));
    assert!(CASES
        .iter()
        .any(|case| matches!(case.boundary, MaterializationBoundary::ManagedBlock)));
}

#[test]
fn every_paired_case_matches_the_independent_eager_reference() {
    let actual = corpus();
    for case in CASES {
        let fixture = CaseFixture::new(&actual, case);
        let recipes = build_recipes(&fixture.corpus, &fixture.references, case);
        let materialized = materialize_recipes(&recipes);
        assert_eq!(materialized.len(), fixture.reference_outputs.len());
        for (recipe, reference) in materialized.iter().zip(&fixture.reference_outputs) {
            assert_eq!(recipe.as_bytes(), reference.as_bytes());
        }
        assert_eq!(
            output_set_digest(materialized.iter().map(|text| text.as_str())),
            fixture.output_digest
        );
        assert_eq!(
            materialized.iter().map(|text| text.len()).sum::<usize>(),
            logical_output_bytes(&fixture.corpus, case)
        );
    }
}

#[test]
fn repository_count_and_fanout_remain_distinct_physical_dimensions() {
    let actual = corpus();
    let repositories = BenchmarkCase::new(
        "two-repositories",
        CorpusScale::Actual,
        2,
        1,
        HeadingTransform::None,
        MaterializationBoundary::ManagedBlock,
    );
    let fanout = BenchmarkCase::new(
        "two-consumers",
        CorpusScale::Actual,
        1,
        2,
        HeadingTransform::None,
        MaterializationBoundary::ManagedBlock,
    );

    let repository_fixture = CaseFixture::new(&actual, repositories);
    let fanout_fixture = CaseFixture::new(&actual, fanout);

    assert_eq!(repository_fixture.reference_outputs.len(), 2);
    assert_eq!(fanout_fixture.reference_outputs.len(), 1);
    assert!(repository_fixture
        .reference_outputs
        .iter()
        .all(|output| output.matches(MANAGED_BEGIN).count() == 1));
    assert!(repository_fixture
        .reference_outputs
        .iter()
        .all(|output| output.matches(MANAGED_END).count() == 1));
    assert_ne!(
        repository_fixture.logical_output_bytes,
        fanout_fixture.logical_output_bytes
    );
}

#[test]
fn copy_bounds_distinguish_reference_cold_warm_and_hash_reads() {
    let fixture = CaseFixture::new(&corpus(), CASES[0]);
    let output = fixture.logical_output_bytes;
    assert_eq!(
        Operation::EagerReference.explicit_text_copy_lower_bound(&fixture),
        output
    );
    assert_eq!(
        Operation::ColdMaterialize.explicit_text_copy_lower_bound(&fixture),
        output
    );
    assert_eq!(
        Operation::BoundaryCopy.explicit_text_copy_lower_bound(&fixture),
        output
    );
    assert_eq!(
        Operation::WarmMaterialize.explicit_text_copy_lower_bound(&fixture),
        0
    );
    assert_eq!(
        Operation::DigestOutput.explicit_text_copy_lower_bound(&fixture),
        0
    );
}

#[test]
fn p99_requires_the_full_independent_sample_floor() {
    assert!(Distribution::new(vec![1; SMOKE_SAMPLES], SMOKE_SAMPLES)
        .p99
        .is_none());
    assert_eq!(
        Distribution::new((1..=FULL_SAMPLES as u128).collect(), FULL_SAMPLES).p99,
        Some(199)
    );
}

#[test]
fn atx_measurement_excludes_non_atx_or_overwide_markers() {
    assert_eq!(
        atx_heading_offsets("# one\nplain # text\n####### no\n## two\n"),
        vec![0, 30]
    );
}

#[test]
fn optional_report_fields_may_be_empty_but_cannot_escape_their_column() {
    validate_tsv_field("");
    assert!(std::panic::catch_unwind(|| validate_tsv_field("one\ttwo")).is_err());
}

#[test]
fn oversized_case_is_rejected_before_recipe_or_output_construction() {
    let mut expanded = ExpandedCorpus::new(&corpus(), CorpusScale::Actual);
    expanded.bytes = MAX_LOGICAL_OUTPUT_BYTES;
    let oversized = BenchmarkCase::new(
        "oversized-output",
        CorpusScale::Actual,
        2,
        1,
        HeadingTransform::None,
        MaterializationBoundary::WholeFile,
    );

    assert!(std::panic::catch_unwind(|| validate_case_bounds(&expanded, oversized)).is_err());
}

#[test]
fn source_capture_is_independent_of_filesystem_creation_order() {
    let left = tempfile::tempdir().expect("create first corpus");
    let right = tempfile::tempdir().expect("create second corpus");
    std::fs::write(left.path().join("b.md"), "# B\n").expect("write first B");
    std::fs::write(left.path().join("a.md"), "# A\n").expect("write first A");
    std::fs::write(right.path().join("a.md"), "# A\n").expect("write second A");
    std::fs::write(right.path().join("b.md"), "# B\n").expect("write second B");

    let first = ActualCorpus::capture(left.path());
    let second = ActualCorpus::capture(right.path());

    assert_eq!(first.digest, second.digest);
    assert_eq!(first.bytes, second.bytes);
}

#[test]
fn source_capture_rejects_excessive_depth() {
    let root = tempfile::tempdir().expect("create corpus");
    let mut directory = root.path().to_owned();
    for index in 0..=MAX_DIRECTORY_DEPTH {
        directory.push(format!("d{index}"));
        std::fs::create_dir(&directory).expect("create nested directory");
    }
    std::fs::write(directory.join("body.md"), "body\n").expect("write nested Markdown");

    assert!(std::panic::catch_unwind(|| ActualCorpus::capture(root.path())).is_err());
}

#[test]
fn source_capture_rejects_oversized_files_before_reading_their_bodies() {
    let root = tempfile::tempdir().expect("create corpus");
    let file = std::fs::File::create(root.path().join("large.md")).expect("create sparse Markdown");
    file.set_len(u64::try_from(MAX_ACTUAL_BYTES).expect("bound fits u64") + 1)
        .expect("size sparse Markdown");

    assert!(std::panic::catch_unwind(|| ActualCorpus::capture(root.path())).is_err());
}

#[cfg(unix)]
#[test]
fn source_capture_rejects_symlink_members() {
    let root = tempfile::tempdir().expect("create corpus");
    std::fs::write(root.path().join("body.md"), "body\n").expect("write Markdown");
    std::os::unix::fs::symlink("body.md", root.path().join("alias.md"))
        .expect("create Markdown symlink");

    assert!(std::panic::catch_unwind(|| ActualCorpus::capture(root.path())).is_err());
}

#[cfg(unix)]
#[test]
fn source_identity_rejects_non_utf8_paths() {
    use std::os::unix::ffi::OsStringExt as _;

    let root = tempfile::tempdir().expect("create corpus");
    let path = root
        .path()
        .join(std::ffi::OsString::from_vec(vec![0xff, b'.', b'm', b'd']));

    assert!(std::panic::catch_unwind(|| confined_relative_path(root.path(), &path)).is_err());
}

#[cfg(feature = "bench-memory")]
#[test]
fn allocation_scope_excludes_cold_and_warm_recipe_prerequisites() {
    let fixture = CaseFixture::new(&corpus(), CASES[0]);
    let cold = fixture
        .allocation_evidence(Operation::ColdMaterialize)
        .expect("allocation evidence is enabled");
    let warm = fixture
        .allocation_evidence(Operation::WarmMaterialize)
        .expect("allocation evidence is enabled");
    assert!(cold.total_bytes >= fixture.logical_output_bytes as u64);
    assert_eq!(warm.total_count, 0);
    assert_eq!(warm.total_bytes, 0);
}
