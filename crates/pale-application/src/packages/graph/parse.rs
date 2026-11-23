//! Manifest-directed resource parsing and body admission.

use super::authoring::{
    index_package_references, package_authoring_source, package_capability_references,
    package_resource_references, PackageAuthoringSource,
};
use super::*;

#[derive(Debug, Clone)]
pub(super) enum ParsedResource {
    Doctrine(DoctrineEntry),
    OperationArchetype(OperationArchetype),
    Operation(OnDemandOperation),
    RoleArchetype(RoleArchetype),
    Role(SpecialistRole),
    Stack(Stack),
    RunMode(RunMode),
    PolicyFamily(PolicyFamilyV1),
    GoverningOrder {
        id: pale_core::records::PackageResourceIdV1,
        record: GoverningOrderV1,
    },
    CapabilityPolicy {
        id: pale_core::records::PackageResourceIdV1,
        record: CapabilityPolicy,
    },
    ToolRecipe {
        id: pale_core::records::PackageResourceIdV1,
        record: ToolRecipe,
    },
}

impl ParsedResource {
    fn id(&self) -> &pale_core::records::PackageResourceIdV1 {
        match self {
            Self::Doctrine(record) => &record.id,
            Self::OperationArchetype(record) => &record.id,
            Self::Operation(record) => &record.id,
            Self::RoleArchetype(record) => &record.id,
            Self::Role(record) => &record.id,
            Self::Stack(record) => &record.id,
            Self::RunMode(record) => &record.id,
            Self::PolicyFamily(record) => &record.id,
            Self::GoverningOrder { id, .. }
            | Self::CapabilityPolicy { id, .. }
            | Self::ToolRecipe { id, .. } => id,
        }
    }

    pub(super) fn kind(&self) -> PackageResourceKindV1 {
        match self {
            Self::Doctrine(_) => PackageResourceKindV1::Doctrine,
            Self::OperationArchetype(_) => PackageResourceKindV1::OperationArchetype,
            Self::Operation(_) => PackageResourceKindV1::Operation,
            Self::RoleArchetype(_) => PackageResourceKindV1::RoleArchetype,
            Self::Role(_) => PackageResourceKindV1::Role,
            Self::Stack(_) => PackageResourceKindV1::Stack,
            Self::RunMode(_) => PackageResourceKindV1::RunMode,
            Self::PolicyFamily(_) => PackageResourceKindV1::PolicyFamily,
            Self::GoverningOrder { .. } => PackageResourceKindV1::GoverningOrder,
            Self::CapabilityPolicy { .. } => PackageResourceKindV1::CapabilityPolicy,
            Self::ToolRecipe { .. } => PackageResourceKindV1::ToolRecipe,
        }
    }

    fn body_path(&self) -> Option<&pale_core::paths::OwnedConfinedRelativePath> {
        match self {
            Self::Doctrine(record) => Some(&record.doctrine),
            Self::OperationArchetype(record) => Some(&record.doctrine),
            Self::Operation(record) => Some(&record.doctrine),
            Self::RoleArchetype(record) => Some(&record.doctrine),
            Self::Role(record) => Some(&record.doctrine),
            Self::RunMode(record) => record.doctrine.as_ref(),
            Self::PolicyFamily(record) => Some(&record.body),
            Self::Stack(_)
            | Self::GoverningOrder { .. }
            | Self::CapabilityPolicy { .. }
            | Self::ToolRecipe { .. } => None,
        }
    }
}

impl AdmittedPackageGraph {
    /// Parse, qualify, and admit every resource in an exact retained closure.
    ///
    /// Successful admission also projects source-free definition/reference
    /// coordinates into `authoring`. That projection consumes the same parsed
    /// records and retains neither Package bytes nor syntax trees.
    ///
    /// # Errors
    ///
    /// Returns [`PackageError`] for any declaration/resource mismatch, body
    /// custody violation, unresolved local link, self-external link, transitive-
    /// only import, non-exported dependency target, or wrong resource kind.
    pub(crate) fn admit(
        snapshot: &DistributionSnapshot,
        closure: &ResourceSet<'_>,
        authoring: &mut crate::workspace::AuthoringSemanticIndex,
    ) -> Result<Self, PackageError> {
        Self::read(snapshot, closure, authoring, None)
    }

