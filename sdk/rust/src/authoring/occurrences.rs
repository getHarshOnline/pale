//! Find where a compiled identity occurs in workspace source.

use std::sync::Arc;

use super::{
    application, authoring_continuation_failure, source_location, AuthoringIdentity,
    AuthoringOccurrenceRole, AuthoringSourceLocation,
};
use crate::{
    ApiError, EffectiveProfileCompilation, PortableWorkspaceCompilation, PortableWorkspaceSubject,
};

/// Maximum UTF-8 bytes copied into occurrence-location strings by one page.
pub const MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES: usize =
    application::MAX_AUTHORING_OCCURRENCE_PAGE_COPIED_STRING_BYTES;

/// Select an identity and limit the entries examined and returned per page.
///
/// Zero selects the application's default of 64; values above 256 are capped.
/// A continuation requires the same raw requested limits, even when different
/// requests would normalize to the same bounds. No source text is reparsed.
#[derive(Clone)]
pub struct AuthoringOccurrencePageRequest {
    identity: AuthoringIdentity,
    maximum_inspected: u16,
    maximum_returned: u16,
    after: Option<AuthoringOccurrencePageCursor>,
}

impl AuthoringOccurrencePageRequest {
    /// Select a compiler-produced identity and both independent page limits.
    #[must_use]
    pub const fn new(
        identity: AuthoringIdentity,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Self {
        Self {
            identity,
            maximum_inspected,
            maximum_returned,
            after: None,
        }
    }

    /// Resume a cursor issued by this exact compilation, identity and limits.
    #[must_use]
    pub fn after(mut self, cursor: AuthoringOccurrencePageCursor) -> Self {
        self.after = Some(cursor);
        self
    }
}

impl std::fmt::Debug for AuthoringOccurrencePageRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuthoringOccurrencePageRequest")
            .field("maximum_inspected", &self.maximum_inspected)
            .field("maximum_returned", &self.maximum_returned)
            .field("has_continuation", &self.after.is_some())
            .finish_non_exhaustive()
    }
}

/// One compiler-resolved occurrence, copied without source text or graph nodes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthoringOccurrence {
    location: AuthoringSourceLocation,
    role: AuthoringOccurrenceRole,
}

impl AuthoringOccurrence {
    pub(crate) fn from_application(occurrence: &application::AuthoringOccurrence) -> Self {
        Self {
            location: source_location(occurrence),
            role: occurrence.role().into(),
        }
    }

    /// Borrow the exact logical source label and decoded field/index path.
    #[must_use]
    pub const fn location(&self) -> &AuthoringSourceLocation {
        &self.location
    }

    /// Return the compiler's definition, reference or projection role.
    #[must_use]
    pub const fn role(&self) -> AuthoringOccurrenceRole {
        self.role
    }
}

/// Resume a source lookup against the same compilation and query.
///
/// Cloning shares the compiler owner; it does not copy the graph. The owner
/// remains alive until all its handles and cursor copies are dropped. Neither
/// Debug nor serialization exposes a reconstructible cursor: this type has no
/// serializer, parser or public constructor. It grants no disclosure authority.
#[derive(Clone)]
pub struct AuthoringOccurrencePageCursor {
    owner: OccurrenceOwner,
    query: OccurrenceQuery,
    maximum_inspected: u16,
    maximum_returned: u16,
    application: application::AuthoringOccurrencePageCursor,
}

/// Identity navigation and direct subject mapping are different queries.
#[derive(Clone, PartialEq, Eq)]
enum OccurrenceQuery {
    Identity(AuthoringIdentity),
    Subject(PortableWorkspaceSubject),
}

impl std::fmt::Debug for AuthoringOccurrencePageCursor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AuthoringOccurrencePageCursor")
            .finish_non_exhaustive()
    }
}

/// Owned matching occurrences and bounded scan progress.
///
/// An empty page can be incomplete. A byte-deferred matching row is re-inspected
/// on the next page and returned once; no location is truncated. Inspected counts
/// describe this retained index, not permission to reveal it to another caller.
#[derive(Clone, Debug)]
pub struct AuthoringOccurrencePage {
    occurrences: Vec<AuthoringOccurrence>,
    inspected: u16,
    copied_string_bytes: u32,
    continuation: Option<AuthoringOccurrencePageCursor>,
}

impl AuthoringOccurrencePage {
    /// Borrow matching occurrences in canonical source/path order.
    #[must_use]
    pub fn occurrences(&self) -> &[AuthoringOccurrence] {
        &self.occurrences
    }

