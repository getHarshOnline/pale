//! Node-native error projection without a second failure taxonomy.

use napi::bindgen_prelude::{JsObjectValue, Object, ToNapiValue};
use napi::{Env, Error, Status};
use pale_sdk::{ApiError, ApiInitializationError};

use crate::{AuthoringCorrection, AuthoringSourceLocation, Diagnostic};

/// Project one canonical API diagnostic into a Node-native `Error`.
///
/// The ordinary `message` is the diagnostic's concise safe summary. The typed
/// `diagnostic` property carries the API-owned classification and exact
/// canonical bytes, so callers never scrape prose or create another diagnostic
/// policy.
/// Separate `authoringCorrections` and `authoringLocation` properties retain
/// compiler-proved edits and exact source coordinates without changing the
/// diagnostic record. They do not authorize writes. Caught values still need
/// ordinary JavaScript narrowing; correction members have a native class identity.
pub(crate) fn diagnostic_error(env: &Env, error: ApiError) -> Error {
    let diagnostic = error.diagnostic();
    let summary = diagnostic.summary().to_owned();
    let structured = Diagnostic::from_api(diagnostic);

    let projected = env
        .create_error(Error::new(Status::GenericFailure, summary.clone()))
        .and_then(|mut value| {
            value.set_named_property("diagnostic", structured)?;
            value.set_named_property(
                "authoringCorrections",
                error
                    .authoring_corrections()
                    .iter()
                    .map(AuthoringCorrection::from_api)
                    .collect::<Vec<_>>(),
            )?;
            value.set_named_property(
                "authoringLocation",
                error
                    .authoring_location()
                    .map(AuthoringSourceLocation::from_api)
                    .transpose()?,
            )?;
            retain_error_object(env, value)
        });

    projected.unwrap_or_else(|_| Error::new(Status::GenericFailure, summary))
}

/// Retain the decorated JavaScript error so napi-rs throws that exact object.
fn retain_error_object(env: &Env, value: Object<'_>) -> napi::Result<Error> {
    value
        .into_unknown(env)
        .map(Error::from_unknown_without_coercion)
}

/// Project the pre-diagnostic initialization boundary distinctly.
pub(crate) fn initialization_error(error: ApiInitializationError) -> Error {
    Error::new(Status::GenericFailure, format!("{}: {error}", error.code()))
}
