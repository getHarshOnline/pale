//! Typed failures from bounded Package capture, admission, and graph sealing.
//!
//! [`PackageError`] separates source confinement, schema admission, copyright
//! metadata, declaration identity, dependency closure, link qualification, and
//! occurrence-selection failures so callers do not infer semantic validity from
//! successful parsing alone. [`PackageDiagnostics`] retains independent graph
//! failures without flattening their typed causes.

use std::path::PathBuf;

use pale_core::records::LogicalRepositoryIdV1;

/// Maximum independently actionable Package diagnostics retained per stage.
pub const MAX_PACKAGE_DIAGNOSTICS: usize = 128;

/// Closed reasons why a Markdown body cannot enter Package custody.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, thiserror::Error)]
pub enum BodyError {
    /// The path is absent from the manifest or is not declared with body role.
    #[error("body path is not an exact manifest-declared body member")]
    UndeclaredBody,
    /// A second semantic resource claims an already owned body member.
    #[error("one body source is claimed by more than one semantic resource")]
    ReusedBody,
}

/// Why an authored local Markdown destination cannot become a host route.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, thiserror::Error)]
pub enum MarkdownLinkViolation {
    /// The destination is not confined to the owning Package.
    #[error("local destination escapes the owning Package or uses an unsupported path form")]
    UnconfinedDestination,
    /// No exact body owner exists at the authored destination.
    #[error("local destination is not a manifest-owned resource body")]
    MissingTarget,
    /// Prose cannot select a dependency missing from the typed declaration.
    #[error("local destination is not an explicitly declared resource relationship")]
    UndeclaredRelationship,
    /// The target has no independent generated doctrine skill.
    #[error("local destination is not standalone doctrine")]
    UnrenderedTarget,
    /// No safe inline destination span was established by the Markdown adapter.
    #[error("local resource links require an inline destination in the document body")]
    UnsupportedSyntax,
    /// The page exists but has no heading or explicit anchor at the requested fragment.
    #[error("local destination names a missing heading or anchor")]
    MissingHeading,
}

/// Closed reasons why an authored Package link cannot be qualified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, thiserror::Error)]
pub enum ResourceLinkViolation {
    /// An external form names the current Package instead of a dependency.
    #[error("self-owned resources must use local scope")]
    SelfExternal,
    /// The external owner is not one of the manifest's direct exact dependencies.
    #[error("external owner is not an exact direct dependency")]
    NotDirectDependency,
    /// No selected declaration has the qualified target identity.
    #[error("target is absent from the selected exact Package closure")]
    MissingTarget,
    /// Another Package exposes no public interface for the target.
    #[error("external target is not exported")]
    InternalTarget,
    /// The target exists but does not have the relation's required semantic kind.
    #[error("target has the wrong semantic resource kind")]
    WrongKind,
    /// A reference without a required kind names several uses of one page.
    #[error("reference needs a kind because the page has multiple uses")]
    AmbiguousTarget,
}

/// Closed semantic relation carried by an authored Package link.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, thiserror::Error)]
pub enum ResourceLinkRelation {
    /// Doctrine inheritance.
    #[error("doctrine extends")]
    DoctrineExtends,
    /// Required doctrine dependency.
    #[error("doctrine requires")]
    DoctrineRequires,
    /// Informational doctrine relationship.
    #[error("doctrine see-also")]
    DoctrineSeeAlso,
    /// Operation-archetype inheritance.
    #[error("operation-archetype extends")]
    OperationArchetypeExtends,
    /// Doctrine used by an operation archetype.
    #[error("operation-archetype doctrine")]
    OperationArchetypeDoctrine,
    /// Operation inheritance.
    #[error("operation extends")]
    OperationExtends,
    /// Doctrine used by an operation.
    #[error("operation doctrine")]
    OperationDoctrine,
    /// Role used by an operation.
    #[error("operation role")]
    OperationRole,
    /// Portable capability ceiling used by an operation.
    #[error("operation capability policy")]
    OperationCapabilityPolicy,
    /// Role-archetype inheritance.
    #[error("role-archetype extends")]
    RoleArchetypeExtends,
    /// Doctrine used by a role archetype.
    #[error("role-archetype doctrine")]
    RoleArchetypeDoctrine,
    /// Role inheritance.
    #[error("role extends")]
    RoleExtends,
    /// Inherited lens used by a role.
    #[error("role inherited lens")]
    RoleInheritedLens,
    /// Doctrine used by a role.
    #[error("role doctrine")]
    RoleDoctrine,
    /// Stack used by a role.
    #[error("role stack")]
    RoleStack,
    /// Stack inheritance.
    #[error("stack extends")]
    StackExtends,
    /// Doctrine used by a stack.
    #[error("stack doctrine")]
    StackDoctrine,
    /// Role doctrine used by a stack.
    #[error("stack role doctrine")]
    StackRoleDoctrine,
    /// Run-mode inheritance.
    #[error("run-mode extends")]
    RunModeExtends,
    /// Recipe selected by one capability requirement.
    #[error("operation requirement recipe")]
    OperationRequirementRecipe,
    /// Recipe selected by one capability fallback.
    #[error("operation fallback recipe")]
    OperationFallbackRecipe,
    /// Scope guarded by one governing constraint.
    #[error("governing constraint scope")]
    GoverningConstraintScope,
    /// Subject narrowed by one governing constraint.
    #[error("governing constraint subject")]
    GoverningConstraintSubject,
    /// Capability policy applied by one governing constraint.
    #[error("governing constraint policy")]
    GoverningConstraintPolicy,
    /// Scope guarded by one governing obligation.
    #[error("governing obligation scope")]
    GoverningObligationScope,
    /// Subject carrying one governing obligation.
    #[error("governing obligation subject")]
    GoverningObligationSubject,
    /// Doctrine or rule required as a governing obligation.
    #[error("governing obligation")]
    GoverningObligation,
    /// Scope guarded by an exclusive-authority claim.
    #[error("governing exclusive scope")]
    GoverningExclusiveScope,
    /// Authority named by an exclusive-authority claim.
    #[error("governing exclusive authority")]
    GoverningExclusiveAuthority,
    /// Scope guarded by a narrowing relation.
    #[error("governing narrowing scope")]
    GoverningNarrowingScope,
    /// Narrower resource in a governing relation.
    #[error("governing narrower resource")]
    GoverningNarrower,
    /// Broader resource in a governing relation.
    #[error("governing broader resource")]
    GoverningBroader,
    /// Scope guarded by an ordering relation.
    #[error("governing order scope")]
    GoverningOrderScope,
    /// Prerequisite resource in a governing ordering relation.
    #[error("governing order prerequisite")]
    GoverningOrderPrerequisite,
    /// Dependent resource in a governing ordering relation.
    #[error("governing order dependent")]
    GoverningOrderDependent,
}

