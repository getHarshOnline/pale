//! Lexical path inventory normalizes without filesystem access and emits stable sorted entries.

use std::collections::BTreeMap;

use pale_core::records::{
    ActionError, ActionErrorCode, PathInventoryEntry, PathInventoryInput, PathInventoryOutput,
    PathStyle, PATH_INVENTORY_ACTION_ID,
};

use super::{error::action_error, MAX_CANONICAL_INTEGER};

// Lexical normalization retains every unique path, so both count and per-path
// bytes are bounded before map insertion.
/// Maximum caller-supplied paths normalized in one pure invocation.
const MAX_PATHS: usize = 100_000;
/// Maximum UTF-8 bytes accepted for one lexical path.
const MAX_PATH_BYTES: usize = 4_096;

fn invalid(index: usize, reason: &str) -> ActionError {
    action_error(
        Some(PATH_INVENTORY_ACTION_ID),
        ActionErrorCode::InvalidPath,
        format!("supplied path at index {index} is invalid: {reason}"),
    )
}

fn normalize(path: &str, style: PathStyle, index: usize) -> Result<String, ActionError> {
    if path.is_empty() {
        return Err(invalid(index, "path is empty"));
    }
    if path.len() > MAX_PATH_BYTES {
        return Err(invalid(index, "path exceeds the 4096-byte action bound"));
    }
    if path.chars().any(char::is_control) {
        return Err(invalid(index, "control characters are not portable"));
    }
    if path.contains('\\') {
        return Err(invalid(
            index,
            "backslashes are outside the POSIX path grammar",
        ));
    }

    let absolute = path.starts_with('/');
    match style {
        PathStyle::PosixRelative if absolute => {
            return Err(invalid(index, "absolute path supplied for relative style"));
        }
        PathStyle::PosixAbsolute if !absolute => {
            return Err(invalid(index, "relative path supplied for absolute style"));
        }
        _ => {}
    }

    let mut components = Vec::new();
    for component in path.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if components.pop().is_none() {
                    return Err(invalid(index, "parent traversal escapes the lexical root"));
                }
            }
            value => components.push(value),
        }
    }

    let joined = components.join("/");
    Ok(match style {
        PathStyle::PosixRelative if joined.is_empty() => ".".to_string(),
        PathStyle::PosixRelative => joined,
        PathStyle::PosixAbsolute if joined.is_empty() => "/".to_string(),
        PathStyle::PosixAbsolute => format!("/{joined}"),
    })
}

/// Summarize caller-supplied paths without reading or normalizing the filesystem.
pub(super) fn execute(input: &PathInventoryInput) -> Result<PathInventoryOutput, ActionError> {
    if input.paths.len() > MAX_PATHS {
        return Err(action_error(
            Some(PATH_INVENTORY_ACTION_ID),
            ActionErrorCode::MalformedInput,
            format!("path inventory exceeds the {MAX_PATHS}-path action bound"),
        ));
    }
    let mut paths = BTreeMap::<String, u64>::new();
    for (index, supplied) in input.paths.iter().enumerate() {
        let normalized = normalize(supplied, input.style, index)?;
        let occurrences = paths.entry(normalized).or_default();
        *occurrences = occurrences.checked_add(1).ok_or_else(|| {
            action_error(
                Some(PATH_INVENTORY_ACTION_ID),
                ActionErrorCode::NumericOverflow,
                "path occurrence count overflowed u64",
            )
        })?;
    }

    let input_count = u64::try_from(input.paths.len()).map_err(|_| {
        action_error(
            Some(PATH_INVENTORY_ACTION_ID),
            ActionErrorCode::NumericOverflow,
            "path input count does not fit in u64",
        )
    })?;
    if input_count > MAX_CANONICAL_INTEGER {
        return Err(action_error(
            Some(PATH_INVENTORY_ACTION_ID),
            ActionErrorCode::NumericOverflow,
            "path input count exceeds the canonical JSON safe-integer bound",
        ));
    }
    let unique_count = u64::try_from(paths.len()).map_err(|_| {
        action_error(
            Some(PATH_INVENTORY_ACTION_ID),
            ActionErrorCode::NumericOverflow,
            "unique path count does not fit in u64",
        )
    })?;

    Ok(PathInventoryOutput {
        input_count,
        unique_count,
        entries: paths
            .into_iter()
            .map(|(normalized_path, occurrences)| PathInventoryEntry {
                normalized_path,
                occurrences,
            })
            .collect(),
    })
}

#[cfg(test)]
#[path = "../../tests/internal/actions/path_inventory.rs"]
mod tests;