    /// Number of index entries examined, including nonmatches.
    #[must_use]
    pub const fn inspected(&self) -> u16 {
        self.inspected
    }

    /// UTF-8 bytes copied into returned source labels and field names.
    #[must_use]
    pub const fn copied_string_bytes(&self) -> u32 {
        self.copied_string_bytes
    }

    /// Borrow the opaque continuation, when exhaustion has not been proved.
    #[must_use]
    pub const fn continuation(&self) -> Option<&AuthoringOccurrencePageCursor> {
        self.continuation.as_ref()
    }

    /// Return whether the underlying scan reached the end of the index.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.continuation.is_none()
    }
}

/// Direct source meaning, separate from membership in the selected proof.
///
/// These are copied observations from the queried compilation, not live editor
/// state or permission to disclose source. A declaration and its derived policy
/// are different subjects; this lookup does not follow proof dependencies.
#[derive(Clone, Debug)]
pub enum PortableWorkspaceSourceSubject {
    /// No semantic occurrence is indexed at the requested path.
    /// This does not prove that the source or field is absent.
    NoOccurrence,
    /// The occurrence exists, but has no supported direct proof-subject mapping.
    NoDenotation {
        /// Compiler-assigned source location and role.
        occurrence: AuthoringOccurrence,
    },
    /// The occurrence names this subject, but the selected proof excludes it.
    Unselected {
        /// Captured source location and role.
        occurrence: AuthoringOccurrence,
        /// Full subject identity, without a claim of selected membership.
        subject: PortableWorkspaceSubject,
    },
    /// The occurrence names a subject present in the selected proof.
    Selected {
        /// Captured source location and role.
        occurrence: AuthoringOccurrence,
        /// Full identity admitted by this compilation's selected proof.
        subject: PortableWorkspaceSubject,
    },
}

/// Proof membership and source matches answer different questions.
///
/// An unselected subject may have captured definitions. A selected subject may
/// have no direct named occurrence, but still have contributing source reachable
/// through [`PortableWorkspaceCompilation::direct_dependencies`]. An empty page
/// is conclusive only after its scan reports completion.
#[derive(Clone, Debug)]
pub enum PortableWorkspaceSubjectOccurrences {
    /// The selected proof does not contain the subject; source matches may exist.
    Unselected(AuthoringOccurrencePage),
    /// The proof contains the subject, independently of the returned matches.
    Selected(AuthoringOccurrencePage),
}

/// A source lookup bound to the exact compilation that supplied its answer.
///
/// Equal semantic digests need not mean equal captured source: unselected input
/// can differ. This handle therefore shares the immutable compilation until
/// dropped. It does not copy its graph or retain an editor's mutable text.
#[derive(Clone)]
pub struct PortableWorkspaceSourceInspection {
    owner: Arc<application::PortableWorkspaceUniverse>,
    location: AuthoringSourceLocation,
    result: PortableWorkspaceSourceSubject,
}

impl std::fmt::Debug for PortableWorkspaceSourceInspection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PortableWorkspaceSourceInspection")
            .finish_non_exhaustive()
    }
}

impl PortableWorkspaceSourceInspection {
    /// Requested logical location, including when no occurrence was found.
    #[must_use]
    pub const fn location(&self) -> &AuthoringSourceLocation {
        &self.location
    }

    /// Direct named meaning and selected-proof membership from the compiler.
    #[must_use]
    pub const fn result(&self) -> &PortableWorkspaceSourceSubject {
        &self.result
    }

    /// Test exact compilation custody, not equality of semantic digests.
    ///
    /// A fresh compilation does not match, even from identical input. This
    /// identity check grants neither disclosure nor execution permission.
    #[must_use]
    pub fn belongs_to(&self, compilation: &PortableWorkspaceCompilation) -> bool {
        std::ptr::eq(self.owner.as_ref(), compilation.application_universe())
    }
}

#[derive(Clone)]
enum OccurrenceOwner {
    Portable(Arc<application::PortableWorkspaceUniverse>),
    Effective(Arc<pale_application::compile::EffectiveProfileCompilation>),
}

impl OccurrenceOwner {
    fn shares_compilation(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Portable(left), Self::Portable(right)) => Arc::ptr_eq(left, right),
            (Self::Effective(left), Self::Effective(right)) => Arc::ptr_eq(left, right),
            _ => false,
        }
    }

    fn semantics(&self) -> &application::AuthoringSemanticIndex {
        match self {
            Self::Portable(owner) => owner.authoring_semantics(),
            Self::Effective(owner) => owner.authoring_semantics(),
        }
    }
}

