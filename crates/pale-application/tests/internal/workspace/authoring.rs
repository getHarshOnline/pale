//! Generated authoring-grammar and source-custody laws.

use super::*;

fn denoted_declaration(owner: &str, namespace: &str, name: &str) -> DenotedAuthoringIdentity {
    DenotedAuthoringIdentity::declaration(&QualifiedDeclarationRef {
        owner_module: WorkspaceModuleId::parse(owner).unwrap(),
        local_ref: pale_core::records::WorkspaceDeclarationRefV1 {
            namespace: pale_core::records::WorkspaceNamespaceId::parse(namespace).unwrap(),
            declaration_id: pale_core::records::WorkspaceDeclarationId::parse(name).unwrap(),
        },
    })
}

#[test]
fn subject_anchors_preserve_navigation_roles_and_share_one_retained_key() {
    let target = denoted_declaration("owner", "names", "same");
    let identity = target.identity.clone();
    let subject = target.subject.clone();
    let mut index = AuthoringSemanticIndex::default();
    for (label, role) in [
        ("a.toml", AuthoringOccurrenceRole::Projection),
        ("b.toml", AuthoringOccurrenceRole::Reference),
        ("c.toml", AuthoringOccurrenceRole::Definition),
    ] {
        index.insert_denoted(
            denoted_declaration("owner", "names", "same"),
            label,
            path([AuthoringInstancePathSegment::Field("id".into())]),
            role,
        );
    }
    // Annotations change no existing navigation result, including first-entry
    // fallback definition behavior. This compares actual public queries.
    let mut baseline = index.clone();
    for candidate in baseline.candidates.values_mut() {
        candidate.proof_subject = None;
    }
    assert_eq!(index.definition(&identity), baseline.definition(&identity));
    assert_eq!(
        index.reference_count(&identity),
        baseline.reference_count(&identity)
    );
    assert_eq!(
        index.references(&identity).collect::<Vec<_>>(),
        baseline.references(&identity).collect::<Vec<_>>()
    );
    assert_eq!(
        index
            .definitions(AuthoringIdentityKind::Declaration)
            .collect::<Vec<_>>(),
        baseline
            .definitions(AuthoringIdentityKind::Declaration)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        index.completion_alternatives(&identity).collect::<Vec<_>>(),
        baseline
            .completion_alternatives(&identity)
            .collect::<Vec<_>>()
    );
    let location = [AuthoringInstancePathSegment::Field("id".into())];
    assert_eq!(
        index
            .completion_alternatives_at("a.toml", &location)
            .collect::<Vec<_>>(),
        baseline
            .completion_alternatives_at("a.toml", &location)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        index.occurrence("a.toml", &location),
        baseline.occurrence("a.toml", &location)
    );
    let old_page = index.occurrences_page(&identity, None, 10, 10);
    let baseline_page = baseline.occurrences_page(&identity, None, 10, 10);
    let subject_page = index.subject_occurrences_page(&subject, None, 10, 10);
    assert_eq!(old_page.occurrences(), baseline_page.occurrences());
    assert_eq!(subject_page.occurrences(), old_page.occurrences());
    assert_eq!(subject_page.inspected(), old_page.inspected());
    assert!(subject_page.is_complete());
    assert_eq!(
        subject_page
            .occurrences()
            .iter()
            .map(|row| row.role())
            .collect::<Vec<_>>(),
        vec![
            AuthoringOccurrenceRole::Projection,
            AuthoringOccurrenceRole::Reference,
            AuthoringOccurrenceRole::Definition,
        ]
    );
    assert_eq!(index.reference_count(&identity), 2);
    assert_eq!(
        index.definition(&identity).unwrap().role(),
        AuthoringOccurrenceRole::Projection
    );
    assert_eq!(
        index
            .candidates
            .values()
            .filter(|candidate| candidate.proof_subject.is_some())
            .count(),
        1
    );
    for occurrence in subject_page.occurrences() {
        assert!(std::ptr::eq(
            index.proof_subject(occurrence.identity()).unwrap(),
            index.proof_subject(&identity).unwrap()
        ));
    }
}

