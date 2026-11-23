//! Build-script entry point for the sealed Rust-first contract bundle.
//!
//! The build regenerates all portable schemas from their Rust producers,
//! verifies the checked-in bundle byte-for-byte, then embeds only the verified
//! bytes and descriptor table beneath Cargo's output directory.

#[path = "build/embedding.rs"]
mod embedding;

/// Exact source grammars exposed to configuration-authoring clients.
///
/// Immutable Package declarations are navigation targets. Resident, journal,
/// receipt, and native-runtime contracts are intentionally absent because they
/// are not authored as Pale Workspace configuration.
const AUTHORING_SCHEMA_IDS: [&str; 23] = [
    "agent-surface-package.v1",
    "capabilities.v1",
    "capability-procedure.v1",
    "distribution-lock.v1",
    "doctrine-entry.v1",
    "governing-order.v1",
    "pale-workspace.v1",
    "machine-binding.v1",
    "operation.v1",
    "organizational-translation.v1",
    "package-data.v1",
    "package-page.v1",
    "policy-family.v1",
    "responsibility-derivation.v1",
    "role.v1",
    "run-mode.v1",
    "stack.v1",
    "workspace-composition-source.v1",
    "workspace-language-primitives.v1",
    "workspace-module.v1",
    "workspace-module-source.v1",
    "workspace-resolution-lock.v1",
    "workspace-universe-lock.v1",
];

fn main() -> Result<(), String> {
    let manifest_dir = std::path::PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR")
            .ok_or_else(|| "Cargo did not supply CARGO_MANIFEST_DIR".to_string())?,
    );
    let bundle_dir = manifest_dir.join("../../contracts/bundle/agent-surface.v1");
    let manifest_path = bundle_dir.join(pale_contract_codegen::BUNDLE_MANIFEST_NAME);
    let manifest = pale_contract_codegen::read_descriptor_manifest(&manifest_path)
        .map_err(|error| format!("descriptor manifest rejected the build: {error}"))?;
    let authoring_schemas = pale_contract_codegen::generate_authoring_schemas(
        manifest.schema_base_uri(),
        AUTHORING_SCHEMA_IDS,
    )
    .map_err(|error| format!("authoring-schema generation failed: {error}"))?;
    let bundle = pale_contract_codegen::generate_bundle(manifest)
        .map_err(|error| format!("Rust-first contract generation failed: {error}"))?;
    pale_contract_codegen::verify_bundle(&bundle, &bundle_dir)
        .map_err(|error| format!("checked-in contract bundle is stale or invalid: {error}"))?;

    println!("cargo:rerun-if-changed={}", bundle_dir.display());
    println!("cargo:rerun-if-changed={}", manifest_path.display());
    for descriptor in bundle.schema_descriptors() {
        println!(
            "cargo:rerun-if-changed={}",
            bundle_dir.join(descriptor.source_path()).display()
        );
    }
    let out_dir = std::path::PathBuf::from(
        std::env::var_os("OUT_DIR").ok_or_else(|| "Cargo did not supply OUT_DIR".to_string())?,
    );
    embedding::write_outputs(&bundle, &authoring_schemas, &out_dir)
        .map_err(|error| format!("schema embedding failed: {error}"))?;
    Ok(())
}