    pub(in crate::packages) fn read(
        snapshot: &DistributionSnapshot,
        closure: &ResourceSet<'_>,
        authoring: &mut crate::workspace::AuthoringSemanticIndex,
        mut catalog: Option<&mut Vec<crate::packages::catalog::CatalogResource>>,
    ) -> Result<Self, PackageError> {
        let mut parsed =
            BTreeMap::<PackageResourceUse, (ParsedResource, PackageAuthoringSource)>::new();
        let mut declarations = BTreeMap::<PackageResourceUse, PackageResourceV1>::new();
        let mut owners = BTreeMap::<PackageIdV1, &RetainedPackageRelease>::new();
        let mut parse_diagnostics = PackageDiagnosticAccumulator::default();

        for (package_id, package) in closure.packages() {
            owners.insert(package_id.clone(), *package);
            let Some(resources) =
                parse_diagnostics.capture(parse_release_resources(snapshot, package))
            else {
                continue;
            };
            for (declaration, resource, source) in resources {
                if let Some(catalog) = catalog.as_mut() {
                    catalog.push(crate::packages::catalog::CatalogResource::from_source(
                        package_id,
                        &declaration,
                        source
                            .page
                            .as_ref()
                            .map(|page| &page.path)
                            .or_else(|| resource.body_path()),
                        if let Some(page) = &source.page {
                            serde_json::to_value(page.header.as_ref()).map_err(|error| {
                                PackageError::TomlToJson {
                                    path: package.source_path(&declaration.definition_source),
                                    source: error,
                                }
                            })?
                        } else {
                            resource.record_value(
                                &package.source_path(&declaration.definition_source),
                            )?
                        },
                    ));
                }
                let qualified = PackageResourceRefV1 {
                    package_id: package_id.clone(),
                    resource_id: declaration.id.clone(),
                };
                let usage = PackageResourceUse::new(qualified, declaration.kind);
                declarations.insert(usage.clone(), declaration);
                parsed.insert(usage, (resource, source));
            }
        }
        parse_diagnostics.finish(())?;

        let mut page_ids =
            BTreeMap::<PackageIdV1, BTreeSet<pale_core::records::PackageResourceIdV1>>::new();
        for (usage, (_, source)) in &parsed {
            if source.page.is_some() {
                page_ids
                    .entry(usage.resource.package_id.clone())
                    .or_default()
                    .insert(usage.resource.resource_id.clone());
            }
        }
        let mut resources = BTreeMap::new();
        let mut bodies = BTreeMap::new();
        let mut body_owners = BTreeMap::<(PackageIdV1, String), PackageResourceRefV1>::new();
        let mut capability_policies = BTreeMap::new();
        let mut recipes = BTreeMap::new();
        let mut governing_sources = BTreeMap::new();
        let mut admission_diagnostics = PackageDiagnosticAccumulator::default();

        for (usage, (source, mut authoring_source)) in parsed {
            if let Some(page) = &mut authoring_source.page {
                if let Some(parent) = crate::packages::page::parent_topic(
                    &usage.resource.resource_id,
                    &page_ids[&usage.resource.package_id],
                ) {
                    let parent = PackageResourceLinkV1::Local {
                        resource_id: parent.clone(),
                    };
                    if !page.required.contains(&parent) {
                        page.required.insert(0, parent);
                    }
                }
            }
            let resource_ref = usage.resource.clone();
            let owner = owners[&resource_ref.package_id];
            let declaration = &declarations[&usage];
            let references = authoring_source.page.as_ref().map_or_else(
                || {
                    package_resource_references(
                        &source,
                        &resource_ref.package_id,
                        &authoring_source.content_path,
                    )
                },
                |page| page.references.clone(),
            );
            let capabilities =
                package_capability_references(&source, &authoring_source.content_path);
            if let Some(body_path) = authoring_source
                .page
                .as_ref()
                .map(|page| &page.path)
                .or_else(|| source.body_path())
            {
                admission_diagnostics.capture(admit_body(
                    snapshot,
                    owner,
                    &resource_ref,
                    body_path,
                    authoring_source.page.as_ref().map(|page| &page.body),
                    &mut body_owners,
                    &mut bodies,
                ));
            }
            match &source {
                ParsedResource::CapabilityPolicy { record: policy, .. } => {
                    capability_policies.insert(resource_ref.clone(), policy.clone());
                }
                ParsedResource::ToolRecipe { record: recipe, .. } => {
                    recipes.insert(resource_ref.clone(), recipe.clone());
                }
                ParsedResource::GoverningOrder { record: order, .. } => {
                    governing_sources.insert(
                        resource_ref.clone(),
                        (order.clone(), authoring_source.clone(), references.clone()),
                    );
                }
                _ => {}
            }
            let page =
                match &authoring_source.page {
                    Some(page) => match admission_diagnostics.capture(
                        super::pages::qualify_reading(page, &resource_ref, owner, &declarations),
                    ) {
                        Some(reading) => Some(reading),
                        None => continue,
                    },
                    None => None,
                };
            if let Some(resource) = admission_diagnostics.capture(qualify_resource(
                source,
                &resource_ref,
                owner,
                &declarations,
            )) {
                authoring.define_package_resource(
                    &resource_ref,
                    declaration.kind,
                    &authoring_source.source_label,
                    authoring_source.definition_path,
                );
                if declaration.kind != PackageResourceKindV1::GoverningOrder {
                    index_package_references(
                        authoring,
                        &declarations,
                        &authoring_source.source_label,
                        references,
                    );
                }
                for (capability, path) in capabilities {
                    authoring.reference_package_capability(
                        &capability,
                        &authoring_source.source_label,
                        path,
                    );
                }
                resources.insert(
                    PackageResourceUse::new(resource_ref, declaration.kind),
                    AdmittedResourceNode {
                        page,
                        exposure: declaration.exposure,
                        resource,
                    },
                );
            }
        }
        admission_diagnostics.finish(())?;

        super::body_links::qualify_body_links(&body_owners, &resources, &mut bodies)?;

        let mut governing = BTreeMap::new();
        let mut governing_diagnostics = PackageDiagnosticAccumulator::default();
        for (contributor, (source, authoring_source, references)) in governing_sources {
            let owner = owners[&contributor.package_id];
            if let Some(contribution) = governing_diagnostics.capture(qualify_governing(
                &contributor,
                &source,
                owner,
                &declarations,
            )) {
                index_package_references(
                    authoring,
                    &declarations,
                    &authoring_source.source_label,
                    references,
                );
                governing.insert(contributor, contribution);
            }
        }
        governing_diagnostics.finish(())?;
        validate_operation_procedures(&resources, &capability_policies, &recipes)?;
        Ok(Self {
            releases: closure.releases().to_vec(),
            resources,
            bodies,
            capability_policies,
            recipes,
            governing,
        })
    }
}