#[test]
fn extending_disjoint_source_indexes_keeps_denotation_and_navigation_custody() {
    let denotation = denoted_declaration("owner", "scope", "target");
    let identity = denotation.identity.clone();
    let subject = denotation.subject.clone();
    let mut first = AuthoringSemanticIndex::default();
    first.reference(identity.clone(), "reference.toml", Vec::new());
    let mut definitions = AuthoringSemanticIndex::default();
    definitions.define_denoted(denotation, "definition.toml", Vec::new());
    first.extend(definitions).unwrap();
    let mut projections = AuthoringSemanticIndex::default();
    projections.insert_denoted(
        denoted_declaration("owner", "scope", "target"),
        "projection.toml",
        Vec::new(),
        AuthoringOccurrenceRole::Projection,
    );
    first.extend(projections).unwrap();
    assert_eq!(first.proof_subject(&identity), Some(&subject));
    assert_eq!(
        first.definition(&identity).unwrap().source_label(),
        "definition.toml"
    );
    assert_eq!(
        first
            .completion_alternatives(&identity)
            .next()
            .unwrap()
            .source_label(),
        "reference.toml"
    );
    assert_eq!(first.reference_count(&identity), 2);
    let page = first.subject_occurrences_page(&subject, None, 10, 10);
    assert_eq!(
        page.occurrences()
            .iter()
            .map(|row| row.role())
            .collect::<Vec<_>>(),
        vec![
            AuthoringOccurrenceRole::Definition,
            AuthoringOccurrenceRole::Projection,
            AuthoringOccurrenceRole::Reference,
        ]
    );
    let before = first.clone();
    let mut overlapping = AuthoringSemanticIndex::default();
    overlapping.define_denoted(
        denoted_declaration("other", "scope", "target"),
        "definition.toml",
        Vec::new(),
    );
    assert_eq!(
        first.extend(overlapping),
        Err(AuthoringSemanticMergeError::OverlappingSourceCoordinate(
            "definition.toml".into()
        ))
    );
    assert_eq!(first, before);
}

#[test]
fn record_anchor_phase_merges_retain_custody_and_reject_conflicts_atomically() {
    let subject = WorkspaceDerivationSubjectV1::Extension {
        owner_module: WorkspaceModuleId::parse("owner").unwrap(),
        namespace: pale_core::records::WorkspaceNamespaceId::parse("augmentation").unwrap(),
        target: pale_core::records::WorkspaceDeclarationRefV1 {
            namespace: pale_core::records::WorkspaceNamespaceId::parse("names").unwrap(),
            declaration_id: pale_core::records::WorkspaceDeclarationId::parse("target").unwrap(),
        },
    };
    let anchor = WorkspaceAuthoringLocation::new(
        "module.toml".into(),
        vec![
            AuthoringInstancePathSegment::Field("module".into()),
            AuthoringInstancePathSegment::Field("extensions".into()),
            AuthoringInstancePathSegment::Index(1),
        ],
    );
    let mut first = AuthoringSemanticIndex::default();
    assert!(first.subject_record_anchor(&subject).is_none());
    let mut incoming = AuthoringSemanticIndex::default();
    incoming
        .record_anchors
        .insert(subject.clone(), anchor.clone());
    first.extend(incoming.clone()).unwrap();
    assert_eq!(first.subject_record_anchor(&subject), Some(&anchor));
    first.extend(incoming).unwrap();
    first.extend(AuthoringSemanticIndex::default()).unwrap();
    assert_eq!(first.record_anchors.len(), 1);

    let identity = denoted_declaration("owner", "names", "target").identity;
    first.reference(identity.clone(), "reference.toml", Vec::new());
    let before = first.clone();
    let mut conflicting = AuthoringSemanticIndex::default();
    let mut earlier_subject = subject.clone();
    let WorkspaceDerivationSubjectV1::Extension { owner_module, .. } = &mut earlier_subject else {
        unreachable!()
    };
    *owner_module = WorkspaceModuleId::parse("earlier-owner").unwrap();
    assert!(earlier_subject < subject);
    conflicting
        .record_anchors
        .insert(earlier_subject.clone(), anchor.clone());
    conflicting.record_anchors.insert(
        subject.clone(),
        WorkspaceAuthoringLocation::new("other-module.toml".into(), anchor.path().to_vec()),
    );
    conflicting.reference(identity.clone(), "new-reference.toml", Vec::new());
    let mut both_conflicts = conflicting.clone();
    both_conflicts.reference(identity, "reference.toml", Vec::new());
    let error = first.extend(both_conflicts).unwrap_err();
    assert_eq!(
        error,
        AuthoringSemanticMergeError::OverlappingSourceCoordinate("reference.toml".into())
    );
    assert_eq!(
        error.to_string(),
        "logical source label `reference.toml` owns overlapping semantic coordinates"
    );
    assert_eq!(first, before);
    assert_eq!(
        first.extend(conflicting.clone()),
        Err(AuthoringSemanticMergeError::ConflictingRecordAnchor)
    );
    assert_eq!(first, before);
    assert!(first.subject_record_anchor(&earlier_subject).is_none());
    let mut different_path = anchor.path().to_vec();
    *different_path.last_mut().unwrap() = AuthoringInstancePathSegment::Index(0);
    conflicting.record_anchors.insert(
        subject.clone(),
        WorkspaceAuthoringLocation::new(anchor.source_label().to_owned(), different_path),
    );
    assert_eq!(
        first.extend(conflicting),
        Err(AuthoringSemanticMergeError::ConflictingRecordAnchor)
    );
    assert_eq!(first, before);
    assert_eq!(first.subject_record_anchor(&subject), Some(&anchor));
}