fn page(
    owner: OccurrenceOwner,
    request: AuthoringOccurrencePageRequest,
    diagnostic_identity: &pale_core::records::RecordSchemaIdentity,
) -> Result<AuthoringOccurrencePage, ApiError> {
    validate_continuation(
        &owner,
        |query| matches!(query, OccurrenceQuery::Identity(identity) if *identity == request.identity),
        request.maximum_inspected,
        request.maximum_returned,
        request.after.as_ref(),
        diagnostic_identity,
    )?;
    let result = owner.semantics().occurrences_page(
        &request.identity.inner,
        request.after.as_ref().map(|after| &after.application),
        usize::from(request.maximum_inspected),
        usize::from(request.maximum_returned),
    );
    Ok(project_page(
        &owner,
        || OccurrenceQuery::Identity(request.identity),
        request.maximum_inspected,
        request.maximum_returned,
        result,
    ))
}

fn validate_continuation(
    owner: &OccurrenceOwner,
    matches_query: impl FnOnce(&OccurrenceQuery) -> bool,
    maximum_inspected: u16,
    maximum_returned: u16,
    after: Option<&AuthoringOccurrencePageCursor>,
    diagnostic_identity: &pale_core::records::RecordSchemaIdentity,
) -> Result<(), ApiError> {
    if let Some(after) = after {
        if !owner.shares_compilation(&after.owner)
            || !matches_query(&after.query)
            || maximum_inspected != after.maximum_inspected
            || maximum_returned != after.maximum_returned
        {
            return Err(authoring_continuation_failure(diagnostic_identity));
        }
    }
    Ok(())
}

fn project_page(
    owner: &OccurrenceOwner,
    query: impl FnOnce() -> OccurrenceQuery,
    maximum_inspected: u16,
    maximum_returned: u16,
    result: application::AuthoringOccurrencePage<'_>,
) -> AuthoringOccurrencePage {
    let occurrences = result
        .occurrences()
        .iter()
        .map(|occurrence| AuthoringOccurrence::from_application(occurrence))
        .collect();
    let inspected =
        u16::try_from(result.inspected()).expect("application occurrence work is capped at 256");
    let copied_string_bytes = u32::try_from(result.copied_string_bytes())
        .expect("application occurrence copied-string ceiling fits u32");
    let continuation =
        result
            .continuation()
            .cloned()
            .map(|application| AuthoringOccurrencePageCursor {
                owner: owner.clone(),
                query: query(),
                maximum_inspected,
                maximum_returned,
                application,
            });
    AuthoringOccurrencePage {
        occurrences,
        inspected,
        copied_string_bytes,
        continuation,
    }
}

impl PortableWorkspaceCompilation {
    /// Read the compiler subject directly named at one source location.
    ///
    /// Uses the retained index without parsing text or inferring identity from
    /// labels. A captured but unadopted definition can return `Unselected`.
    /// Policy, import and extension provenance must use their own proof subjects;
    /// this method does not turn nearby declarations into those records.
    ///
    /// The result retains the exact compilation, including captured occurrences
    /// outside selected proof. An editor must also retain its own generation.
    /// Neither the source occurrence nor selected membership grants disclosure
    /// permission or establishes native readiness.
    ///
    /// # Errors
    /// Returns the existing authoring diagnostic when the path exceeds
    /// [`super::AUTHORING_PATH_SEGMENT_LIMIT`].
    pub fn source_subject(
        &self,
        request: super::AuthoringNavigationRequest,
    ) -> Result<PortableWorkspaceSourceInspection, ApiError> {
        if request.path.len() > super::AUTHORING_PATH_SEGMENT_LIMIT {
            return Err(super::authoring_query_failure(
                &self.diagnostic_identity,
                super::AuthoringQueryError::PathTooDeep {
                    observed: request.path.len(),
                    maximum: super::AUTHORING_PATH_SEGMENT_LIMIT,
                },
            ));
        }
        let location = AuthoringSourceLocation {
            source_label: request.source_label,
            path: request.path,
        };
        let path = location
            .path
            .iter()
            .cloned()
            .map(Into::into)
            .collect::<Vec<_>>();
        let result = match self
            .application_universe()
            .source_subject(&location.source_label, &path)
        {
            application::PortableWorkspaceSourceSubject::NoOccurrence => {
                PortableWorkspaceSourceSubject::NoOccurrence
            }
            application::PortableWorkspaceSourceSubject::NoDenotation { occurrence } => {
                PortableWorkspaceSourceSubject::NoDenotation {
                    occurrence: AuthoringOccurrence::from_application(occurrence),
                }
            }
            application::PortableWorkspaceSourceSubject::Unselected {
                occurrence,
                subject,
            } => PortableWorkspaceSourceSubject::Unselected {
                occurrence: AuthoringOccurrence::from_application(occurrence),
                subject: PortableWorkspaceSubject::from_internal(subject),
            },
            application::PortableWorkspaceSourceSubject::Selected {
                occurrence,
                subject,
            } => PortableWorkspaceSourceSubject::Selected {
                occurrence: AuthoringOccurrence::from_application(occurrence),
                subject: PortableWorkspaceSubject::from_internal(subject),
            },
        };
        Ok(PortableWorkspaceSourceInspection {
            owner: self.application_universe_handle(),
            location,
            result,
        })
    }

