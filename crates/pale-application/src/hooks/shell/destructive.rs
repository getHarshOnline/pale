//! Conservative classification of literal recursive-force deletion commands.
//!
//! The bounded parser recognizes direct `rm` and a finite depth of literal
//! shell `-c` nesting. Unsupported syntax returns unknown so the caller can
//! fail closed instead of treating an unparsed command as safe.

use std::path::PathBuf;

use super::{is_command_boundary, static_command, tokens, ShellToken};

fn rm_has_recursive_force(words: &[&str]) -> bool {
    let mut recursive = false;
    let mut force = false;
    let mut options = true;
    for word in words.iter().skip(1) {
        if options && *word == "--" {
            options = false;
            continue;
        }
        if !options || !word.starts_with('-') || *word == "-" {
            continue;
        }
        match *word {
            "--recursive" => recursive = true,
            "--force" => force = true,
            value if value.starts_with("--") => {}
            value => {
                for flag in value[1..].chars() {
                    recursive |= matches!(flag, 'r' | 'R');
                    force |= flag == 'f';
                }
            }
        }
    }
    recursive && force
}

fn segment_has_recursive_force_rm(segment: &[ShellToken], depth: usize) -> Option<bool> {
    let projection = static_command(segment)?;
    let words = projection.argv();
    let program = words
        .first()
        .and_then(|word| PathBuf::from(word).file_name().map(|name| name.to_owned()))?;
    let program = program.to_string_lossy();
    if program == "rm" {
        return Some(rm_has_recursive_force(words));
    }
    if matches!(program.as_ref(), "sh" | "bash" | "zsh" | "dash") {
        if let Some(index) = words.iter().position(|word| *word == "-c") {
            return recursive_force_delete_inner(words.get(index + 1)?, depth + 1);
        }
    }
    Some(false)
}

fn recursive_force_delete_inner(command: &str, depth: usize) -> Option<bool> {
    if depth > 4 {
        return None;
    }
    let tokens = tokens(command)?;
    let mut start = 0;
    for end in 0..=tokens.len() {
        if end == tokens.len() || is_command_boundary(tokens.get(end)) {
            if segment_has_recursive_force_rm(&tokens[start..end], depth)? {
                return Some(true);
            }
            start = end.saturating_add(1);
        }
    }
    Some(false)
}

/// Detect a direct literal `rm` invocation that combines recursive and force
/// flags. Returns `None` when the bounded parser cannot evaluate the command.
pub(in crate::hooks) fn recursive_force_delete(command: &str) -> Option<bool> {
    recursive_force_delete_inner(command, 0)
}

#[cfg(test)]
#[path = "../../../tests/internal/hooks/shell/destructive.rs"]
mod tests;
