//! Package diagnostic aggregation and identity tests.

use super::*;

fn error(index: usize) -> PackageError {
    PackageError::MissingSnapshotFile {
        path: PathBuf::from(format!("packages/contradiction-{index:03}.toml")),
    }
}

#[test]
fn aggregate_retains_a_bounded_canonical_prefix_without_flattening_causes() {
    let mut accumulator = PackageDiagnosticAccumulator::default();
    for index in 0..MAX_PACKAGE_DIAGNOSTICS + 5 {
        accumulator.push(error(index));
    }
    let PackageError::Diagnostics(diagnostics) = accumulator.finish(()).unwrap_err() else {
        panic!("multiple independent failures must retain their typed aggregate");
    };
    assert_eq!(diagnostics.entries().len(), MAX_PACKAGE_DIAGNOSTICS);
    assert_eq!(diagnostics.truncated(), 5);
    assert_eq!(
        diagnostics.entries()[0].code(),
        PackageDiagnosticCode::MissingSnapshotFile
    );
}

#[test]
fn nested_stage_aggregates_merge_without_hiding_typed_causes() {
    let mut inner = PackageDiagnosticAccumulator::default();
    inner.push(error(1));
    inner.push(error(2));

    let mut outer = PackageDiagnosticAccumulator::default();
    outer.push(inner.finish(()).unwrap_err());
    outer.push(error(3));

    let PackageError::Diagnostics(diagnostics) = outer.finish(()).unwrap_err() else {
        panic!("stage composition must preserve one typed diagnostic boundary");
    };
    assert_eq!(diagnostics.observed_len(), 3);
    assert!(diagnostics
        .entries()
        .iter()
        .all(|entry| entry.code() == PackageDiagnosticCode::MissingSnapshotFile));
}

fn accumulated(indices: impl IntoIterator<Item = usize>) -> PackageDiagnosticAccumulator {
    let mut accumulator = PackageDiagnosticAccumulator::default();
    for index in indices {
        accumulator.push(error(index));
    }
    accumulator
}

fn summary(accumulator: PackageDiagnosticAccumulator) -> (Vec<PackageDiagnostic>, usize, usize) {
    let diagnostics = accumulator.finish(()).unwrap_err();
    let PackageError::Diagnostics(diagnostics) = diagnostics else {
        panic!("a populated accumulator must finish with typed diagnostics");
    };
    (
        diagnostics.entries().to_vec(),
        diagnostics.truncated(),
        diagnostics.observed_len(),
    )
}

#[test]
fn merge_is_associative_commutative_and_has_an_empty_identity() {
    let a = || accumulated(0..70);
    let b = || accumulated(70..140);
    let c = || accumulated(140..210);

    let mut left = a();
    left.merge(b());
    let mut left_then_c = left;
    left_then_c.merge(c());

    let mut right = b();
    right.merge(c());
    let mut a_then_right = a();
    a_then_right.merge(right);
    assert_eq!(summary(left_then_c), summary(a_then_right));

    let mut ab = a();
    ab.merge(b());
    let mut ba = b();
    ba.merge(a());
    assert_eq!(summary(ab), summary(ba));

    let mut identity = a();
    identity.merge(PackageDiagnosticAccumulator::default());
    assert_eq!(summary(identity), summary(a()));
}

#[test]
fn partition_and_permutation_preserve_the_prefix_and_exact_omitted_count() {
    let direct = summary(accumulated((0..210).rev()));
    let mut partitioned = accumulated((0..70).rev());
    partitioned.merge(accumulated((70..140).rev()));
    partitioned.merge(accumulated((140..210).rev()));
    let partitioned = summary(partitioned);

    assert_eq!(direct, partitioned);
    assert_eq!(direct.0.len(), MAX_PACKAGE_DIAGNOSTICS);
    assert_eq!(direct.1, 210 - MAX_PACKAGE_DIAGNOSTICS);
    assert_eq!(direct.2, 210);
}

#[test]
fn platform_error_prose_cannot_change_diagnostic_retention() {
    let first = PackageError::Read {
        path: PathBuf::from("packages/alpha/package.toml"),
        source: std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "host-specific path and user name A",
        ),
    };
    let second = PackageError::Read {
        path: PathBuf::from("packages/alpha/package.toml"),
        source: std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "different operating-system prose B",
        ),
    };

    assert_eq!(
        package_error_diagnostic(&first),
        package_error_diagnostic(&second),
        "opaque platform prose must not decide which diagnostic survives the bound"
    );

    let later_path = PackageError::Read {
        path: PathBuf::from("packages/zeta/package.toml"),
        source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "irrelevant"),
    };
    assert!(package_error_diagnostic(&first) < package_error_diagnostic(&later_path));
}

#[test]
fn over_capacity_equal_key_collisions_are_order_and_partition_independent() {
    let build = |count: usize, reversed: bool| {
        let mut accumulator = PackageDiagnosticAccumulator::default();
        let indices: Box<dyn Iterator<Item = usize>> = if reversed {
            Box::new((0..count).rev())
        } else {
            Box::new(0..count)
        };
        for index in indices {
            accumulator.push(PackageError::UnsupportedPlatform {
                platform: if index % 2 == 0 {
                    "first presentation label"
                } else {
                    "second presentation label"
                },
            });
        }
        accumulator
    };

    let direct = summary(build(300, false));
    let reversed = summary(build(300, true));
    assert_eq!(direct, reversed);
    assert_eq!(direct.0.len(), MAX_PACKAGE_DIAGNOSTICS);
    assert_eq!(direct.1, 300 - MAX_PACKAGE_DIAGNOSTICS);

    let mut three_way = build(100, false);
    three_way.merge(build(100, true));
    three_way.merge(build(100, false));
    assert_eq!(direct, summary(three_way));
}

