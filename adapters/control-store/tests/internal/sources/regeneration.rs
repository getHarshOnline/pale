//! Owner-local development generator and its independent behavior checks.

//! The ignored writer is a tool entry point, not behavioral qualification.

use super::*;
use std::{
    fs,
    io::{self, Write},
    path::Path,
};

#[derive(Debug, PartialEq, Eq)]
struct GeneratedSources {
    manifest: Vec<u8>,
    pin: String,
    pin_source: Vec<u8>,
}

fn derive_schema(sql: &str) -> Result<String, ControlSqliteError> {
    let connection = Connection::open_in_memory()
        .map_err(|error| ControlSqliteError::sqlite("open development schema derivation", error))?;
    connection
        .execute_batch(sql)
        .map_err(|error| ControlSqliteError::sqlite("apply development migration", error))?;
    resultant_schema_sha256(&connection)
}

// The original Value preserves all non-derived fields. The existing owner
// decodes and validates it before mutation and verifies the complete result.
fn render(
    manifest_bytes: &[u8],
    input_pin: &str,
    principal_sql: &'static str,
    deployment_sql: &'static str,
) -> Result<GeneratedSources, ControlSqliteError> {
    let mut value = decode_manifest_value(manifest_bytes, input_pin)?;
    decode_manifest(value.clone())?;
    for (migration_pointer, schema_pointer, sql) in [
        (
            "/migration_sets/principal_trust/0/sha256",
            "/stores/principal_trust/schema_sha256",
            principal_sql,
        ),
        (
            "/migration_sets/deployment_control/0/sha256",
            "/stores/deployment_control/schema_sha256",
            deployment_sql,
        ),
    ] {
        for (pointer, digest) in [
            (migration_pointer, sha256_hex(sql.as_bytes())),
            (schema_pointer, derive_schema(sql)?),
        ] {
            let slot = value
                .pointer_mut(pointer)
                .ok_or(ControlSqliteError::SourceInvariant(
                    "derived storage coordinate",
                ))?;
            *slot = serde_json::Value::String(digest);
        }
    }
    let canonical = serde_jcs::to_vec(&value)
        .map_err(|_| ControlSqliteError::SourceInvariant("generated manifest JCS encoding"))?;
    let pin = sha256_hex(&canonical);
    let mut manifest =
        serde_json::to_vec_pretty(&value).map_err(ControlSqliteError::ManifestDecode)?;
    manifest.push(b'\n');
    // Same production verifier, including declared headers, dependency/profile
    // decisions and exact independently derived resultant schema identities.
    verify_storage_sources(&manifest, &pin, principal_sql, deployment_sql)?;
    let pin_source = format!(
        "/// Generated control-storage manifest pin.\n\
         /// Regenerate this file and contracts/manifests/pale-control-storage.v1.json together.\n\
         /// Pinned SHA-256 of the manifest value's RFC 8785 encoding.\n\
         const MANIFEST_SHA256: &str = \"{pin}\";\n"
    )
    .into_bytes();
    Ok(GeneratedSources {
        manifest,
        pin,
        pin_source,
    })
}

// No live path is inferred, no existing directory/file is replaced. A failed
// write may leave the explicit output directory for inspection; no pair is
// announced as complete until both create-new writes finish successfully.
fn write_new_output(output: &Path, generated: &GeneratedSources) -> io::Result<()> {
    if !output.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "output directory must be absolute",
        ));
    }
    fs::create_dir(output)?;
    for (name, bytes) in [
        (
            "pale-control-storage.v1.json",
            generated.manifest.as_slice(),
        ),
        ("storage_manifest_pin.rs", generated.pin_source.as_slice()),
    ] {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join(name))?;
        file.write_all(bytes)?;
        file.flush()?;
    }
    Ok(())
}

#[test]
#[ignore = "development generator; explicitly invoked, not behavioral qualification"]
fn regenerate_storage_manifest() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::var_os("PALE_STORAGE_GENERATOR_OUTPUT")
        .ok_or("set a new absolute PALE_STORAGE_GENERATOR_OUTPUT directory")?;
    let generated = render(
        MANIFEST_BYTES,
        MANIFEST_SHA256,
        PRINCIPAL_TRUST_MIGRATION_V1,
        DEPLOYMENT_CONTROL_MIGRATION_V1,
    )?;
    write_new_output(Path::new(&output), &generated)?;
    println!(
        "development generator completed; JCS pin {}; not a behavioral test result",
        generated.pin
    );
    Ok(())
}

