//! Content-bearing declarations and their finite dependency grammar.
//!
//! Declaration records carry portable semantics only; compilation and native
//! effect admission remain separate trust transitions.

use super::*;

/// One exact declaration dependency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceDeclarationDependencyV1 {
    /// Referenced declaration.
    pub target: WorkspaceDeclarationRefV1,
    /// Semantic kind of the edge.
    pub kind: DeclarationDependencyKindV1,
}

/// Canonical namespace-qualified declaration reference.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceDeclarationRefV1 {
    /// Exact namespace.
    pub namespace: WorkspaceNamespaceId,
    /// Declaration identity local to that namespace.
    pub declaration_id: WorkspaceDeclarationId,
}

impl std::fmt::Display for WorkspaceDeclarationRefV1 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}::{}", self.namespace, self.declaration_id)
    }
}

/// Closed portable declaration language.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WorkspaceDeclarationV1 {
    /// Named trait with positive declaration dependencies.
    Trait {
        /// Declaration identity.
        id: WorkspaceDeclarationId,
        /// Owning namespace.
        namespace: WorkspaceNamespaceId,
        /// Exact dependency edges.
        #[schemars(schema_with = "dependencies_schema")]
        dependencies: Vec<WorkspaceDeclarationDependencyV1>,
    },
    /// Typed attribute declaration.
    Attribute {
        /// Declaration identity.
        id: WorkspaceDeclarationId,
        /// Owning namespace.
        namespace: WorkspaceNamespaceId,
        /// Closed scalar value kind.
        value_kind: WorkspaceValueKindV1,
        /// Exact dependency edges.
        #[schemars(schema_with = "dependencies_schema")]
        dependencies: Vec<WorkspaceDeclarationDependencyV1>,
    },
    /// Directional relationship declaration.
    Relationship {
        /// Declaration identity.
        id: WorkspaceDeclarationId,
        /// Owning namespace.
        namespace: WorkspaceNamespaceId,
        /// Source declaration type.
        from: WorkspaceDeclarationRefV1,
        /// Destination declaration type.
        to: WorkspaceDeclarationRefV1,
        /// Exact dependency edges.
        #[schemars(schema_with = "dependencies_schema")]
        dependencies: Vec<WorkspaceDeclarationDependencyV1>,
    },
    /// Policy family applied through explicit references.
    Policy {
        /// Declaration identity.
        id: WorkspaceDeclarationId,
        /// Owning namespace.
        namespace: WorkspaceNamespaceId,
        /// Policy ceiling contributed by this declaration.
        ceiling: WorkspacePolicyCeilingV1,
        /// Obligations accumulated by import and extension.
        #[schemars(schema_with = "obligation_ids_schema")]
        obligations: Vec<WorkspaceObligationId>,
        /// Exclusive authority claims that may not conflict.
        #[schemars(schema_with = "authority_claims_schema")]
        exclusive_authority: Vec<WorkspaceExclusiveAuthorityClaimV1>,
        /// Exact dependency edges.
        #[schemars(schema_with = "dependencies_schema")]
        dependencies: Vec<WorkspaceDeclarationDependencyV1>,
    },
    /// Logical resource contract; it contains no machine/native payload.
    Resource {
        /// Declaration identity.
        id: WorkspaceDeclarationId,
        /// Owning namespace.
        namespace: WorkspaceNamespaceId,
        /// Logical resource kind.
        resource_kind: WorkspaceResourceKindV1,
        /// Exact dependency edges.
        #[schemars(schema_with = "dependencies_schema")]
        dependencies: Vec<WorkspaceDeclarationDependencyV1>,
    },
}

impl WorkspaceDeclarationV1 {
    /// Return the namespace-qualified identity shared by every variant.
    #[must_use]
    pub fn declaration_ref(&self) -> WorkspaceDeclarationRefV1 {
        match self {
            Self::Trait { id, namespace, .. }
            | Self::Attribute { id, namespace, .. }
            | Self::Relationship { id, namespace, .. }
            | Self::Policy { id, namespace, .. }
            | Self::Resource { id, namespace, .. } => WorkspaceDeclarationRefV1 {
                namespace: namespace.clone(),
                declaration_id: id.clone(),
            },
        }
    }

    /// Borrow the declaration's exact dependency edges.
    #[must_use]
    pub fn dependencies(&self) -> &[WorkspaceDeclarationDependencyV1] {
        match self {
            Self::Trait { dependencies, .. }
            | Self::Attribute { dependencies, .. }
            | Self::Relationship { dependencies, .. }
            | Self::Policy { dependencies, .. }
            | Self::Resource { dependencies, .. } => dependencies,
        }
    }
}

wire_enum!(
    /// Closed scalar kinds available to portable attributes.
    WorkspaceValueKindV1 {
        /// Boolean scalar.
        Boolean => "boolean",
        /// Signed integer scalar.
        Integer => "integer",
        /// UTF-8 text scalar.
        Text => "text",
        /// Logical identity reference.
        LogicalId => "logical_id"
    }
);

