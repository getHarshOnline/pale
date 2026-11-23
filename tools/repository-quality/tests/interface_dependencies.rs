//! Repository-owned dependency boundaries for standalone interoperability leaves.

use std::collections::BTreeMap;
use std::fs;

#[path = "support/repository.rs"]
mod repository;

use repository::repository_root;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DependencyScope {
    Runtime,
    Development,
    Build,
    TargetRuntime,
    TargetDevelopment,
    TargetBuild,
}

#[derive(Clone, Debug, Default)]
struct DependencySpecification {
    package: Option<String>,
    version: Option<String>,
    path: Option<String>,
    git: Option<String>,
    optional: Option<bool>,
}

impl DependencySpecification {
    fn decode(value: &toml::Value) -> Self {
        match value {
            toml::Value::String(version) => Self {
                version: Some(version.clone()),
                ..Self::default()
            },
            toml::Value::Table(fields) => Self {
                package: string_field(fields, "package"),
                version: string_field(fields, "version"),
                path: string_field(fields, "path"),
                git: string_field(fields, "git"),
                optional: fields.get("optional").and_then(toml::Value::as_bool),
            },
            _ => Self::default(),
        }
    }

    fn overlay(&mut self, value: &toml::Value) {
        let local = Self::decode(value);
        self.package = local.package.or(self.package.take());
        self.version = local.version.or(self.version.take());
        self.path = local.path.or(self.path.take());
        self.git = local.git.or(self.git.take());
        self.optional = local.optional.or(self.optional);
    }
}

#[derive(Debug)]
struct InternalDependency {
    alias: String,
    package: String,
    scope: DependencyScope,
    table: String,
    specification: DependencySpecification,
}

fn string_field(fields: &toml::Table, name: &str) -> Option<String> {
    fields
        .get(name)
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
}

fn inherited_dependencies(manifest: &toml::Table) -> BTreeMap<String, DependencySpecification> {
    manifest
        .get("workspace")
        .and_then(toml::Value::as_table)
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(toml::Value::as_table)
        .into_iter()
        .flat_map(toml::Table::iter)
        .map(|(alias, value)| (alias.clone(), DependencySpecification::decode(value)))
        .collect()
}

fn collect_table(
    parent: &toml::Table,
    name: &str,
    scope: DependencyScope,
    table_name: String,
    inherited: &BTreeMap<String, DependencySpecification>,
    output: &mut Vec<InternalDependency>,
) {
    let Some(table) = parent.get(name) else {
        return;
    };
    let table = table
        .as_table()
        .unwrap_or_else(|| panic!("Cargo `{table_name}` must be a table"));
    for (alias, value) in table {
        let inherits = value
            .as_table()
            .and_then(|fields| fields.get("workspace"))
            .and_then(toml::Value::as_bool)
            .unwrap_or(false);
        let mut specification = inherits
            .then(|| inherited.get(alias).cloned())
            .flatten()
            .unwrap_or_default();
        specification.overlay(value);
        let package = specification
            .package
            .clone()
            .unwrap_or_else(|| alias.clone());
        if package.starts_with("pale-") {
            output.push(InternalDependency {
                alias: alias.clone(),
                package,
                scope,
                table: table_name.clone(),
                specification,
            });
        }
    }
}

fn internal_dependencies(manifest: &toml::Table) -> Vec<InternalDependency> {
    let inherited = inherited_dependencies(manifest);
    let mut dependencies = Vec::new();
    for (name, scope) in [
        ("dependencies", DependencyScope::Runtime),
        ("dev-dependencies", DependencyScope::Development),
        ("build-dependencies", DependencyScope::Build),
    ] {
        collect_table(
            manifest,
            name,
            scope,
            name.to_owned(),
            &inherited,
            &mut dependencies,
        );
    }
    if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
        for (selector, target) in targets {
            let target = target
                .as_table()
                .unwrap_or_else(|| panic!("Cargo target `{selector}` must be a table"));
            for (name, scope) in [
                ("dependencies", DependencyScope::TargetRuntime),
                ("dev-dependencies", DependencyScope::TargetDevelopment),
                ("build-dependencies", DependencyScope::TargetBuild),
            ] {
                collect_table(
                    target,
                    name,
                    scope,
                    format!("target.{selector}.{name}"),
                    &inherited,
                    &mut dependencies,
                );
            }
        }
    }
    dependencies
}

