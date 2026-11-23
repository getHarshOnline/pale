//! Validated tool-call matcher values used by lifecycle admission.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Maximum Unicode scalar values in one lifecycle or run-policy matcher.
pub(super) const MAX_MATCHER_SCALARS: usize = 4_096;
/// Maximum matcher entries admitted by one lifecycle or run-policy record.
pub(super) const MAX_MATCHERS: usize = 256;
/// Closed lexical grammar projected into generated JSON Schema.
const TOOL_CALL_MATCHER_PATTERN: &str = r"^(\*|[A-Za-z0-9_][A-Za-z0-9_.:-]*)(\([^()\r\n]+\))?$";

/// One exact native-harness tool, optionally constrained by a command glob.
///
/// The wire grammar is `Tool`, `Tool(command*)`, or `*`. Only `*` is special in
/// an argument pattern. Parentheses belong to this grammar rather than to that
/// pattern, so nested or unmatched parentheses are rejected before a policy can
/// be sealed. Values exceeding the fixed scalar bound are rejected before the
/// input is copied into this type.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ToolCallMatcher {
    raw: String,
    tool_end: usize,
    argument_start: Option<usize>,
}

impl ToolCallMatcher {
    /// Parse and validate one matcher without interpreting the argument glob.
    ///
    /// # Examples
    ///
    /// ```
    /// use pale_core::records::ToolCallMatcher;
    ///
    /// let matcher = ToolCallMatcher::parse("Bash(git status*)")?;
    /// assert_eq!(matcher.tool(), "Bash");
    /// assert_eq!(matcher.argument_glob(), Some("git status*"));
    /// # Ok::<(), pale_core::records::InvalidToolCallMatcher>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`InvalidToolCallMatcher`] when the scalar bound is exceeded, or
    /// for whitespace, malformed parentheses, control characters, empty globs,
    /// or a tool outside the closed ASCII name grammar. The error deliberately
    /// omits the rejected value.
    pub fn parse(value: &str) -> Result<Self, InvalidToolCallMatcher> {
        if !matcher_within_scalar_limit(value) {
            return Err(InvalidToolCallMatcher::TooLong {
                maximum: MAX_MATCHER_SCALARS,
            });
        }
        if value.is_empty() {
            return Err(InvalidToolCallMatcher::new("matcher is empty"));
        }
        if value.trim() != value {
            return Err(InvalidToolCallMatcher::new(
                "outer whitespace is not allowed",
            ));
        }
        let (tool, argument_glob) = match value.find('(') {
            Some(open) => {
                if !value.ends_with(')') {
                    return Err(InvalidToolCallMatcher::new(
                        "an argument glob must end with `)`",
                    ));
                }
                let glob = &value[open + 1..value.len() - 1];
                if glob.is_empty() {
                    return Err(InvalidToolCallMatcher::new(
                        "an argument glob cannot be empty",
                    ));
                }
                if glob.contains(['(', ')']) || glob.chars().any(char::is_control) {
                    return Err(InvalidToolCallMatcher::new(
                        "an argument glob cannot contain parentheses or control characters",
                    ));
                }
                (&value[..open], Some((open + 1, glob)))
            }
            None if value.contains(')') => {
                return Err(InvalidToolCallMatcher::new(
                    "matcher contains an unmatched `)`",
                ));
            }
            None => (value, None),
        };
        if !valid_tool_name(tool) {
            return Err(InvalidToolCallMatcher::new(
                "tool must be `*` or an ASCII identifier",
            ));
        }

        Ok(Self {
            raw: value.to_string(),
            tool_end: tool.len(),
            argument_start: argument_glob.map(|(start, _)| start),
        })
    }

    /// Borrow the exact validated wire spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// Borrow the native-harness tool name, or `*` for every tool.
    #[must_use]
    pub fn tool(&self) -> &str {
        &self.raw[..self.tool_end]
    }

    /// Optional command-argument glob inside the outer parentheses.
    pub fn argument_glob(&self) -> Option<&str> {
        self.argument_start
            .map(|start| &self.raw[start..self.raw.len() - 1])
    }
}

/// Whether a matcher stays within the shared scalar ceiling.
pub(super) fn matcher_within_scalar_limit(value: &str) -> bool {
    value.chars().take(MAX_MATCHER_SCALARS + 1).count() <= MAX_MATCHER_SCALARS
}

/// Validate the closed native-harness tool-name grammar.
fn valid_tool_name(tool: &str) -> bool {
    if tool == "*" {
        return true;
    }
    let mut chars = tool.chars();
    chars.next().is_some_and(|first| {
        (first.is_ascii_alphanumeric() || first == '_')
            && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
    })
}

impl fmt::Debug for ToolCallMatcher {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("ToolCallMatcher")
            .field(&self.raw)
            .finish()
    }
}

impl fmt::Display for ToolCallMatcher {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.raw)
    }
}

impl FromStr for ToolCallMatcher {
    type Err = InvalidToolCallMatcher;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl Serialize for ToolCallMatcher {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.raw)
    }
}

impl<'de> Deserialize<'de> for ToolCallMatcher {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).map_err(serde::de::Error::custom)
    }
}

impl schemars::JsonSchema for ToolCallMatcher {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "ToolCallMatcher".into()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        concat!(module_path!(), "::ToolCallMatcher").into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let mut schema = <String as schemars::JsonSchema>::json_schema(generator);
        schema.insert(
            "pattern".into(),
            serde_json::Value::String(TOOL_CALL_MATCHER_PATTERN.into()),
        );
        schema.insert("maxLength".into(), serde_json::json!(MAX_MATCHER_SCALARS));
        schema
    }
}

/// Schema for a bounded inventory of heterogeneous lifecycle matcher strings.
pub(super) fn matcher_strings_schema(
    _generator: &mut schemars::SchemaGenerator,
) -> schemars::Schema {
    schemars::Schema::try_from(serde_json::json!({
        "type": "array",
        "maxItems": MAX_MATCHERS,
        "items": {
            "type": "string",
            "maxLength": MAX_MATCHER_SCALARS
        }
    }))
    .expect("the static lifecycle matcher inventory schema is valid")
}

/// Schema for a bounded inventory of validated tool-call matchers.
pub(super) fn tool_call_matchers_schema(
    generator: &mut schemars::SchemaGenerator,
) -> schemars::Schema {
    let item = generator.subschema_for::<ToolCallMatcher>();
    schemars::Schema::try_from(serde_json::json!({
        "type": "array",
        "maxItems": MAX_MATCHERS,
        "items": item
    }))
    .expect("the static tool-call matcher inventory schema is valid")
}

/// A tool-call matcher did not satisfy the bounded closed wire grammar.
///
/// The rejected value is intentionally not retained, displayed, or formatted,
/// so untrusted input cannot create an unbounded diagnostic payload.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InvalidToolCallMatcher {
    /// The value exceeded the fixed parser and matching-work ceiling.
    #[error("invalid tool-call matcher: exceeds {maximum} Unicode scalar values")]
    TooLong {
        /// Maximum Unicode scalar values admitted for one matcher.
        maximum: usize,
    },
    /// The bounded value did not satisfy the closed lexical grammar.
    #[error("invalid tool-call matcher: {reason}")]
    Grammar {
        /// Stable explanation that contains no rejected input bytes.
        reason: &'static str,
    },
}

impl InvalidToolCallMatcher {
    /// Construct a redacted grammar error from one stable reason.
    fn new(reason: &'static str) -> Self {
        Self::Grammar { reason }
    }
}
