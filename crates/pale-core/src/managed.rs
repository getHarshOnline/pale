//! Exact parsing of the one marker-delimited block Pale owns in a user file.

/// Exact opening marker recognized by [`split_managed_block`].
///
/// This token is part of the managed-file contract; changing it would orphan
/// blocks written with the previous token.
pub const MANAGED_BEGIN: &str = "<!-- pale:begin managed -->";
/// Exact closing marker recognized by [`split_managed_block`].
///
/// This token is part of the managed-file contract and must change in lockstep
/// with [`MANAGED_BEGIN`].
pub const MANAGED_END: &str = "<!-- pale:end managed -->";

/// A valid managed block split from its surrounding user-owned text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManagedBlock<'a> {
    /// User-owned bytes before the opening marker.
    before: &'a str,
    /// Opening marker, owned payload, and closing marker.
    block: &'a str,
    /// User-owned bytes after the closing marker.
    after: &'a str,
}

impl<'a> ManagedBlock<'a> {
    /// Return the user-owned bytes before the opening marker.
    #[must_use]
    pub const fn before(&self) -> &'a str {
        self.before
    }

    /// Return the complete managed block, including both markers.
    #[must_use]
    pub const fn block(&self) -> &'a str {
        self.block
    }

    /// Return the user-owned bytes after the closing marker.
    #[must_use]
    pub const fn after(&self) -> &'a str {
        self.after
    }
}

/// Why a user-owned file cannot be divided into one Pale-managed block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ManagedBlockError {
    /// Neither managed marker is present.
    #[error("managed block is missing")]
    Missing,
    /// A marker is missing or either marker occurs more than once.
    #[error("managed block markers are duplicated or incomplete")]
    Malformed,
    /// The closing marker precedes the opening marker.
    #[error("managed block markers are out of order")]
    OutOfOrder,
}

/// Split exactly one managed block while preserving every byte outside it.
///
/// The value returned by [`ManagedBlock::block`] includes both markers. No newline is
/// inserted or normalized.
///
/// # Examples
///
/// ```
/// use pale_core::managed::split_managed_block;
/// use pale_core::{MANAGED_BEGIN, MANAGED_END};
///
/// let text = format!("before\n{MANAGED_BEGIN}\nowned\n{MANAGED_END}\nafter\n");
/// let split = split_managed_block(&text)?;
/// assert_eq!(split.before(), "before\n");
/// assert_eq!(split.after(), "\nafter\n");
/// # Ok::<(), pale_core::managed::ManagedBlockError>(())
/// ```
///
/// # Errors
///
/// Returns [`ManagedBlockError`] when the marker pair is absent, incomplete,
/// duplicated, or reversed.
pub fn split_managed_block(content: &str) -> Result<ManagedBlock<'_>, ManagedBlockError> {
    split_delimited_block(content, MANAGED_BEGIN, MANAGED_END)
}

/// Split one exact marker pair without changing any surrounding bytes.
///
/// Marker choice belongs to the caller's native owner. This lexical utility
/// establishes neither ownership nor permission to replace the returned block.
///
/// # Errors
///
/// Rejects empty or overlapping marker texts and absent, incomplete, duplicated
/// or reversed pairs.
pub fn split_delimited_block<'a>(
    content: &'a str,
    begin_marker: &str,
    end_marker: &str,
) -> Result<ManagedBlock<'a>, ManagedBlockError> {
    if begin_marker.is_empty()
        || end_marker.is_empty()
        || begin_marker.contains(end_marker)
        || end_marker.contains(begin_marker)
    {
        return Err(ManagedBlockError::Malformed);
    }
    let begins = content.matches(begin_marker).count();
    let ends = content.matches(end_marker).count();
    if begins == 0 && ends == 0 {
        return Err(ManagedBlockError::Missing);
    }
    if begins != 1 || ends != 1 {
        return Err(ManagedBlockError::Malformed);
    }

    let begin = content
        .find(begin_marker)
        .ok_or(ManagedBlockError::Malformed)?;
    let end_position = content
        .find(end_marker)
        .ok_or(ManagedBlockError::Malformed)?;
    if end_position < begin {
        return Err(ManagedBlockError::OutOfOrder);
    }
    let end = end_position + end_marker.len();
    Ok(ManagedBlock {
        before: &content[..begin],
        block: &content[begin..end],
        after: &content[end..],
    })
}