impl ParsedResource {
    fn record_value(&self, path: &std::path::Path) -> Result<serde_json::Value, PackageError> {
        let value = match self {
            Self::Doctrine(record) => serde_json::to_value(record),
            Self::OperationArchetype(record) => serde_json::to_value(record),
            Self::Operation(record) => serde_json::to_value(record),
            Self::RoleArchetype(record) => serde_json::to_value(record),
            Self::Role(record) => serde_json::to_value(record),
            Self::Stack(record) => serde_json::to_value(record),
            Self::RunMode(record) => serde_json::to_value(record),
            Self::PolicyFamily(record) => serde_json::to_value(record),
            Self::GoverningOrder { record, .. } => serde_json::to_value(GoverningOrderFile {
                governing_order: record.clone(),
            }),
            Self::CapabilityPolicy { record, .. } => serde_json::to_value(record),
            Self::ToolRecipe { record, .. } => serde_json::to_value(record),
        };
        value.map_err(|source| PackageError::TomlToJson {
            path: path.to_owned(),
            source,
        })
    }
}

fn parse_release_resources(
    snapshot: &DistributionSnapshot,
    package: &RetainedPackageRelease,
) -> Result<Vec<(PackageResourceV1, ParsedResource, PackageAuthoringSource)>, PackageError> {
    let mut by_source = BTreeMap::<String, Vec<PackageResourceV1>>::new();
    for resource in &package.manifest().package.resources {
        by_source
            .entry(resource.definition_source.as_str().to_string())
            .or_default()
            .push(resource.clone());
    }
    let mut admitted = Vec::new();
    let mut diagnostics = PackageDiagnosticAccumulator::default();
    for (source_path, declarations) in by_source {
        let confined = &declarations[0].definition_source;
        let Some(text) = diagnostics.capture(package.source_text(snapshot, confined)) else {
            continue;
        };
        let diagnostic_path = package.source_path(confined);
        if source_path.ends_with(".md") {
            let Some(rows) = diagnostics.capture(super::pages::parse(
                text,
                &diagnostic_path,
                &package.release().id,
                confined,
            )) else {
                continue;
            };
            let expected = declarations
                .iter()
                .map(|declaration| {
                    (
                        declaration.id.clone(),
                        declaration.kind,
                        declaration.exposure,
                    )
                })
                .collect::<BTreeSet<_>>();
            let actual = rows
                .iter()
                .map(|(declaration, _, _)| {
                    (
                        declaration.id.clone(),
                        declaration.kind,
                        declaration.exposure,
                    )
                })
                .collect::<BTreeSet<_>>();
            if expected != actual {
                diagnostics.push(PackageError::ResourceDeclarationMismatch {
                    package: package.release().id.clone(),
                    source_path,
                    resources: declaration_ids(&declarations),
                });
            } else {
                admitted.extend(rows);
            }
            continue;
        }
        if package.is_typed_data(confined) {
            let Some(data) = diagnostics.capture(crate::packages::inventory::read_data(
                text,
                &diagnostic_path,
            )) else {
                continue;
            };
            let Some(declaration) =
                diagnostics.capture(crate::packages::inventory::data_declaration(
                    &package.release().id,
                    confined,
                    &data,
                ))
            else {
                continue;
            };
            if declarations != [declaration.clone()] {
                diagnostics.push(PackageError::ResourceDeclarationMismatch {
                    package: package.release().id.clone(),
                    source_path,
                    resources: declaration_ids(&declarations),
                });
                continue;
            }
            let (resource, field) = match data {
                pale_core::records::PackageDataV1::ToolRecipe { recipe, .. } => (
                    ParsedResource::ToolRecipe {
                        id: declaration.id.clone(),
                        record: recipe,
                    },
                    "recipe",
                ),
                pale_core::records::PackageDataV1::CapabilityPolicy { policy, .. } => (
                    ParsedResource::CapabilityPolicy {
                        id: declaration.id.clone(),
                        record: policy,
                    },
                    "policy",
                ),
                pale_core::records::PackageDataV1::GoverningOrder {
                    governing_order, ..
                } => (
                    ParsedResource::GoverningOrder {
                        id: declaration.id.clone(),
                        record: governing_order,
                    },
                    "governing_order",
                ),
            };
            admitted.push((
                declaration,
                resource,
                PackageAuthoringSource {
                    page: None,
                    source_label: diagnostic_path.to_string_lossy().into_owned(),
                    content_path: vec![crate::workspace::AuthoringInstancePathSegment::Field(
                        field.into(),
                    )],
                    definition_path: Vec::new(),
                },
            ));
            continue;
        }
        let Some(parsed) = diagnostics.capture(parse_declaration_group(
            text,
            &diagnostic_path,
            &package.release().id,
            &declarations,
        )) else {
            continue;
        };
        let expected = declarations
            .iter()
            .map(|resource| (resource.id.clone(), resource.kind))
            .collect::<BTreeSet<_>>();
        let actual = parsed
            .iter()
            .map(|resource| (resource.id().clone(), resource.kind()))
            .collect::<BTreeSet<_>>();
        if expected != actual || expected.len() != parsed.len() {
            diagnostics.push(PackageError::ResourceDeclarationMismatch {
                package: package.release().id.clone(),
                source_path,
                resources: declaration_ids(&declarations),
            });
            continue;
        }
        let source_label = diagnostic_path.to_string_lossy().into_owned();
        let mut kind_ordinals = BTreeMap::<PackageResourceKindV1, usize>::new();
        let mut parsed_by_id = parsed
            .into_iter()
            .map(|resource| {
                let ordinal = kind_ordinals.entry(resource.kind()).or_default();
                let authoring_source =
                    package_authoring_source(&resource, *ordinal, source_label.clone());
                *ordinal += 1;
                (
                    (resource.id().clone(), resource.kind()),
                    (resource, authoring_source),
                )
            })
            .collect::<BTreeMap<_, _>>();
        for declaration in declarations {
            let (resource, authoring_source) = parsed_by_id
                .remove(&(declaration.id.clone(), declaration.kind))
                .expect("the exact declaration bijection was established");
            admitted.push((declaration, resource, authoring_source));
        }
    }
    diagnostics.finish(admitted)
}

