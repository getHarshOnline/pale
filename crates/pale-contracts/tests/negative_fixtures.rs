//! Canonical retained negative contract-fixture conformance.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use pale_contracts::{
    embedded_schema, schema_count, schema_descriptor, schema_registry_identity,
    validate_definition, validate_value, CompiledSchema, SchemaError, SchemaViolations,
};
use pale_core::records::{InvalidResidentRecordV1, TypedDigestV1, TypedDigestWireV1};
use serde::{Deserialize, Serialize};

/// Embedded index for every retained negative fixture admitted by this test.
const FIXTURE_SET_BYTES: &[u8] = include_bytes!("fixtures/negative/fixture-set.v1.json");
/// Schema identity expected inside the private fixture-set envelope.
const FIXTURE_SET_SCHEMA: &str = "pale.contract-negative-fixture-set.v1";
/// Stable identity of the active negative-fixture set.
const FIXTURE_SET_ID: &str = "pale-contracts/active-negative.v1";

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FixtureSet {
    schema: String,
    fixture_set_id: String,
    active_registry: ActiveRegistry,
    keyword_coverage: Vec<KeywordCoverage>,
    one_of_ambiguity: OneOfAmbiguity,
    semantic_boundaries: Vec<SemanticBoundary>,
    historical_registry_evidence: HistoricalRegistryEvidence,
    cases: Vec<FixtureCase>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ActiveRegistry {
    schema_manifest_sha256: String,
    schema_set_sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct KeywordCoverage {
    class: CoverageClass,
    generated_keywords: Vec<String>,
    case_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum CoverageClass {
    UnknownMember,
    MissingRequired,
    Type,
    ConstValue,
    Pattern,
    Bounds,
    Uniqueness,
    DependentRequired,
    OneOf,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OneOfAmbiguity {
    disposition: OneOfAmbiguityDisposition,
    validator_discriminators: Vec<String>,
    reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum OneOfAmbiguityDisposition {
    NoActiveWitness,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SemanticBoundary {
    owner: String,
    route: String,
    source_path: String,
    visibility: String,
    target_schema_id: String,
    disposition: BoundaryDisposition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum BoundaryDisposition {
    Callable,
    ExcludedOwnerBoundary,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct HistoricalRegistryEvidence {
    disposition: HistoricalDisposition,
    owner_path: String,
    reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum HistoricalDisposition {
    SeparateFromActiveFixtureSet,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FixtureCase {
    id: String,
    file: String,
    sha256: String,
    target: FixtureTarget,
    expected: ExpectedRejection,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FixtureTarget {
    schema_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    definition: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExpectedRejection {
    stage: RejectionStage,
    class: RejectionClass,
    keyword: String,
    instance_path: String,
    schema_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    semantic_discriminator: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum RejectionStage {
    StrictJson,
    Schema,
    Identity,
    Semantic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum RejectionClass {
    DuplicateKey,
    UnknownMember,
    MissingRequired,
    Type,
    ConstValue,
    Pattern,
    Bounds,
    Uniqueness,
    DependentRequired,
    OneOfNoneValid,
    DescriptorIdentityMismatch,
    SemanticDigestMismatch,
}

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/negative")
}

fn fixture_set() -> FixtureSet {
    let value = pale_core::canon::parse_strict(FIXTURE_SET_BYTES)
        .expect("negative fixture-set manifest is strict JSON");
    serde_json::from_value(value).expect("negative fixture-set manifest has the typed shape")
}

fn fixture_bytes(case: &FixtureCase) -> Vec<u8> {
    let relative = Path::new(&case.file);
    assert!(
        relative
            .components()
            .all(|component| matches!(component, Component::Normal(_))),
        "fixture {} is not a direct confined filename",
        case.id
    );
    std::fs::read(fixture_dir().join(relative))
        .unwrap_or_else(|error| panic!("read fixture {}: {error}", case.id))
}

fn parse_case(case: &FixtureCase) -> serde_json::Value {
    pale_core::canon::parse_strict(&fixture_bytes(case))
        .unwrap_or_else(|error| panic!("strict-parse fixture {}: {error}", case.id))
}

fn validate_structure(
    case: &FixtureCase,
    value: &serde_json::Value,
) -> Result<(), SchemaViolations> {
    if let Some(definition) = &case.target.definition {
        match validate_definition(value, &case.target.schema_id, definition) {
            Ok(()) => Ok(()),
            Err(SchemaError::Invalid(violations)) => Err(violations),
            Err(error) => panic!("resolve definition target for {}: {error}", case.id),
        }
    } else {
        CompiledSchema::for_id(&case.target.schema_id)
            .unwrap_or_else(|error| panic!("compile target for {}: {error}", case.id))
            .validate(value)
    }
}

fn coverage_class(class: RejectionClass) -> Option<CoverageClass> {
    match class {
        RejectionClass::UnknownMember => Some(CoverageClass::UnknownMember),
        RejectionClass::MissingRequired => Some(CoverageClass::MissingRequired),
        RejectionClass::Type => Some(CoverageClass::Type),
        RejectionClass::ConstValue => Some(CoverageClass::ConstValue),
        RejectionClass::Pattern => Some(CoverageClass::Pattern),
        RejectionClass::Bounds => Some(CoverageClass::Bounds),
        RejectionClass::Uniqueness => Some(CoverageClass::Uniqueness),
        RejectionClass::DependentRequired => Some(CoverageClass::DependentRequired),
        RejectionClass::OneOfNoneValid => Some(CoverageClass::OneOf),
        RejectionClass::DuplicateKey
        | RejectionClass::DescriptorIdentityMismatch
        | RejectionClass::SemanticDigestMismatch => None,
    }
}

fn keyword_class(keyword: &str) -> Option<CoverageClass> {
    match keyword {
        "additionalProperties" => Some(CoverageClass::UnknownMember),
        "required" => Some(CoverageClass::MissingRequired),
        "type" => Some(CoverageClass::Type),
        "const" => Some(CoverageClass::ConstValue),
        "pattern" => Some(CoverageClass::Pattern),
        "exclusiveMaximum" | "exclusiveMinimum" | "maxItems" | "maxLength" | "maxProperties"
        | "maximum" | "minItems" | "minLength" | "minProperties" | "minimum" | "multipleOf" => {
            Some(CoverageClass::Bounds)
        }
        "uniqueItems" => Some(CoverageClass::Uniqueness),
        "dependentRequired" => Some(CoverageClass::DependentRequired),
        "oneOf" => Some(CoverageClass::OneOf),
        _ => None,
    }
}

fn generated_keyword_class(keyword: &str, value: &serde_json::Value) -> Option<CoverageClass> {
    match keyword {
        "additionalProperties" if value != &serde_json::Value::Bool(false) => None,
        "required" if value.as_array().is_none_or(|members| members.is_empty()) => None,
        "uniqueItems" if value != &serde_json::Value::Bool(true) => None,
        _ => keyword_class(keyword),
    }
}

fn collect_generated_keyword_classes(
    value: &serde_json::Value,
    classes: &mut BTreeSet<CoverageClass>,
    keywords: &mut BTreeSet<String>,
) {
    match value {
        serde_json::Value::Object(object) => {
            for (keyword, child) in object {
                if let Some(class) = generated_keyword_class(keyword, child) {
                    classes.insert(class);
                    keywords.insert(keyword.clone());
                }
                collect_generated_keyword_classes(child, classes, keywords);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_generated_keyword_classes(item, classes, keywords);
            }
        }
        _ => {}
    }
}

#[test]
fn fixture_manifest_seals_exact_active_targets_and_case_bytes() {
    let set = fixture_set();
    verify_fixture_manifest(&set);
}

fn verify_fixture_manifest(set: &FixtureSet) {
    assert_eq!(set.schema, FIXTURE_SET_SCHEMA);
    assert_eq!(set.fixture_set_id, FIXTURE_SET_ID);

    let identity = schema_registry_identity().expect("active registry identity");
    assert_eq!(
        set.active_registry.schema_manifest_sha256,
        identity.schema_manifest_sha256.as_str()
    );
    assert_eq!(
        set.active_registry.schema_set_sha256,
        identity.schema_set_sha256.as_str()
    );

    let mut ids = BTreeSet::new();
    let mut files = BTreeMap::new();
    for case in &set.cases {
        assert!(
            ids.insert(case.id.as_str()),
            "duplicate case id {}",
            case.id
        );
        if let Some(existing_sha256) = files.insert(case.file.as_str(), case.sha256.as_str()) {
            assert_eq!(
                existing_sha256, case.sha256,
                "shared fixture {} has contradictory digests",
                case.file
            );
        }
        schema_descriptor(&case.target.schema_id)
            .unwrap_or_else(|error| panic!("inactive target for {}: {error}", case.id));
        if let Some(definition) = &case.target.definition {
            let schema: serde_json::Value = serde_json::from_str(
                embedded_schema(&case.target.schema_id).expect("embedded target source"),
            )
            .expect("generated schema parses");
            assert!(
                schema
                    .get("$defs")
                    .and_then(serde_json::Value::as_object)
                    .is_some_and(|definitions| definitions.contains_key(definition)),
                "inactive definition target for {}",
                case.id
            );
        }
        let bytes = fixture_bytes(case);
        assert_eq!(
            pale_core::sha256_hex(&bytes),
            case.sha256,
            "fixture digest drift for {}",
            case.id
        );
    }

    let observed_files = std::fs::read_dir(fixture_dir())
        .expect("read fixture directory")
        .map(|entry| {
            entry
                .expect("fixture directory entry")
                .file_name()
                .into_string()
                .expect("UTF-8 fixture filename")
        })
        .filter(|name| name != "fixture-set.v1.json")
        .collect::<BTreeSet<_>>();
    let declared_files = files.keys().map(|name| (*name).to_owned()).collect();
    assert_eq!(observed_files, declared_files);
}

/// Recompute registry coordinates without changing cases or expected failures.
fn regenerate_fixture_manifest(bytes: &[u8]) -> Vec<u8> {
    let value =
        pale_core::canon::parse_strict(bytes).expect("source fixture manifest is strict JSON");
    let mut set: FixtureSet =
        serde_json::from_value(value).expect("source fixture manifest retains its closed shape");
    let identity = schema_registry_identity().expect("current generated registry identity");
    set.active_registry.schema_manifest_sha256 =
        identity.schema_manifest_sha256.as_str().to_owned();
    set.active_registry.schema_set_sha256 = identity.schema_set_sha256.as_str().to_owned();
    verify_fixture_manifest(&set);
    let mut output = serde_json::to_vec_pretty(&set).expect("fixture manifest serializes");
    output.push(b'\n');
    output
}

fn write_fixture_manifest(directory: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;

    assert!(directory.is_absolute(), "output directory must be absolute");
    std::fs::create_dir(directory)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join("fixture-set.v1.json"))?;
    file.write_all(bytes)?;
    file.flush()
}

#[test]
fn fixture_regeneration_changes_only_current_registry_coordinates() {
    let mut old = pale_core::canon::parse_strict(FIXTURE_SET_BYTES).unwrap();
    old["active_registry"]["schema_manifest_sha256"] = "0".repeat(64).into();
    old["active_registry"]["schema_set_sha256"] = "1".repeat(64).into();
    let bytes = regenerate_fixture_manifest(&serde_json::to_vec(&old).unwrap());
    assert!(bytes.ends_with(b"\n"));
    assert_eq!(regenerate_fixture_manifest(&bytes), bytes);
    let mut regenerated = pale_core::canon::parse_strict(&bytes).unwrap();
    regenerated["active_registry"] = old["active_registry"].clone();
    assert_eq!(
        regenerated, old,
        "cases, coverage and ownership remain unchanged"
    );
}

#[test]
fn fixture_regeneration_rejects_changed_case_evidence_and_unknown_fields() {
    let original = pale_core::canon::parse_strict(FIXTURE_SET_BYTES).unwrap();
    let mut changed_case = original.clone();
    changed_case["cases"][0]["sha256"] = "0".repeat(64).into();
    let mut unknown = original;
    unknown["unexpected"] = true.into();
    for invalid in [changed_case, unknown] {
        let bytes = serde_json::to_vec(&invalid).unwrap();
        assert!(std::panic::catch_unwind(|| regenerate_fixture_manifest(&bytes)).is_err());
    }
}

#[test]
fn fixture_output_never_replaces_an_existing_directory() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = temporary.path().join("output");
    write_fixture_manifest(&directory, b"first\n").unwrap();
    assert!(write_fixture_manifest(&directory, b"second\n").is_err());
    assert_eq!(
        std::fs::read(directory.join("fixture-set.v1.json")).unwrap(),
        b"first\n"
    );
}

#[test]
#[ignore = "development-only regeneration; requires PALE_NEGATIVE_FIXTURE_OUTPUT"]
fn regenerate_negative_fixture_coordinates() -> std::io::Result<()> {
    let directory = std::env::var_os("PALE_NEGATIVE_FIXTURE_OUTPUT")
        .map(PathBuf::from)
        .expect("set PALE_NEGATIVE_FIXTURE_OUTPUT to a new absolute directory");
    let bytes = regenerate_fixture_manifest(FIXTURE_SET_BYTES);
    // Updating coordinates cannot repair a changed rejection or coverage law.
    strict_and_schema_cases_reject_at_the_declared_coordinates();
    identity_and_semantic_cases_pass_shape_before_their_owner_rejects();
    generated_keyword_coverage_is_derived_once_from_the_active_bundle();
    ambiguity_history_and_application_semantics_stay_outside_active_cases();
    write_fixture_manifest(&directory, &bytes)
}

#[test]
fn strict_and_schema_cases_reject_at_the_declared_coordinates() {
    let set = fixture_set();
    for case in &set.cases {
        match case.expected.stage {
            RejectionStage::StrictJson => {
                assert_eq!(case.expected.class, RejectionClass::DuplicateKey);
                assert_eq!(case.expected.keyword, "duplicateKey");
                assert!(case.expected.instance_path.is_empty());
                assert!(case.expected.schema_path.is_empty());
                let error = pale_core::canon::parse_strict(&fixture_bytes(case))
                    .expect_err("duplicate member must fail strict JSON parsing");
                assert!(error.to_string().contains("duplicate"));
            }
            RejectionStage::Schema => {
                let value = parse_case(case);
                let violations = validate_structure(case, &value)
                    .expect_err("declared structural fixture must be rejected");
                let matched = violations.violations().iter().find(|violation| {
                    violation.keyword() == case.expected.keyword
                        && violation.instance_path() == case.expected.instance_path
                        && violation.schema_path() == case.expected.schema_path
                });
                let Some(violation) = matched else {
                    panic!(
                        "{} did not emit {}/{}/{}; got {:?}",
                        case.id,
                        case.expected.keyword,
                        case.expected.instance_path,
                        case.expected.schema_path,
                        violations.violations()
                    );
                };
                if let Some(expected) = &case.expected.semantic_discriminator {
                    assert_eq!(violation.semantic_identity().3, expected, "{}", case.id);
                }
            }
            RejectionStage::Identity | RejectionStage::Semantic => {}
        }
    }
}

#[test]
fn identity_and_semantic_cases_pass_shape_before_their_owner_rejects() {
    let set = fixture_set();
    for case in &set.cases {
        match case.expected.stage {
            RejectionStage::Identity => {
                assert_eq!(
                    case.expected.class,
                    RejectionClass::DescriptorIdentityMismatch
                );
                assert_eq!(case.expected.keyword, "descriptorIdentity");
                assert_eq!(case.expected.instance_path, "/schema_sha256");
                let value = parse_case(case);
                validate_structure(case, &value)
                    .unwrap_or_else(|error| panic!("identity case failed shape first: {error}"));
                assert!(matches!(
                    validate_value(&value, &case.target.schema_id),
                    Err(SchemaError::IdentityMismatch(schema_id))
                        if schema_id == case.target.schema_id
                ));
            }
            RejectionStage::Semantic => {
                assert_eq!(case.expected.class, RejectionClass::SemanticDigestMismatch);
                assert_eq!(case.expected.keyword, "digest");
                assert_eq!(case.expected.instance_path, "/digest");
                let value = parse_case(case);
                validate_structure(case, &value)
                    .unwrap_or_else(|error| panic!("semantic case failed shape first: {error}"));
                validate_value(&value, &case.target.schema_id)
                    .expect("semantic case passes descriptor-aware structure");
                let wire: TypedDigestWireV1 = serde_json::from_value(value)
                    .expect("semantic case decodes as the declared owner wire");
                assert!(matches!(
                    TypedDigestV1::admit_wire(wire, |_| None),
                    Err(InvalidResidentRecordV1::DigestMismatch { family })
                        if family == "typed digest inline JSON"
                ));
            }
            RejectionStage::StrictJson | RejectionStage::Schema => {}
        }
    }
}

#[test]
fn generated_keyword_coverage_is_derived_once_from_the_active_bundle() {
    let set = fixture_set();
    let bundle_manifest = pale_core::canon::parse_strict(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/bundle/agent-surface.v1/agent-surface-schemas.v1.json"
    )))
    .expect("active generated descriptor manifest is strict JSON");
    let active_schemas = bundle_manifest["schemas"]
        .as_object()
        .expect("active generated schemas object");
    assert_eq!(
        active_schemas.len(),
        schema_count().expect("active schema count")
    );

    let mut derived_classes = BTreeSet::new();
    let mut derived_keywords = BTreeSet::new();
    for schema_id in active_schemas.keys() {
        let schema = pale_core::canon::parse_strict(
            embedded_schema(schema_id)
                .unwrap_or_else(|error| panic!("embedded active schema {schema_id}: {error}"))
                .as_bytes(),
        )
        .unwrap_or_else(|error| panic!("strict generated schema {schema_id}: {error}"));
        collect_generated_keyword_classes(&schema, &mut derived_classes, &mut derived_keywords);
    }

    let declared_classes = set
        .keyword_coverage
        .iter()
        .map(|coverage| coverage.class)
        .collect::<BTreeSet<_>>();
    assert_eq!(declared_classes.len(), set.keyword_coverage.len());
    assert_eq!(declared_classes, derived_classes);

    let cases = set
        .cases
        .iter()
        .map(|case| (case.id.as_str(), case))
        .collect::<BTreeMap<_, _>>();
    let mut declared_keywords = BTreeSet::new();
    for coverage in &set.keyword_coverage {
        assert!(!coverage.generated_keywords.is_empty());
        assert!(!coverage.case_ids.is_empty());
        for keyword in &coverage.generated_keywords {
            assert!(
                declared_keywords.insert(keyword.as_str()),
                "generated keyword `{keyword}` is assigned more than once"
            );
            assert!(
                derived_keywords.contains(keyword),
                "coverage keyword `{keyword}` is absent from the active generated bundle"
            );
            assert_eq!(keyword_class(keyword), Some(coverage.class));
        }
        for case_id in &coverage.case_ids {
            let case = cases
                .get(case_id.as_str())
                .unwrap_or_else(|| panic!("coverage names unknown case {case_id}"));
            assert_eq!(coverage_class(case.expected.class), Some(coverage.class));
        }
    }
    assert_eq!(
        declared_keywords,
        derived_keywords.iter().map(String::as_str).collect()
    );
}

#[test]
fn ambiguity_history_and_application_semantics_stay_outside_active_cases() {
    let set = fixture_set();
    assert_eq!(
        set.one_of_ambiguity.disposition,
        OneOfAmbiguityDisposition::NoActiveWitness
    );
    assert_eq!(
        set.one_of_ambiguity.validator_discriminators,
        ["one_of_not_valid", "one_of_multiple_valid"]
    );
    assert!(!set.one_of_ambiguity.reason.is_empty());

    assert_eq!(
        set.historical_registry_evidence.disposition,
        HistoricalDisposition::SeparateFromActiveFixtureSet
    );
    assert_eq!(
        set.historical_registry_evidence.owner_path,
        "crates/pale-contracts/tests/internal/registry.rs"
    );
    assert!(Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/internal/registry.rs")
        .is_file());
    assert!(!set.historical_registry_evidence.reason.is_empty());
    assert!(set
        .cases
        .iter()
        .all(|case| !case.id.starts_with("historical.")));

    let callable = set
        .semantic_boundaries
        .iter()
        .find(|boundary| boundary.disposition == BoundaryDisposition::Callable)
        .expect("one callable core-owned semantic boundary");
    assert_eq!(callable.owner, "pale_core::records::TypedDigestV1");
    assert_eq!(callable.route, "TypedDigestV1::admit_wire");
    assert_eq!(
        callable.source_path,
        "crates/pale-core/src/records/resident/effect.rs"
    );
    assert_eq!(callable.visibility, "pub");
    assert_eq!(callable.target_schema_id, "typed-digest.v1");
    assert!(callable.reason.is_none());

    let excluded = set
        .semantic_boundaries
        .iter()
        .find(|boundary| boundary.disposition == BoundaryDisposition::ExcludedOwnerBoundary)
        .expect("application-owned semantic boundary remains excluded");
    assert_eq!(
        excluded.owner,
        "pale_application::identity::ProjectIdentity"
    );
    assert_eq!(excluded.route, "ProjectIdentity::verify");
    assert_eq!(
        excluded.source_path,
        "crates/pale-application/src/identity.rs"
    );
    assert_eq!(excluded.visibility, "pub(crate)");
    assert_eq!(excluded.target_schema_id, "resolved-instance.v1");
    assert!(excluded.reason.as_deref().is_some_and(|reason| {
        reason.contains("pale-contracts must not import pale-application")
    }));
}
