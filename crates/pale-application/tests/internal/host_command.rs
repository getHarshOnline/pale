//! Host-command matcher proofs for literal argv constraints and effect classification.

use super::*;

fn prefix(executable: &str, argv_prefix: &[&str]) -> HostCommandConstraint {
    HostCommandConstraint {
        executable: executable.into(),
        argv_prefix: argv_prefix.iter().map(|arg| (*arg).into()).collect(),
        allow_trailing_args: true,
    }
}

#[test]
fn shell_language_never_enters_the_literal_command_matcher() {
    let constraint = prefix("cat", &[]);
    assert!(matches(&constraint, "cat status.txt"));
    for command in [
        "cat status.txt; touch escaped",
        "cat $(touch escaped)",
        "cat status.txt > escaped",
        "cat *.rs",
    ] {
        assert!(!matches(&constraint, command), "admitted {command}");
    }
}

#[test]
fn read_policy_accepts_safe_arguments_and_rejects_effectful_forms() {
    for (constraint, command) in [
        (prefix("cat", &[]), "cat notes.md"),
        (
            prefix("find", &["src"]),
            "find src -type f -name Cargo.toml",
        ),
        (
            prefix("rg", &["--no-config"]),
            "rg --no-config -n needle src",
        ),
        (
            prefix("git", &["diff", "--no-ext-diff", "--no-textconv"]),
            "git diff --no-ext-diff --no-textconv -- src/lib.rs",
        ),
        (
            prefix("git", &["--no-optional-locks", "status"]),
            "git --no-optional-locks status --short",
        ),
    ] {
        assert!(supports_read_only_constraint(&constraint));
        assert!(matches(&constraint, command), "rejected {command}");
    }

    for (constraint, command) in [
        (prefix("find", &[]), "find . -delete"),
        (prefix("find", &[]), "find . -exec touch escaped +"),
        (prefix("find", &[]), "find . -fprint escaped"),
        (
            prefix("rg", &["--no-config"]),
            "rg --no-config --pre=cat needle",
        ),
        (
            prefix("git", &["diff", "--no-ext-diff", "--no-textconv"]),
            "git diff --no-ext-diff --no-textconv --output escaped",
        ),
    ] {
        assert!(
            !matches(&constraint, command),
            "generic match admitted {command}"
        );
    }

    for (constraint, command) in [
        (prefix("rg", &[]), "rg needle src"),
        (prefix("git", &["status"]), "git status --short"),
        (prefix("git", &["diff"]), "git diff -- src/lib.rs"),
    ] {
        assert!(matches(&constraint, command));
        assert!(!supports_read_only_constraint(&constraint));
    }
}

#[test]
fn effectful_command_requires_an_effectful_declared_prefix() {
    let constraint = prefix("find", &[".", "-delete"]);

    assert!(!supports_read_only_constraint(&constraint));
    assert!(matches(&constraint, "find . -delete"));
    assert!(!matches(&constraint, "find src -delete"));
}

#[test]
fn read_policy_rejects_unknown_commands_even_when_exact() {
    for command in ["cargo test 'one integration'", "rm -rf build"] {
        let constraint = exact_constraint(command).expect("literal command");
        assert!(matches(&constraint, command));
        assert!(!supports_read_only_constraint(&constraint));
    }
}

#[test]
fn option_terminator_keeps_option_shaped_paths_inert() {
    assert!(matches(
        &prefix("rg", &["--no-config"]),
        "rg --no-config needle -- --pre=cat"
    ));
    assert!(matches(
        &prefix("git", &["diff", "--no-ext-diff", "--no-textconv"]),
        "git diff --no-ext-diff --no-textconv -- --output"
    ));
}

#[test]
fn posix_word_quoting_preserves_empty_space_and_apostrophe() {
    assert_eq!(quote_posix_word(""), "''");
    assert_eq!(quote_posix_word("two words"), "'two words'");
    assert_eq!(quote_posix_word("Harsh's"), "'Harsh'\\''s'");
}
