//! Standard LSP projections over tolerant structure and API-owned grammar facts.

use std::collections::BTreeMap;

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, CodeActionResponse,
    CompletionItem, CompletionItemKind, CompletionList, CompletionResponse, CompletionTextEdit,
    DocumentChanges, DocumentSymbolResponse, GotoDefinitionResponse, Hover, HoverContents,
    Location, MarkupContent, MarkupKind, OneOf, OptionalVersionedTextDocumentIdentifier, Position,
    Range, SemanticToken, SemanticTokens, SemanticTokensResult, SymbolKind, TextDocumentEdit,
    TextEdit, Uri, WorkspaceEdit,
};
use pale_sdk::{
    ApiError, AuthoringCorrection, AuthoringCorrectionKind, AuthoringInspection,
    AuthoringInspectionRequest, AuthoringInspectionState, AuthoringLiteral,
    AuthoringNavigationRequest, AuthoringNodeFact, AuthoringOccurrenceRole,
    AuthoringSourceLocation, EffectiveProfileCompilation, EffectiveProfileEffectClass,
    EffectiveProfileEntryDetail, EffectiveProfileFinding, EffectiveProfileOperationExposure,
    EffectiveProfileProviderDisposition, EffectiveProfileRequirementDisposition,
    EffectiveProfileSelection, PaleApi, PortableWorkspaceCompilation, StandaloneAuthoringRole,
};

use crate::document::{AnalysisDocument, DocumentRole, Snapshot};
use crate::protocol::{AuthoringQuery, AuthoringQueryResult};
use crate::syntax::{AuthoringSyntax, AuthoringSyntaxSnapshot, ByteRange, SyntaxFactKind};

mod presentation;
pub(super) mod references;
use presentation::{effective_authoring_markdown, identity_kind_label};

/// Maximum members in one standard LSP authoring response.
pub(super) const MAX_STANDARD_AUTHORING_RESULTS: usize = 4_096;

/// Failure to project one standard authoring request.
#[derive(Debug, thiserror::Error)]
pub(super) enum AuthoringQueryError {
    /// The supplied UTF-16 position is outside the exact retained document.
    #[error("authoring position is outside the retained editor document")]
    InvalidPosition,
    /// The canonical façade rejected a structural authoring query.
    #[error(transparent)]
    Api(#[from] ApiError),
    /// The admitted identity has more references than one bounded response.
    #[error("authoring reference result exceeds the bounded response ceiling")]
    ReferenceSetTooLarge,
    /// A compiler-selected occurrence has no exact retained editor range.
    #[error("a compiler-selected source location cannot be mapped to the exact retained editor snapshot")]
    SourceMappingUnavailable,
    /// The snapshot-derived finite location bound could not be represented.
    #[error("the retained reference location bound is exhausted")]
    ReferenceLocationBound,
    /// The process-local reference-page counter has no successor.
    #[error("the retained reference page coordinate is exhausted")]
    ReferencePageBound,
    /// The tolerant source exceeded the finite structural-fact ceiling.
    #[error("authoring syntax exceeds the bounded structural-fact ceiling")]
    SyntaxFactLimitExceeded,
    /// A complete standard response would exceed the protocol leaf ceiling.
    #[error("authoring result exceeds the bounded standard-response ceiling")]
    StandardResponseTooLarge,
}

/// Last admitted semantic owners available only for explicitly stale help.
#[derive(Clone, Copy, Default)]
pub(super) struct StaleAuthoringCompilation<'a> {
    pub(super) portable: Option<&'a PortableWorkspaceCompilation>,
}

/// Client features that change only the legal LSP transport shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct AuthoringClientCapabilities {
    pub(super) versioned_document_changes: bool,
    pub(super) completion_markup: AuthoringMarkupKind,
    pub(super) hover_markup: AuthoringMarkupKind,
}

/// Borrowed semantic owners for one authoring query.
///
/// Navigation and inspection always prefer the exact effective compilation,
/// then the admitted portable compilation, and finally the API-owned
/// standalone grammar. Keeping that order here prevents individual LSP
/// features from becoming competing semantic dispatchers.
#[derive(Clone, Copy)]
pub(super) struct AuthoringSemanticContext<'a> {
    /// Exact admitted portable compilation, when available.
    pub(super) portable: Option<&'a PortableWorkspaceCompilation>,
    /// Exact effective-profile compilation, when available.
    pub(super) effective: Option<&'a EffectiveProfileCompilation>,
    /// Complete narrow API façade used for standalone grammar inspection.
    pub(super) api: &'a PaleApi,
}

/// Complete borrowed state for one standard LSP authoring request.
pub(super) struct AuthoringExecution<'a> {
    /// Exact attempted editor snapshot.
    pub(super) snapshot: &'a Snapshot,
    /// Tolerant syntax facts derived from the same snapshot.
    pub(super) syntax: &'a AuthoringSyntaxSnapshot,
    /// Current admitted semantic owners.
    pub(super) semantics: AuthoringSemanticContext<'a>,
    /// Prior admitted compilation available only as explicitly stale help.
    pub(super) stale: StaleAuthoringCompilation<'a>,
    /// Exact compiler-owned candidate corrections for this attempt.
    pub(super) corrections: &'a [AuthoringCorrection],
    /// Client-supported transport shapes.
    pub(super) client: AuthoringClientCapabilities,
}

