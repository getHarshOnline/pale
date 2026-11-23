//! Bounded shell-token mechanics shared by lifecycle predicates and authority checks.
//!
//! This is deliberately not a shell interpreter. It recognizes only static
//! command structure, common execution wrappers, and bounded nested `sh -c`
//! invocations. Dynamic expansion is unevaluable; callers decide how an
//! unevaluable command follows the owning policy's failure posture.

use std::path::Path;

mod destructive;
mod git;

pub(in crate::hooks) use destructive::recursive_force_delete;
pub(in crate::hooks) use git::git_hook_bypass;

#[derive(Debug, Clone, PartialEq, Eq)]
/// Literal word or control operator from the intentionally bounded shell lexer.
pub(super) enum ShellToken {
    Word(String),
    Operator(String),
}

/// Static command projection shared by the bounded policy classifiers.
///
/// `prefix` retains literal environment assignments and recognized wrapper
/// arguments that precede the effective program. `argv` contains the effective
/// program and its arguments after redirection operands have been removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StaticCommand<'a> {
    prefix: Vec<&'a str>,
    argv: Vec<&'a str>,
}

impl<'a> StaticCommand<'a> {
    /// Return literal words consumed before the effective program.
    pub(super) fn prefix(&self) -> &[&'a str] {
        &self.prefix
    }

    /// Return the effective literal program and its arguments.
    pub(super) fn argv(&self) -> &[&'a str] {
        &self.argv
    }
}

/// Tokenize shell syntax only while every produced word remains literal.
///
/// Single quotes and backslash escapes preserve literal metacharacters. An
/// unescaped parameter, command, arithmetic, tilde, pathname, or brace
/// expansion returns `None` because its resulting argument vector depends on
/// shell state that this bounded lexer neither owns nor evaluates.
pub(super) fn tokens(command: &str) -> Option<Vec<ShellToken>> {
    let mut tokens = Vec::new();
    let mut word = String::new();
    let mut chars = command.chars().peekable();
    let mut quote = None;
    let flush = |word: &mut String, tokens: &mut Vec<ShellToken>| {
        if !word.is_empty() {
            tokens.push(ShellToken::Word(std::mem::take(word)));
        }
    };

    while let Some(ch) = chars.next() {
        match quote {
            Some('\'') => {
                if ch == '\'' {
                    quote = None;
                } else {
                    word.push(ch);
                }
            }
            Some('"') => match ch {
                '"' => quote = None,
                '\\' => {
                    let escaped = chars.next()?;
                    if matches!(escaped, '$' | '`' | '"' | '\\') {
                        word.push(escaped);
                    } else if escaped == '\n' {
                        // A quoted line continuation contributes no byte.
                    } else if escaped == '\r' && chars.peek() == Some(&'\n') {
                        chars.next();
                    } else {
                        // In double quotes, a backslash before any other byte
                        // remains part of the literal word.
                        word.push('\\');
                        word.push(escaped);
                    }
                }
                '`' | '$' => return None,
                _ => word.push(ch),
            },
            Some(_) => return None,
            None => match ch {
                '\'' | '"' => quote = Some(ch),
                '\\' => {
                    let escaped = chars.next()?;
                    if escaped == '\n' {
                        // An unquoted line continuation contributes no byte.
                    } else if escaped == '\r' && chars.peek() == Some(&'\n') {
                        chars.next();
                    } else {
                        word.push(escaped);
                    }
                }
                '`' | '$' | '(' | ')' | '~' | '*' | '?' | '[' | ']' | '{' | '}' => {
                    return None;
                }
                '\r' => {
                    flush(&mut word, &mut tokens);
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    tokens.push(ShellToken::Operator("newline".into()));
                }
                '\n' => {
                    flush(&mut word, &mut tokens);
                    tokens.push(ShellToken::Operator("newline".into()));
                }
                c if c.is_whitespace() => flush(&mut word, &mut tokens),
                ';' | '|' | '&' | '>' | '<' => {
                    flush(&mut word, &mut tokens);
                    if ch == '&' && chars.peek() == Some(&'>') {
                        // Bash/Zsh combined-output redirection is outside the
                        // portable bounded grammar and must not become a false
                        // command boundary followed by a separate redirect.
                        return None;
                    }
                    let mut operator = ch.to_string();
                    if chars.peek().is_some_and(|next| *next == ch)
                        || ((ch == '>' || ch == '<') && chars.peek() == Some(&'&'))
                    {
                        operator.push(chars.next()?);
                    }
                    tokens.push(ShellToken::Operator(operator));
                }
                _ => word.push(ch),
            },
        }
    }
    if quote.is_some() {
        return None;
    }
    flush(&mut word, &mut tokens);
    Some(tokens)
}

fn is_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn words_without_redirections(segment: &[ShellToken]) -> Option<Vec<&str>> {
    let mut words = Vec::new();
    let mut index = 0;
    while index < segment.len() {
        match &segment[index] {
            ShellToken::Word(word) => {
                words.push(word.as_str());
                index += 1;
            }
            ShellToken::Operator(operator)
                if matches!(operator.as_str(), ">" | ">>" | "<" | ">&" | "<&") =>
            {
                if words.last().is_some_and(|word| {
                    !word.is_empty() && word.bytes().all(|byte| byte.is_ascii_digit())
                }) {
                    // Source positions are intentionally absent from this
                    // bounded token model, so `2>file` cannot be distinguished
                    // from the separate numeric argument in `2 >file`.
                    return None;
                }
                let Some(ShellToken::Word(_target)) = segment.get(index + 1) else {
                    return None;
                };
                index += 2;
            }
            // Here-documents and all control operators outside the segment
            // boundary grammar are intentionally unevaluable.
            ShellToken::Operator(_) => return None,
        }
    }
    Some(words)
}