#[test]
fn denotation_keys_keep_owner_namespace_kind_and_source_grammar_distinct() {
    let mut index = AuthoringSemanticIndex::default();
    for (row, (owner, namespace)) in [
        ("owner", "scope"),
        ("other", "scope"),
        ("owner", "other"),
        ("owner:part", "scope/part"),
    ]
    .into_iter()
    .enumerate()
    {
        index.define_denoted(
            denoted_declaration(owner, namespace, "same"),
            &format!("{row}.toml"),
            Vec::new(),
        );
    }
    let wanted = denoted_declaration("owner", "scope", "same");
    assert_eq!(
        index
            .subject_occurrences_page(&wanted.subject, None, 10, 10)
            .occurrences()
            .len(),
        1
    );
    assert_ne!(
        wanted.identity,
        denoted_declaration("other", "scope", "same").identity
    );
    assert_ne!(
        wanted.identity,
        denoted_declaration("owner", "other", "same").identity
    );
    let WorkspaceDerivationSubjectV1::Declaration {
        owner_module,
        declaration,
    } = wanted.subject
    else {
        unreachable!()
    };
    let policy = WorkspaceDerivationSubjectV1::Policy {
        owner_module: owner_module.clone(),
        declaration,
    };
    assert!(index
        .subject_occurrences_page(&policy, None, 10, 10)
        .occurrences()
        .is_empty());
    let import = WorkspaceDerivationSubjectV1::Import {
        importer_module: owner_module.clone(),
        upstream_module: owner_module,
        upstream_release: pale_core::records::WorkspaceReleaseId::parse("same").unwrap(),
        export_id: pale_core::records::WorkspaceExportId::parse("same").unwrap(),
        source_namespace: pale_core::records::WorkspaceNamespaceId::parse("scope").unwrap(),
        local_namespace: pale_core::records::WorkspaceNamespaceId::parse("same").unwrap(),
    };
    assert!(index
        .subject_occurrences_page(&import, None, 10, 10)
        .occurrences()
        .is_empty());
    assert!(WorkspaceModuleId::parse("owner::part").is_err());

    let source = WorkspaceDerivationSourceV1 {
        source_id: pale_core::records::WorkspaceDerivationSourceId::parse("module:owner").unwrap(),
        source_path: pale_core::paths::OwnedConfinedRelativePath::parse("same.toml").unwrap(),
        source_digest: pale_core::records::WorkspaceSourceContentDigest::hash_bytes(b"source"),
        byte_len: 6,
    };
    index.define_denoted(
        DenotedAuthoringIdentity::source(&source, "workspace-module.v1"),
        "same.toml",
        Vec::new(),
    );
    assert!(index
        .proof_subject(&source_document_identity(
            "same.toml",
            "workspace-module.v1"
        ))
        .is_some());
    assert!(index
        .proof_subject(&source_document_identity(
            "same.toml",
            "workspace-language-primitives.v1"
        ))
        .is_none());
    let renamed = WorkspaceDerivationSourceV1 {
        source_path: pale_core::paths::OwnedConfinedRelativePath::parse("renamed.toml").unwrap(),
        ..source.clone()
    };
    assert_eq!(
        DenotedAuthoringIdentity::source(&source, "workspace-module.v1").subject,
        DenotedAuthoringIdentity::source(&renamed, "workspace-module.v1").subject
    );
    assert_ne!(
        DenotedAuthoringIdentity::source(&source, "workspace-module.v1").identity,
        DenotedAuthoringIdentity::source(&renamed, "workspace-module.v1").identity
    );
}

