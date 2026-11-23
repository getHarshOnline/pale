//! Parse the shell-shaped command an agent sends into a deliberately smaller
//! command language.
//!
//! `hooks`, `host_tool`, `render` and `semantic` build on it; it builds on no
//! other module in this crate. Native harnesses expose command execution through
//! shell tools. A capability contract still describes executable/argv identity,
//! never shell source. Rejecting shell syntax before tokenization prevents a
//! permitted prefix from authorizing a chained or expanded second command. This
//! proves tokens and arguments, not PATH resolution or executable bytes; runtime
//! and provider identity remain higher-layer trust obligations.

use pale_core::records::HostCommandConstraint;

/// Shell-language characters rejected before executable/argument admission.
const SHELL_META_CHARACTERS: &[char] = &[
    ';', '&', '|', '<', '>', '`', '$', '(', ')', '{', '}', '[', ']', '*', '?', '~', '#',
];

/// `find` actions that can execute code or write data.
const FIND_EFFECTFUL_ACTIONS: &[&str] = &[
    "-delete", "-exec", "-execdir", "-fls", "-fprint", "-fprint0", "-fprintf", "-ok", "-okdir",
];

/// Git options that can execute helpers or write selected output.
const GIT_EFFECTFUL_OPTIONS: &[&str] =
    &["--ext-diff", "--output", "--show-signature", "--textconv"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadOnlyCommandPolicy {
    PassiveReader,
    Find,
    Ripgrep,
    GitStatus,
    GitDiff,
}

impl ReadOnlyCommandPolicy {
    fn for_command<T: AsRef<str>>(executable: &str, argv: &[T]) -> Option<Self> {
        match executable {
            "cat" | "head" | "ls" | "nl" | "tail" | "wc" => Some(Self::PassiveReader),
            "find" => Some(Self::Find),
            "rg" => Some(Self::Ripgrep),
            "git" => match (
                argv.first().map(AsRef::as_ref),
                argv.get(1).map(AsRef::as_ref),
            ) {
                (Some("--no-optional-locks"), Some("status")) => Some(Self::GitStatus),
                (Some("diff" | "log" | "show"), _) => Some(Self::GitDiff),
                _ => None,
            },
            _ => None,
        }
    }

    fn admits<T: AsRef<str>>(self, argv: &[T]) -> bool {
        match self {
            Self::PassiveReader => true,
            Self::Find => !argv
                .iter()
                .any(|arg| FIND_EFFECTFUL_ACTIONS.contains(&arg.as_ref())),
            Self::Ripgrep => {
                let mut no_config = false;
                for arg in argv
                    .iter()
                    .map(AsRef::as_ref)
                    .take_while(|arg| *arg != "--")
                {
                    if is_option(arg, "--pre") {
                        return false;
                    }
                    no_config |= arg == "--no-config";
                }
                no_config
            }
            Self::GitStatus => {
                argv.first()
                    .is_some_and(|arg| arg.as_ref() == "--no-optional-locks")
                    && argv.get(1).is_some_and(|arg| arg.as_ref() == "status")
            }
            Self::GitDiff => {
                let mut no_external_diff = false;
                let mut no_text_conversion = false;
                for arg in argv
                    .iter()
                    .map(AsRef::as_ref)
                    .take_while(|arg| *arg != "--")
                {
                    if GIT_EFFECTFUL_OPTIONS
                        .iter()
                        .any(|option| is_option(arg, option))
                    {
                        return false;
                    }
                    no_external_diff |= arg == "--no-ext-diff";
                    no_text_conversion |= arg == "--no-textconv";
                }
                no_external_diff && no_text_conversion
            }
        }
    }
}

/// Whether an exact or open-ended read constraint has a runtime-enforceable
/// argument policy. Compilation uses the same policy table as hook admission,
/// so a declaration cannot promise a boundary the runtime does not implement.
pub(crate) fn supports_read_only_constraint(constraint: &HostCommandConstraint) -> bool {
    let Some(policy) =
        ReadOnlyCommandPolicy::for_command(&constraint.executable, &constraint.argv_prefix)
    else {
        return false;
    };
    policy.admits(&constraint.argv_prefix)
}

fn is_option(arg: &str, option: &str) -> bool {
    arg == option
        || arg
            .strip_prefix(option)
            .is_some_and(|rest| rest.starts_with('='))
}

/// Parse literal executable/argv input after rejecting shell language.
fn parse(value: &str) -> Option<Vec<String>> {
    let value = value.trim();
    if value.is_empty()
        || value
            .chars()
            .any(|ch| ch == '\\' || ch == '!' || (ch.is_control() && ch != '\t'))
        || value
            .chars()
            .any(|character| SHELL_META_CHARACTERS.contains(&character))
    {
        return None;
    }
    let words = shlex::split(value)?;
    (!words.is_empty() && words.iter().all(|word| !word.is_empty())).then_some(words)
}

/// Convert one literal command into an exact, no-trailing-arguments constraint.
pub(crate) fn exact_constraint(value: &str) -> Option<HostCommandConstraint> {
    let mut words = parse(value)?.into_iter();
    Some(HostCommandConstraint {
        executable: words.next()?,
        argv_prefix: words.collect(),
        allow_trailing_args: false,
    })
}

/// Match host input after shell rejection and declared-prefix admission.
///
/// A constraint admitted as read-only keeps that policy for trailing arguments.
/// Other effect classes retain ordinary exact/prefix matching.
pub(crate) fn matches(constraint: &HostCommandConstraint, value: &str) -> bool {
    let Some(words) = parse(value) else {
        return false;
    };
    if !matches_parsed(constraint, &words) {
        return false;
    }
    !supports_read_only_constraint(constraint)
        || ReadOnlyCommandPolicy::for_command(&constraint.executable, &constraint.argv_prefix)
            .is_some_and(|policy| policy.admits(&words[1..]))
}

/// Match an already parsed executable and argument vector.
fn matches_parsed(constraint: &HostCommandConstraint, words: &[String]) -> bool {
    let Some((executable, argv)) = words.split_first() else {
        return false;
    };
    executable == &constraint.executable
        && argv.starts_with(&constraint.argv_prefix)
        && (constraint.allow_trailing_args || argv.len() == constraint.argv_prefix.len())
}

/// Whether one parsed host command is intrinsically read-only under the same
/// closed argument policy used by capability admission.
pub(crate) fn is_known_read_only_command(executable: &str, argv: &[&str]) -> bool {
    ReadOnlyCommandPolicy::for_command(executable, argv).is_some_and(|policy| policy.admits(argv))
}

/// Quote one opaque argument for the POSIX command strings accepted by native harnesses.
pub(crate) fn quote_posix_word(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
#[path = "../tests/internal/host_command.rs"]
mod tests;