const CHANGED_DEPLOYMENT: &str = concat!(
    include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/migrations/deployment-control/0001_initial_v1.sql"
    )),
    "\nCREATE TABLE generation_fixture (id INTEGER PRIMARY KEY) STRICT;\n"
);

#[test]
fn deterministic_rendering_preserves_every_nonderived_manifest_value() {
    let first = render(
        MANIFEST_BYTES,
        MANIFEST_SHA256,
        PRINCIPAL_TRUST_MIGRATION_V1,
        CHANGED_DEPLOYMENT,
    )
    .unwrap();
    let second = render(
        MANIFEST_BYTES,
        MANIFEST_SHA256,
        PRINCIPAL_TRUST_MIGRATION_V1,
        CHANGED_DEPLOYMENT,
    )
    .unwrap();
    assert_eq!(first, second);
    let original: serde_json::Value = serde_json::from_slice(MANIFEST_BYTES).unwrap();
    let generated: serde_json::Value = serde_json::from_slice(&first.manifest).unwrap();
    let mut expected = original.clone();
    for pointer in [
        "/migration_sets/principal_trust/0/sha256",
        "/stores/principal_trust/schema_sha256",
        "/migration_sets/deployment_control/0/sha256",
        "/stores/deployment_control/schema_sha256",
    ] {
        *expected.pointer_mut(pointer).unwrap() = generated.pointer(pointer).unwrap().clone();
    }
    assert_eq!(generated, expected);
    assert_eq!(
        generated["authority_journal_profile"],
        original["authority_journal_profile"]
    );
    assert_eq!(generated["sqlite_profile"], original["sqlite_profile"]);
    assert_eq!(
        generated["backup_restore_policy"],
        original["backup_restore_policy"]
    );
    assert_eq!(
        sha256_hex(&serde_jcs::to_vec(&generated).unwrap()),
        first.pin
    );
    assert!(first.manifest.ends_with(b"\n"));
    let verified = verify_storage_sources(
        &first.manifest,
        &first.pin,
        PRINCIPAL_TRUST_MIGRATION_V1,
        CHANGED_DEPLOYMENT,
    )
    .unwrap();
    assert!(!verified.is_runtime_profile_selected());
    assert_eq!(
        verified.unresolved_profile_requirements(),
        ControlStorageProfileRequirement::ALL.to_vec()
    );
}

#[test]
fn actual_sqlite_schema_derivation_and_the_existing_verifier_agree() {
    let generated = render(
        MANIFEST_BYTES,
        MANIFEST_SHA256,
        PRINCIPAL_TRUST_MIGRATION_V1,
        CHANGED_DEPLOYMENT,
    )
    .unwrap();
    let connection = Connection::open_in_memory().unwrap();
    connection.execute_batch(CHANGED_DEPLOYMENT).unwrap();
    assert_eq!(connection.query_row("SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name='generation_fixture'",[],|row| row.get::<_,i64>(0)).unwrap(),1);
    let verified = verify_storage_sources(
        &generated.manifest,
        &generated.pin,
        PRINCIPAL_TRUST_MIGRATION_V1,
        CHANGED_DEPLOYMENT,
    )
    .unwrap();
    let deployment = verified.store(ControlStoreKind::DeploymentControl);
    assert_eq!(
        deployment.schema_sha256(),
        resultant_schema_sha256(&connection).unwrap()
    );
    assert_eq!(
        deployment.migrations()[0].sha256(),
        sha256_hex(CHANGED_DEPLOYMENT.as_bytes())
    );
    assert_ne!(
        deployment.schema_sha256(),
        deployment.migrations()[0].sha256()
    );
    assert!(verify_storage_sources(
        &generated.manifest,
        &generated.pin,
        PRINCIPAL_TRUST_MIGRATION_V1,
        DEPLOYMENT_CONTROL_MIGRATION_V1
    )
    .is_err());
    assert!(matches!(
        verify_storage_sources(
            &generated.manifest,
            &"0".repeat(64),
            PRINCIPAL_TRUST_MIGRATION_V1,
            CHANGED_DEPLOYMENT
        ),
        Err(ControlSqliteError::ManifestDigestMismatch)
    ));
    // Updating a pin cannot make an invented resultant schema true.
    let mut corrupt: serde_json::Value = serde_json::from_slice(&generated.manifest).unwrap();
    corrupt["stores"]["deployment_control"]["schema_sha256"] =
        serde_json::Value::String("0".repeat(64));
    let corrupt_pin = sha256_hex(&serde_jcs::to_vec(&corrupt).unwrap());
    assert!(matches!(
        verify_storage_sources(
            &serde_json::to_vec(&corrupt).unwrap(),
            &corrupt_pin,
            PRINCIPAL_TRUST_MIGRATION_V1,
            CHANGED_DEPLOYMENT
        ),
        Err(ControlSqliteError::SourceInvariant(
            "resultant physical schema hash"
        ))
    ));
}