#[test]
fn subject_occurrence_pages_charge_candidate_lookups_and_exact_terminal_stops() {
    let subject = denoted_declaration("owner", "scope", "target").subject;
    let mut index = AuthoringSemanticIndex::default();
    for (label, name) in [
        ("a", "other"),
        ("b", "other"),
        ("c", "target"),
        ("d", "target"),
        ("e", "other"),
    ] {
        index.reference_denoted(
            denoted_declaration("owner", "scope", name),
            label,
            Vec::new(),
        );
    }
    let sparse = index.subject_occurrences_page(&subject, None, 2, 2);
    assert!(sparse.occurrences().is_empty());
    assert_eq!(sparse.inspected(), 2);
    assert!(!sparse.is_complete());
    let matched = index.subject_occurrences_page(&subject, sparse.continuation(), 4, 1);
    assert_eq!(matched.inspected(), 1);
    assert_eq!(matched.occurrences()[0].source_label(), "c");
    let final_bound = index.subject_occurrences_page(&subject, matched.continuation(), 2, 2);
    assert_eq!(final_bound.inspected(), 2);
    assert_eq!(final_bound.occurrences()[0].source_label(), "d");
    assert!(!final_bound.is_complete());
    let terminal = index.subject_occurrences_page(&subject, final_bound.continuation(), 2, 2);
    assert!(terminal.occurrences().is_empty());
    assert_eq!(terminal.inspected(), 0);
    assert!(terminal.is_complete());

    let mut many = AuthoringSemanticIndex::default();
    for row in 0..300 {
        many.reference_denoted(
            denoted_declaration("owner", "scope", "target"),
            &format!("{row:03}"),
            Vec::new(),
        );
    }
    for (inspected, returned, expected) in [
        (0, usize::MAX, 64),
        (usize::MAX, 0, 64),
        (usize::MAX, usize::MAX, 256),
    ] {
        let page = many.subject_occurrences_page(&subject, None, inspected, returned);
        assert_eq!(page.inspected(), expected);
        assert_eq!(page.occurrences().len(), expected);
        assert!(!page.is_complete());
    }
    // Report representation observations without making target layout a contract.
    eprintln!("denotation retention: occurrences={}, candidates={}, anchors={}, subject_layout={}, optional_box_layout={}",
        many.occurrences.len(), many.candidates.len(),
        many.candidates.values().filter(|candidate| candidate.proof_subject.is_some()).count(),
        std::mem::size_of::<WorkspaceDerivationSubjectV1>(),
        std::mem::size_of::<Option<Box<WorkspaceDerivationSubjectV1>>>());
}

fn field(name: &str) -> AuthoringPathSegment {
    AuthoringPathSegment::Field(name.to_owned())
}

