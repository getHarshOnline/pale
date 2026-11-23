//! Isolated Package distribution for Zola projection tests.

use std::fs;
use std::path::PathBuf;

/// Build one exact synthetic Package containing the Zola projection graph.
///
/// The fixture owns its Package manifest, declarations, and bodies. It does
/// not copy the repository's accepted Package corpus or depend on unrelated
/// Package-lane source shapes.
pub(super) fn synthetic_contract_distribution() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().expect("create synthetic distribution root");
    let distribution = temp.path().join("distribution");
    let package = distribution.join("packages/example-zola");
    fs::create_dir_all(&package).expect("create synthetic Zola Package");

    fs::write(
        package.join("package.toml"),
        r#"[package]
id = "example.zola"
version = "1.0.0"
schema_version = 1
title = "Synthetic Zola projection"
description = "Exact test-only semantic graph for Zola projection."
compatible_compilers = ["pale-surface@1"]
sources = [
  { path = "operations.toml", role = "declaration" },
  { path = "roles.toml", role = "declaration" },
  { path = "stacks.toml", role = "declaration" },
  { path = "doctrine.toml", role = "declaration" },
  { path = "zola-judgment.md", role = "body" },
  { path = "zola-review.md", role = "body" },
  { path = "zola-build.md", role = "body" },
  { path = "site.md", role = "body" },
  { path = "zola-reviewer.md", role = "body" },
  { path = "zola-build-resolver.md", role = "body" },
]
resources = [
  { id = "zola-review", kind = "operation", definition_source = "operations.toml", exposure = "exported" },
  { id = "zola-build", kind = "operation", definition_source = "operations.toml", exposure = "exported" },
  { id = "site", kind = "role", definition_source = "roles.toml", exposure = "exported" },
  { id = "zola-reviewer", kind = "role", definition_source = "roles.toml", exposure = "exported" },
  { id = "zola-build-resolver", kind = "role", definition_source = "roles.toml", exposure = "exported" },
  { id = "zola", kind = "stack", definition_source = "stacks.toml", exposure = "exported" },
  { id = "rust", kind = "stack", definition_source = "stacks.toml", exposure = "exported" },
  { id = "zola-judgment", kind = "doctrine", definition_source = "doctrine.toml", exposure = "exported" },
]
"#,
    )
    .expect("write synthetic Zola Package manifest");
    fs::write(
        package.join("operations.toml"),
        r#"[operations]
schema_version = 1

[[operations.operation]]
id = "zola-review"
description = "Review one bounded Zola source surface."
doctrine = "zola-review.md"
roles = [{ scope = "local", resource_id = "zola-reviewer" }]
outputs = ["bounded Zola review evidence"]

[[operations.operation]]
id = "zola-build"
description = "Resolve one bounded Zola build failure."
doctrine = "zola-build.md"
roles = [{ scope = "local", resource_id = "zola-build-resolver" }]
outputs = ["bounded Zola build diagnosis"]
"#,
    )
    .expect("write synthetic Zola operations");
    fs::write(
        package.join("roles.toml"),
        r#"[roles]
schema_version = 1

[[roles.role]]
id = "site"
description = "Repository role for one synthetic site."
doctrine = "site.md"

[[roles.role]]
id = "zola-reviewer"
description = "Review one bounded Zola source surface."
doctrine = "zola-reviewer.md"
applies_to = [{ scope = "local", resource_id = "zola" }]

[[roles.role]]
id = "zola-build-resolver"
description = "Resolve one bounded Zola build failure."
doctrine = "zola-build-resolver.md"
applies_to = [{ scope = "local", resource_id = "zola" }]
"#,
    )
    .expect("write synthetic Zola roles");
    fs::write(
        package.join("stacks.toml"),
        r#"[stacks]
schema_version = 1

[[stacks.stack]]
id = "zola"
description = "Synthetic Zola stack."
doctrine_refs = [{ scope = "local", resource_id = "zola-judgment" }]

[[stacks.stack]]
id = "rust"
description = "Synthetic Rust stack."
"#,
    )
    .expect("write synthetic stack declarations");
    fs::write(
        package.join("doctrine.toml"),
        r#"[doctrine]
schema_version = 1

[[doctrine.entry]]
id = "zola-judgment"
description = "Check Zola source and output boundaries."
doctrine = "zola-judgment.md"
placement = "standalone_judgment"
"#,
    )
    .expect("write synthetic stack doctrine");

    for (name, body) in [
        (
            "zola-judgment.md",
            "# Zola judgment\n\nDistinguish site configuration from generated output.\n",
        ),
        (
            "zola-review.md",
            "# Zola review\n\nReview the supplied source evidence.\n",
        ),
        (
            "zola-build.md",
            "# Zola build\n\nDiagnose the supplied build evidence.\n",
        ),
        (
            "site.md",
            "# Site repository\n\nKeep source and generated output distinct.\n",
        ),
        (
            "zola-reviewer.md",
            "# Zola reviewer\n\nApply Zola-specific review judgment.\n",
        ),
        (
            "zola-build-resolver.md",
            "# Zola build resolver\n\nApply Zola-specific build diagnosis.\n",
        ),
    ] {
        fs::write(package.join(name), body).expect("write synthetic Package body");
    }
    (temp, distribution)
}