/// Closed reasons why an operation procedure contradicts Package policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, thiserror::Error)]
pub enum OperationProcedureViolation {
    /// A zero-requirement operation points at a ceiling it cannot consume.
    #[error("semantic-only operation carries an unused capability ceiling")]
    UnusedCapabilityCeiling,
    /// A capability-bearing operation has no Package-owned portable ceiling.
    #[error("capability-bearing operation has no Package-owned ceiling")]
    MissingCapabilityCeiling,
    /// The qualified ceiling is absent from the admitted policy inventory.
    #[error("qualified capability ceiling is absent from the admitted policy inventory")]
    MissingPolicy,
    /// A declared requirement exceeds its portable Package ceiling.
    #[error("requirement exceeds its Package-owned capability ceiling")]
    RequirementExceedsPolicy,
    /// A qualified recipe is absent from the admitted procedure inventory.
    #[error("qualified recipe is absent from the admitted procedure inventory")]
    MissingRecipe,
    /// A recipe's capability or effect differs from the requirement selecting it.
    #[error("selected recipe capability/effect differs from its requirement")]
    RecipeMismatch,
}

/// Closed reasons why a workspace selector cannot seed semantic closure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, thiserror::Error)]
pub enum ResourceSelectorViolation {
    /// The selector names a Package outside the exact selected release closure.
    #[error("Package owner is outside the selected exact release closure")]
    OwnerOutsideClosure,
    /// The selector names no admitted resource at its qualified identity.
    #[error("exact resource is absent from the selected Package closure")]
    MissingResource,
    /// The selector attempts to use a Package-internal resource as an occurrence.
    #[error("internal Package resource cannot be a workspace occurrence seed")]
    InternalResource,
    /// The selected declaration has another semantic kind.
    #[error("exact resource has the wrong semantic kind")]
    WrongKind,
    /// A previously qualified positive dependency is absent while closing the graph.
    #[error("qualified dependency disappeared before semantic closure")]
    MissingQualifiedDependency,
    /// An untyped governing link names a page with more than one use.
    #[error("governing reference needs a kind because the page has multiple uses")]
    AmbiguousQualifiedDependency,
}

/// Closed failures of the sealed Package semantic-IR boundary.
#[derive(Debug, thiserror::Error)]
pub enum PackageGraphInvariant {
    /// Recomputed selected closure does not match its retained digest.
    #[error("selected Package semantic IR seal mismatch")]
    SealMismatch,
    /// A repository requests a scope absent from the sealed seed map.
    #[error("sealed Package graph has no occurrence seeds for repository `{repository}`")]
    MissingRepositorySeeds {
        /// Repository identity absent from the sealed occurrence map.
        repository: LogicalRepositoryIdV1,
    },
    /// Canonical encoding of the exact graph preimage failed.
    #[error("cannot seal Package semantic IR: {0}")]
    CanonicalEncoding(#[source] pale_core::canon::CanonError),
}

/// Closed cause vocabulary retained at a bounded Package aggregate boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PackageDiagnosticCode {
    /// A Package manifest violates its local invariant.
    InvalidManifest,
    /// A retained Package contains another manifest.
    NestedManifest,
    /// Manifest source custody is not a bijection.
    SourceInventory,
    /// A release differs from its exact lock coordinate.
    ReleaseMismatch,
    /// A selected Package is absent from the lock.
    MissingLockedPackage,
    /// An exact dependency is absent or mismatched.
    MissingExactDependency,
    /// A Package excludes the selected compiler.
    IncompatibleCompiler,
    /// The exact release closure is contradictory.
    ReleaseClosure,
    /// A resource declaration differs from manifest ownership.
    ResourceDeclaration,
    /// A Markdown body violates source custody.
    BodyCustody,
    /// An authored Link cannot be qualified.
    ResourceLink,
    /// An operation procedure contradicts its ceiling or recipe.
    OperationProcedure,
    /// A workspace selector cannot seed the graph.
    ResourceSelector,
    /// Global governing composition is contradictory.
    GoverningAlgebra,
    /// TOML syntax is invalid.
    TomlSyntax,
    /// A Markdown page cannot be separated into its declaration and body.
    PageHeader,
    /// TOML cannot enter the JSON-shaped validation grammar.
    TomlProjection,
    /// A schema-valid value cannot enter its typed source record.
    SourceDecode,
    /// A source value violates its named schema.
    SourceSchema,
    /// The sealed graph boundary is contradictory.
    GraphInvariant,
    /// Contract lookup, encoding, or validation failed.
    SchemaCheck,
    /// Two directories declare one Package identity.
    DuplicatePackage,
    /// A distribution entry cannot be read.
    Read,
    /// A bounded allocation failed.
    Allocation,
    /// A distribution entry violates confinement or type law.
    InvalidEntry,
    /// A required retained file is absent.
    MissingSnapshotFile,
    /// A retained text member is not UTF-8.
    NotUtf8,
    /// The platform lacks the required safe capture primitive.
    UnsupportedPlatform,
    /// A retained snapshot dimension exceeds its ceiling.
    SnapshotLimit,
}

