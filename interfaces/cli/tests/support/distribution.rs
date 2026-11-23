//! Self-contained Package distribution used by CLI boundary tests.
//!
//! The accepted Package corpus is an independently owned input and may retain a
//! different editorial frontier. CLI tests therefore compile this minimal,
//! current-contract distribution instead of copying or rewriting Package bytes.

use std::path::{Path, PathBuf};

/// Write the exact two-Package distribution selected by CLI fixtures.
///
/// # Panics
///
/// Panics when the isolated test distribution cannot be created or written.
pub(crate) fn lay_out(root: &Path) -> PathBuf {
    let packages = root.join("packages");
    write_package(
        &packages.join("governance"),
        GOVERNANCE_MANIFEST,
        GOVERNANCE_MEMBERS,
    );
    write_package(
        &packages.join("engineering"),
        ENGINEERING_MANIFEST,
        ENGINEERING_MEMBERS,
    );
    root.to_path_buf()
}

/// Render the exact lock for the synthetic distribution's current bytes.
///
/// The production inspector remains the sole owner of Package release-content
/// identity. This helper only projects its result into the lock shape consumed
/// by CLI journeys.
///
/// # Panics
///
/// Panics when the synthetic distribution cannot be inspected or serialized.
pub(crate) fn render_lock(root: &Path) -> String {
    fn collect(root: &Path, directory: &Path, output: &mut Vec<pale_sdk::CompilationDocument>) {
        let mut entries = std::fs::read_dir(directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                collect(root, &path, output);
            } else {
                output.push(pale_sdk::CompilationDocument::new(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_owned(),
                    std::fs::read(path).unwrap(),
                ));
            }
        }
    }

    let packages_root = root.join("packages");
    let mut documents = Vec::new();
    collect(&packages_root, &packages_root, &mut documents);
    let info = pale_sdk::PaleApi::new()
        .expect("initialize Package inspection API")
        .inspect_package_distribution(pale_sdk::PackageDistributionRequest::new(documents))
        .expect("inspect synthetic Package distribution");
    info.lockfile_toml().to_owned()
}

fn write_package(root: &Path, manifest: &str, members: &[(&str, &str)]) {
    std::fs::create_dir_all(root).expect("create synthetic Package root");
    std::fs::write(root.join("package.toml"), manifest).expect("write synthetic Package manifest");
    for (relative, contents) in members {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create synthetic Package member parent");
        }
        std::fs::write(path, contents).expect("write synthetic Package member");
    }
}

const GOVERNANCE_MANIFEST: &str = r#"[package]
id = "pale.governance"
version = "0.1.0"
schema_version = 1
title = "Synthetic governance"
description = "Test-only governance policy-family owners."
compatible_compilers = ["pale-surface@1"]
sources = [
  { path = "authority-resolver.toml", role = "declaration" },
  { path = "doctrine/authority-resolver.md", role = "body" },
  { path = "execution-contract.toml", role = "declaration" },
  { path = "doctrine/execution-contract.md", role = "body" },
]
resources = [
  { id = "authority-resolver", kind = "policy_family", definition_source = "authority-resolver.toml", exposure = "exported" },
  { id = "execution-contract", kind = "policy_family", definition_source = "execution-contract.toml", exposure = "exported" },
]
"#;

const GOVERNANCE_MEMBERS: &[(&str, &str)] = &[
    (
        "authority-resolver.toml",
        "id = 'authority-resolver'\ntitle = 'Authority resolver'\npurpose = 'Resolve one bounded authority root.'\nbody = 'doctrine/authority-resolver.md'\n",
    ),
    (
        "doctrine/authority-resolver.md",
        "# Authority resolver\n\nResolve only the authority selected by the fixture.\n",
    ),
    (
        "execution-contract.toml",
        "id = 'execution-contract'\ntitle = 'Execution contract'\npurpose = 'Preserve the test execution boundary.'\nbody = 'doctrine/execution-contract.md'\n",
    ),
    (
        "doctrine/execution-contract.md",
        "# Execution contract\n\nKeep request, admission, and effect evidence distinct.\n",
    ),
];