/// Preferred documentation encoding admitted by one LSP client.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum AuthoringMarkupKind {
    /// Plain text is the safe LSP default when no richer format is advertised.
    #[default]
    PlainText,
    /// GitHub-flavored Markdown carried through LSP markup content.
    Markdown,
}

/// Execute one standard authoring query at the exact attempted editor snapshot.
pub(super) fn execute(
    query: AuthoringQuery,
    execution: AuthoringExecution<'_>,
) -> Result<AuthoringQueryResult, AuthoringQueryError> {
    let AuthoringExecution {
        snapshot,
        syntax,
        semantics,
        stale,
        corrections,
        client,
    } = execution;
    match query {
        AuthoringQuery::SemanticTokens(params) => {
            let Some((document, syntax)) = document(snapshot, syntax, &params.text_document.uri)
            else {
                return Ok(AuthoringQueryResult::SemanticTokens(None));
            };
            semantic_tokens(document, syntax, semantics)
                .map(Some)
                .map(AuthoringQueryResult::SemanticTokens)
        }
        AuthoringQuery::Completion(params) => {
            let position = params.text_document_position.position;
            let Some((document, syntax)) = document(
                snapshot,
                syntax,
                &params.text_document_position.text_document.uri,
            ) else {
                return Ok(AuthoringQueryResult::Completion(None));
            };
            completion(
                document,
                syntax,
                semantics,
                stale,
                client.completion_markup,
                position,
            )
            .map(Some)
            .map(AuthoringQueryResult::Completion)
        }
        AuthoringQuery::Hover(params) => {
            let position = params.text_document_position_params.position;
            let Some((document, document_syntax)) = document(
                snapshot,
                syntax,
                &params.text_document_position_params.text_document.uri,
            ) else {
                return Ok(AuthoringQueryResult::Hover(None));
            };
            hover(
                document,
                document_syntax,
                semantics,
                client.hover_markup,
                position,
            )
            .map(AuthoringQueryResult::Hover)
        }
        AuthoringQuery::DocumentSymbols(params) => {
            let Some((document, syntax)) = document(snapshot, syntax, &params.text_document.uri)
            else {
                return Ok(AuthoringQueryResult::DocumentSymbols(None));
            };
            document_symbols(document, syntax, semantics)
                .map(Some)
                .map(AuthoringQueryResult::DocumentSymbols)
        }
        AuthoringQuery::Definition(params) => {
            let position = params.text_document_position_params.position;
            let Some((document, document_syntax)) = document(
                snapshot,
                syntax,
                &params.text_document_position_params.text_document.uri,
            ) else {
                return Ok(AuthoringQueryResult::Definition(None));
            };
            definition(
                snapshot,
                syntax,
                document,
                document_syntax,
                semantics,
                position,
            )
            .map(AuthoringQueryResult::Definition)
        }
        AuthoringQuery::References(params) => {
            let position = params.text_document_position.position;
            let Some((document, document_syntax)) = document(
                snapshot,
                syntax,
                &params.text_document_position.text_document.uri,
            ) else {
                return Ok(AuthoringQueryResult::References(None));
            };
            references(
                snapshot,
                syntax,
                document,
                document_syntax,
                semantics,
                position,
                params.context.include_declaration,
            )
            .map(AuthoringQueryResult::References)
        }
        AuthoringQuery::CodeActions(params) => {
            let Some((document, document_syntax)) =
                document(snapshot, syntax, &params.text_document.uri)
            else {
                return Ok(AuthoringQueryResult::CodeActions(None));
            };
            code_actions(document, document_syntax, corrections, client, params)
                .map(AuthoringQueryResult::CodeActions)
        }
    }
}

fn code_actions(
    document: &AnalysisDocument,
    syntax: &AuthoringSyntax,
    corrections: &[AuthoringCorrection],
    client: AuthoringClientCapabilities,
    params: CodeActionParams,
) -> Result<Option<CodeActionResponse>, AuthoringQueryError> {
    ensure_complete(syntax)?;
    if !client.versioned_document_changes {
        return Ok(None);
    }
    let Some(version) = document.version() else {
        return Ok(None);
    };
    let mut actions = Vec::new();
    for correction in corrections
        .iter()
        .filter(|correction| correction.source_label() == document.logical_label())
    {
        let Some(range) = correction_range(document, syntax, correction) else {
            continue;
        };
        if !ranges_touch(range, params.range) {
            continue;
        }
        let diagnostics = params
            .context
            .diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic
                    .data
                    .as_ref()
                    .and_then(|data| data.get("identity"))
                    .and_then(serde_json::Value::as_str)
                    == Some(correction.diagnostic_identity().as_str())
            })
            .cloned()
            .collect::<Vec<_>>();
        let title = match correction.kind() {
            AuthoringCorrectionKind::NarrowImportVisibility => format!(
                "Narrow visibility for {}",
                correction.proof_subject().value()
            ),
        };
        let edit = WorkspaceEdit {
            changes: None,
            document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
                text_document: OptionalVersionedTextDocumentIdentifier::new(
                    document.uri().clone(),
                    version,
                ),
                edits: vec![OneOf::Left(TextEdit::new(
                    range,
                    correction.replacement_source().to_owned(),
                ))],
            }])),
            change_annotations: None,
        };
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title,
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: (!diagnostics.is_empty()).then_some(diagnostics),
            edit: Some(edit),
            is_preferred: Some(true),
            data: Some(serde_json::json!({
                "diagnosticIdentity": correction.diagnostic_identity(),
                "proofSubject": correction.proof_subject().value(),
            })),
            ..CodeAction::default()
        }));
    }
    Ok((!actions.is_empty()).then_some(actions))
}