impl PackageDiagnosticCode {
    const fn label(self) -> &'static str {
        match self {
            Self::InvalidManifest => "invalid_manifest",
            Self::NestedManifest => "nested_manifest",
            Self::SourceInventory => "source_inventory",
            Self::ReleaseMismatch => "release_mismatch",
            Self::MissingLockedPackage => "missing_locked_package",
            Self::MissingExactDependency => "missing_exact_dependency",
            Self::IncompatibleCompiler => "incompatible_compiler",
            Self::ReleaseClosure => "release_closure",
            Self::ResourceDeclaration => "resource_declaration",
            Self::BodyCustody => "body_custody",
            Self::ResourceLink => "resource_link",
            Self::OperationProcedure => "operation_procedure",
            Self::ResourceSelector => "resource_selector",
            Self::GoverningAlgebra => "governing_algebra",
            Self::TomlSyntax => "toml_syntax",
            Self::PageHeader => "page_header",
            Self::TomlProjection => "toml_projection",
            Self::SourceDecode => "source_decode",
            Self::SourceSchema => "source_schema",
            Self::GraphInvariant => "graph_invariant",
            Self::SchemaCheck => "schema_check",
            Self::DuplicatePackage => "duplicate_package",
            Self::Read => "read",
            Self::Allocation => "allocation",
            Self::InvalidEntry => "invalid_entry",
            Self::MissingSnapshotFile => "missing_snapshot_file",
            Self::NotUtf8 => "not_utf8",
            Self::UnsupportedPlatform => "unsupported_platform",
            Self::SnapshotLimit => "snapshot_limit",
        }
    }
}

/// Canonical typed Package diagnostic retained by aggregate validation.
///
/// This value is both the observable retained entry and its total-order key.
/// It deliberately excludes rejected bytes, human-facing labels and schema
/// expectations, opaque foreign prose, and lossy native path rendering.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PackageDiagnostic {
    code: PackageDiagnosticCode,
    coordinates: Vec<String>,
}

impl PackageDiagnostic {
    /// Stable typed cause of the contradiction.
    #[must_use]
    pub const fn code(&self) -> PackageDiagnosticCode {
        self.code
    }

    /// Canonical owner, semantic identity, and normalized-path coordinates.
    #[must_use]
    pub fn coordinates(&self) -> &[String] {
        &self.coordinates
    }
}

impl std::fmt::Display for PackageDiagnostic {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.code.label())?;
        for coordinate in &self.coordinates {
            write!(formatter, " [{coordinate}]")?;
        }
        Ok(())
    }
}

/// Deterministically ordered Package failures from one aggregate boundary.
#[derive(Debug)]
pub struct PackageDiagnostics {
    entries: Vec<PackageDiagnostic>,
    truncated: usize,
}

impl PackageDiagnostics {
    /// Retained typed failures in canonical diagnostic order.
    #[must_use]
    pub fn entries(&self) -> &[PackageDiagnostic] {
        &self.entries
    }

    /// Number of additional failures omitted at the aggregate boundary.
    #[must_use]
    pub const fn truncated(&self) -> usize {
        self.truncated
    }

    /// Total failures observed before bounded projection.
    #[must_use]
    pub fn observed_len(&self) -> usize {
        self.entries.len() + self.truncated
    }
}

impl std::fmt::Display for PackageDiagnostics {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} Package admission contradiction(s)",
            self.observed_len()
        )?;
        for diagnostic in &self.entries {
            write!(formatter, "; {diagnostic}")?;
        }
        if self.truncated > 0 {
            write!(
                formatter,
                "; {} additional contradiction(s) truncated",
                self.truncated
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for PackageDiagnostics {}

/// Stable bounded multiset of independently actionable Package failures.
///
/// This uses the bounded multiset merge law rendered on
/// [`crate::semantic::SemanticDiagnostics`], with
/// [`MAX_PACKAGE_DIAGNOSTICS`] as its bound. Since [`PackageDiagnostic`] is both
/// key and retained value, key collisions cannot make selection depend on
/// traversal order; `truncated` records the exact omitted cardinality.
#[derive(Default)]
pub(crate) struct PackageDiagnosticAccumulator {
    entries: Vec<PackageDiagnostic>,
    truncated: usize,
}

impl PackageDiagnosticAccumulator {
    /// Retain one typed failure without exceeding [`MAX_PACKAGE_DIAGNOSTICS`].
    pub(crate) fn push(&mut self, error: PackageError) {
        match error {
            PackageError::Diagnostics(diagnostics) => {
                let PackageDiagnostics { entries, truncated } = *diagnostics;
                self.merge(Self { entries, truncated });
            }
            error => self.insert(package_error_diagnostic(&error)),
        }
    }

    fn insert(&mut self, diagnostic: PackageDiagnostic) {
        let position = self
            .entries
            .binary_search(&diagnostic)
            .unwrap_or_else(|position| position);
        if self.entries.len() < MAX_PACKAGE_DIAGNOSTICS {
            self.entries.insert(position, diagnostic);
            return;
        }
        self.truncated += 1;
        if position < MAX_PACKAGE_DIAGNOSTICS {
            self.entries.insert(position, diagnostic);
            self.entries.pop();
        }
    }

    /// Merge an independently evaluated validation partition.
    pub(crate) fn merge(&mut self, other: Self) {
        for diagnostic in other.entries {
            self.insert(diagnostic);
        }
        self.truncated += other.truncated;
    }

    /// Capture one independent result and retain its typed failure.
    pub(crate) fn capture<T>(&mut self, result: Result<T, PackageError>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.push(error);
                None
            }
        }
    }

    /// Return the success value or one bounded aggregate preserving every cause.
    pub(crate) fn finish<T>(self, value: T) -> Result<T, PackageError> {
        if self.entries.is_empty() {
            Ok(value)
        } else {
            Err(PackageError::Diagnostics(Box::new(PackageDiagnostics {
                entries: self.entries,
                truncated: self.truncated,
            })))
        }
    }
}