const ENGINEERING_MANIFEST: &str = r#"[package]
id = "pale.engineering"
version = "0.1.0"
schema_version = 1
title = "Synthetic engineering"
description = "Test-only operations, roles, modes, and policy owners."
compatible_compilers = ["pale-surface@1"]
dependencies = [{ id = "pale.governance", version = "0.1.0" }]
sources = [
  { path = "operations.toml", role = "declaration" },
  { path = "doctrine/repo-scan.md", role = "body" },
  { path = "doctrine/verify-project.md", role = "body" },
  { path = "doctrine/spec-commit.md", role = "body" },
  { path = "doctrine/quality-gate.md", role = "body" },
  { path = "doctrine/path-inventory.md", role = "body" },
  { path = "roles.toml", role = "declaration" },
  { path = "doctrine/engine.md", role = "body" },
  { path = "doctrine/theme.md", role = "body" },
  { path = "doctrine/code-reviewer.md", role = "body" },
  { path = "run-modes.toml", role = "declaration" },
  { path = "doctrine/standard.md", role = "body" },
  { path = "doctrine/lite.md", role = "body" },
  { path = "impact-analysis.toml", role = "declaration" },
  { path = "doctrine/impact-analysis.md", role = "body" },
  { path = "verification.toml", role = "declaration" },
  { path = "doctrine/verification.md", role = "body" },
  { path = "review.toml", role = "declaration" },
  { path = "doctrine/review.md", role = "body" },
  { path = "project-policy.toml", role = "declaration" },
  { path = "repository-verification.toml", role = "declaration" },
]
resources = [
  { id = "repo-scan", kind = "operation", definition_source = "operations.toml", exposure = "exported" },
  { id = "verify-project", kind = "operation", definition_source = "operations.toml", exposure = "exported" },
  { id = "spec-commit", kind = "operation", definition_source = "operations.toml", exposure = "exported" },
  { id = "quality-gate", kind = "operation", definition_source = "operations.toml", exposure = "exported" },
  { id = "path-inventory", kind = "operation", definition_source = "operations.toml", exposure = "exported" },
  { id = "engine", kind = "role", definition_source = "roles.toml", exposure = "exported" },
  { id = "theme", kind = "role", definition_source = "roles.toml", exposure = "exported" },
  { id = "code-reviewer", kind = "role", definition_source = "roles.toml", exposure = "exported" },
  { id = "standard", kind = "run_mode", definition_source = "run-modes.toml", exposure = "exported" },
  { id = "lite", kind = "run_mode", definition_source = "run-modes.toml", exposure = "exported" },
  { id = "impact_analysis", kind = "policy_family", definition_source = "impact-analysis.toml", exposure = "exported" },
  { id = "verification", kind = "policy_family", definition_source = "verification.toml", exposure = "exported" },
  { id = "review", kind = "policy_family", definition_source = "review.toml", exposure = "exported" },
  { id = "project-policy", kind = "capability_policy", definition_source = "project-policy.toml", exposure = "exported" },
  { id = "repository-verification", kind = "tool_recipe", definition_source = "repository-verification.toml", exposure = "exported" },
]
"#;