/// Resolve one compiler correction only while its exact scalar preimage survives.
pub(super) fn correction_range(
    document: &AnalysisDocument,
    syntax: &AuthoringSyntax,
    correction: &AuthoringCorrection,
) -> Option<Range> {
    if correction.source_label() != document.logical_label() {
        return None;
    }
    let fact = syntax.fact_for_instance_path(correction.path())?;
    let text = document.text();
    (fact.kind == SyntaxFactKind::Value
        && text.get(fact.range.start..fact.range.end) == Some(correction.expected_source()))
    .then(|| range_from(document, fact.range))?
}

fn ranges_touch(left: Range, right: Range) -> bool {
    left.start <= right.end && right.start <= left.end
}

fn document<'a>(
    snapshot: &'a Snapshot,
    syntax: &'a AuthoringSyntaxSnapshot,
    uri: &Uri,
) -> Option<(&'a AnalysisDocument, &'a AuthoringSyntax)> {
    let document = snapshot.document(uri)?;
    let syntax = syntax.document(document.logical_label())?;
    Some((document, syntax))
}

fn inspect(
    semantics: AuthoringSemanticContext<'_>,
    document: &AnalysisDocument,
    path: Vec<pale_sdk::AuthoringPathSegment>,
) -> Result<Option<AuthoringInspection>, AuthoringQueryError> {
    if let Some(effective) = semantics.effective {
        let inspection = effective.inspect_authoring(AuthoringInspectionRequest::new(
            document.logical_label().to_owned(),
            path.clone(),
        ))?;
        if inspection.state() == AuthoringInspectionState::Available {
            return Ok(Some(inspection));
        }
    }
    if let Some(compilation) = semantics.portable {
        let inspection = compilation.inspect_authoring(AuthoringInspectionRequest::new(
            document.logical_label().to_owned(),
            path.clone(),
        ))?;
        if inspection.state() == AuthoringInspectionState::Available {
            return Ok(Some(inspection));
        }
    }
    let role = standalone_role(document.role());
    role.map(|role| semantics.api.inspect_standalone_authoring(role, path))
        .transpose()
        .map(|inspection| {
            inspection
                .filter(|inspection| inspection.state() == AuthoringInspectionState::Available)
        })
        .map_err(Into::into)
}

pub(super) const fn standalone_role(role: DocumentRole) -> Option<StandaloneAuthoringRole> {
    Some(match role {
        DocumentRole::WorkspaceManifest => StandaloneAuthoringRole::WorkspaceManifest,
        DocumentRole::WorkspaceModule => StandaloneAuthoringRole::WorkspaceModule,
        DocumentRole::WorkspaceLanguagePrimitives => {
            StandaloneAuthoringRole::WorkspaceLanguagePrimitives
        }
        DocumentRole::ResponsibilityDerivation => StandaloneAuthoringRole::ResponsibilityDerivation,
        DocumentRole::OrganizationalTranslation => {
            StandaloneAuthoringRole::OrganizationalTranslation
        }
        DocumentRole::ExperienceComposition => StandaloneAuthoringRole::ExperienceComposition,
        DocumentRole::WorkspaceResolutionLock => StandaloneAuthoringRole::WorkspaceResolutionLock,
        DocumentRole::WorkspaceUniverseLock => StandaloneAuthoringRole::WorkspaceUniverseLock,
        DocumentRole::Capabilities => StandaloneAuthoringRole::Capabilities,
        DocumentRole::MachineBinding => StandaloneAuthoringRole::MachineBinding,
        DocumentRole::DistributionLock => StandaloneAuthoringRole::DistributionLock,
        DocumentRole::PackageManifest => StandaloneAuthoringRole::PackageManifest,
        DocumentRole::PackagePage => StandaloneAuthoringRole::PackagePage,
        DocumentRole::PackageData => StandaloneAuthoringRole::PackageData,
        DocumentRole::PackageDoctrine => StandaloneAuthoringRole::PackageDoctrine,
        DocumentRole::PackageOperations => StandaloneAuthoringRole::PackageOperations,
        DocumentRole::PackageRoles => StandaloneAuthoringRole::PackageRoles,
        DocumentRole::PackageStacks => StandaloneAuthoringRole::PackageStacks,
        DocumentRole::PackageRunModes => StandaloneAuthoringRole::PackageRunModes,
        DocumentRole::PackagePolicyFamily => StandaloneAuthoringRole::PackagePolicyFamily,
        DocumentRole::PackageGoverningOrder => StandaloneAuthoringRole::PackageGoverningOrder,
        DocumentRole::PackageCapabilityPolicy => StandaloneAuthoringRole::PackageCapabilityPolicy,
        DocumentRole::PackageToolRecipe => StandaloneAuthoringRole::PackageToolRecipe,
        DocumentRole::LifecycleGuidance | DocumentRole::PackageText => return None,
    })
}

