//! Bounded reference scanning over the sole retained SDK compilation.

use std::collections::BTreeSet;

use pale_sdk::{AuthoringIdentity, AuthoringOccurrencePageCursor, AuthoringOccurrencePageRequest};

use super::*;

enum SemanticOwner {
    Effective,
    Portable,
}

/// One request's cursor and duplicate custody, owned only by the analysis worker.
/// The cursor retains the exact SDK owner; the worker does not replace that
/// compilation or its snapshot while this request is active.
pub(crate) struct ReferenceScan {
    owner: SemanticOwner,
    identity: AuthoringIdentity,
    after: Option<AuthoringOccurrencePageCursor>,
    declaration: Option<(LocationKey, Location)>,
    seen: BTreeSet<LocationKey>,
    maximum_locations: usize,
}

/// One bounded page, including empty pages that still have a continuation.
pub(crate) struct ReferencePage {
    pub(crate) locations: Vec<Location>,
    pub(crate) complete: bool,
}

impl ReferenceScan {
    /// Resolve identity once using the existing navigation and definition owner.
    pub(crate) fn start(
        params: &lsp_types::ReferenceParams,
        snapshot: &Snapshot,
        syntax: &AuthoringSyntaxSnapshot,
        semantics: AuthoringSemanticContext<'_>,
    ) -> Result<Option<Self>, AuthoringQueryError> {
        let Some((document, document_syntax)) = super::document(
            snapshot,
            syntax,
            &params.text_document_position.text_document.uri,
        ) else {
            return Ok(None);
        };
        ensure_complete(document_syntax)?;
        let Some(navigation) = navigation(
            document,
            document_syntax,
            semantics,
            params.text_document_position.position,
        )?
        else {
            return Ok(None);
        };
        let identity = navigation
            .identity()
            .expect("navigation retains an identity")
            .clone();
        let owner = if semantics.effective.is_some() {
            SemanticOwner::Effective
        } else {
            SemanticOwner::Portable
        };
        // One root location plus at most one distinct range per retained fact.
        // This is a location-storage bound, not a total scan-work or RSS claim.
        let maximum_locations = snapshot
            .documents()
            .iter()
            .try_fold(0_usize, |bound, doc| {
                let facts = syntax
                    .document(doc.logical_label())
                    .map_or(0, |syntax| syntax.facts().len());
                bound
                    .checked_add(facts)
                    .and_then(|bound| bound.checked_add(1))
                    .ok_or(AuthoringQueryError::ReferenceLocationBound)
            })?;
        let declaration = if params.context.include_declaration {
            navigation
                .definition()
                .map(|location| exact_location(snapshot, syntax, location))
                .transpose()?
        } else {
            None
        };
        Ok(Some(Self {
            owner,
            identity,
            after: None,
            declaration,
            seen: BTreeSet::new(),
            maximum_locations,
        }))
    }

    /// Perform at most one SDK page. A separate declaration page avoids an
    /// overflow member or a second pending page beside the fixed 256-row page.
    pub(crate) fn next(
        &mut self,
        snapshot: &Snapshot,
        syntax: &AuthoringSyntaxSnapshot,
        semantics: AuthoringSemanticContext<'_>,
    ) -> Result<ReferencePage, AuthoringQueryError> {
        if let Some((key, location)) = self.declaration.take() {
            self.retain(key)?;
            return Ok(ReferencePage {
                locations: vec![location],
                complete: false,
            });
        }
        let mut request = AuthoringOccurrencePageRequest::new(self.identity.clone(), 256, 256);
        if let Some(after) = self.after.take() {
            request = request.after(after);
        }
        let page = match self.owner {
            SemanticOwner::Effective => semantics
                .effective
                .ok_or(AuthoringQueryError::SourceMappingUnavailable)?
                .authoring_occurrences(request)?,
            SemanticOwner::Portable => semantics
                .portable
                .ok_or(AuthoringQueryError::SourceMappingUnavailable)?
                .authoring_occurrences(request)?,
        };
        let mut locations = Vec::new();
        for occurrence in page.occurrences() {
            if matches!(
                occurrence.role(),
                AuthoringOccurrenceRole::Reference | AuthoringOccurrenceRole::Projection
            ) {
                let (key, location) = exact_location(snapshot, syntax, occurrence.location())?;
                if self.retain(key)? {
                    locations.push(location);
                }
            }
        }
        self.after = page.continuation().cloned();
        Ok(ReferencePage {
            locations,
            complete: page.is_complete(),
        })
    }

    fn retain(&mut self, key: LocationKey) -> Result<bool, AuthoringQueryError> {
        if self.seen.contains(&key) {
            return Ok(false);
        }
        if self.seen.len() >= self.maximum_locations {
            return Err(AuthoringQueryError::ReferenceLocationBound);
        }
        Ok(self.seen.insert(key))
    }
}

#[cfg(test)]
#[path = "../../tests/internal/reference_scan.rs"]
pub(crate) mod tests;