fn parse_declaration_group(
    text: &str,
    source: &std::path::Path,
    package_id: &PackageIdV1,
    declarations: &[PackageResourceV1],
) -> Result<Vec<ParsedResource>, PackageError> {
    let Some(grammar) = declaration_grammar(declarations) else {
        return Err(PackageError::ResourceDeclarationMismatch {
            package: package_id.clone(),
            source_path: source.to_string_lossy().into_owned(),
            resources: declaration_ids(declarations),
        });
    };
    let resources = match grammar {
        PackageDeclarationGrammar::Doctrine => {
            let file: DoctrineFile = parse_root(text, source, "doctrine-entry.v1", "doctrine")?;
            file.doctrine
                .entries
                .into_iter()
                .map(ParsedResource::Doctrine)
                .collect()
        }
        PackageDeclarationGrammar::Operations => {
            let file: OperationsFile = parse_root(text, source, "operation.v1", "operations")?;
            file.operations
                .archetypes
                .into_iter()
                .map(ParsedResource::OperationArchetype)
                .chain(
                    file.operations
                        .operations
                        .into_iter()
                        .map(ParsedResource::Operation),
                )
                .collect()
        }
        PackageDeclarationGrammar::Roles => {
            let file: RolesFile = parse_root(text, source, "role.v1", "roles")?;
            file.roles
                .archetypes
                .into_iter()
                .map(ParsedResource::RoleArchetype)
                .chain(file.roles.roles.into_iter().map(ParsedResource::Role))
                .collect()
        }
        PackageDeclarationGrammar::Stacks => {
            let file: StacksFile = parse_root(text, source, "stack.v1", "stacks")?;
            file.stacks
                .stacks
                .into_iter()
                .map(ParsedResource::Stack)
                .collect()
        }
        PackageDeclarationGrammar::RunModes => {
            let file: RunModesFile = parse_root(text, source, "run-mode.v1", "run modes")?;
            file.run_modes
                .modes
                .into_iter()
                .map(ParsedResource::RunMode)
                .collect()
        }
        PackageDeclarationGrammar::PolicyFamily => vec![with_manifest_identity(
            ParsedResource::PolicyFamily(parse_root(
                text,
                source,
                "policy-family.v1",
                "policy family",
            )?),
            &declarations[0],
        )],
        PackageDeclarationGrammar::GoverningOrder => {
            let file: GoverningOrderFile =
                parse_root(text, source, "governing-order.v1", "governing order")?;
            vec![with_manifest_identity(
                ParsedResource::GoverningOrder {
                    id: declarations[0].id.clone(),
                    record: file.governing_order,
                },
                &declarations[0],
            )]
        }
        PackageDeclarationGrammar::CapabilityPolicy => vec![with_manifest_identity(
            ParsedResource::CapabilityPolicy {
                id: declarations[0].id.clone(),
                record: parse_named_definition(
                    text,
                    source,
                    "capability-procedure.v1",
                    "CapabilityPolicy",
                    "capability policy",
                )?,
            },
            &declarations[0],
        )],
        PackageDeclarationGrammar::ToolRecipe => vec![with_manifest_identity(
            ParsedResource::ToolRecipe {
                id: declarations[0].id.clone(),
                record: parse_named_definition(
                    text,
                    source,
                    "capability-procedure.v1",
                    "ToolRecipe",
                    "tool recipe",
                )?,
            },
            &declarations[0],
        )],
    };
    Ok(resources)
}

