//! Bounded shell-command inspection for writes into sealed read-only roots.

use std::path::{Path, PathBuf};

use pale_ports::MachineFilesystemPort;

use super::super::shell::{is_command_boundary, static_command, tokens, ShellToken};
use super::path::{path_violation, resolve_target};

fn embedded_path_violation(
    filesystem: &dyn MachineFilesystemPort,
    word: &str,
    cwd: Option<&str>,
    readonly_dirs: &[PathBuf],
) -> Result<Option<String>, String> {
    let path_like = |candidate: &str| {
        candidate.starts_with('/')
            || candidate.starts_with("./")
            || candidate.starts_with("../")
            || (!candidate.starts_with('-') && candidate.contains('/'))
    };
    for candidate in std::iter::once(word).chain(word.split_once('=').map(|(_, value)| value)) {
        let candidate = candidate.trim_end_matches(['\'', '"', ',', ')', ']', '}']);
        if path_like(candidate) {
            if let Some(reason) = path_violation(filesystem, candidate, cwd, readonly_dirs)? {
                return Ok(Some(reason));
            }
        }
    }
    for (start, character) in word.char_indices() {
        if character != '/' {
            continue;
        }
        let boundary = start == 0
            || word[..start].chars().next_back().is_some_and(|prior| {
                !prior.is_ascii_alphanumeric() && !matches!(prior, '.' | '_' | '-')
            });
        if !boundary {
            continue;
        }
        let tail = &word[start..];
        let end = tail
            .char_indices()
            .skip(1)
            .find_map(|(index, value)| {
                matches!(value, '\'' | '"' | ',' | ')' | ']' | '}').then_some(index)
            })
            .unwrap_or(tail.len());
        if let Some(reason) = path_violation(filesystem, &tail[..end], cwd, readonly_dirs)? {
            return Ok(Some(reason));
        }
    }
    Ok(None)
}

fn inspect_shell_segment(
    filesystem: &dyn MachineFilesystemPort,
    segment: &[ShellToken],
    cwd: Option<&str>,
    readonly_dirs: &[PathBuf],
    depth: usize,
) -> Result<Option<String>, String> {
    for pair in segment.windows(2) {
        if matches!(&pair[0], ShellToken::Operator(op) if op.contains('>')) {
            let ShellToken::Word(target) = &pair[1] else {
                return Err("shell redirection has no literal destination".to_string());
            };
            if let Some(reason) = path_violation(filesystem, target, cwd, readonly_dirs)? {
                return Ok(Some(reason));
            }
        }
    }

    let projection = static_command(segment)
        .ok_or_else(|| "shell segment cannot be projected into a static command".to_string())?;
    let words = projection.argv();
    let Some(program) = words.first().and_then(|word| Path::new(word).file_name()) else {
        return Ok(None);
    };
    let program = program.to_string_lossy();
    if matches!(program.as_ref(), "sh" | "bash" | "zsh" | "dash") {
        if let Some(command_index) = words.iter().position(|word| *word == "-c") {
            let command = words
                .get(command_index + 1)
                .ok_or_else(|| "nested shell has no command after -c".to_string())?;
            return shell_violation_inner(filesystem, command, cwd, readonly_dirs, depth + 1);
        }
    }
    let operands: Vec<&str> = words[1..]
        .iter()
        .copied()
        .filter(|word| !word.starts_with('-'))
        .collect();
    let candidates: Vec<&str> = match program.as_ref() {
        "rm" | "rmdir" | "unlink" | "touch" | "mkdir" | "mkfifo" | "truncate" | "mv" | "chmod"
        | "chown" | "chgrp" => operands.clone(),
        "cp" | "install" | "ln" => operands.last().copied().into_iter().collect(),
        "tee" => operands.clone(),
        "dd" => operands
            .iter()
            .filter_map(|word| word.strip_prefix("of="))
            .collect(),
        _ => Vec::new(),
    };
    for candidate in candidates {
        if let Some(reason) = path_violation(filesystem, candidate, cwd, readonly_dirs)? {
            return Ok(Some(reason));
        }
    }
    let argv = &words[1..];
    if !crate::host_command::is_known_read_only_command(&program, argv) {
        let cwd_is_protected = cwd
            .and_then(|path| resolve_target(filesystem, path, None))
            .is_some_and(|path| readonly_dirs.iter().any(|root| path.starts_with(root)));
        let mut names_protected_root = false;
        for word in argv {
            if embedded_path_violation(filesystem, word, cwd, readonly_dirs)?.is_some() {
                names_protected_root = true;
                break;
            }
        }
        if cwd_is_protected || names_protected_root {
            return Ok(Some(format!(
                "command `{program}` can mutate a write-protected path but has no read-only argument policy"
            )));
        }
    }
    Ok(None)
}

/// Inspect a bounded shell command for a write into any sealed read-only root.
pub(super) fn shell_violation(
    filesystem: &dyn MachineFilesystemPort,
    command: &str,
    cwd: Option<&str>,
    readonly_dirs: &[PathBuf],
) -> Result<Option<String>, String> {
    shell_violation_inner(filesystem, command, cwd, readonly_dirs, 0)
}

fn shell_violation_inner(
    filesystem: &dyn MachineFilesystemPort,
    command: &str,
    cwd: Option<&str>,
    readonly_dirs: &[PathBuf],
    depth: usize,
) -> Result<Option<String>, String> {
    if depth > 4 {
        return Err("nested shell command exceeds the bounded inspection depth".to_string());
    }
    let tokens = tokens(command).ok_or_else(|| {
        "shell command cannot be parsed well enough to verify write targets".to_string()
    })?;
    let mut start = 0;
    let mut current_cwd = cwd.map(str::to_string);
    for end in 0..=tokens.len() {
        let boundary = end == tokens.len() || is_command_boundary(tokens.get(end));
        if boundary {
            let segment = &tokens[start..end];
            if let Some(reason) = inspect_shell_segment(
                filesystem,
                segment,
                current_cwd.as_deref(),
                readonly_dirs,
                depth,
            )? {
                return Ok(Some(reason));
            }
            let projection = static_command(segment).ok_or_else(|| {
                "shell segment cannot be projected into a static command".to_string()
            })?;
            let words = projection.argv();
            if words.first().is_some_and(|program| {
                Path::new(program)
                    .file_name()
                    .is_some_and(|name| name == "cd")
            }) {
                let target = words
                    .iter()
                    .skip(1)
                    .find(|word| !word.starts_with('-'))
                    .ok_or_else(|| "shell cd has no literal destination".to_string())?;
                current_cwd = Some(
                    resolve_target(filesystem, target, current_cwd.as_deref())
                        .ok_or_else(|| "shell cd destination cannot be resolved".to_string())?
                        .to_string_lossy()
                        .into_owned(),
                );
            }
            start = end.saturating_add(1);
        }
    }
    Ok(None)
}