fn consume_option(
    words: &[&str],
    index: &mut usize,
    option: &str,
    short_with_value: &[char],
    long_with_value: &[&str],
    short_flags: &[char],
    long_flags: &[&str],
) -> Option<()> {
    if let Some(long) = option.strip_prefix("--") {
        let (name, attached) = long
            .split_once('=')
            .map_or((long, false), |(name, _)| (name, true));
        if long_with_value.contains(&name) && !attached {
            *index += 1;
            words.get(*index)?;
        } else if !long_with_value.contains(&name) && !long_flags.contains(&name) {
            return None;
        }
        *index += 1;
        return Some(());
    }
    let mut chars = option.strip_prefix('-')?.chars();
    while let Some(flag) = chars.next() {
        if short_with_value.contains(&flag) {
            if chars.as_str().is_empty() {
                *index += 1;
                words.get(*index)?;
            }
            break;
        } else if !short_flags.contains(&flag) {
            return None;
        }
    }
    *index += 1;
    Some(())
}

fn consume_wrapper(words: &[&str], start: usize, program: &str) -> Option<Option<usize>> {
    let mut index = start + 1;
    match program {
        "command" => {
            while let Some(option) = words
                .get(index)
                .copied()
                .filter(|word| word.starts_with('-'))
            {
                if option == "--" {
                    index += 1;
                    break;
                }
                if option != "-p" {
                    return None;
                }
                index += 1;
            }
        }
        "nohup" => {
            if words.get(index) == Some(&"--") {
                index += 1;
            } else if words.get(index).is_some_and(|word| word.starts_with('-')) {
                return None;
            }
        }
        "sudo" => {
            while let Some(option) = words
                .get(index)
                .copied()
                .filter(|word| word.starts_with('-'))
            {
                if option == "--" {
                    index += 1;
                    break;
                }
                consume_option(
                    words,
                    &mut index,
                    option,
                    &[
                        'C', 'D', 'a', 'c', 'g', 'h', 'p', 'R', 'r', 'T', 't', 'U', 'u',
                    ],
                    &[
                        "auth-type",
                        "chdir",
                        "chroot",
                        "close-from",
                        "command-timeout",
                        "group",
                        "host",
                        "login-class",
                        "other-user",
                        "prompt",
                        "role",
                        "type",
                        "user",
                    ],
                    &['A', 'b', 'E', 'H', 'n', 'P', 'S'],
                    &[
                        "askpass",
                        "background",
                        "non-interactive",
                        "preserve-env",
                        "set-home",
                        "stdin",
                    ],
                )?;
            }
            while words.get(index).is_some_and(|word| is_assignment(word)) {
                index += 1;
            }
        }
        "env" => {
            while let Some(word) = words.get(index).copied() {
                if word == "--" {
                    index += 1;
                    break;
                }
                if word.starts_with('-') {
                    consume_option(
                        words,
                        &mut index,
                        word,
                        &['C', 'P', 'S', 'a', 'u'],
                        &["argv0", "chdir", "split-string", "unset"],
                        &['0', 'i', 'v'],
                        &["debug", "ignore-environment", "null"],
                    )?;
                    continue;
                }
                if is_assignment(word) {
                    index += 1;
                    continue;
                }
                break;
            }
        }
        _ => return Some(None),
    }
    Some(Some(index))
}

fn is_unsupported_control_word(program: &str) -> bool {
    matches!(
        program,
        "!" | "."
            | "break"
            | "builtin"
            | "case"
            | "continue"
            | "coproc"
            | "declare"
            | "do"
            | "done"
            | "elif"
            | "else"
            | "end"
            | "esac"
            | "eval"
            | "exec"
            | "export"
            | "fi"
            | "for"
            | "foreach"
            | "function"
            | "if"
            | "in"
            | "local"
            | "nocorrect"
            | "noglob"
            | "readonly"
            | "repeat"
            | "return"
            | "select"
            | "set"
            | "shift"
            | "source"
            | "then"
            | "time"
            | "times"
            | "trap"
            | "typeset"
            | "unset"
            | "until"
            | "while"
    )
}

/// Project one literal segment into an effective command without interpreting shell state.
///
/// Recognized redirections and execution wrappers are removed structurally.
/// Unsupported wrapper options, control words, and shell grammar return `None`
/// so authority checks cannot infer safety from a lossy argument vector.
pub(super) fn static_command(segment: &[ShellToken]) -> Option<StaticCommand<'_>> {
    let words = words_without_redirections(segment)?;
    let mut index = 0;
    while words.get(index).is_some_and(|word| is_assignment(word)) {
        index += 1;
    }
    let mut command_start = index;
    loop {
        let Some(program) = words
            .get(command_start)
            .and_then(|word| Path::new(word).file_name())
            .and_then(|name| name.to_str())
        else {
            break;
        };
        if is_unsupported_control_word(program) {
            return None;
        }
        let Some(next) = consume_wrapper(&words, command_start, program)? else {
            break;
        };
        command_start = next;
    }
    Some(StaticCommand {
        prefix: words[..command_start].to_vec(),
        argv: words[command_start..].to_vec(),
    })
}

/// Recognize operators that terminate one independently inspected command segment.
pub(super) fn is_command_boundary(token: Option<&ShellToken>) -> bool {
    matches!(token, Some(ShellToken::Operator(op)) if matches!(op.as_str(), ";" | "&" | "&&" | "||" | "|" | "newline"))
}