fn completion(
    document: &AnalysisDocument,
    syntax: &AuthoringSyntax,
    semantics: AuthoringSemanticContext<'_>,
    stale: StaleAuthoringCompilation<'_>,
    markup: AuthoringMarkupKind,
    position: Position,
) -> Result<CompletionResponse, AuthoringQueryError> {
    ensure_complete(syntax)?;
    let byte = document
        .byte_at_position(position)
        .ok_or(AuthoringQueryError::InvalidPosition)?;
    let Some(position) = syntax.position(byte) else {
        return Ok(CompletionResponse::Array(Vec::new()));
    };
    let value_context = position
        .fact
        .filter(|fact| matches!(fact.kind, SyntaxFactKind::Value | SyntaxFactKind::ValueSlot));
    let key_context = position
        .fact
        .filter(|fact| matches!(fact.kind, SyntaxFactKind::Key | SyntaxFactKind::HeaderKey));
    let (path, replacement) = value_context.map_or_else(
        || {
            key_context.map_or_else(
                || {
                    (
                        position.object_path.to_vec(),
                        Range::new(position_from(document, byte), position_from(document, byte)),
                    )
                },
                |fact| {
                    let mut parent = fact.path.clone();
                    parent.pop();
                    (
                        parent,
                        range_from(document, fact.range).unwrap_or_else(|| {
                            Range::new(position_from(document, byte), position_from(document, byte))
                        }),
                    )
                },
            )
        },
        |fact| {
            (
                fact.path.clone(),
                if fact.kind == SyntaxFactKind::ValueSlot {
                    incomplete_value_replacement(document, fact.range, byte)
                } else {
                    range_from(document, fact.range).unwrap_or_else(|| {
                        Range::new(position_from(document, byte), position_from(document, byte))
                    })
                },
            )
        },
    );
    let Some(inspection) = inspect(semantics, document, path)? else {
        return Ok(CompletionResponse::Array(Vec::new()));
    };
    let node = inspection
        .node()
        .expect("an available authoring inspection contains its node");
    let mut items: Vec<CompletionItem> = if value_context.is_some() {
        node.allowed_values()
            .iter()
            .map(|literal| CompletionItem {
                label: literal_label(literal.value()),
                kind: Some(CompletionItemKind::ENUM_MEMBER),
                detail: literal.description().map(str::to_owned),
                documentation: literal
                    .description()
                    .map(|description| documentation(description, markup)),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    range: replacement,
                    new_text: literal_source(literal.value()),
                })),
                ..CompletionItem::default()
            })
            .collect()
    } else {
        let active_key = key_context.and_then(|fact| match fact.path.last() {
            Some(pale_sdk::AuthoringPathSegment::Field(field)) => Some(field.as_str()),
            Some(pale_sdk::AuthoringPathSegment::ArrayItem) | None => None,
        });
        node.fields()
            .iter()
            .filter(|field| {
                active_key == Some(field.name()) || !position.existing_fields.contains(field.name())
            })
            .map(|field| CompletionItem {
                label: field.name().to_owned(),
                kind: Some(CompletionItemKind::PROPERTY),
                detail: Some(format!("{:?}", field.presence()).to_lowercase()),
                documentation: field
                    .description()
                    .map(|description| documentation(description, markup)),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    range: replacement,
                    new_text: field.name().to_owned(),
                })),
                ..CompletionItem::default()
            })
            .collect()
    };
    let semantic_context = value_context.or(key_context);
    let current_navigation = semantic_context.and_then(|fact| {
        let request = || {
            AuthoringNavigationRequest::new(
                document.logical_label().to_owned(),
                fact.instance_path.clone(),
            )
        };
        semantics.effective.map_or_else(
            || {
                semantics
                    .portable
                    .map(|compilation| compilation.navigate_authoring(request()))
            },
            |effective| Some(effective.navigate_authoring(request())),
        )
    });
    let stale_navigation = semantic_context
        .filter(|_| {
            current_navigation
                .as_ref()
                .is_none_or(|navigation| navigation.alternatives().is_empty())
        })
        .and_then(|fact| {
            let request = || {
                AuthoringNavigationRequest::new(
                    document.logical_label().to_owned(),
                    fact.instance_path.clone(),
                )
            };
            stale
                .portable
                .map(|compilation| compilation.navigate_authoring(request()))
        })
        .filter(|navigation| !navigation.alternatives().is_empty());
    let (navigation, navigation_is_stale) = stale_navigation
        .as_ref()
        .map_or((current_navigation.as_ref(), false), |navigation| {
            (Some(navigation), true)
        });
    if let Some(navigation) = &navigation {
        items.extend(
            navigation
                .alternatives()
                .iter()
                .map(|identity| CompletionItem {
                    label: identity.authored_value().to_owned(),
                    kind: Some(CompletionItemKind::REFERENCE),
                    detail: Some(if navigation_is_stale {
                        format!("Last admitted: {}", identity.value())
                    } else {
                        identity.value().to_owned()
                    }),
                    documentation: navigation_is_stale.then(|| {
                        documentation(
                            "Candidate from the last admitted configuration; the current buffer is not admitted.",
                            markup,
                        )
                    }),
                    text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                        range: replacement,
                        new_text: if key_context.is_some() {
                            toml_key_source(identity.authored_value())
                        } else {
                            serde_json::to_string(identity.authored_value())
                                .expect("an admitted identity always has a JSON string encoding")
                        },
                    })),
                    ..CompletionItem::default()
                }),
        );
    }
    items.sort_by(|left, right| {
        left.label
            .cmp(&right.label)
            .then_with(|| left.detail.cmp(&right.detail))
    });
    if navigation.is_some_and(|value| value.alternatives_truncated()) {
        Ok(CompletionResponse::List(CompletionList {
            is_incomplete: true,
            items,
        }))
    } else {
        Ok(CompletionResponse::Array(items))
    }
}