    /// Page source occurrences that directly name a full compiler-proof subject.
    ///
    /// The application determines both membership and matches. Every inspected
    /// occurrence counts toward work, including nonmatches; a candidate lookup
    /// is performed for each. Zero selects 64 and values above 256 are capped.
    /// Row limits combine with a fixed copied-string ceiling. Neither bounds
    /// elapsed time or retained compilation memory.
    ///
    /// Resume on this exact compilation with the same subject and both raw
    /// limits. Identity-navigation cursors are not subject-mapping cursors, even
    /// if the two queries happen to return identical locations. Empty incomplete
    /// pages must be resumed to establish exhaustion. This supplied-input query
    /// does not admit a recipient or filter disclosure.
    ///
    /// # Errors
    /// Returns the existing authoring continuation diagnostic before scanning
    /// when the owner, query kind, full subject or either raw limit differs.
    pub fn subject_occurrences(
        &self,
        subject: &PortableWorkspaceSubject,
        after: Option<&AuthoringOccurrencePageCursor>,
        maximum_inspected: u16,
        maximum_returned: u16,
    ) -> Result<PortableWorkspaceSubjectOccurrences, ApiError> {
        let owner = OccurrenceOwner::Portable(self.application_universe_handle());
        validate_continuation(
            &owner,
            |query| matches!(query, OccurrenceQuery::Subject(expected) if expected == subject),
            maximum_inspected,
            maximum_returned,
            after,
            &self.diagnostic_identity,
        )?;
        let result = self.application_universe().subject_occurrences(
            subject.application_subject(),
            after.map(|cursor| &cursor.application),
            usize::from(maximum_inspected),
            usize::from(maximum_returned),
        );
        let project = |page| {
            project_page(
                &owner,
                || OccurrenceQuery::Subject(subject.clone()),
                maximum_inspected,
                maximum_returned,
                page,
            )
        };
        Ok(match result {
            application::PortableWorkspaceSubjectOccurrences::Unselected(page) => {
                PortableWorkspaceSubjectOccurrences::Unselected(project(page))
            }
            application::PortableWorkspaceSubjectOccurrences::Selected(page) => {
                PortableWorkspaceSubjectOccurrences::Selected(project(page))
            }
        })
    }

    /// Page exact occurrences of a compiler identity in this retained workspace.
    ///
    /// An identity may originate in another compilation; a continuation may not.
    /// A fresh compilation with identical digests is still a different cursor
    /// owner. Results copy only locations and roles; cursors share owner custody.
    /// The caller remains responsible for the permitted source-disclosure scope.
    ///
    /// # Errors
    /// Returns [`ApiError`] before scanning if a continuation's owner, complete
    /// identity or either raw requested limit differs.
    pub fn authoring_occurrences(
        &self,
        request: AuthoringOccurrencePageRequest,
    ) -> Result<AuthoringOccurrencePage, ApiError> {
        page(
            OccurrenceOwner::Portable(self.application_universe_handle()),
            request,
            &self.diagnostic_identity,
        )
    }
}

impl EffectiveProfileCompilation {
    /// Page exact occurrences in this retained profile's combined source index.
    ///
    /// Aliases of this compilation can resume; a fresh compilation cannot, even
    /// with equal semantic coordinates. The identity itself remains reusable.
    /// Results own their locations and roles; cursors share the compiler owner
    /// until dropped. The caller owns permission to disclose the source scope.
    ///
    /// # Errors
    /// Returns [`ApiError`] before scanning if a continuation's owner, complete
    /// identity or either raw requested limit differs.
    pub fn authoring_occurrences(
        &self,
        request: AuthoringOccurrencePageRequest,
    ) -> Result<AuthoringOccurrencePage, ApiError> {
        page(
            OccurrenceOwner::Effective(Arc::clone(&self.compilation)),
            request,
            &self.diagnostic_identity,
        )
    }
}