fn declaration_ids(
    declarations: &[PackageResourceV1],
) -> Vec<pale_core::records::PackageResourceIdV1> {
    let mut resources = declarations
        .iter()
        .map(|resource| resource.id.clone())
        .collect::<Vec<_>>();
    resources.sort();
    resources
}

fn with_manifest_identity(
    resource: ParsedResource,
    _declaration: &PackageResourceV1,
) -> ParsedResource {
    resource
}

fn parse_root<T: serde::de::DeserializeOwned>(
    text: &str,
    source: &std::path::Path,
    schema_id: &str,
    label: &str,
) -> Result<T, PackageError> {
    let schema = CompiledSchema::for_id(schema_id)?;
    parse_source_with_schema(text, source, &schema, label)
}

fn admit_body(
    snapshot: &DistributionSnapshot,
    package: &RetainedPackageRelease,
    resource: &PackageResourceRefV1,
    body_path: &pale_core::paths::OwnedConfinedRelativePath,
    page_body: Option<&std::ops::Range<usize>>,
    owners: &mut BTreeMap<(PackageIdV1, String), PackageResourceRefV1>,
    bodies: &mut BTreeMap<PackageResourceRefV1, AdmittedMarkdown>,
) -> Result<(), PackageError> {
    let declared_role = package
        .manifest()
        .package
        .sources
        .iter()
        .find(|source| source.path == *body_path)
        .map(|source| source.role);
    let expected_role = if page_body.is_some() {
        PackageSourceRoleV1::Declaration
    } else {
        PackageSourceRoleV1::Body
    };
    if declared_role != Some(expected_role) {
        return Err(PackageError::InvalidBodyCustody {
            resource: resource.clone(),
            body: body_path.as_str().to_string(),
            reason: BodyError::UndeclaredBody,
        });
    }
    let key = (resource.package_id.clone(), body_path.as_str().to_string());
    if let Some(previous) = owners.get(&key) {
        if previous == resource && page_body.is_some() {
            return Ok(());
        }

        return Err(PackageError::InvalidBodyCustody {
            resource: resource.clone(),
            body: body_path.as_str().to_string(),
            reason: BodyError::ReusedBody,
        });
    }
    owners.insert(key, resource.clone());
    let text = package.source_text(snapshot, body_path)?;
    let text = page_body.map_or(text, |range| &text[range.clone()]);
    bodies.insert(
        resource.clone(),
        AdmittedMarkdown::from_text(resource.clone(), text),
    );
    Ok(())
}