fn package_error_diagnostic(error: &PackageError) -> PackageDiagnostic {
    let (code, coordinates) = match error {
        PackageError::Diagnostics(_) => (PackageDiagnosticCode::GraphInvariant, Vec::new()),
        PackageError::InvalidManifest { path, source } => (
            PackageDiagnosticCode::InvalidManifest,
            prepend_path(path, invalid_manifest_coordinates(source)),
        ),
        PackageError::NestedPackageManifest { package, path } => (
            PackageDiagnosticCode::NestedManifest,
            vec![package.to_string(), path_key(path)],
        ),
        PackageError::SourceInventoryMismatch {
            package,
            missing,
            undeclared,
        } => (
            PackageDiagnosticCode::SourceInventory,
            std::iter::once(package.to_string())
                .chain(canonical_paths("missing", missing))
                .chain(canonical_paths("undeclared", undeclared))
                .collect(),
        ),
        PackageError::ReleaseMismatch { package } => (
            PackageDiagnosticCode::ReleaseMismatch,
            vec![package.to_string()],
        ),
        PackageError::MissingLockedPackage { package } => (
            PackageDiagnosticCode::MissingLockedPackage,
            vec![package.to_string()],
        ),
        PackageError::MissingExactDependency {
            package,
            dependency,
            version,
        } => (
            PackageDiagnosticCode::MissingExactDependency,
            vec![
                package.to_string(),
                dependency.to_string(),
                version.to_string(),
            ],
        ),
        PackageError::IncompatibleCompiler { package, compiler } => (
            PackageDiagnosticCode::IncompatibleCompiler,
            vec![package.to_string(), compiler.clone()],
        ),
        PackageError::ReleaseClosure(_) => (PackageDiagnosticCode::ReleaseClosure, Vec::new()),
        PackageError::ResourceDeclarationMismatch {
            package,
            source_path,
            resources,
        } => (
            PackageDiagnosticCode::ResourceDeclaration,
            std::iter::once(package.to_string())
                .chain(std::iter::once(path_key(std::path::Path::new(source_path))))
                .chain(sorted_strings(resources.iter().map(ToString::to_string)))
                .collect(),
        ),
        PackageError::InvalidBodyCustody {
            resource,
            body,
            reason,
        } => (
            PackageDiagnosticCode::BodyCustody,
            vec![
                resource_key(resource),
                path_key(std::path::Path::new(body)),
                body_error_rank(*reason).to_string(),
            ],
        ),
        PackageError::InvalidMarkdownLink {
            from,
            destination_sha256,
            reason,
        } => (
            PackageDiagnosticCode::BodyCustody,
            vec![
                "markdown_link".into(),
                resource_key(from),
                destination_sha256.to_string(),
                markdown_link_violation_rank(*reason).to_string(),
            ],
        ),
        PackageError::InvalidResourceLink {
            from,
            target,
            relation,
            reason,
        } => (
            PackageDiagnosticCode::ResourceLink,
            vec![
                resource_key(from),
                resource_key(target),
                resource_link_relation_rank(*relation).to_string(),
                resource_link_violation_rank(*reason).to_string(),
            ],
        ),
        PackageError::InvalidOperationProcedure { operation, reason } => (
            PackageDiagnosticCode::OperationProcedure,
            vec![
                resource_key(operation),
                operation_procedure_violation_rank(*reason).to_string(),
            ],
        ),
        PackageError::InvalidResourceSelector {
            kind,
            package,
            resource,
            reason,
        } => (
            PackageDiagnosticCode::ResourceSelector,
            vec![
                resource_kind_key(*kind).to_string(),
                package.to_string(),
                resource
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
                resource_selector_violation_rank(*reason).to_string(),
            ],
        ),
        PackageError::GoverningAlgebra(_) => (PackageDiagnosticCode::GoverningAlgebra, Vec::new()),
        PackageError::TomlSyntax { path, .. } => {
            (PackageDiagnosticCode::TomlSyntax, vec![path_key(path)])
        }
        PackageError::PageHeader { path, .. } => {
            (PackageDiagnosticCode::PageHeader, vec![path_key(path)])
        }
        PackageError::TomlToJson { path, source } => (
            PackageDiagnosticCode::TomlProjection,
            vec![path_key(path), serde_json_category(source).to_string()],
        ),
        PackageError::SourceDecode {
            path,
            value_path,
            source,
            ..
        } => (
            PackageDiagnosticCode::SourceDecode,
            vec![
                path_key(path),
                value_path.clone(),
                serde_json_category(source.inner()).to_string(),
            ],
        ),
        PackageError::SourceSchema { path, source } => (
            PackageDiagnosticCode::SourceSchema,
            prepend_path(path, schema_violation_coordinates(source)),
        ),
        PackageError::GraphInvariant(source) => (
            PackageDiagnosticCode::GraphInvariant,
            graph_invariant_coordinates(source),
        ),
        PackageError::SchemaCheck(source) => (
            PackageDiagnosticCode::SchemaCheck,
            schema_error_coordinates(source),
        ),
        PackageError::DuplicatePackage { id, first, second } => {
            let mut paths = [path_key(first), path_key(second)];
            paths.sort();
            (
                PackageDiagnosticCode::DuplicatePackage,
                vec![id.clone(), paths[0].clone(), paths[1].clone()],
            )
        }
        PackageError::Read { path, source } => (
            PackageDiagnosticCode::Read,
            vec![
                path_key(path),
                io_error_kind_rank(source.kind()).to_string(),
            ],
        ),
        PackageError::Allocation { path, .. } => {
            (PackageDiagnosticCode::Allocation, vec![path_key(path)])
        }
        PackageError::InvalidEntry { path } => {
            (PackageDiagnosticCode::InvalidEntry, vec![path_key(path)])
        }
        PackageError::MissingSnapshotFile { path } => (
            PackageDiagnosticCode::MissingSnapshotFile,
            vec![path_key(path)],
        ),
        PackageError::NotUtf8 { path } => (PackageDiagnosticCode::NotUtf8, vec![path_key(path)]),
        PackageError::UnsupportedPlatform { .. } => {
            (PackageDiagnosticCode::UnsupportedPlatform, Vec::new())
        }
        PackageError::SnapshotLimit {
            limit,
            value,
            maximum,
        } => (
            PackageDiagnosticCode::SnapshotLimit,
            vec![
                limit_rank(limit).to_string(),
                number(*value),
                number(*maximum),
            ],
        ),
    };
    PackageDiagnostic { code, coordinates }
}

