//! Shared command-error presentation without changing diagnostic meaning.

use pale_sdk::{ApiError, AuthoringCorrection, AuthoringSourceLocation, DiagnosticV1};

/// An admitted diagnostic and the source evidence retained by its failed call.
///
/// The diagnostic remains the sole owner of classification, canonical bytes and
/// content identity. Corrections and locations are copied SDK values for the
/// requesting caller, within that request's workspace and disclosure limits.
/// They are not an editor revision, permission to write, or proof that source is
/// still current. This value neither reads more source nor broadcasts evidence.
pub struct DiagnosticPresentation {
    diagnostic: DiagnosticV1,
    authoring_corrections: Vec<AuthoringCorrection>,
    authoring_location: Option<AuthoringSourceLocation>,
}

impl DiagnosticPresentation {
    /// Copy a failed call's exact diagnostic and source evidence together.
    #[must_use]
    pub fn from_api_error(error: &ApiError) -> Self {
        Self {
            diagnostic: error.diagnostic().clone(),
            authoring_corrections: error.authoring_corrections().to_vec(),
            authoring_location: error.authoring_location().cloned(),
        }
    }

    /// Present an admitted diagnostic with no separately retained source evidence.
    ///
    /// Successful result documents and CLI-owned host diagnostics use this path;
    /// source edits and exact field paths are never inferred from display text.
    #[must_use]
    pub fn from_diagnostic(diagnostic: &DiagnosticV1) -> Self {
        Self {
            diagnostic: diagnostic.clone(),
            authoring_corrections: Vec::new(),
            authoring_location: None,
        }
    }

    /// Borrow the unchanged admitted diagnostic, including its content identity.
    #[must_use]
    pub const fn diagnostic(&self) -> &DiagnosticV1 {
        &self.diagnostic
    }

    /// Borrow the compiler-proved corrections in SDK order.
    #[must_use]
    pub fn authoring_corrections(&self) -> &[AuthoringCorrection] {
        &self.authoring_corrections
    }

    /// Borrow the owner's exact field/index coordinate, when one was proved.
    #[must_use]
    pub const fn authoring_location(&self) -> Option<&AuthoringSourceLocation> {
        self.authoring_location.as_ref()
    }

    /// Encode readable CLI/MCP JSON around the unchanged diagnostic object.
    ///
    /// This presentation object is not a new canonical contract. The nested
    /// diagnostic retains its original schema; absent evidence is explicit as
    /// an empty correction array and a null location.
    ///
    /// # Errors
    ///
    /// Returns a JSON encoding error if the admitted diagnostic or its SDK-owned
    /// evidence cannot be projected. No partial envelope is returned.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.to_json_value()?)
    }

    /// Build the readable JSON object used by terminal and protocol presenters.
    ///
    /// This form lets a protocol or terminal presenter add bounded local
    /// evidence without parsing an already rendered JSON string.
    ///
    /// # Errors
    ///
    /// Returns a JSON projection error if the admitted diagnostic or retained
    /// source evidence cannot be represented without reinterpretation.
    pub fn to_json_value(&self) -> Result<serde_json::Value, serde_json::Error> {
        let mut envelope = serde_json::Map::new();
        envelope.insert(
            "diagnostic".to_owned(),
            serde_json::from_slice(self.diagnostic.canonical_bytes())?,
        );
        envelope.insert(
            "diagnosticIdentity".to_owned(),
            serde_json::to_value(self.diagnostic.identity())?,
        );
        envelope.insert(
            "authoringCorrections".to_owned(),
            serde_json::to_value(&self.authoring_corrections)?,
        );
        envelope.insert(
            "authoringLocation".to_owned(),
            serde_json::to_value(&self.authoring_location)?,
        );
        Ok(serde_json::Value::Object(envelope))
    }
}

#[cfg(test)]
#[path = "../../tests/internal/command_diagnostic.rs"]
mod tests;