fn definition(
    snapshot: &Snapshot,
    syntax_snapshot: &AuthoringSyntaxSnapshot,
    document: &AnalysisDocument,
    syntax: &AuthoringSyntax,
    semantics: AuthoringSemanticContext<'_>,
    position: Position,
) -> Result<Option<GotoDefinitionResponse>, AuthoringQueryError> {
    ensure_complete(syntax)?;
    let Some(navigation) = navigation(document, syntax, semantics, position)? else {
        return Ok(None);
    };
    navigation
        .definition()
        .map(|location| {
            exact_location(snapshot, syntax_snapshot, location)
                .map(|(_, location)| GotoDefinitionResponse::Scalar(location))
        })
        .transpose()
}

fn references(
    snapshot: &Snapshot,
    syntax_snapshot: &AuthoringSyntaxSnapshot,
    document: &AnalysisDocument,
    syntax: &AuthoringSyntax,
    semantics: AuthoringSemanticContext<'_>,
    position: Position,
    include_declaration: bool,
) -> Result<Option<Vec<Location>>, AuthoringQueryError> {
    ensure_complete(syntax)?;
    let Some(navigation) = navigation(document, syntax, semantics, position)? else {
        return Ok(None);
    };
    if navigation.references_truncated() {
        return Err(AuthoringQueryError::ReferenceSetTooLarge);
    }
    let mut locations = navigation
        .references()
        .iter()
        .map(|location| {
            exact_location(snapshot, syntax_snapshot, location).map(|(_, location)| location)
        })
        .collect::<Result<Vec<_>, _>>()?;
    if include_declaration {
        if let Some(location) = navigation.definition() {
            locations.push(exact_location(snapshot, syntax_snapshot, location)?.1);
        }
    }
    locations.sort_by(|left, right| {
        left.uri.as_str().cmp(right.uri.as_str()).then_with(|| {
            (
                left.range.start.line,
                left.range.start.character,
                left.range.end.line,
                left.range.end.character,
            )
                .cmp(&(
                    right.range.start.line,
                    right.range.start.character,
                    right.range.end.line,
                    right.range.end.character,
                ))
        })
    });
    locations.dedup();
    Ok(Some(locations))
}

fn navigation(
    document: &AnalysisDocument,
    syntax: &AuthoringSyntax,
    semantics: AuthoringSemanticContext<'_>,
    position: Position,
) -> Result<Option<pale_sdk::AuthoringNavigation>, AuthoringQueryError> {
    let byte = document
        .byte_at_position(position)
        .ok_or(AuthoringQueryError::InvalidPosition)?;
    let Some(fact) = syntax.position(byte).and_then(|position| position.fact) else {
        return Ok(None);
    };
    let request = || {
        AuthoringNavigationRequest::new(
            document.logical_label().to_owned(),
            fact.instance_path.clone(),
        )
    };
    let Some(navigation) = semantics.effective.map_or_else(
        || {
            semantics
                .portable
                .map(|compilation| compilation.navigate_authoring(request()))
        },
        |effective| Some(effective.navigate_authoring(request())),
    ) else {
        return Ok(None);
    };
    Ok(navigation.identity().is_some().then_some(navigation))
}

/// Fixed-size editor coordinates; source labels and URI strings are not copied.
type LocationKey = (usize, u32, u32, u32, u32);