/// Exact reference from a module release to a source component.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceSourceComponentRefV1 {
    /// Component identity.
    pub component_id: WorkspaceComponentId,
    /// Domain-separated canonical identity of the component content.
    pub component_digest: WorkspaceComponentDigest,
}

/// Finer-grained authored source used for provenance and impact analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceLanguagePrimitivesV1 {
    /// Stable wire schema identity.
    #[schemars(extend("const" = "pale/workspace-language-primitives.v1"))]
    pub schema: String,
    /// Initial contract version.
    #[schemars(extend("const" = 1))]
    pub schema_version: u32,
    /// Component identity.
    pub component_id: WorkspaceComponentId,
    /// Owning module identity.
    pub module_id: WorkspaceModuleId,
    /// Closed declaration set authored in this component.
    #[schemars(schema_with = "declarations_schema")]
    pub declarations: Vec<WorkspaceDeclarationV1>,
}

impl WorkspaceLanguagePrimitivesV1 {
    /// Compute the domain-separated semantic identity of this component.
    ///
    /// This identity does not change when a TOML carrier changes only whitespace
    /// or key order.
    ///
    /// # Errors
    ///
    /// Returns a canonicalization error when the typed component cannot be
    /// represented by RFC 8785 JSON.
    pub fn component_digest(&self) -> Result<WorkspaceComponentDigest, crate::canon::CanonError> {
        let mut component = self.clone();
        canonicalize_component(&mut component);
        WorkspaceComponentDigest::hash_canonical(&(
            "pale.workspace-source-component.v1",
            &component,
        ))
    }

    /// Render this component's canonical RFC 8785 representation.
    ///
    /// # Errors
    ///
    /// Returns a canonicalization error when the typed component cannot be
    /// represented by RFC 8785 JSON.
    pub fn canonical_component(&self) -> Result<String, crate::canon::CanonError> {
        let mut component = self.clone();
        canonicalize_component(&mut component);
        crate::canon::to_canonical_string(&component)
    }

    /// Verify this component's schema coordinate and canonical logical sets.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidWorkspaceComponent`] when a collection exceeds its
    /// owning bound, an identity repeats, the retained representation is not
    /// canonical, or canonical encoding fails.
    pub fn verify(&self) -> Result<(), InvalidWorkspaceComponent> {
        if self.schema != "pale/workspace-language-primitives.v1" || self.schema_version != 1 {
            return Err(InvalidWorkspaceComponent::SchemaCoordinate);
        }
        let mut canonical = self.clone();
        canonicalize_component(&mut canonical);
        validate_component_sets(&canonical)?;
        if canonical != *self {
            return Err(InvalidWorkspaceComponent::NonCanonical);
        }
        crate::canon::to_canonical_string(self).map_err(InvalidWorkspaceComponent::Canonical)?;
        Ok(())
    }
}

