//! Conservative classification of literal Git hook-bypass attempts.
//!
//! Recognition covers effectful `--no-verify` subcommands and configuration
//! that replaces `core.hooksPath`. Unknown shell syntax remains unknown; this
//! module never executes Git or infers safety from command text it cannot parse.

use std::path::Path;

use super::{is_command_boundary, static_command, tokens, ShellToken};

/// Effectful Git subcommands for which `--no-verify` bypasses hooks.
const GIT_NO_VERIFY_COMMANDS: &[&str] = &["am", "cherry-pick", "commit", "merge", "push", "rebase"];

fn git_config_key(value: &str) -> Option<&str> {
    value.split_once('=').map(|(key, _)| key)
}

fn overrides_git_hooks(value: &str) -> bool {
    git_config_key(value).is_some_and(|key| key.eq_ignore_ascii_case("core.hookspath"))
}

fn git_global_option_takes_value(value: &str) -> bool {
    matches!(
        value,
        "-C" | "--git-dir" | "--work-tree" | "--namespace" | "--super-prefix"
    )
}

fn is_git_program(value: Option<&&str>) -> bool {
    value
        .and_then(|word| Path::new(word).file_name())
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            name.eq_ignore_ascii_case("git") || name.eq_ignore_ascii_case("git.exe")
        })
}

fn environment_overrides_git_hooks(prefix: &[&str]) -> bool {
    prefix.iter().any(|word| {
        word.split_once('=').is_some_and(|(name, value)| {
            name.strip_prefix("GIT_CONFIG_KEY_")
                .is_some_and(|index| !index.is_empty())
                && value.eq_ignore_ascii_case("core.hookspath")
        })
    })
}

fn git_command<'a>(words: &'a [&'a str]) -> Option<(Option<&'a str>, &'a [&'a str], bool)> {
    let program = words
        .first()
        .and_then(|word| Path::new(word).file_name())
        .and_then(|name| name.to_str())?;
    if !program.eq_ignore_ascii_case("git") && !program.eq_ignore_ascii_case("git.exe") {
        return Some((None, &[], false));
    }

    let mut index = 1;
    while index < words.len() {
        let word = words[index];
        if word == "--" {
            index += 1;
            break;
        }
        if word == "-c" {
            let value = words.get(index + 1).copied()?;
            if overrides_git_hooks(value) {
                return Some((None, &[], true));
            }
            index += 2;
            continue;
        }
        if let Some(value) = word.strip_prefix("-c").filter(|value| !value.is_empty()) {
            if overrides_git_hooks(value) {
                return Some((None, &[], true));
            }
            index += 1;
            continue;
        }
        if word == "--config-env" {
            let value = words.get(index + 1).copied()?;
            if overrides_git_hooks(value) {
                return Some((None, &[], true));
            }
            index += 2;
            continue;
        }
        if let Some(value) = word.strip_prefix("--config-env=") {
            if overrides_git_hooks(value) {
                return Some((None, &[], true));
            }
            index += 1;
            continue;
        }
        if git_global_option_takes_value(word) {
            words.get(index + 1)?;
            index += 2;
            continue;
        }
        if word.starts_with("-C") && word.len() > 2
            || word.starts_with("--git-dir=")
            || word.starts_with("--work-tree=")
            || word.starts_with("--namespace=")
            || word.starts_with("--super-prefix=")
        {
            index += 1;
            continue;
        }
        if word.starts_with('-') {
            index += 1;
            continue;
        }
        break;
    }

    let command = words.get(index).copied();
    let args = words
        .get(index + usize::from(command.is_some())..)
        .unwrap_or(&[]);
    Some((command, args, false))
}

fn command_option_takes_value(command: &str, value: &str) -> bool {
    match command {
        "commit" => matches!(
            value,
            "-m" | "--message"
                | "-F"
                | "--file"
                | "-C"
                | "--reuse-message"
                | "-c"
                | "--reedit-message"
                | "--author"
                | "--date"
                | "-t"
                | "--template"
                | "--fixup"
                | "--squash"
                | "--pathspec-from-file"
                | "--trailer"
        ),
        "push" => matches!(
            value,
            "--repo" | "--receive-pack" | "--exec" | "-o" | "--push-option"
        ),
        "merge" => matches!(
            value,
            "-m" | "--message"
                | "-F"
                | "--file"
                | "-s"
                | "--strategy"
                | "-X"
                | "--strategy-option"
                | "--cleanup"
                | "--into-name"
        ),
        "cherry-pick" => matches!(
            value,
            "-m" | "--mainline" | "-s" | "--strategy" | "-X" | "--strategy-option" | "--cleanup"
        ),
        "rebase" => matches!(
            value,
            "--onto"
                | "--exec"
                | "-x"
                | "-s"
                | "--strategy"
                | "-X"
                | "--strategy-option"
                | "--empty"
                | "--whitespace"
        ),
        "am" => matches!(value, "--patch-format" | "--resolvemsg"),
        _ => false,
    }
}