fn path_key(path: &std::path::Path) -> String {
    use std::path::Component;
    path.components()
        .filter_map(|component| match component {
            Component::Prefix(prefix) => Some(format!("p:{}", encoded_os(prefix.as_os_str()))),
            Component::RootDir => Some("r".to_string()),
            Component::CurDir => None,
            Component::ParentDir => Some("u".to_string()),
            Component::Normal(value) => Some(format!("n:{}", encoded_os(value))),
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn encoded_os(value: &std::ffi::OsStr) -> String {
    value
        .as_encoded_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn canonical_paths<'a>(
    kind: &'static str,
    paths: &'a [String],
) -> impl Iterator<Item = String> + 'a {
    let mut paths = paths
        .iter()
        .map(|path| format!("{kind}:{}", path_key(std::path::Path::new(path))))
        .collect::<Vec<_>>();
    paths.sort();
    paths.into_iter()
}

fn sorted_strings(values: impl Iterator<Item = String>) -> impl Iterator<Item = String> {
    let mut values = values.collect::<Vec<_>>();
    values.sort();
    values.into_iter()
}

fn number(value: impl TryInto<u64>) -> String {
    let value = value.try_into().ok().unwrap_or(u64::MAX);
    format!("{value:020}")
}

fn prepend_path(path: &std::path::Path, coordinates: Vec<String>) -> Vec<String> {
    std::iter::once(path_key(path)).chain(coordinates).collect()
}

fn resource_key(resource: &pale_core::records::PackageResourceRefV1) -> String {
    format!("{}/{}", resource.package_id, resource.resource_id)
}

fn resource_kind_key(kind: pale_core::records::PackageResourceKindV1) -> &'static str {
    use pale_core::records::PackageResourceKindV1;
    match kind {
        PackageResourceKindV1::Doctrine => "doctrine",
        PackageResourceKindV1::OperationArchetype => "operation_archetype",
        PackageResourceKindV1::Operation => "operation",
        PackageResourceKindV1::RoleArchetype => "role_archetype",
        PackageResourceKindV1::Role => "role",
        PackageResourceKindV1::Stack => "stack",
        PackageResourceKindV1::RunMode => "run_mode",
        PackageResourceKindV1::PolicyFamily => "policy_family",
        PackageResourceKindV1::GoverningOrder => "governing_order",
        PackageResourceKindV1::CapabilityPolicy => "capability_policy",
        PackageResourceKindV1::ToolRecipe => "tool_recipe",
    }
}

const fn markdown_link_violation_rank(reason: MarkdownLinkViolation) -> u8 {
    match reason {
        MarkdownLinkViolation::UnconfinedDestination => 0,
        MarkdownLinkViolation::MissingTarget => 1,
        MarkdownLinkViolation::UndeclaredRelationship => 2,
        MarkdownLinkViolation::UnrenderedTarget => 3,
        MarkdownLinkViolation::UnsupportedSyntax => 4,
        MarkdownLinkViolation::MissingHeading => 5,
    }
}

const fn body_error_rank(reason: BodyError) -> u8 {
    match reason {
        BodyError::UndeclaredBody => 0,
        BodyError::ReusedBody => 1,
    }
}

const fn resource_link_violation_rank(reason: ResourceLinkViolation) -> u8 {
    match reason {
        ResourceLinkViolation::SelfExternal => 0,
        ResourceLinkViolation::NotDirectDependency => 1,
        ResourceLinkViolation::MissingTarget => 2,
        ResourceLinkViolation::InternalTarget => 3,
        ResourceLinkViolation::WrongKind => 4,
        ResourceLinkViolation::AmbiguousTarget => 5,
    }
}

const fn resource_link_relation_rank(relation: ResourceLinkRelation) -> u8 {
    match relation {
        ResourceLinkRelation::DoctrineExtends => 0,
        ResourceLinkRelation::DoctrineRequires => 1,
        ResourceLinkRelation::DoctrineSeeAlso => 2,
        ResourceLinkRelation::OperationArchetypeExtends => 3,
        ResourceLinkRelation::OperationArchetypeDoctrine => 4,
        ResourceLinkRelation::OperationExtends => 5,
        ResourceLinkRelation::OperationDoctrine => 6,
        ResourceLinkRelation::OperationRole => 7,
        ResourceLinkRelation::OperationCapabilityPolicy => 8,
        ResourceLinkRelation::RoleArchetypeExtends => 9,
        ResourceLinkRelation::RoleArchetypeDoctrine => 10,
        ResourceLinkRelation::RoleExtends => 11,
        ResourceLinkRelation::RoleInheritedLens => 12,
        ResourceLinkRelation::RoleDoctrine => 13,
        ResourceLinkRelation::RoleStack => 14,
        ResourceLinkRelation::StackExtends => 15,
        ResourceLinkRelation::StackDoctrine => 16,
        ResourceLinkRelation::StackRoleDoctrine => 17,
        ResourceLinkRelation::RunModeExtends => 18,
        ResourceLinkRelation::OperationRequirementRecipe => 19,
        ResourceLinkRelation::OperationFallbackRecipe => 20,
        ResourceLinkRelation::GoverningConstraintScope => 21,
        ResourceLinkRelation::GoverningConstraintSubject => 22,
        ResourceLinkRelation::GoverningConstraintPolicy => 23,
        ResourceLinkRelation::GoverningObligationScope => 24,
        ResourceLinkRelation::GoverningObligationSubject => 25,
        ResourceLinkRelation::GoverningObligation => 26,
        ResourceLinkRelation::GoverningExclusiveScope => 27,
        ResourceLinkRelation::GoverningExclusiveAuthority => 28,
        ResourceLinkRelation::GoverningNarrowingScope => 29,
        ResourceLinkRelation::GoverningNarrower => 30,
        ResourceLinkRelation::GoverningBroader => 31,
        ResourceLinkRelation::GoverningOrderScope => 32,
        ResourceLinkRelation::GoverningOrderPrerequisite => 33,
        ResourceLinkRelation::GoverningOrderDependent => 34,
    }
}

const fn operation_procedure_violation_rank(reason: OperationProcedureViolation) -> u8 {
    match reason {
        OperationProcedureViolation::UnusedCapabilityCeiling => 0,
        OperationProcedureViolation::MissingCapabilityCeiling => 1,
        OperationProcedureViolation::MissingPolicy => 2,
        OperationProcedureViolation::RequirementExceedsPolicy => 3,
        OperationProcedureViolation::MissingRecipe => 4,
        OperationProcedureViolation::RecipeMismatch => 5,
    }
}

const fn resource_selector_violation_rank(reason: ResourceSelectorViolation) -> u8 {
    match reason {
        ResourceSelectorViolation::OwnerOutsideClosure => 0,
        ResourceSelectorViolation::MissingResource => 1,
        ResourceSelectorViolation::InternalResource => 2,
        ResourceSelectorViolation::WrongKind => 3,
        ResourceSelectorViolation::MissingQualifiedDependency => 4,
        ResourceSelectorViolation::AmbiguousQualifiedDependency => 5,
    }
}

fn graph_invariant_coordinates(source: &PackageGraphInvariant) -> Vec<String> {
    match source {
        PackageGraphInvariant::SealMismatch => vec!["seal_mismatch".to_string()],
        PackageGraphInvariant::MissingRepositorySeeds { repository } => {
            vec![
                "missing_repository_seeds".to_string(),
                repository.to_string(),
            ]
        }
        PackageGraphInvariant::CanonicalEncoding(_) => {
            vec!["canonical_encoding".to_string()]
        }
    }
}

fn invalid_manifest_coordinates(
    source: &pale_core::records::InvalidPackageManifestV1,
) -> Vec<String> {
    use pale_core::records::InvalidPackageManifestV1;
    match source {
        InvalidPackageManifestV1::UnsupportedSchemaVersion(version) => {
            vec![
                "unsupported_schema_version".to_string(),
                version.to_string(),
            ]
        }
        InvalidPackageManifestV1::EmptySources => vec!["empty_sources".to_string()],
        InvalidPackageManifestV1::EmptyResources => vec!["empty_resources".to_string()],
        InvalidPackageManifestV1::InventoryTooLarge {
            inventory,
            actual,
            maximum,
        } => vec![
            "inventory_too_large".to_string(),
            manifest_inventory_rank(inventory).to_string(),
            number(*actual),
            number(*maximum),
        ],
        InvalidPackageManifestV1::DuplicateDependency { package_id } => {
            vec!["duplicate_dependency".to_string(), package_id.to_string()]
        }
        InvalidPackageManifestV1::DuplicateSource { path } => {
            vec!["duplicate_source".to_string(), path.as_str().to_string()]
        }
        InvalidPackageManifestV1::DuplicateResource { resource_id } => {
            vec!["duplicate_resource".to_string(), resource_id.to_string()]
        }
        InvalidPackageManifestV1::MissingDefinitionSource { resource_id, path } => vec![
            "missing_definition_source".to_string(),
            resource_id.to_string(),
            path.as_str().to_string(),
        ],
        InvalidPackageManifestV1::DefinitionSourceRole { resource_id, path } => vec![
            "definition_source_role".to_string(),
            resource_id.to_string(),
            path.as_str().to_string(),
        ],
    }
}

fn manifest_inventory_rank(inventory: &str) -> u8 {
    match inventory {
        "dependency" => 0,
        "source" => 1,
        "resource" => 2,
        _ => u8::MAX,
    }
}

fn schema_violation_coordinates(source: &pale_contracts::SchemaViolations) -> Vec<String> {
    let mut violations = source
        .violations()
        .iter()
        .map(|violation| {
            let (instance_path, schema_path, keyword, discriminator) =
                violation.semantic_identity();
            (
                instance_path.to_string(),
                schema_path.to_string(),
                keyword.to_string(),
                discriminator.to_string(),
            )
        })
        .collect::<Vec<_>>();
    violations.sort();
    std::iter::once(source.schema_id().to_string())
        .chain(violations.into_iter().flat_map(
            |(instance_path, schema_path, keyword, discriminator)| {
                [instance_path, schema_path, keyword, discriminator]
            },
        ))
        .chain(std::iter::once(number(source.truncated_count())))
        .collect()
}

fn schema_error_coordinates(source: &pale_contracts::SchemaError) -> Vec<String> {
    use pale_contracts::SchemaError;
    match source {
        SchemaError::UnknownId(id) => vec!["unknown_id".to_string(), id.clone()],
        SchemaError::UnknownDefinition {
            schema_id,
            definition,
        } => vec![
            "unknown_definition".to_string(),
            schema_id.clone(),
            definition.clone(),
        ],
        SchemaError::Encode(source) => vec![
            "encode".to_string(),
            serde_json_category(source).to_string(),
        ],
        SchemaError::Parse(_) => vec!["parse".to_string()],
        SchemaError::Compile(_) => vec!["compile".to_string()],
        SchemaError::Integrity(_) => vec!["integrity".to_string()],
        SchemaError::IdentityMismatch(id) => {
            vec!["identity_mismatch".to_string(), id.clone()]
        }
        SchemaError::Invalid(source) => std::iter::once("invalid".to_string())
            .chain(schema_violation_coordinates(source))
            .collect(),
    }
}

fn limit_rank(limit: &str) -> u8 {
    match limit {
        "directory depth" => 0,
        "entry count" => 1,
        "file bytes" => 2,
        "total bytes" => 3,
        _ => u8::MAX,
    }
}

fn serde_json_category(source: &serde_json::Error) -> u8 {
    use serde_json::error::Category;
    match source.classify() {
        Category::Io => 0,
        Category::Syntax => 1,
        Category::Data => 2,
        Category::Eof => 3,
    }
}

fn io_error_kind_rank(kind: std::io::ErrorKind) -> u8 {
    use std::io::ErrorKind;
    match kind {
        ErrorKind::NotFound => 0,
        ErrorKind::PermissionDenied => 1,
        ErrorKind::AlreadyExists => 2,
        ErrorKind::InvalidInput => 3,
        ErrorKind::InvalidData => 4,
        ErrorKind::UnexpectedEof => 5,
        ErrorKind::OutOfMemory => 6,
        _ => u8::MAX,
    }
}

/// Errors from Package capture, source admission, and semantic-graph sealing.
#[derive(Debug, thiserror::Error)]
pub enum PackageError {
    /// Multiple independent contradictions established at one graph boundary.
    #[error(transparent)]
    Diagnostics(#[from] Box<PackageDiagnostics>),
    /// A parsed manifest violates its own bounded ownership contract.
    #[error("invalid Package manifest `{path}`: {source}", path = .path.display())]
    InvalidManifest {
        /// Snapshot-relative manifest path.
        path: PathBuf,
        /// Exact manifest-owned invariant that failed.
        #[source]
        source: pale_core::records::InvalidPackageManifestV1,
    },
    /// One retained Package contains a second manifest beneath its source root.
    #[error("Package `{package}` contains nested manifest `{path}`", path = .path.display())]
    NestedPackageManifest {
        /// Owner of the outer retained release.
        package: pale_core::records::PackageIdV1,
        /// Nested snapshot-relative manifest path.
        path: PathBuf,
    },
    /// The manifest source inventory is not a bijection over retained members.
    #[error(
        "Package `{package}` source inventory differs from retained members (missing: {missing:?}; undeclared: {undeclared:?})"
    )]
    SourceInventoryMismatch {
        /// Package whose byte custody is incomplete or overbroad.
        package: pale_core::records::PackageIdV1,
        /// Manifest-declared paths absent from the retained snapshot.
        missing: Vec<String>,
        /// Retained paths absent from the manifest source inventory.
        undeclared: Vec<String>,
    },
    /// A manifest release does not match its exact distribution-lock coordinate.
    #[error("Package `{package}` release does not match the distribution lock")]
    ReleaseMismatch {
        /// Package whose version or content digest differs.
        package: pale_core::records::PackageIdV1,
    },
    /// A selected Package seed is absent from the exact lock.
    #[error("Package seed `{package}` is absent from the distribution lock")]
    MissingLockedPackage {
        /// Missing selected Package identity.
        package: pale_core::records::PackageIdV1,
    },
    /// One exact Package dependency is absent or selects another release.
    #[error("Package `{package}` requires exact dependency `{dependency}` at `{version}`")]
    MissingExactDependency {
        /// Package declaring the edge.
        package: pale_core::records::PackageIdV1,
        /// Required dependency owner.
        dependency: pale_core::records::PackageIdV1,
        /// Required exact release label.
        version: pale_core::records::PackageVersionV1,
    },
    /// A selected Package release excludes the lock's compiler identity.
    #[error("Package `{package}` is incompatible with compiler `{compiler}`")]
    IncompatibleCompiler {
        /// Selected Package whose compatibility set rejected the compiler.
        package: pale_core::records::PackageIdV1,
        /// Exact compiler identity pinned by the distribution lock.
        compiler: String,
    },
    /// A lock contains duplicate or conflicting Package release coordinates.
    #[error("invalid Package release closure: {0}")]
    ReleaseClosure(#[from] pale_core::records::ReleaseClosureErrorV1),
    /// A manifest resource and its declaration source are not an exact bijection.
    #[error(
        "Package `{package}` declaration `{source_path}` does not define exactly the manifest resources {resources:?}"
    )]
    ResourceDeclarationMismatch {
        /// Owning Package release.
        package: pale_core::records::PackageIdV1,
        /// Confined declaration source.
        source_path: String,
        /// Manifest resources assigned to that source.
        resources: Vec<pale_core::records::PackageResourceIdV1>,
    },
    /// A semantic body path is absent, has the wrong source role, or is reused.
    #[error("invalid body custody for {resource:?} at `{body}`: {reason}")]
    InvalidBodyCustody {
        /// Resource claiming the body.
        resource: pale_core::records::PackageResourceRefV1,
        /// Confined Package-relative body path.
        body: String,
        /// Stable source-custody failure.
        reason: BodyError,
    },
    /// A local prose link is not a qualified, independently rendered resource.
    #[error("invalid Markdown link from {from:?} (destination {destination_sha256}): {reason}")]
    InvalidMarkdownLink {
        /// Resource that owns the authored Markdown.
        from: pale_core::records::PackageResourceRefV1,
        /// Bounded identity; rejected prose never enters a diagnostic.
        destination_sha256: pale_core::records::ArtifactDigest,
        /// Closed navigation failure, independent of effect authority.
        reason: MarkdownLinkViolation,
    },
    /// An authored semantic link cannot be qualified under Package law.
    #[error("invalid {relation} link from {from:?} to {target:?}: {reason}")]
    InvalidResourceLink {
        /// Resource containing the authored link.
        from: pale_core::records::PackageResourceRefV1,
        /// Qualified target requested by the source link.
        target: pale_core::records::PackageResourceRefV1,
        /// Stable relation name used for diagnostics.
        relation: ResourceLinkRelation,
        /// Exact dependency/export/kind reason for rejection.
        reason: ResourceLinkViolation,
    },
    /// A qualified operation's requirements and Package ceiling disagree.
    #[error("invalid capability procedure on operation {operation:?}: {reason}")]
    InvalidOperationProcedure {
        /// Exact Package operation being admitted.
        operation: pale_core::records::PackageResourceRefV1,
        /// Stable procedure-law failure.
        reason: OperationProcedureViolation,
    },
    /// A workspace selector points outside its selected, exported typed frontier.
    #[error("invalid {kind:?} selector for Package `{package}` resource {resource:?}: {reason}")]
    InvalidResourceSelector {
        /// Required resource kind supplied by the enclosing selection field.
        kind: pale_core::records::PackageResourceKindV1,
        /// Package selected by the exact or wildcard selector.
        package: pale_core::records::PackageIdV1,
        /// Exact selected resource, or [`None`] for a Package-scoped wildcard.
        resource: Option<pale_core::records::PackageResourceIdV1>,
        /// Stable selection-law failure.
        reason: ResourceSelectorViolation,
    },
    /// Selected governing fragments violate the global algebra.
    #[error("cannot compose selected Package governing order: {0}")]
    GoverningAlgebra(#[from] pale_core::records::GoverningAlgebraErrorV1),
    /// One admitted Package source file is not syntactically valid TOML.
    #[error("cannot parse Package TOML `{path}`: {source}", path = .path.display())]
    TomlSyntax {
        /// Exact authored or snapshot-relative source path.
        path: PathBuf,
        /// Typed parser failure after its retained source text has been removed.
        #[source]
        source: Box<toml::de::Error>,
    },
    /// A Package page has no bounded declaration and body pair.
    #[error("invalid Package page header in `{path}`: {reason}", path = .path.display())]
    PageHeader {
        /// Exact source path within the retained Package snapshot.
        path: PathBuf,
        /// The missing delimiter, exceeded bound or absent body.
        reason: super::PageHeaderError,
    },
    /// A parsed Package TOML value could not be represented for schema admission.
    #[error("cannot project Package TOML `{path}` to JSON: {source}", path = .path.display())]
    TomlToJson {
        /// Exact authored or snapshot-relative source path.
        path: PathBuf,
        /// Typed serialization failure.
        #[source]
        source: serde_json::Error,
    },
    /// A schema-valid Package value could not be decoded into its typed record.
    #[error("cannot decode {label} `{path}` at `{value_path}`: {source}", path = .path.display())]
    SourceDecode {
        /// Exact authored or snapshot-relative source path.
        path: PathBuf,
        /// Catalog or record identity being decoded.
        label: String,
        /// Nested Serde value path at which typed decoding stopped.
        value_path: String,
        /// Typed decoding failure with its structured Serde path intact.
        #[source]
        source: Box<serde_path_to_error::Error<serde_json::Error>>,
    },
    /// A source document fails its pinned record schema.
    #[error("schema violation in `{path}`: {source}", path = .path.display())]
    SourceSchema {
        /// Authored or snapshot-relative source document that was rejected.
        path: PathBuf,
        /// Bounded structured reasons from the exact embedded schema.
        #[source]
        source: pale_contracts::SchemaViolations,
    },
    /// Schema-admitted resources violate the sealed graph boundary.
    #[error("Package graph invariant violation: {0}")]
    GraphInvariant(#[source] PackageGraphInvariant),
    /// A source value violates its pinned JSON-schema contract.
    #[error(transparent)]
    SchemaCheck(#[from] pale_contracts::SchemaError),
    /// Two package directories declare the same globally unique package identity.
    #[error("duplicate package id `{id}` in `{first}` and `{second}`")]
    DuplicatePackage {
        /// Duplicate package identity.
        id: String,
        /// First package metadata path.
        first: PathBuf,
        /// Second package metadata path.
        second: PathBuf,
    },
    /// Opening, inspecting, or streaming a distribution entry failed.
    #[error("cannot read distribution entry `{path}`")]
    Read {
        /// Distribution path being read.
        path: PathBuf,
        /// Underlying filesystem failure.
        #[source]
        source: std::io::Error,
    },
    /// Memory for a bounded distribution entry could not be reserved.
    #[error("cannot allocate bounded distribution entry `{path}`: {source}")]
    Allocation {
        /// Distribution path being retained.
        path: PathBuf,
        /// Failed bounded vector reservation.
        #[source]
        source: std::collections::TryReserveError,
    },
    /// A distribution entry is escaped, special, or not the expected regular type.
    #[error("distribution entry is not a confined regular file or directory: `{path}`", path = .path.display())]
    InvalidEntry {
        /// Escaped, special, symlinked, or incorrectly typed entry.
        path: PathBuf,
    },
    /// A required path is absent from the retained distribution snapshot.
    #[error("distribution snapshot is missing `{path}`", path = .path.display())]
    MissingSnapshotFile {
        /// Snapshot-relative path required by the selected source record.
        path: PathBuf,
    },
    /// A text-bearing distribution entry is not valid UTF-8.
    #[error("distribution entry is not UTF-8: `{path}`", path = .path.display())]
    NotUtf8 {
        /// Text-bearing distribution entry containing non-UTF-8 bytes.
        path: PathBuf,
    },
    /// This target cannot provide the no-follow primitive required by source admission.
    #[error("bounded distribution reads are unsupported on `{platform}`")]
    UnsupportedPlatform {
        /// Target platform without the required no-follow open primitive.
        platform: &'static str,
    },
    /// A bounded snapshot dimension exceeds its configured ceiling.
    #[error("distribution snapshot exceeds {limit}: {value} > {maximum}")]
    SnapshotLimit {
        /// Bounded dimension that was exceeded.
        limit: &'static str,
        /// Exact observed value, or the smallest value proven to exceed the limit.
        value: u64,
        /// Maximum admitted value.
        maximum: u64,
    },
}

impl From<pale_ports::SourceFilesystemError> for PackageError {
    fn from(error: pale_ports::SourceFilesystemError) -> Self {
        use pale_ports::SourceFilesystemError;
        match error {
            SourceFilesystemError::Io { path, source } => Self::Read { path, source },
            SourceFilesystemError::InvalidEntry { path } => Self::InvalidEntry { path },
            SourceFilesystemError::Allocation { path, source } => Self::Allocation { path, source },
            SourceFilesystemError::Limit {
                limit,
                value,
                maximum,
            } => Self::SnapshotLimit {
                limit,
                value,
                maximum,
            },
            SourceFilesystemError::UnsupportedPlatform { platform } => {
                Self::UnsupportedPlatform { platform }
            }
        }
    }
}

#[cfg(test)]
#[path = "../../tests/internal/packages/errors.rs"]
mod tests;