#[test]
fn workspace_visibility_literals_and_variant_docs_come_from_one_generated_owner() {
    let fact = inspect_authoring_node(
        AuthoringTomlGrammar::record("workspace-module.v1"),
        &[
            field("module"),
            field("exports"),
            AuthoringPathSegment::ArrayItem,
            field("visibility"),
        ],
    )
    .expect("inspect generated visibility grammar");

    assert_eq!(fact.value_kinds, vec![AuthoringValueKind::String]);
    let values = fact
        .allowed_values
        .iter()
        .map(|value| (value.value.clone(), value.description.clone()))
        .collect::<Vec<_>>();
    assert_eq!(
        values,
        vec![
            (
                AuthoringLiteral::String("private".into()),
                Some("The surface is confined to its declaring module.".into())
            ),
            (
                AuthoringLiteral::String("protected".into()),
                Some("Resolution requires a separately verified protected-delivery grant.".into())
            ),
            (
                AuthoringLiteral::String("public".into()),
                Some("Any admitted importer may resolve the declared surface.".into())
            ),
        ]
    );
    assert!(fact
        .description
        .as_deref()
        .is_some_and(|description| description.contains("Visibility of the surface")));
}

#[test]
fn authoring_roots_follow_only_their_generated_dependency_closure() {
    let fact = inspect_authoring_node(
        AuthoringTomlGrammar::record("capabilities.v1"),
        &[
            field("lifecycle"),
            field("behaviour"),
            AuthoringPathSegment::ArrayItem,
            field("event"),
        ],
    )
    .expect("follow lifecycle-event dependency");

    let events = fact
        .allowed_values
        .iter()
        .filter_map(|value| match &value.value {
            AuthoringLiteral::String(value) => Some(value.as_str()),
            AuthoringLiteral::Integer(_) | AuthoringLiteral::Boolean(_) => None,
        })
        .collect::<Vec<_>>();
    assert!(events.contains(&"pre-tool-use"));
    assert!(events.contains(&"post-tool-use"));
    assert!(events.contains(&"session-end"));
}

#[test]
fn disjoint_source_map_merge_is_atomic_on_duplicate_labels() {
    let mut left = AuthoringSourceMap::default();
    left.insert(
        "workspace.toml",
        AuthoringDocumentKind::Toml(AuthoringTomlGrammar::record("pale-workspace.v1")),
    )
    .expect("first source");
    let mut right = AuthoringSourceMap::default();
    right
        .insert(
            "package.toml",
            AuthoringDocumentKind::Toml(AuthoringTomlGrammar::record("operation.v1")),
        )
        .expect("disjoint source");
    right
        .insert(
            "workspace.toml",
            AuthoringDocumentKind::Toml(AuthoringTomlGrammar::record("workspace-module.v1")),
        )
        .expect("duplicate exists only across maps");

    assert_eq!(left.extend(right), Err("workspace.toml".to_owned()));
    assert!(left.document("package.toml").is_none());
    assert_eq!(left.documents().len(), 1);
}

#[test]
fn structural_completion_domains_do_not_require_an_exact_admitted_occurrence() {
    let instance_field = |name: &str| AuthoringInstancePathSegment::Field(name.to_owned());
    let mut index = AuthoringSemanticIndex::default();
    let first = scoped_identity(
        AuthoringIdentityKind::PackageRole,
        "owner:first".into(),
        "owner".into(),
        "first".into(),
    );
    let second = scoped_identity(
        AuthoringIdentityKind::PackageRole,
        "owner:second".into(),
        "owner".into(),
        "second".into(),
    );
    let other_owner = scoped_identity(
        AuthoringIdentityKind::PackageRole,
        "other:first".into(),
        "other".into(),
        "first".into(),
    );
    index.define(
        first.clone(),
        "owner/roles.toml",
        path([instance_field("id")]),
    );
    index.define(
        second,
        "owner/roles.toml",
        path([instance_field("other_id")]),
    );
    index.define(
        other_owner,
        "other/roles.toml",
        path([instance_field("id")]),
    );
    index.reference(
        first,
        "workspace.toml",
        path([
            instance_field("roles"),
            at(0),
            instance_field("resource_id"),
        ]),
    );

    let incomplete = path([
        instance_field("roles"),
        at(7),
        instance_field("resource_id"),
    ]);
    assert!(index.occurrence("workspace.toml", &incomplete).is_none());
    assert_eq!(
        index
            .completion_alternatives_at("workspace.toml", &incomplete)
            .map(|occurrence| occurrence.identity().value())
            .collect::<Vec<_>>(),
        vec!["owner:first", "owner:second"]
    );
}