fn commit_short_no_verify(value: &str) -> bool {
    if !value.starts_with('-') || value.starts_with("--") || value == "-" {
        return false;
    }
    for option in value[1..].chars() {
        if matches!(option, 'm' | 'F' | 'C' | 'c' | 't') {
            return false;
        }
        if option == 'n' {
            return true;
        }
    }
    false
}

fn config_command_overrides_hooks(args: &[&str]) -> bool {
    let Some(key_index) = args
        .iter()
        .position(|arg| arg.eq_ignore_ascii_case("core.hookspath"))
    else {
        return false;
    };
    let read_or_restore = args[..key_index].iter().any(|arg| {
        matches!(
            *arg,
            "get"
                | "get-all"
                | "get-regexp"
                | "get-urlmatch"
                | "list"
                | "unset"
                | "unset-all"
                | "remove-section"
                | "--get"
                | "--get-all"
                | "--get-regexp"
                | "--get-urlmatch"
                | "--list"
                | "-l"
                | "--unset"
                | "--unset-all"
                | "--remove-section"
        )
    });
    !read_or_restore
        && (args.get(key_index + 1).is_some()
            || args[..key_index]
                .iter()
                .any(|arg| matches!(*arg, "set" | "--add" | "--replace-all")))
}

fn command_bypasses_hooks(command: &str, args: &[&str]) -> bool {
    if command == "config" {
        return config_command_overrides_hooks(args);
    }
    if !GIT_NO_VERIFY_COMMANDS.contains(&command) {
        return false;
    }
    let mut consumes_next = false;
    for arg in args {
        if consumes_next {
            consumes_next = false;
            continue;
        }
        if *arg == "--" {
            break;
        }
        if *arg == "--no-verify" || command == "commit" && commit_short_no_verify(arg) {
            return true;
        }
        consumes_next = command_option_takes_value(command, arg);
    }
    false
}

fn segment_has_git_hook_bypass(segment: &[ShellToken], depth: usize) -> Option<bool> {
    let projection = static_command(segment)?;
    let words = projection.argv();
    let is_git = is_git_program(words.first());
    let prefix = projection.prefix();
    let program = words
        .first()
        .and_then(|word| Path::new(word).file_name())
        .and_then(|name| name.to_str());
    if matches!(program, Some("sh" | "bash" | "zsh" | "dash")) {
        if let Some(index) = words.iter().position(|word| *word == "-c") {
            return git_hook_bypass_inner(words.get(index + 1)?, depth + 1);
        }
    }
    let (command, args, override_hooks) = git_command(words)?;
    Some(
        override_hooks
            || is_git && environment_overrides_git_hooks(prefix)
            || command.is_some_and(|command| command_bypasses_hooks(command, args)),
    )
}

fn git_hook_bypass_inner(command: &str, depth: usize) -> Option<bool> {
    if depth > 4 {
        return None;
    }
    let tokens = tokens(command)?;
    let mut start = 0;
    for end in 0..=tokens.len() {
        if end == tokens.len() || is_command_boundary(tokens.get(end)) {
            if segment_has_git_hook_bypass(&tokens[start..end], depth)? {
                return Some(true);
            }
            start = end.saturating_add(1);
        }
    }
    Some(false)
}

/// Detect a literal Git command that disables repository hooks.
///
/// Dynamic expansion and malformed literal syntax are unevaluable, so the
/// lifecycle record's failure posture decides instead of trusting an argument
/// vector the shell could change at execution time.
pub(in crate::hooks) fn git_hook_bypass(command: &str) -> Option<bool> {
    git_hook_bypass_inner(command, 0)
}

#[cfg(test)]
#[path = "../../../tests/internal/hooks/shell/git.rs"]
mod tests;