/// Map a compiler location without turning missing/incomplete source into an
/// apparently complete result. Definition and both reference paths share this owner.
fn exact_location(
    snapshot: &Snapshot,
    syntax: &AuthoringSyntaxSnapshot,
    location: &AuthoringSourceLocation,
) -> Result<(LocationKey, Location), AuthoringQueryError> {
    let (ordinal, document) = snapshot
        .documents()
        .iter()
        .enumerate()
        .find(|(_, document)| document.logical_label() == location.source_label())
        .ok_or(AuthoringQueryError::SourceMappingUnavailable)?;
    if !location.path().is_empty() {
        let syntax = syntax
            .document(document.logical_label())
            .ok_or(AuthoringQueryError::SourceMappingUnavailable)?;
        ensure_complete(syntax)?;
    }
    let location = source_location(snapshot, syntax, location)
        .ok_or(AuthoringQueryError::SourceMappingUnavailable)?;
    let range = location.range;
    Ok((
        (
            ordinal,
            range.start.line,
            range.start.character,
            range.end.line,
            range.end.character,
        ),
        location,
    ))
}

fn source_location(
    snapshot: &Snapshot,
    syntax: &AuthoringSyntaxSnapshot,
    location: &AuthoringSourceLocation,
) -> Option<Location> {
    let document = snapshot
        .documents()
        .iter()
        .find(|document| document.logical_label() == location.source_label())?;
    if location.path().is_empty() {
        return Some(Location::new(
            document.uri().clone(),
            Range::new(Position::new(0, 0), Position::new(0, 0)),
        ));
    }
    let syntax = syntax.document(location.source_label())?;
    let fact = syntax.fact_for_instance_path(location.path())?;
    Some(Location::new(
        document.uri().clone(),
        range_from(document, fact.range)?,
    ))
}

fn hover(
    document: &AnalysisDocument,
    syntax: &AuthoringSyntax,
    semantics: AuthoringSemanticContext<'_>,
    markup: AuthoringMarkupKind,
    position: Position,
) -> Result<Option<Hover>, AuthoringQueryError> {
    ensure_complete(syntax)?;
    let byte = document
        .byte_at_position(position)
        .ok_or(AuthoringQueryError::InvalidPosition)?;
    let Some(position) = syntax.position(byte) else {
        return Ok(None);
    };
    let Some(fact) = position.fact else {
        return Ok(None);
    };
    let Some(inspection) = inspect(semantics, document, fact.path.clone())? else {
        return Ok(None);
    };
    let node = inspection
        .node()
        .expect("an available authoring inspection contains its node");
    let mut content = node_markdown(node);
    let request = || {
        AuthoringNavigationRequest::new(
            document.logical_label().to_owned(),
            fact.instance_path.clone(),
        )
    };
    let navigation = semantics.effective.map_or_else(
        || {
            semantics
                .portable
                .map(|compilation| compilation.navigate_authoring(request()))
        },
        |effective| Some(effective.navigate_authoring(request())),
    );
    if let Some(identity) = navigation
        .as_ref()
        .and_then(pale_sdk::AuthoringNavigation::identity)
    {
        let value = match markup {
            AuthoringMarkupKind::Markdown => format!("`{}`", identity.value()),
            AuthoringMarkupKind::PlainText => identity.value().to_owned(),
        };
        let semantic = format!("Resolved {}: {value}", identity_kind_label(identity.kind()),);
        content = if content.is_empty() {
            semantic
        } else {
            format!("{content}\n\n{semantic}")
        };
        if let Some(profile) = semantics.effective {
            let inspection = profile.inspect_effective_authoring(identity);
            let derived = effective_authoring_markdown(&inspection, markup);
            if !derived.is_empty() {
                content.push_str("\n\n");
                content.push_str(&derived);
            }
        }
    }
    if content.is_empty() {
        return Ok(None);
    }
    Ok(Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: markup.into(),
            value: content,
        }),
        range: range_from(document, fact.range),
    }))
}

fn semantic_tokens(
    document: &AnalysisDocument,
    syntax: &AuthoringSyntax,
    semantics: AuthoringSemanticContext<'_>,
) -> Result<SemanticTokensResult, AuthoringQueryError> {
    ensure_complete(syntax)?;
    struct AbsoluteToken {
        line: u32,
        start: u32,
        length: u32,
        token_type: u32,
        token_modifiers_bitset: u32,
    }

    let mut tokens = Vec::new();
    let mut inspections = BTreeMap::new();
    for fact in syntax.facts() {
        let token = match fact.kind {
            SyntaxFactKind::Table | SyntaxFactKind::ArrayTable => None,
            SyntaxFactKind::HeaderKey | SyntaxFactKind::Key => {
                let navigation = semantic_navigation_for_fact(semantics, document, fact);
                if let Some(role) = navigation
                    .as_ref()
                    .and_then(pale_sdk::AuthoringNavigation::occurrence_role)
                {
                    let modifiers = u32::from(role == AuthoringOccurrenceRole::Definition);
                    Some((3, modifiers))
                } else {
                    inspection(&mut inspections, semantics, document, &fact.path)?
                        .is_some()
                        .then_some((u32::from(fact.kind == SyntaxFactKind::Key), 0))
                }
            }
            SyntaxFactKind::Value => {
                let navigation = semantic_navigation_for_fact(semantics, document, fact);
                if let Some(role) = navigation
                    .as_ref()
                    .and_then(pale_sdk::AuthoringNavigation::occurrence_role)
                {
                    let modifiers = u32::from(role == AuthoringOccurrenceRole::Definition);
                    Some((3, modifiers))
                } else {
                    inspection(&mut inspections, semantics, document, &fact.path)?.and_then(
                        |inspection| {
                            inspection
                                .node()
                                .is_some_and(|node| !node.allowed_values().is_empty())
                                .then_some((2, 0))
                        },
                    )
                }
            }
            SyntaxFactKind::ValueSlot => None,
        };
        let Some((token_type, token_modifiers_bitset)) = token else {
            continue;
        };
        let Some(range) = range_from(document, fact.range) else {
            continue;
        };
        if range.start.line != range.end.line || range.start == range.end {
            continue;
        }
        tokens.push(AbsoluteToken {
            line: range.start.line,
            start: range.start.character,
            length: range.end.character - range.start.character,
            token_type,
            token_modifiers_bitset,
        });
    }
    tokens.sort_by_key(|token| (token.line, token.start, token.length, token.token_type));
    if tokens.len() > MAX_STANDARD_AUTHORING_RESULTS {
        return Err(AuthoringQueryError::StandardResponseTooLarge);
    }
    let mut previous_line = 0;
    let mut previous_start = 0;
    let data = tokens
        .into_iter()
        .map(|token| {
            let delta_line = token.line - previous_line;
            let delta_start = if delta_line == 0 {
                token.start - previous_start
            } else {
                token.start
            };
            previous_line = token.line;
            previous_start = token.start;
            SemanticToken {
                delta_line,
                delta_start,
                length: token.length,
                token_type: token.token_type,
                token_modifiers_bitset: token.token_modifiers_bitset,
            }
        })
        .collect();
    Ok(SemanticTokensResult::Tokens(SemanticTokens {
        result_id: None,
        data,
    }))
}

