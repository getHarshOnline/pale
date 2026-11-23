//! Git hook-bypass classification proofs over literal arguments rather than substrings.

use super::git_hook_bypass;

#[test]
fn classifies_literal_argv_without_message_false_positives() {
    for command in [
        "git commit --no-verify -m safe",
        "git commit -an -m safe",
        "git push --no-verify",
        "git -c core.hooksPath=/dev/null commit -m safe",
        "git -c 'core.HOOKSPATH=/dev/null' push",
        "git --config-env=core.hooksPath=HOOKS commit -m safe",
        "GIT_CONFIG_KEY_0=core.hooksPath GIT_CONFIG_VALUE_0=/dev/null GIT_CONFIG_COUNT=1 git commit -m safe",
        "git config --local core.hooksPath /dev/null",
        "git config set core.hooksPath /dev/null",
        "echo ready && /usr/bin/git merge --no-verify topic",
        "bash -c 'git cherry-pick --no-verify abc123'",
        "git commit \\\n--no-verify",
        "git commit \\\r\n--no-verify",
        "sudo -u root git commit --no-verify",
        "sudo MODE=checked git commit --no-verify",
        "env -u NAME git push --no-verify",
        "env -P /usr/bin git commit --no-verify",
        "git >out commit --no-verify",
    ] {
        assert_eq!(git_hook_bypass(command), Some(true), "{command}");
    }
    for command in [
        "git commit -m '--no-verify is dangerous'",
        "git commit -mn -m safe",
        "git log -n 5 && git commit -m safe",
        "echo --no-verify && git push origin main",
        "git rebase --exec 'echo --no-verify' main",
        "git commit -m safe -- --no-verify",
        "git config --get core.hooksPath",
        "git config --unset core.hooksPath",
        "git config core.hooksPath",
        "echo GIT_CONFIG_KEY_0=core.hooksPath",
        "cargo test",
    ] {
        assert_eq!(git_hook_bypass(command), Some(false), "{command}");
    }
    assert_eq!(git_hook_bypass("git commit 'unterminated"), None);
    assert_eq!(
        git_hook_bypass("git status\ngit commit --no-verify -m safe"),
        Some(true)
    );
    assert_eq!(
        git_hook_bypass("git status & git push --no-verify"),
        Some(true)
    );
    assert_eq!(git_hook_bypass("echo $(git push --no-verify)"), None);
    for command in [
        "! git commit --no-verify",
        "time git commit --no-verify",
        "exec git commit --no-verify",
        "builtin eval 'git commit --no-verify'",
        "coproc git commit --no-verify",
        "repeat 2 git commit --no-verify",
        "noglob git commit --no-verify",
        "sudo --unsupported root git commit --no-verify",
        "env --unsupported NAME git commit --no-verify",
        "git 2>out commit --no-verify",
        "git &>out commit --no-verify",
        "git &>>out commit --no-verify",
    ] {
        assert_eq!(git_hook_bypass(command), None, "{command}");
    }
}

#[test]
fn dynamic_shell_arguments_are_unevaluable_but_quoted_literals_remain_literal() {
    for command in [
        "git commit $ARGS",
        "git push ${FLAGS}",
        "git commit \"$ARGS\"",
        "git commit ~/message",
        "git commit *.patch",
        "git commit arg?.patch",
        "git commit [ab].patch",
        "git commit {safe,--no-verify}",
    ] {
        assert_eq!(git_hook_bypass(command), None, "{command}");
    }

    for command in [
        "git commit '$ARGS'",
        "git commit \\$ARGS",
        "git commit \"*.patch\"",
        "git commit 'arg?.patch'",
        "git commit \\{safe,--no-verify\\}",
    ] {
        assert_eq!(git_hook_bypass(command), Some(false), "{command}");
    }
}

#[test]
fn unsupported_shell_structure_never_becomes_hook_bypass_evidence() {
    for command in [
        "git commit --no-verify <<EOF",
        "git 2 >out commit --no-verify",
        "(git commit --no-verify)",
    ] {
        assert_eq!(git_hook_bypass(command), None, "{command}");
    }
    assert_eq!(git_hook_bypass("git commit --no-verify | cat"), Some(true));
    assert_eq!(
        git_hook_bypass("printf safe || git push --no-verify"),
        Some(true)
    );
    assert_eq!(
        git_hook_bypass("printf 'a|b' && git commit --no-verify"),
        Some(true)
    );
}
