//! Diagnosis retains independent observations without modifying selected inputs.

use super::{local_workspace, request, PaleApi};
use pale_sdk::LocalDoctorRequest;

fn inspect(selection: LocalDoctorRequest) -> serde_json::Value {
    let report = PaleApi::new()
        .unwrap()
        .inspect_local_doctor(selection)
        .unwrap();
    assert!(
        !report.checks_passed(),
        "unimplemented runtime evidence must not become readiness"
    );
    serde_json::from_slice(report.canonical_bytes()).unwrap()
}

fn check<'a>(report: &'a serde_json::Value, code: &str) -> &'a serde_json::Value {
    report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|check| check["code"] == code)
        .unwrap()
}

#[test]
fn doctor_compiles_declared_workspace_but_keeps_runtime_evidence_unavailable() {
    let (temporary, manifest) = local_workspace();
    let original = std::fs::read(&manifest).unwrap();
    let report = inspect(LocalDoctorRequest {
        workspace: Some(request(manifest.clone())),
        ..Default::default()
    });
    assert_eq!(
        check(&report, "workspace_compilation")["state"],
        "passed",
        "{report}"
    );
    assert_eq!(check(&report, "package_contract_lock")["state"], "passed");
    for code in [
        "resident_reachability",
        "hook_delivery",
        "toolchains",
        "distribution_release",
    ] {
        assert_eq!(check(&report, code)["state"], "unavailable");
    }
    assert_eq!(std::fs::read(manifest).unwrap(), original);
    assert_eq!(
        std::fs::read_dir(temporary.path().join("checkout"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(
        std::fs::read_dir(temporary.path().join("lineage"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn doctor_retains_build_results_after_workspace_failure_and_applies_exact_limit() {
    let (temporary, manifest) = local_workspace();
    std::fs::write(&manifest, "not valid TOML = [").unwrap();
    let build = temporary.path().canonicalize().unwrap().join("build");
    std::fs::create_dir(&build).unwrap();
    std::fs::write(build.join("artifact"), b"12345").unwrap();
    for (limit, state) in [(5, "passed"), (4, "failed")] {
        let report = inspect(LocalDoctorRequest {
            workspace: Some(request(manifest.clone())),
            build_roots: vec![build.to_str().unwrap().to_owned()],
            build_limit: Some(limit),
        });
        assert_eq!(check(&report, "workspace_compilation")["state"], "failed");
        assert_eq!(
            check(&report, "package_contract_lock")["state"],
            "unavailable"
        );
        assert_eq!(check(&report, "build_size")["state"], state);
        assert_eq!(report["builds"][0]["logical_bytes"], 5);
    }
    assert_eq!(std::fs::read(build.join("artifact")).unwrap(), b"12345");
}

#[test]
fn doctor_rejects_mismatched_distribution_lock_without_repairing_it() {
    let (temporary, manifest) = local_workspace();
    let lock = temporary.path().join("distribution-lock.toml");
    let original = std::fs::read_to_string(&lock).unwrap();
    let mut document: toml::Value = toml::from_str(&original).unwrap();
    document["package"][0]["content_digest"] = toml::Value::String("0".repeat(64));
    let altered = toml::to_string(&document).unwrap();
    assert_ne!(original, altered);
    std::fs::write(&lock, &altered).unwrap();
    let report = inspect(LocalDoctorRequest {
        workspace: Some(request(manifest)),
        ..Default::default()
    });
    assert_eq!(check(&report, "workspace_compilation")["state"], "failed");
    assert_eq!(std::fs::read_to_string(lock).unwrap(), altered);
}

#[test]
fn doctor_bounds_root_selection_before_scanning() {
    let report = inspect(LocalDoctorRequest {
        build_roots: vec!["/not-selected".to_owned(); 33],
        ..Default::default()
    });
    assert_eq!(check(&report, "build_roots")["state"], "failed");
    assert_eq!(report["builds"].as_array().unwrap().len(), 0);
}

#[test]
fn doctor_accepts_an_exact_authority_file_and_keeps_other_checks_after_a_missing_root() {
    let (temporary, manifest) = local_workspace();
    let root = temporary.path().canonicalize().unwrap();
    let authority = root.join("authority-file.md");
    std::fs::write(&authority, "Owned source").unwrap();
    let machine = root.join("machines/machine.toml");
    let mut document: toml::Value =
        toml::from_str(&std::fs::read_to_string(&machine).unwrap()).unwrap();
    document["authority_roots"]["authority"] =
        toml::Value::String(authority.to_str().unwrap().to_owned());
    std::fs::write(&machine, toml::to_string(&document).unwrap()).unwrap();
    let report = inspect(LocalDoctorRequest {
        workspace: Some(request(manifest.clone())),
        ..Default::default()
    });
    assert_eq!(
        check(&report, "authority_root")["state"],
        "passed",
        "{report}"
    );
    document["authority_roots"]["authority"] =
        toml::Value::String(root.join("missing").to_str().unwrap().to_owned());
    std::fs::write(&machine, toml::to_string(&document).unwrap()).unwrap();
    let report = inspect(LocalDoctorRequest {
        workspace: Some(request(manifest)),
        ..Default::default()
    });
    assert_eq!(check(&report, "authority_root")["state"], "failed");
    assert_eq!(check(&report, "repository_root")["state"], "passed");
    assert_eq!(std::fs::read_to_string(authority).unwrap(), "Owned source");
}

#[test]
fn doctor_failed_executable_handshake_does_not_hide_other_bindings() {
    let (temporary, manifest) = local_workspace();
    // This binds the test binary's real bytes, but it does not implement the
    // Pale executable identity protocol and must fail that bounded handshake.
    super::bind_current_executable(&temporary.path().canonicalize().unwrap());
    let report = inspect(LocalDoctorRequest {
        workspace: Some(request(manifest)),
        ..Default::default()
    });
    assert_eq!(
        check(&report, "pale_executable")["state"],
        "failed",
        "{report}"
    );
    assert_eq!(check(&report, "repository_root")["state"], "passed");
    assert_eq!(
        check(&report, "resident_reachability")["state"],
        "unavailable"
    );
}
