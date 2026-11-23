"""Strict, immutable Python models for the confined workspace-module builder.

These models provide Python-native construction and tagged-union ergonomics.
They are not contract or admission authority: :mod:`pypale._native` always
decodes the resulting preimage through the Rust record, seals its digest,
validates the generated schema, and produces canonical JSON.
"""

from __future__ import annotations

from typing import Annotated, Literal, Optional, Tuple, Union

from pydantic import BaseModel, ConfigDict, Field


class _FrozenModel(BaseModel):
    """Shared strict, immutable, deny-unknown Python projection policy."""

    model_config = ConfigDict(
        extra="forbid",
        frozen=True,
        populate_by_name=True,
        strict=True,
    )


class WorkspaceDeclarationRef(_FrozenModel):
    """Qualified declaration identity inside one module namespace."""

    declaration_id: str
    namespace: str


class WorkspaceDeclarationDependency(_FrozenModel):
    """Typed dependency edge from one declaration to another."""

    kind: Literal["positive", "negation", "effect", "callback", "unbounded_generation"]
    target: WorkspaceDeclarationRef


class TraitDeclaration(_FrozenModel):
    """Trait declaration."""

    kind: Literal["trait"] = "trait"
    id: str
    namespace: str
    dependencies: Tuple[WorkspaceDeclarationDependency, ...] = ()


class AttributeDeclaration(_FrozenModel):
    """Typed attribute declaration."""

    kind: Literal["attribute"] = "attribute"
    id: str
    namespace: str
    value_kind: Literal["boolean", "integer", "text", "logical_id"]
    dependencies: Tuple[WorkspaceDeclarationDependency, ...] = ()


class RelationshipDeclaration(_FrozenModel):
    """Directed semantic relationship declaration."""

    kind: Literal["relationship"] = "relationship"
    id: str
    namespace: str
    from_: WorkspaceDeclarationRef = Field(alias="from")
    to: WorkspaceDeclarationRef
    dependencies: Tuple[WorkspaceDeclarationDependency, ...] = ()


class WorkspaceAuthorityCeiling(_FrozenModel):
    """Permitted logical authority scopes."""

    permitted: Tuple[str, ...] = ()


class WorkspaceControllerCeiling(_FrozenModel):
    """Permitted logical controller scopes."""

    permitted: Tuple[str, ...] = ()


class WorkspaceRightsCeiling(_FrozenModel):
    """Permitted logical rights."""

    permitted: Tuple[str, ...] = ()


class WorkspacePolicyCeiling(_FrozenModel):
    """Independent policy axes narrowed during module composition."""

    authorities: WorkspaceAuthorityCeiling
    controllers: WorkspaceControllerCeiling
    disclosure: Literal["private", "protected", "public"]
    effect: Literal["none", "read", "mutate"]
    model_use_permitted: bool
    retention_seconds: int
    rights: WorkspaceRightsCeiling


class WorkspaceExclusiveAuthorityClaim(_FrozenModel):
    """Logical resolver selected for one exclusive authority scope."""

    authority_resolver: str
    scope: str


class PolicyDeclaration(_FrozenModel):
    """Policy-bearing declaration."""

    kind: Literal["policy"] = "policy"
    id: str
    namespace: str
    ceiling: WorkspacePolicyCeiling
    dependencies: Tuple[WorkspaceDeclarationDependency, ...] = ()
    exclusive_authority: Tuple[WorkspaceExclusiveAuthorityClaim, ...] = ()
    obligations: Tuple[str, ...] = ()


class ResourceDeclaration(_FrozenModel):
    """Logical resource declaration."""

    kind: Literal["resource"] = "resource"
    id: str
    namespace: str
    resource_kind: Literal[
        "principal",
        "organization",
        "native_application",
        "participant",
        "package",
        "normative_pack",
    ]
    dependencies: Tuple[WorkspaceDeclarationDependency, ...] = ()


WorkspaceDeclaration = Annotated[
    Union[
        TraitDeclaration,
        AttributeDeclaration,
        RelationshipDeclaration,
        PolicyDeclaration,
        ResourceDeclaration,
    ],
    Field(discriminator="kind"),
]


class WorkspaceExportProjection(_FrozenModel):
    """Content-bearing declaration projected by one named export."""

    owner_module: str
    local_ref: WorkspaceDeclarationRef
    declaration: WorkspaceDeclaration


class WorkspaceExport(_FrozenModel):
    """Self-contained module export and its policy ceiling."""

    export_id: str
    declarations: Tuple[WorkspaceExportProjection, ...]
    visibility: Literal["public", "protected", "private"]
    delivery: Literal["interface", "source"]
    propagation: Literal["transitive", "prevent"]
    ceiling: WorkspacePolicyCeiling
    obligations: Tuple[str, ...] = ()
    exclusive_authority: Tuple[WorkspaceExclusiveAuthorityClaim, ...] = ()