fn source_decode_from(label: &str, source_text: &str) -> PackageError {
    #[derive(Debug, serde::Deserialize)]
    struct Record {
        #[serde(rename = "count")]
        _count: u64,
    }

    let mut deserializer = serde_json::Deserializer::from_str(source_text);
    let source = serde_path_to_error::deserialize::<_, Record>(&mut deserializer)
        .expect_err("fixture must fail typed decoding");
    PackageError::SourceDecode {
        path: PathBuf::from("packages/example/record.toml"),
        label: label.to_string(),
        value_path: source.path().to_string(),
        source: Box::new(source),
    }
}

fn source_decode(label: &str) -> PackageError {
    source_decode_from(label, r#"{"count":"invalid"}"#)
}

#[test]
fn presentation_labels_and_redundant_path_syntax_do_not_change_identity() {
    assert_eq!(
        package_error_diagnostic(&source_decode("first human label")),
        package_error_diagnostic(&source_decode("second human label"))
    );

    let plain = PackageError::MissingSnapshotFile {
        path: PathBuf::from("packages/example/body.md"),
    };
    let redundant = PackageError::MissingSnapshotFile {
        path: PathBuf::from("./packages/./example/body.md"),
    };
    assert_eq!(
        package_error_diagnostic(&plain),
        package_error_diagnostic(&redundant)
    );
}

#[test]
fn toml_spans_and_json_layout_coordinates_do_not_change_retention_identity() {
    let toml_plain = PackageError::TomlSyntax {
        path: PathBuf::from("packages/example/package.toml"),
        source: Box::new(
            toml::from_str::<toml::Value>("[broken").expect_err("fixture must be invalid TOML"),
        ),
    };
    let toml_shifted = PackageError::TomlSyntax {
        path: PathBuf::from("packages/example/package.toml"),
        source: Box::new(
            toml::from_str::<toml::Value>("\n\n[broken").expect_err("fixture must be invalid TOML"),
        ),
    };
    let (
        PackageError::TomlSyntax { source: plain, .. },
        PackageError::TomlSyntax {
            source: shifted, ..
        },
    ) = (&toml_plain, &toml_shifted)
    else {
        unreachable!("fixtures have the declared variants")
    };
    assert_ne!(plain.span(), shifted.span());
    assert_eq!(
        package_error_diagnostic(&toml_plain),
        package_error_diagnostic(&toml_shifted)
    );

    let json_plain = PackageError::TomlToJson {
        path: PathBuf::from("packages/example/package.toml"),
        source: serde_json::from_str::<serde_json::Value>("{")
            .expect_err("fixture must be invalid JSON"),
    };
    let json_shifted = PackageError::TomlToJson {
        path: PathBuf::from("packages/example/package.toml"),
        source: serde_json::from_str::<serde_json::Value>("\n\n    {")
            .expect_err("fixture must be invalid JSON"),
    };
    let (
        PackageError::TomlToJson { source: plain, .. },
        PackageError::TomlToJson {
            source: shifted, ..
        },
    ) = (&json_plain, &json_shifted)
    else {
        unreachable!("fixtures have the declared variants")
    };
    assert_ne!(plain.line(), shifted.line());
    assert_ne!(plain.column(), shifted.column());
    assert_eq!(
        package_error_diagnostic(&json_plain),
        package_error_diagnostic(&json_shifted)
    );

    let schema_plain = pale_contracts::SchemaError::Encode(
        serde_json::from_str::<serde_json::Value>("{").expect_err("fixture must be invalid JSON"),
    );
    let schema_shifted = pale_contracts::SchemaError::Encode(
        serde_json::from_str::<serde_json::Value>("\n\n    {")
            .expect_err("fixture must be invalid JSON"),
    );
    assert_eq!(
        schema_error_coordinates(&schema_plain),
        schema_error_coordinates(&schema_shifted)
    );
}

#[test]
fn source_decode_keeps_semantic_value_path_but_excludes_line_and_column() {
    let plain = source_decode_from("record", r#"{"count":"invalid"}"#);
    let shifted = source_decode_from("record", "\n\n{\n  \"count\": \"invalid\"\n}");
    let (
        PackageError::SourceDecode {
            value_path: plain_path,
            source: plain_source,
            ..
        },
        PackageError::SourceDecode {
            value_path: shifted_path,
            source: shifted_source,
            ..
        },
    ) = (&plain, &shifted)
    else {
        unreachable!("fixtures have the declared variants")
    };
    assert_eq!(plain_path, shifted_path);
    assert_ne!(plain_source.inner().line(), shifted_source.inner().line());
    assert_ne!(
        plain_source.inner().column(),
        shifted_source.inner().column()
    );
    assert_eq!(
        package_error_diagnostic(&plain),
        package_error_diagnostic(&shifted)
    );
}

#[test]
fn schema_expectation_text_is_not_part_of_package_diagnostic_identity() {
    let schema =
        pale_contracts::CompiledSchema::for_id("sha256-digest.v1").expect("embedded digest schema");
    let violations = schema
        .validate(&serde_json::json!("not-a-digest"))
        .expect_err("invalid digest must be rejected");
    let coordinates = schema_violation_coordinates(&violations);
    assert!(violations
        .violations()
        .iter()
        .all(|violation| !coordinates.contains(&violation.expectation().to_string())));
}