const ENGINEERING_MEMBERS: &[(&str, &str)] = &[
    (
        "operations.toml",
        r#"[operations]
schema_version = 1

[[operations.operation]]
id = "repo-scan"
description = "Inspect one bounded repository without mutating it."
doctrine = "doctrine/repo-scan.md"
outputs = ["bounded source observations"]
capability_policy = { scope = "local", resource_id = "project-policy" }
capability_requirements = [
  { id = "scan-source", description = "Read the repository sources selected by the request.", capability = "local-inspection", effect_class = "read", policy = { kind = "required_block" }, failure_posture = "fail_closed" },
]

[[operations.operation]]
id = "verify-project"
description = "Run the exact repository verifier after an admitted protected write."
doctrine = "doctrine/verify-project.md"
outputs = ["repository-verification evidence"]
capability_policy = { scope = "local", resource_id = "project-policy" }
capability_requirements = [
  { id = "verify-repository", description = "Run the repository-selected verifier and retain its bounded result.", capability = "project-test", recipe = { scope = "local", resource_id = "repository-verification" }, effect_class = "execute", policy = { kind = "required_block" }, failure_posture = "fail_closed" },
]

[[operations.operation]]
id = "spec-commit"
description = "Create one explicitly authorized local source-control commit."
doctrine = "doctrine/spec-commit.md"
outputs = ["local source-control commit"]
capability_policy = { scope = "local", resource_id = "project-policy" }
capability_requirements = [
  { id = "stage-and-commit", description = "Stage the authorized local change set and create its commit.", capability = "source-control-local-change", effect_class = "write", policy = { kind = "required_block" }, failure_posture = "fail_closed" },
]

[[operations.operation]]
id = "quality-gate"
description = "Run a repository-selected format check and report any drift."
doctrine = "doctrine/quality-gate.md"
outputs = ["format-check evidence"]
capability_policy = { scope = "local", resource_id = "project-policy" }
capability_requirements = [
  { id = "repair-format-drift", description = "Correct format drift only when a separate write provider is admitted.", capability = "project-format-write", effect_class = "write", policy = { kind = "optional", fallback = { guidance = "Report the drift without changing repository files." } }, failure_posture = "fail_closed" },
  { id = "run-format-check", description = "Run the repository-selected non-mutating format check.", capability = "project-format-check", effect_class = "execute", policy = { kind = "required_block" }, failure_posture = "fail_closed" },
]

[[operations.operation]]
id = "path-inventory"
description = "Compute a deterministic inventory of confined repository paths."
doctrine = "doctrine/path-inventory.md"
outputs = ["confined path inventory"]
capability_policy = { scope = "local", resource_id = "project-policy" }
capability_requirements = [
  { id = "normalize-paths", description = "Run the selected deterministic path-inventory action.", capability = "path-inventory-compute", effect_class = "compute", policy = { kind = "required_block" }, failure_posture = "fail_closed" },
]
"#,
    ),
    (
        "doctrine/repo-scan.md",
        "# Repository scan\n\nInspect the bounded repository without mutation.\n",
    ),
    (
        "doctrine/verify-project.md",
        "# Verify project\n\nRun only the repository-selected verifier.\n",
    ),
    (
        "doctrine/spec-commit.md",
        "# Specification commit\n\nCommit only the exact source-control change set authorized for the invocation.\n",
    ),
    (
        "doctrine/quality-gate.md",
        "# Quality gate\n\nRun the selected format check before making any optional repair.\n",
    ),
    (
        "doctrine/path-inventory.md",
        "# Path inventory\n\nNormalize only confined paths through the selected native action.\n",
    ),
    (
        "roles.toml",
        r#"[roles]
schema_version = 1

[[roles.role]]
id = "engine"
description = "Synthetic engine repository role."
doctrine = "doctrine/engine.md"

[[roles.role]]
id = "theme"
description = "Synthetic theme repository role."
doctrine = "doctrine/theme.md"

[[roles.role]]
id = "code-reviewer"
description = "Review one bounded source change."
doctrine = "doctrine/code-reviewer.md"
"#,
    ),
    (
        "doctrine/engine.md",
        "# Engine role\n\nOwn only the synthetic Engine boundary.\n",
    ),
    (
        "doctrine/theme.md",
        "# Theme role\n\nOwn only the synthetic Theme boundary.\n",
    ),
    (
        "doctrine/code-reviewer.md",
        "# Code reviewer\n\nReview the bounded change against its contract.\n",
    ),
    (
        "run-modes.toml",
        r#"[run_modes]
schema_version = 1

[[run_modes.mode]]
id = "standard"
axis = "intensity"
description = "Test-only default intensity."
doctrine = "doctrine/standard.md"
scope = "runtime"

[[run_modes.mode]]
id = "lite"
axis = "intensity"
description = "Test-only minimal intensity."
doctrine = "doctrine/lite.md"
aliases = ["/lite"]
scope = "runtime"
"#,
    ),
    (
        "doctrine/standard.md",
        "# Standard mode\n\nUse the ordinary test reasoning path.\n",
    ),
    (
        "doctrine/lite.md",
        "# Lite mode\n\nUse the smallest sufficient reasoning path.\n",
    ),
    (
        "impact-analysis.toml",
        "id = 'impact_analysis'\ntitle = 'Impact analysis'\npurpose = 'Trace bounded impact.'\nbody = 'doctrine/impact-analysis.md'\n",
    ),
    (
        "doctrine/impact-analysis.md",
        "# Impact analysis\n\nTrace only the affected test boundary.\n",
    ),
    (
        "verification.toml",
        "id = 'verification'\ntitle = 'Verification'\npurpose = 'Verify selected behavior.'\nbody = 'doctrine/verification.md'\n",
    ),
    (
        "doctrine/verification.md",
        "# Verification\n\nReport only evidence established by the test.\n",
    ),
    (
        "review.toml",
        "id = 'review'\ntitle = 'Review'\npurpose = 'Review one bounded change.'\nbody = 'doctrine/review.md'\n",
    ),
    (
        "doctrine/review.md",
        "# Review\n\nKeep semantic and mechanical owners distinct.\n",
    ),
    (
        "project-policy.toml",
        "permitted_capabilities = ['local-inspection', 'path-inventory-compute', 'project-format-check', 'project-format-write', 'project-test', 'source-control-local-change']\nforbidden_capabilities = []\npermitted_effects = ['compute', 'execute', 'read', 'write']\nforbidden_effects = []\n",
    ),
    (
        "repository-verification.toml",
        "description = 'Run the repository-selected verifier and retain its bounded result.'\ncapability = 'project-test'\ntool_contract = 'repository-verifier.v1'\ntool_operation = 'verify'\neffect_class = 'execute'\ninput_mapping.repository_id = '$.repository_id'\nevidence.required_fields = ['/exit_code']\nevidence.proof_ceiling = 'repository-verification'\nevidence.repository_verifier = true\n",
    ),
];
