//! Host-neutral lexical path contracts shared by configuration and adapters.
//!
//! These types reject ambiguous path syntax only. They do not canonicalize a
//! filesystem path or defend against symlink traversal; an effectful adapter must
//! establish those properties beneath its trusted root.
//!
//! # Examples
//!
//! ```
//! use pale_core::paths::{ConfinedRelativePath, SurfaceTarget};
//!
//! assert!(ConfinedRelativePath::parse("docs/guide.md").is_some());
//! assert!(ConfinedRelativePath::parse("../outside").is_none());
//!
//! let target = SurfaceTarget::parse("{state}/session.json")
//!     .ok_or("the example target must satisfy the documented grammar")?;
//! assert_eq!(target.logical_root(), Some("state"));
//! assert_eq!(target.relative_path().as_str(), "session.json");
//! # Ok::<(), &'static str>(())
//! ```

use serde::{Deserialize, Deserializer, Serialize};

/// Maximum UTF-8 byte length of one portable confined relative path.
pub const MAX_CONFINED_RELATIVE_PATH_BYTES: usize = 4_096;

/// An owned path does not satisfy [`ConfinedRelativePath`]'s lexical contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("path must be a non-empty confined relative path of at most 4096 UTF-8 bytes")]
pub struct InvalidConfinedRelativePath;

/// A non-empty relative filesystem path with explicit components only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfinedRelativePath<'a>(&'a str);

impl<'a> ConfinedRelativePath<'a> {
    /// Parse a non-empty slash-delimited path with explicit safe components.
    ///
    /// Returns [`None`] for an absolute path, a logical-root prefix, an empty,
    /// `.` or `..` component, a backslash, or a control character. This remains
    /// a lexical check; see the [module-level contract](crate::paths).
    pub fn parse(value: &'a str) -> Option<Self> {
        if value.is_empty()
            || value.starts_with(['/', '{'])
            || value.contains('\\')
            || value.chars().any(char::is_control)
            || value
                .split('/')
                .any(|component| component.is_empty() || matches!(component, "." | ".."))
        {
            return None;
        }
        Some(Self(value))
    }

    /// Return the original validated representation.
    #[must_use]
    pub fn as_str(self) -> &'a str {
        self.0
    }

    /// Iterate the validated, non-empty path components.
    pub fn components(self) -> impl Iterator<Item = &'a str> {
        self.0.split('/')
    }
}

/// Owned, serializable form of [`ConfinedRelativePath`].
///
/// This value is suitable for portable source labels and Package-owned source
/// references. It is not a filesystem capability: an adapter must still resolve
/// it beneath an already trusted root and defend against symlink traversal.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, schemars::JsonSchema)]
#[serde(transparent)]
pub struct OwnedConfinedRelativePath(#[schemars(length(min = 1, max = 4096))] String);

impl OwnedConfinedRelativePath {
    /// Validate and own one portable confined relative path.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidConfinedRelativePath`] when `value` exceeds the wire
    /// bound or fails [`ConfinedRelativePath::parse`].
    pub fn parse(value: impl Into<String>) -> Result<Self, InvalidConfinedRelativePath> {
        let value = value.into();
        if value.len() > MAX_CONFINED_RELATIVE_PATH_BYTES
            || ConfinedRelativePath::parse(&value).is_none()
        {
            return Err(InvalidConfinedRelativePath);
        }
        Ok(Self(value))
    }

    /// Borrow the validated slash-delimited representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Borrow this value through the zero-copy lexical view.
    #[must_use]
    pub fn as_confined(&self) -> ConfinedRelativePath<'_> {
        ConfinedRelativePath(self.as_str())
    }
}

impl<'de> Deserialize<'de> for OwnedConfinedRelativePath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl std::fmt::Display for OwnedConfinedRelativePath {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl AsRef<str> for OwnedConfinedRelativePath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// One filename in the current directory, never a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlainFilename<'a>(ConfinedRelativePath<'a>);

impl<'a> PlainFilename<'a> {
    /// Parse one [`ConfinedRelativePath`] that contains no directory separator.
    pub fn parse(value: &'a str) -> Option<Self> {
        if value.contains('/') {
            return None;
        }
        ConfinedRelativePath::parse(value).map(Self)
    }

    /// Return the original validated filename.
    #[must_use]
    pub fn as_str(self) -> &'a str {
        self.0.as_str()
    }
}

/// A lexically validated rendered target.
///
/// Construction is intentionally confined to [`SurfaceTarget::parse`], so a
/// logical target cannot bypass root-name or relative-path validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceTarget<'a>(SurfaceTargetKind<'a>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SurfaceTargetKind<'a> {
    Filesystem(ConfinedRelativePath<'a>),
    Logical {
        root: &'a str,
        path: ConfinedRelativePath<'a>,
    },
}

impl<'a> SurfaceTarget<'a> {
    /// Parse either a confined repository path or a `{root}/path` logical target.
    ///
    /// A logical root uses only ASCII alphanumerics, `-`, and `_`; its tail must
    /// satisfy [`ConfinedRelativePath::parse`]. Returns [`None`] for any other
    /// spelling.
    pub fn parse(value: &'a str) -> Option<Self> {
        if !value.starts_with('{') {
            return ConfinedRelativePath::parse(value)
                .map(SurfaceTargetKind::Filesystem)
                .map(Self);
        }
        let close = value.find('}')?;
        let root = &value[1..close];
        if root.is_empty()
            || !root
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return None;
        }
        let path = value.get(close + 1..)?.strip_prefix('/')?;
        Some(Self(SurfaceTargetKind::Logical {
            root,
            path: ConfinedRelativePath::parse(path)?,
        }))
    }

    /// Whether this target names the repository filesystem rather than an
    /// adapter-owned logical root.
    #[must_use]
    pub fn is_filesystem(self) -> bool {
        matches!(self.0, SurfaceTargetKind::Filesystem(_))
    }

    /// Lexically validated path suffix for the repository or declared logical root.
    ///
    /// The effect adapter must still resolve it beneath an already opened,
    /// trusted root without following an escaping filesystem traversal.
    #[must_use]
    pub fn relative_path(self) -> ConfinedRelativePath<'a> {
        match self.0 {
            SurfaceTargetKind::Filesystem(path) | SurfaceTargetKind::Logical { path, .. } => path,
        }
    }

    /// Declarative logical-root name, or `None` for a repository path.
    pub fn logical_root(self) -> Option<&'a str> {
        match self.0 {
            SurfaceTargetKind::Filesystem(_) => None,
            SurfaceTargetKind::Logical { root, .. } => Some(root),
        }
    }
}