fn semantic_navigation_for_fact(
    semantics: AuthoringSemanticContext<'_>,
    document: &AnalysisDocument,
    fact: &crate::syntax::SyntaxFact,
) -> Option<pale_sdk::AuthoringNavigation> {
    let request = || {
        AuthoringNavigationRequest::new(
            document.logical_label().to_owned(),
            fact.instance_path.clone(),
        )
    };
    semantics.effective.map_or_else(
        || {
            semantics
                .portable
                .map(|compilation| compilation.navigate_authoring(request()))
        },
        |effective| Some(effective.navigate_authoring(request())),
    )
}

fn inspection<'a>(
    cache: &'a mut BTreeMap<Vec<pale_sdk::AuthoringPathSegment>, Option<AuthoringInspection>>,
    semantics: AuthoringSemanticContext<'_>,
    document: &AnalysisDocument,
    path: &[pale_sdk::AuthoringPathSegment],
) -> Result<Option<&'a AuthoringInspection>, AuthoringQueryError> {
    if !cache.contains_key(path) {
        cache.insert(path.to_vec(), inspect(semantics, document, path.to_vec())?);
    }
    Ok(cache.get(path).and_then(Option::as_ref))
}

fn document_symbols(
    document: &AnalysisDocument,
    syntax: &AuthoringSyntax,
    semantics: AuthoringSemanticContext<'_>,
) -> Result<DocumentSymbolResponse, AuthoringQueryError> {
    ensure_complete(syntax)?;
    let symbols: Vec<_> = syntax
        .facts()
        .iter()
        .filter_map(|fact| {
            let semantic = (fact.kind == SyntaxFactKind::Value)
                .then(|| semantic_navigation_for_fact(semantics, document, fact))
                .flatten()
                .filter(|navigation| {
                    matches!(
                        navigation.occurrence_role(),
                        Some(
                            AuthoringOccurrenceRole::Definition
                                | AuthoringOccurrenceRole::Projection
                        )
                    )
                });
            let (name, detail, kind) = match (fact.kind, semantic) {
                (SyntaxFactKind::Table, _) => (path_label(&fact.path)?, None, SymbolKind::OBJECT),
                (SyntaxFactKind::ArrayTable, _) => {
                    (path_label(&fact.path)?, None, SymbolKind::ARRAY)
                }
                (SyntaxFactKind::Value, Some(navigation)) => {
                    let identity = navigation.identity()?;
                    (
                        identity.authored_value().to_owned(),
                        Some(identity_kind_label(identity.kind()).to_owned()),
                        SymbolKind::KEY,
                    )
                }
                (
                    SyntaxFactKind::HeaderKey
                    | SyntaxFactKind::Key
                    | SyntaxFactKind::Value
                    | SyntaxFactKind::ValueSlot,
                    _,
                ) => return None,
            };
            let range = range_from(document, fact.range)?;
            let container_name = parent_path_label(&fact.path);
            #[allow(deprecated)]
            Some(lsp_types::SymbolInformation {
                name,
                kind,
                tags: None,
                deprecated: None,
                location: Location::new(document.uri().clone(), range),
                container_name: detail.or(container_name),
            })
        })
        .collect();
    if symbols.len() > MAX_STANDARD_AUTHORING_RESULTS {
        return Err(AuthoringQueryError::StandardResponseTooLarge);
    }
    Ok(DocumentSymbolResponse::Flat(symbols))
}

fn ensure_complete(syntax: &AuthoringSyntax) -> Result<(), AuthoringQueryError> {
    syntax
        .is_complete()
        .then_some(())
        .ok_or(AuthoringQueryError::SyntaxFactLimitExceeded)
}