fn assert_exact_internal_runtime_dependency(manifest_path: &str, expected_package: &str) {
    let path = repository_root().join(manifest_path);
    let manifest = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
        .parse::<toml::Table>()
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
    let dependencies = internal_dependencies(&manifest);
    let build_dependencies = dependencies
        .iter()
        .filter(|dependency| {
            matches!(
                dependency.scope,
                DependencyScope::Build | DependencyScope::TargetBuild
            )
        })
        .collect::<Vec<_>>();
    assert!(
        build_dependencies.is_empty(),
        "{} must not compile Pale internals into a build script: {build_dependencies:#?}",
        path.display(),
    );
    let runtime_dependencies = dependencies
        .iter()
        .filter(|dependency| {
            matches!(
                dependency.scope,
                DependencyScope::Runtime | DependencyScope::TargetRuntime
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        runtime_dependencies.len(),
        1,
        "{} must declare exactly one internal Pale runtime dependency: {runtime_dependencies:#?}",
        path.display(),
    );
    let dependency = runtime_dependencies[0];
    assert_eq!(dependency.package, expected_package);
    assert_eq!(
        dependency.alias,
        dependency.package,
        "{} aliases internal package `{}` as `{}` in [{}]",
        path.display(),
        dependency.package,
        dependency.alias,
        dependency.table,
    );
    assert_eq!(dependency.scope, DependencyScope::Runtime);
    assert_eq!(dependency.specification.version.as_deref(), Some("=0.0.1"));
    assert_eq!(dependency.specification.path, None);
    assert_eq!(dependency.specification.git, None);
    assert_ne!(dependency.specification.optional, Some(true));
}

fn assert_complete_runtime_dependency_set(manifest_path: &str, expected: &[&str]) {
    let path = repository_root().join(manifest_path);
    let manifest = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
        .parse::<toml::Table>()
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
    let actual = manifest
        .get("dependencies")
        .and_then(toml::Value::as_table)
        .into_iter()
        .flat_map(toml::Table::keys)
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        expected,
        "{} dependency wall drifted",
        path.display()
    );
    for forbidden in ["dev-dependencies", "build-dependencies", "target"] {
        assert!(
            !manifest.contains_key(forbidden),
            "{} unexpectedly owns a [{forbidden}] dependency scope",
            path.display()
        );
    }
}

#[test]
fn standalone_interface_leaves_reach_engine_semantics_through_one_facade() {
    for (manifest, dependency) in [
        ("interfaces/cli/Cargo.toml", "pale-sdk"),
        ("interfaces/mcp/Cargo.toml", "pale-cli"),
        ("interfaces/lsp/Cargo.toml", "pale-sdk"),
        ("interfaces/acp/Cargo.toml", "pale-sdk"),
        ("interfaces/a2a/Cargo.toml", "pale-sdk"),
        ("sdk/js/Cargo.toml", "pale-sdk"),
        ("sdk/go/Cargo.toml", "pale-sdk"),
        ("sdk/python/Cargo.toml", "pale-sdk"),
        ("sdk/wasm/Cargo.toml", "pale-sdk"),
    ] {
        assert_exact_internal_runtime_dependency(manifest, dependency);
    }
}

#[test]
fn resident_composition_owns_only_the_public_application_and_native_transport_boundary() {
    // A resident composition root hosts the service; it is not a translating
    // client leaf. Keep its native owners explicit and prevent a second SDK,
    // core-policy, contract-parser or kernel dependency from creeping in.
    let path = repository_root().join("interfaces/resident/Cargo.toml");
    let manifest = fs::read_to_string(&path)
        .unwrap()
        .parse::<toml::Table>()
        .unwrap();
    let dependencies = internal_dependencies(&manifest);
    let runtime = dependencies
        .iter()
        .filter(|dependency| {
            matches!(
                dependency.scope,
                DependencyScope::Runtime
                    | DependencyScope::TargetRuntime
                    | DependencyScope::Build
                    | DependencyScope::TargetBuild
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(runtime.len(), 3);
    for (package, source) in [
        ("pale-adapter-uds", "../../adapters/uds"),
        ("pale-application", "../../crates/pale-application"),
        ("pale-ports", "../../crates/pale-ports"),
    ] {
        let dependency = runtime
            .iter()
            .find(|dependency| dependency.package == package)
            .expect("resident owner dependency");
        assert_eq!(dependency.alias, package);
        assert_eq!(dependency.scope, DependencyScope::Runtime);
        assert_eq!(dependency.specification.path.as_deref(), Some(source));
        assert_eq!(dependency.specification.git, None);
        assert_ne!(dependency.specification.optional, Some(true));
    }
}

#[test]
fn shared_agent_host_uses_only_sdk_at_runtime() {
    let manifest = fs::read_to_string(repository_root().join("integrations/agents/Cargo.toml"))
        .unwrap()
        .parse::<toml::Table>()
        .unwrap();
    let dependencies = internal_dependencies(&manifest);
    let runtime = dependencies
        .iter()
        .filter(|dependency| {
            matches!(
                dependency.scope,
                DependencyScope::Runtime
                    | DependencyScope::TargetRuntime
                    | DependencyScope::Build
                    | DependencyScope::TargetBuild
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(runtime.len(), 1);
    for (package, path, scope) in [
        ("pale-sdk", "../../sdk/rust", DependencyScope::TargetRuntime),
        (
            "pale-adapter-uds",
            "../../adapters/uds",
            DependencyScope::TargetDevelopment,
        ),
        (
            "pale-ports",
            "../../crates/pale-ports",
            DependencyScope::TargetDevelopment,
        ),
    ] {
        let dependency = dependencies
            .iter()
            .find(|dependency| dependency.package == package)
            .unwrap();
        assert_eq!(dependency.alias, package);
        assert_eq!(dependency.scope, scope);
        assert_eq!(dependency.specification.path.as_deref(), Some(path));
        assert_eq!(dependency.specification.git, None);
        assert_ne!(dependency.specification.optional, Some(true));
    }
}

#[test]
fn a2a_interface_uses_facade_and_adapter_pins_protocol_owner() {
    assert_complete_runtime_dependency_set("interfaces/a2a/Cargo.toml", &["pale-sdk"]);

    let manifest_path = "adapters/a2a/Cargo.toml";
    assert_complete_runtime_dependency_set(
        manifest_path,
        &[
            "a2a",
            "pale-core",
            "pale-ports",
            "reqwest",
            "serde",
            "serde_json",
            "tokio",
        ],
    );

    let path = repository_root().join(manifest_path);
    let manifest = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
        .parse::<toml::Table>()
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
    let protocol = manifest["dependencies"]["a2a"]
        .as_table()
        .expect("A2A protocol dependency must be explicit");
    assert_eq!(string_field(protocol, "package").as_deref(), Some("a2a-lf"));
    assert_eq!(string_field(protocol, "version").as_deref(), Some("=0.3.0"));
    assert_eq!(string_field(protocol, "path"), None);
    assert_eq!(string_field(protocol, "git"), None);
    assert_ne!(
        protocol.get("optional").and_then(toml::Value::as_bool),
        Some(true)
    );
}