#[test]
fn composition_participants_share_one_namespace_without_erasing_variant_identity() {
    use pale_core::records::ParticipantRefV1;

    let participants = [
        ParticipantRefV1::Interface {
            interface_id: "shared".into(),
        },
        ParticipantRefV1::Application {
            application_id: "shared".into(),
        },
        ParticipantRefV1::Engine {
            engine_id: "shared".into(),
        },
        ParticipantRefV1::Capability {
            capability_id: "shared".into(),
        },
        ParticipantRefV1::TerminalLeaf {
            terminal_leaf_id: "shared".into(),
        },
        ParticipantRefV1::Plugin {
            plugin_id: "shared".into(),
        },
        ParticipantRefV1::Skill {
            skill_id: "shared".into(),
        },
    ];
    let identities = participants
        .iter()
        .map(|participant| composition_participant_identity(participant).0)
        .collect::<BTreeSet<_>>();

    assert_eq!(identities.len(), participants.len());
    assert!(identities
        .iter()
        .all(|identity| identity.kind() == AuthoringIdentityKind::CompositionParticipant));
    assert!(identities
        .iter()
        .all(|identity| identity.authored_value() == "shared"));
}

#[test]
fn reverse_occurrences_use_full_identity_and_canonical_source_order() {
    let target = scoped_identity(
        AuthoringIdentityKind::PackageRole,
        "owner:shared".into(),
        "owner".into(),
        "shared".into(),
    );
    let same_display_identity = scoped_identity(
        AuthoringIdentityKind::PackageRole,
        "owner:shared".into(),
        "other-owner".into(),
        "shared".into(),
    );
    let mut index = AuthoringSemanticIndex::default();
    index.reference(
        target.clone(),
        "z.toml",
        path([AuthoringInstancePathSegment::Field("reference".into())]),
    );
    index.reference(
        same_display_identity,
        "a.toml",
        path([AuthoringInstancePathSegment::Field("other".into())]),
    );
    index.reference(
        target.clone(),
        "a.toml",
        path([AuthoringInstancePathSegment::Field("reference".into())]),
    );
    index.define(
        target.clone(),
        "a.toml",
        path([AuthoringInstancePathSegment::Field("definition".into())]),
    );
    for projection_index in [1, 0] {
        index.insert(
            target.clone(),
            "m.toml",
            path([
                AuthoringInstancePathSegment::Field("items".into()),
                at(projection_index),
            ]),
            AuthoringOccurrenceRole::Projection,
        );
    }

    let reference_count = index.reference_count(&target);
    let references = index.references(&target).cloned().collect::<Vec<_>>();
    let page = index.occurrences_page(&target, None, 10, 10);
    let coordinates = page
        .occurrences()
        .iter()
        .map(|occurrence| {
            (
                occurrence.source_label().to_owned(),
                occurrence.path().to_vec(),
                occurrence.role(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        coordinates,
        vec![
            (
                "a.toml".to_owned(),
                vec![AuthoringInstancePathSegment::Field("definition".into())],
                AuthoringOccurrenceRole::Definition,
            ),
            (
                "a.toml".to_owned(),
                vec![AuthoringInstancePathSegment::Field("reference".into())],
                AuthoringOccurrenceRole::Reference,
            ),
            (
                "m.toml".to_owned(),
                vec![AuthoringInstancePathSegment::Field("items".into()), at(0),],
                AuthoringOccurrenceRole::Projection,
            ),
            (
                "m.toml".to_owned(),
                vec![AuthoringInstancePathSegment::Field("items".into()), at(1),],
                AuthoringOccurrenceRole::Projection,
            ),
            (
                "z.toml".to_owned(),
                vec![AuthoringInstancePathSegment::Field("reference".into())],
                AuthoringOccurrenceRole::Reference,
            ),
        ]
    );
    assert_eq!(page.inspected(), 6);
    assert!(page.is_complete());
    assert_eq!(index.reference_count(&target), reference_count);
    assert_eq!(
        index.references(&target).cloned().collect::<Vec<_>>(),
        references
    );
}

#[test]
fn reverse_occurrence_pages_charge_sparse_work_and_resume_after_exact_coordinate() {
    let target = simple_identity(AuthoringIdentityKind::Capability, "target".into());
    let other = simple_identity(AuthoringIdentityKind::Capability, "other".into());
    let mut index = AuthoringSemanticIndex::default();
    for (label, identity) in [
        ("a.toml", &other),
        ("b.toml", &other),
        ("c.toml", &target),
        ("d.toml", &target),
        ("e.toml", &other),
    ] {
        index.reference(
            identity.clone(),
            label,
            path([AuthoringInstancePathSegment::Field("value".into())]),
        );
    }

    let sparse = index.occurrences_page(&target, None, 2, 2);
    assert!(sparse.occurrences().is_empty());
    assert_eq!(sparse.inspected(), 2);
    assert!(!sparse.is_complete());
    assert_eq!(sparse.continuation().unwrap().source_label(), "b.toml");
    let sparse_cursor = sparse.continuation().unwrap().clone();
    drop(sparse);

    let returned_bound = index.occurrences_page(&target, Some(&sparse_cursor), 4, 1);
    assert_eq!(returned_bound.inspected(), 1);
    assert_eq!(returned_bound.occurrences().len(), 1);
    assert_eq!(returned_bound.occurrences()[0].source_label(), "c.toml");
    assert!(!returned_bound.is_complete());
    let returned_cursor = returned_bound.continuation().unwrap().clone();

    let exact_terminal_bound = index.occurrences_page(&target, Some(&returned_cursor), 2, 2);
    assert_eq!(exact_terminal_bound.inspected(), 2);
    assert_eq!(exact_terminal_bound.occurrences().len(), 1);
    assert_eq!(
        exact_terminal_bound.occurrences()[0].source_label(),
        "d.toml"
    );
    assert!(!exact_terminal_bound.is_complete());
    assert_eq!(
        exact_terminal_bound.continuation().unwrap().source_label(),
        "e.toml"
    );
    let terminal_cursor = exact_terminal_bound.continuation().unwrap().clone();

    let terminal = index.occurrences_page(&target, Some(&terminal_cursor), 2, 2);
    assert!(terminal.occurrences().is_empty());
    assert_eq!(terminal.inspected(), 0);
    assert!(terminal.is_complete());
}

#[test]
fn occurrence_pages_bound_copied_utf8_and_reinspect_one_deferred_match() {
    let target = simple_identity(AuthoringIdentityKind::Capability, "target".into());
    let exact_label = "a".repeat(MAX_CONFINED_RELATIVE_PATH_BYTES);
    let exact_field = "é".repeat(WORKSPACE_FILE_LIMIT.max_bytes() / "é".len());
    let mut index = AuthoringSemanticIndex::default();
    index.reference(
        target.clone(),
        &exact_label,
        path([AuthoringInstancePathSegment::Field(exact_field)]),
    );
    index.reference(target.clone(), "z", Vec::new());

    let exact = index.occurrences_page(&target, None, 256, 256);
    assert_eq!(
        exact.copied_string_bytes(),
        MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES
    );
    assert_eq!(exact.occurrences().len(), 1);
    assert_eq!(exact.inspected(), 2, "the deferred row is charged once");
    assert!(!exact.is_complete());

    let resumed = index.occurrences_page(&target, exact.continuation(), 256, 256);
    assert_eq!(
        resumed.inspected(),
        1,
        "the deferred row is safely reinspected"
    );
    assert_eq!(resumed.copied_string_bytes(), 1);
    assert_eq!(resumed.occurrences().len(), 1);
    assert_eq!(resumed.occurrences()[0].source_label(), "z");
    assert!(resumed.is_complete());
}

#[test]
fn occurrence_byte_ceiling_splits_many_rows_without_skipping_one() {
    let target = simple_identity(AuthoringIdentityKind::Capability, "target".into());
    let field = "x".repeat(20_000);
    let mut index = AuthoringSemanticIndex::default();
    for occurrence_index in 0..256 {
        index.reference(
            target.clone(),
            &format!("{occurrence_index:03}.toml"),
            path([AuthoringInstancePathSegment::Field(field.clone())]),
        );
    }

    let first = index.occurrences_page(&target, None, 256, 256);
    assert!(first.copied_string_bytes() <= MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES);
    assert!(
        first.occurrences().len() < 256,
        "the byte ceiling must split this page"
    );
    let deferred_label = first.continuation().unwrap().source_label().to_owned();
    let mut labels = first
        .occurrences()
        .iter()
        .map(|row| row.source_label().to_owned())
        .collect::<Vec<_>>();

    let second = index.occurrences_page(&target, first.continuation(), 256, 256);
    assert_eq!(second.occurrences()[0].source_label(), deferred_label);
    assert!(second.is_complete());
    labels.extend(
        second
            .occurrences()
            .iter()
            .map(|row| row.source_label().to_owned()),
    );
    assert_eq!(labels.len(), 256);
    assert_eq!(
        labels,
        (0..256)
            .map(|index| format!("{index:03}.toml"))
            .collect::<Vec<_>>()
    );
}

#[test]
fn reverse_occurrence_page_limits_default_and_cap_independently() {
    let target = simple_identity(AuthoringIdentityKind::Capability, "target".into());
    let mut index = AuthoringSemanticIndex::default();
    for occurrence_index in 0..300 {
        index.reference(
            target.clone(),
            &format!("{occurrence_index:03}.toml"),
            path([AuthoringInstancePathSegment::Field("value".into())]),
        );
    }

    let defaulted_inspection = index.occurrences_page(&target, None, 0, usize::MAX);
    assert_eq!(
        defaulted_inspection.inspected(),
        DEFAULT_AUTHORING_OCCURRENCE_PAGE_LIMIT
    );
    assert_eq!(
        defaulted_inspection.occurrences().len(),
        DEFAULT_AUTHORING_OCCURRENCE_PAGE_LIMIT
    );
    assert!(!defaulted_inspection.is_complete());

    let defaulted_return = index.occurrences_page(&target, None, usize::MAX, 0);
    assert_eq!(
        defaulted_return.occurrences().len(),
        DEFAULT_AUTHORING_OCCURRENCE_PAGE_LIMIT
    );
    assert_eq!(
        defaulted_return.inspected(),
        DEFAULT_AUTHORING_OCCURRENCE_PAGE_LIMIT
    );
    assert!(!defaulted_return.is_complete());

    let first = index.occurrences_page(&target, None, usize::MAX, usize::MAX);
    assert_eq!(first.inspected(), MAX_AUTHORING_OCCURRENCE_PAGE_LIMIT);
    assert_eq!(
        first.occurrences().len(),
        MAX_AUTHORING_OCCURRENCE_PAGE_LIMIT
    );
    assert!(!first.is_complete());
    let first_cursor = first.continuation().unwrap().clone();
    let terminal = index.occurrences_page(&target, Some(&first_cursor), usize::MAX, usize::MAX);
    assert_eq!(terminal.inspected(), 44);
    assert_eq!(terminal.occurrences().len(), 44);
    assert!(terminal.is_complete());

    let seen = first
        .occurrences()
        .iter()
        .chain(terminal.occurrences())
        .map(|occurrence| occurrence.source_label().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        seen,
        (0..300)
            .map(|occurrence_index| format!("{occurrence_index:03}.toml"))
            .collect::<Vec<_>>()
    );

    let absent = simple_identity(AuthoringIdentityKind::Capability, "absent".into());
    let absent_first = index.occurrences_page(&absent, None, usize::MAX, 1);
    assert!(absent_first.occurrences().is_empty());
    assert_eq!(
        absent_first.inspected(),
        MAX_AUTHORING_OCCURRENCE_PAGE_LIMIT
    );
    assert!(!absent_first.is_complete());
    let absent_cursor = absent_first.continuation().unwrap().clone();
    let absent_terminal = index.occurrences_page(&absent, Some(&absent_cursor), usize::MAX, 1);
    assert!(absent_terminal.occurrences().is_empty());
    assert_eq!(absent_terminal.inspected(), 44);
    assert!(absent_terminal.is_complete());
}