/// A workspace source component violates its initial-v1 self-integrity contract.
#[derive(Debug, thiserror::Error)]
pub enum InvalidWorkspaceComponent {
    /// The schema identity or initial-v1 version differs.
    #[error("workspace source component has an invalid schema coordinate")]
    SchemaCoordinate,
    /// A bounded logical set exceeds its owning initial-v1 ceiling.
    #[error("workspace component {kind} count {observed} exceeds {limit}")]
    Limit {
        /// Closed logical set whose bound was exceeded.
        kind: &'static str,
        /// Observed member count.
        observed: usize,
        /// Maximum admitted member count.
        limit: usize,
    },
    /// A logical set repeats one identity.
    #[error("workspace component repeats {kind} identity `{id}`")]
    Duplicate {
        /// Closed logical set containing the duplicate.
        kind: &'static str,
        /// Repeated logical identity.
        id: String,
    },
    /// A policy's bounded obligations or exclusive authority cannot combine.
    #[error("workspace component policy is invalid: {0}")]
    PolicyCombination(#[source] WorkspacePolicyCombinationError),
    /// A logical-set vector is not in its canonical order.
    #[error("workspace component logical sets are not canonically ordered")]
    NonCanonical,
    /// Canonical encoding failed.
    #[error("workspace component canonicalization failed: {0}")]
    Canonical(#[source] crate::canon::CanonError),
}

fn canonicalize_component(component: &mut WorkspaceLanguagePrimitivesV1) {
    for declaration in &mut component.declarations {
        canonicalize_declaration(declaration);
    }
    component
        .declarations
        .sort_by_key(WorkspaceDeclarationV1::declaration_ref);
}

pub(super) fn canonicalize_declaration(declaration: &mut WorkspaceDeclarationV1) {
    match declaration {
        WorkspaceDeclarationV1::Trait { dependencies, .. }
        | WorkspaceDeclarationV1::Attribute { dependencies, .. }
        | WorkspaceDeclarationV1::Resource { dependencies, .. }
        | WorkspaceDeclarationV1::Relationship { dependencies, .. } => {
            sort_dependencies(dependencies);
        }
        WorkspaceDeclarationV1::Policy {
            obligations,
            exclusive_authority,
            dependencies,
            ..
        } => {
            obligations.sort();
            exclusive_authority.sort_by(|left, right| {
                (&left.scope, &left.authority_resolver)
                    .cmp(&(&right.scope, &right.authority_resolver))
            });
            sort_dependencies(dependencies);
        }
    }
}

fn validate_component_sets(
    component: &WorkspaceLanguagePrimitivesV1,
) -> Result<(), InvalidWorkspaceComponent> {
    bounded_component_len(
        "declaration",
        component.declarations.len(),
        MAX_WORKSPACE_DECLARATIONS,
    )?;
    reject_component_duplicate(
        component
            .declarations
            .iter()
            .map(WorkspaceDeclarationV1::declaration_ref),
        "declaration",
    )?;
    for declaration in &component.declarations {
        validate_declaration(declaration).map_err(component_declaration_error)?;
    }
    Ok(())
}

#[derive(Debug)]
pub(super) enum InvalidWorkspaceDeclaration {
    Limit {
        kind: &'static str,
        observed: usize,
        limit: usize,
    },
    Duplicate {
        kind: &'static str,
        id: String,
    },
    Policy(WorkspacePolicyCombinationError),
}

pub(super) fn validate_declaration(
    declaration: &WorkspaceDeclarationV1,
) -> Result<(), InvalidWorkspaceDeclaration> {
    let dependencies = declaration.dependencies();
    if dependencies.len() > MAX_WORKSPACE_DECLARATION_DEPENDENCIES {
        return Err(InvalidWorkspaceDeclaration::Limit {
            kind: "declaration dependency",
            observed: dependencies.len(),
            limit: MAX_WORKSPACE_DECLARATION_DEPENDENCIES,
        });
    }
    reject_declaration_duplicate(
        dependencies
            .iter()
            .map(|dependency| (dependency.target.clone(), dependency.kind)),
        "declaration dependency",
    )?;
    if let WorkspaceDeclarationV1::Policy {
        obligations,
        exclusive_authority,
        ..
    } = declaration
    {
        WorkspaceObligationId::try_union_sets(&[], obligations)
            .map_err(InvalidWorkspaceDeclaration::Policy)?;
        WorkspaceExclusiveAuthorityClaimV1::try_combine_sets(&[], exclusive_authority)
            .map_err(InvalidWorkspaceDeclaration::Policy)?;
        reject_declaration_duplicate(obligations.iter().cloned(), "policy obligation")?;
        reject_declaration_duplicate(
            exclusive_authority.iter().cloned(),
            "policy authority claim",
        )?;
    }
    Ok(())
}

fn reject_declaration_duplicate<T: Ord + std::fmt::Debug>(
    mut values: impl Iterator<Item = T>,
    kind: &'static str,
) -> Result<(), InvalidWorkspaceDeclaration> {
    let Some(mut prior) = values.next() else {
        return Ok(());
    };
    for value in values {
        if prior == value {
            return Err(InvalidWorkspaceDeclaration::Duplicate {
                kind,
                id: format!("{value:?}"),
            });
        }
        prior = value;
    }
    Ok(())
}

fn component_declaration_error(error: InvalidWorkspaceDeclaration) -> InvalidWorkspaceComponent {
    match error {
        InvalidWorkspaceDeclaration::Limit {
            kind,
            observed,
            limit,
        } => InvalidWorkspaceComponent::Limit {
            kind,
            observed,
            limit,
        },
        InvalidWorkspaceDeclaration::Duplicate { kind, id } => {
            InvalidWorkspaceComponent::Duplicate { kind, id }
        }
        InvalidWorkspaceDeclaration::Policy(error) => {
            InvalidWorkspaceComponent::PolicyCombination(error)
        }
    }
}

pub(super) fn module_declaration_error(
    error: InvalidWorkspaceDeclaration,
) -> InvalidWorkspaceModule {
    match error {
        InvalidWorkspaceDeclaration::Limit {
            kind,
            observed,
            limit,
        } => InvalidWorkspaceModule::Limit {
            kind,
            observed,
            limit,
        },
        InvalidWorkspaceDeclaration::Duplicate { kind, id } => {
            InvalidWorkspaceModule::Duplicate { kind, id }
        }
        InvalidWorkspaceDeclaration::Policy(error) => {
            InvalidWorkspaceModule::PolicyCombination(error)
        }
    }
}

fn bounded_component_len(
    kind: &'static str,
    observed: usize,
    limit: usize,
) -> Result<(), InvalidWorkspaceComponent> {
    if observed > limit {
        Err(InvalidWorkspaceComponent::Limit {
            kind,
            observed,
            limit,
        })
    } else {
        Ok(())
    }
}

fn reject_component_duplicate<T: Ord + std::fmt::Debug>(
    mut values: impl Iterator<Item = T>,
    kind: &'static str,
) -> Result<(), InvalidWorkspaceComponent> {
    let Some(mut prior) = values.next() else {
        return Ok(());
    };
    for value in values {
        if prior == value {
            return Err(InvalidWorkspaceComponent::Duplicate {
                kind,
                id: format!("{value:?}"),
            });
        }
        prior = value;
    }
    Ok(())
}

fn sort_dependencies(dependencies: &mut [WorkspaceDeclarationDependencyV1]) {
    dependencies.sort_by(|left, right| (&left.target, left.kind).cmp(&(&right.target, right.kind)));
}