#[test]
fn generation_cannot_fill_unselected_profiles_or_accept_wrong_headers() {
    let mut changed: serde_json::Value = serde_json::from_slice(MANIFEST_BYTES).unwrap();
    changed["sqlite"]["library_version"] =
        serde_json::Value::String("not-the-pinned-library".into());
    let pin = sha256_hex(&serde_jcs::to_vec(&changed).unwrap());
    assert!(render(
        &serde_json::to_vec(&changed).unwrap(),
        &pin,
        PRINCIPAL_TRUST_MIGRATION_V1,
        DEPLOYMENT_CONTROL_MIGRATION_V1
    )
    .is_err());
    let mut selected: serde_json::Value = serde_json::from_slice(MANIFEST_BYTES).unwrap();
    selected["sqlite_profile"]["synchronous"] = serde_json::Value::String("full".into());
    let selected_pin = sha256_hex(&serde_jcs::to_vec(&selected).unwrap());
    assert!(matches!(
        render(
            &serde_json::to_vec(&selected).unwrap(),
            &selected_pin,
            PRINCIPAL_TRUST_MIGRATION_V1,
            DEPLOYMENT_CONTROL_MIGRATION_V1
        ),
        Err(ControlSqliteError::SourceInvariant(
            "control-storage profile frontier"
        ))
    ));
    let mut unknown: serde_json::Value = serde_json::from_slice(MANIFEST_BYTES).unwrap();
    unknown["unrecognized"] = serde_json::Value::Bool(true);
    let unknown_pin = sha256_hex(&serde_jcs::to_vec(&unknown).unwrap());
    assert!(matches!(
        render(
            &serde_json::to_vec(&unknown).unwrap(),
            &unknown_pin,
            PRINCIPAL_TRUST_MIGRATION_V1,
            DEPLOYMENT_CONTROL_MIGRATION_V1
        ),
        Err(ControlSqliteError::ManifestDecode(_))
    ));
    assert!(render(
        MANIFEST_BYTES,
        &"0".repeat(64),
        PRINCIPAL_TRUST_MIGRATION_V1,
        DEPLOYMENT_CONTROL_MIGRATION_V1
    )
    .is_err());
    const WRONG_HEADERS: &str = "PRAGMA application_id=1; PRAGMA user_version=1; CREATE TABLE item(id INTEGER PRIMARY KEY) STRICT;";
    assert!(matches!(
        render(
            MANIFEST_BYTES,
            MANIFEST_SHA256,
            PRINCIPAL_TRUST_MIGRATION_V1,
            WRONG_HEADERS
        ),
        Err(ControlSqliteError::SourceInvariant(
            "migration application ID"
        ))
    ));
}

#[test]
fn generated_pair_only_writes_to_a_new_explicit_directory() {
    let generated = render(
        MANIFEST_BYTES,
        MANIFEST_SHA256,
        PRINCIPAL_TRUST_MIGRATION_V1,
        CHANGED_DEPLOYMENT,
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("new-output");
    write_new_output(&output, &generated).unwrap();
    assert_eq!(
        fs::read(output.join("pale-control-storage.v1.json")).unwrap(),
        generated.manifest
    );
    assert_eq!(
        fs::read(output.join("storage_manifest_pin.rs")).unwrap(),
        generated.pin_source
    );
    assert_eq!(fs::read_dir(&output).unwrap().count(), 2);
    assert_eq!(
        write_new_output(&output, &generated).unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(
        write_new_output(Path::new("relative-output"), &generated)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(
        fs::read(output.join("pale-control-storage.v1.json")).unwrap(),
        generated.manifest
    );
}
