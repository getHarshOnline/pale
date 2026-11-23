//! Bounded JSON Pointer identity for non-root Pale targets.
//!
//! [`jsonptr`] owns RFC 6901 parsing and escape validation. This module owns
//! the narrower Pale policy: a pointer must select a non-root, nonempty
//! target, contain no control characters, and fit the portable scalar bound.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize};

/// Maximum Unicode-scalar length of one portable pointer.
const MAX_JSON_POINTER_SCALARS: usize = 1_024;

/// JSON Schema projection of the Pale-owned target policy.
const NON_ROOT_JSON_POINTER_PATTERN: &str = r"^/(?:[^~\u0000-\u001F\u007F-\u009F]|~[01])+$";

/// A string cannot represent an admitted [`NonRootJsonPointer`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error(
    "JSON Pointer must select a non-root, nonempty target, contain no control characters, and be at most 1024 Unicode scalars"
)]
pub struct InvalidNonRootJsonPointer;

/// A bounded RFC 6901 JSON Pointer that selects a non-root Pale target.
///
/// The private string is admitted through [`Self::parse`] or deserialization.
/// The root spellings `""` and `"/"` are rejected; empty reference tokens
/// remain valid inside a larger pointer, such as `/a//b`. The foreign
/// [`jsonptr::Pointer`] type never crosses this Pale-owned boundary.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct NonRootJsonPointer(
    #[schemars(
        length(min = 2, max = 1_024),
        regex(pattern = NON_ROOT_JSON_POINTER_PATTERN)
    )]
    String,
);

impl NonRootJsonPointer {
    /// Validate and own a portable pointer received across a trust boundary.
    ///
    /// Syntax and RFC 6901 escaping are delegated to [`jsonptr::Pointer::parse`].
    /// Pale then enforces its non-root, nonempty-target, control-free, and
    /// Unicode-scalar bounds before copying the borrowed input.
    ///
    /// # Examples
    ///
    /// ```
    /// use pale_core::records::NonRootJsonPointer;
    ///
    /// let pointer = NonRootJsonPointer::parse("/a~1b/m~0n")?;
    /// assert_eq!(pointer.as_str(), "/a~1b/m~0n");
    /// assert!(NonRootJsonPointer::parse("").is_err());
    /// # Ok::<(), pale_core::records::InvalidNonRootJsonPointer>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`InvalidNonRootJsonPointer`] for the document root, the sole
    /// empty-token target `/`, malformed RFC 6901 escaping, control characters,
    /// or more than 1,024 Unicode scalar values.
    pub fn parse(value: &str) -> Result<Self, InvalidNonRootJsonPointer> {
        Self::validate(value)?;
        Ok(Self(value.to_owned()))
    }

    /// Check borrowed input without allocating or exposing the parser type.
    fn validate(value: &str) -> Result<(), InvalidNonRootJsonPointer> {
        let valid = value != "/"
            && value.chars().count() <= MAX_JSON_POINTER_SCALARS
            && !value.chars().any(char::is_control)
            && jsonptr::Pointer::parse(value).is_ok_and(|pointer| !pointer.is_root());
        if valid {
            Ok(())
        } else {
            Err(InvalidNonRootJsonPointer)
        }
    }

    /// Borrow the validated RFC 6901 spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for NonRootJsonPointer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::try_from(value).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for NonRootJsonPointer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl AsRef<str> for NonRootJsonPointer {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl FromStr for NonRootJsonPointer {
    type Err = InvalidNonRootJsonPointer;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<String> for NonRootJsonPointer {
    type Error = InvalidNonRootJsonPointer;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::validate(&value)?;
        Ok(Self(value))
    }
}

impl TryFrom<&str> for NonRootJsonPointer {
    type Error = InvalidNonRootJsonPointer;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

#[cfg(test)]
#[path = "../../tests/internal/records/json_pointer.rs"]
mod tests;