fn parent_path_label(path: &[pale_sdk::AuthoringPathSegment]) -> Option<String> {
    path.get(..path.len().checked_sub(1)?).and_then(path_label)
}

fn node_markdown(node: &AuthoringNodeFact) -> String {
    let mut sections = Vec::new();
    if let Some(description) = node.description() {
        sections.push(description.to_owned());
    }
    if !node.allowed_values().is_empty() {
        sections.push(format!(
            "Allowed values: {}",
            node.allowed_values()
                .iter()
                .map(|value| format!("`{}`", literal_label(value.value())))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let mut bounds = Vec::new();
    if let Some(minimum) = node.minimum() {
        bounds.push(format!("minimum `{minimum}`"));
    }
    if let Some(maximum) = node.maximum() {
        bounds.push(format!("maximum `{maximum}`"));
    }
    if let Some(minimum) = node.min_length() {
        bounds.push(format!("minimum length `{minimum}`"));
    }
    if let Some(maximum) = node.max_length() {
        bounds.push(format!("maximum length `{maximum}`"));
    }
    if let Some(minimum) = node.min_items() {
        bounds.push(format!("minimum items `{minimum}`"));
    }
    if let Some(maximum) = node.max_items() {
        bounds.push(format!("maximum items `{maximum}`"));
    }
    if let Some(format) = node.format() {
        bounds.push(format!("format `{format}`"));
    }
    if let Some(pattern) = node.pattern() {
        bounds.push(format!("pattern `{pattern}`"));
    }
    if !bounds.is_empty() {
        sections.push(bounds.join("; "));
    }
    sections.join("\n\n")
}

fn literal_label(value: &AuthoringLiteral) -> String {
    match value {
        AuthoringLiteral::String(value) => value.clone(),
        AuthoringLiteral::Integer(value) => value.to_string(),
        AuthoringLiteral::Boolean(value) => value.to_string(),
    }
}

fn literal_source(value: &AuthoringLiteral) -> String {
    match value {
        AuthoringLiteral::String(value) => {
            serde_json::to_string(value).expect("a Rust string always has a JSON string encoding")
        }
        AuthoringLiteral::Integer(value) => value.to_string(),
        AuthoringLiteral::Boolean(value) => value.to_string(),
    }
}

/// Render one admitted identity as an exact TOML key without changing it.
fn toml_key_source(value: &str) -> String {
    let is_bare = !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'));
    if is_bare {
        value.to_owned()
    } else {
        serde_json::to_string(value).expect("a Rust string always has a TOML basic-string encoding")
    }
}

fn documentation(value: &str, markup: AuthoringMarkupKind) -> lsp_types::Documentation {
    match markup {
        AuthoringMarkupKind::PlainText => lsp_types::Documentation::String(value.to_owned()),
        AuthoringMarkupKind::Markdown => lsp_types::Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: value.to_owned(),
        }),
    }
}

impl From<AuthoringMarkupKind> for MarkupKind {
    fn from(value: AuthoringMarkupKind) -> Self {
        match value {
            AuthoringMarkupKind::PlainText => Self::PlainText,
            AuthoringMarkupKind::Markdown => Self::Markdown,
        }
    }
}

fn path_label(path: &[pale_sdk::AuthoringPathSegment]) -> Option<String> {
    let fields = path
        .iter()
        .filter_map(|segment| match segment {
            pale_sdk::AuthoringPathSegment::Field(field) => Some(field.as_str()),
            pale_sdk::AuthoringPathSegment::ArrayItem => None,
        })
        .collect::<Vec<_>>();
    (!fields.is_empty()).then(|| fields.join("."))
}

fn position_from(document: &AnalysisDocument, byte: usize) -> Position {
    document
        .range_at_bytes(byte, byte)
        .map(|range| range.start)
        .expect("the caller supplied a byte boundary from the selected document")
}

fn range_from(document: &AnalysisDocument, range: ByteRange) -> Option<Range> {
    document.range_at_bytes(range.start, range.end)
}

fn incomplete_value_replacement(
    document: &AnalysisDocument,
    range: ByteRange,
    cursor: usize,
) -> Range {
    let text = document.text();
    let Some(source) = text.get(range.start..range.end) else {
        let position = position_from(document, cursor);
        return Range::new(position, position);
    };
    let Some((leading, _)) = source
        .char_indices()
        .find(|(_, value)| !value.is_whitespace())
    else {
        let position = position_from(document, cursor);
        return Range::new(position, position);
    };
    let trailing = source
        .char_indices()
        .rev()
        .find(|(_, value)| !value.is_whitespace())
        .map(|(index, value)| index + value.len_utf8())
        .expect("a leading non-whitespace scalar has a trailing boundary");
    range_from(
        document,
        ByteRange::new(range.start + leading, range.start + trailing),
    )
    .unwrap_or_else(|| {
        let position = position_from(document, cursor);
        Range::new(position, position)
    })
}

#[cfg(test)]
#[path = "../tests/internal/authoring.rs"]
pub(crate) mod tests;