class WorkspaceImportPath(_FrozenModel):
    """Namespace path selected from one upstream export."""

    export_id: str
    source_namespace: str
    local_namespace: str


class WorkspacePublisherRef(_FrozenModel):
    """Publisher and recovery-key identities carried by one release."""

    publisher_id: str
    root_key_id: str
    delegated_release_key_id: str
    recovery_authority_id: str


class WorkspaceImport(_FrozenModel):
    """Exact immutable upstream module selection."""

    module_id: str
    release_id: str
    module_digest: str
    publisher: WorkspacePublisherRef
    paths: Tuple[WorkspaceImportPath, ...]
    protected_delivery_authority: Optional[str] = None
    visibility_ceiling: Literal["public", "protected", "private"]
    propagation: Literal["transitive", "prevent"]
    edge_policy: WorkspacePolicyCeiling
    adoption_policy: Literal["manual"] = "manual"


class WorkspaceExtension(_FrozenModel):
    """Typed extension and optional policy narrowing."""

    target: WorkspaceDeclarationRef
    namespace: str
    added_traits: Tuple[WorkspaceDeclarationRef, ...] = ()
    narrowed_ceiling: Optional[WorkspacePolicyCeiling] = None
    added_obligations: Tuple[str, ...] = ()


class PackageRelease(_FrozenModel):
    """Exact Package release selected by a logical resource."""

    id: str
    version: str
    content_digest: str


class NormativePackRelease(_FrozenModel):
    """Exact attested Normative-Pack release selected by a logical resource."""

    id: str
    version: str
    attestor: str
    content_digest: str


class AuthorityResolvedResource(_FrozenModel):
    """Logical resource resolved by its named authority owner."""

    kind: Literal["principal", "organization", "native_application", "participant"]
    resource_id: str
    authority_resolver: str


class PackageResource(_FrozenModel):
    """Package resource bound to one exact release."""

    kind: Literal["package"] = "package"
    resource_id: str
    release: PackageRelease


class NormativePackResource(_FrozenModel):
    """Normative-Pack resource bound to one exact attested release."""

    kind: Literal["normative_pack"] = "normative_pack"
    resource_id: str
    release: NormativePackRelease


WorkspaceResourceSelection = Annotated[
    Union[AuthorityResolvedResource, PackageResource, NormativePackResource],
    Field(discriminator="kind"),
]


class WorkspaceSourceComponentRef(_FrozenModel):
    """Content-addressed semantic source component."""

    component_id: str
    component_digest: str


class WorkspaceModulePreimage(_FrozenModel):
    """Secret-free candidate consumed by the Rust module-sealing kernel."""

    module_id: str
    release_id: str
    publisher: WorkspacePublisherRef
    source_components: Tuple[WorkspaceSourceComponentRef, ...] = ()
    imports: Tuple[WorkspaceImport, ...] = ()
    exports: Tuple[WorkspaceExport, ...] = ()
    extensions: Tuple[WorkspaceExtension, ...] = ()
    resources: Tuple[WorkspaceResourceSelection, ...] = ()


class WorkspaceModule(_FrozenModel):
    """Sealed initial-v1 workspace module returned by Rust."""

    schema_id: Literal["pale/workspace-module.v1"] = Field(alias="schema")
    schema_version: Literal[1]
    module: WorkspaceModulePreimage
    module_digest: str


class BuiltWorkspaceModule(_FrozenModel):
    """Typed module plus the exact Rust-produced contract coordinate and bytes."""

    family: Literal["workspace-module.v1"] = "workspace-module.v1"
    value: WorkspaceModule
    schema_sha256: str
    canonical_bytes: bytes


__all__ = [
    "AttributeDeclaration",
    "AuthorityResolvedResource",
    "BuiltWorkspaceModule",
    "NormativePackRelease",
    "NormativePackResource",
    "PackageRelease",
    "PackageResource",
    "PolicyDeclaration",
    "RelationshipDeclaration",
    "ResourceDeclaration",
    "TraitDeclaration",
    "WorkspaceAuthorityCeiling",
    "WorkspaceControllerCeiling",
    "WorkspaceDeclaration",
    "WorkspaceDeclarationDependency",
    "WorkspaceDeclarationRef",
    "WorkspaceExclusiveAuthorityClaim",
    "WorkspaceExport",
    "WorkspaceExportProjection",
    "WorkspaceExtension",
    "WorkspaceImport",
    "WorkspaceImportPath",
    "WorkspaceModule",
    "WorkspaceModulePreimage",
    "WorkspacePolicyCeiling",
    "WorkspacePublisherRef",
    "WorkspaceResourceSelection",
    "WorkspaceRightsCeiling",
    "WorkspaceSourceComponentRef",
]
