//! Destructive-shell classification proofs over literal command structure.

use super::recursive_force_delete;

#[test]
fn classifies_literal_command_forms() {
    for command in [
        "rm -rf build",
        "rm -R -f build",
        "sudo -- rm --recursive --force build",
        "echo ready && env MODE=clean rm -fr build",
        "bash -c 'rm -r -f build'",
        "rm \\\n-rf build",
        "rm \\\r\n-rf build",
        "sudo -u root rm -rf build",
        "sudo MODE=checked rm -rf build",
        "env -u NAME rm -rf build",
        "env -P /usr/bin rm -rf build",
        "rm >out -rf build",
    ] {
        assert_eq!(recursive_force_delete(command), Some(true), "{command}");
    }
    for command in ["rm -r build", "rm -f file", "echo rm -rf", "ls -rf"] {
        assert_eq!(recursive_force_delete(command), Some(false), "{command}");
    }
    assert_eq!(recursive_force_delete("rm -rf 'build"), None);
    assert_eq!(recursive_force_delete("cat file\nrm -rf build"), Some(true));
    assert_eq!(
        recursive_force_delete("cat file & rm -rf build"),
        Some(true)
    );
    assert_eq!(recursive_force_delete("echo $(rm -rf build)"), None);
    assert_eq!(recursive_force_delete("echo `rm -rf build`"), None);
    for command in [
        "! rm -rf build",
        "time rm -rf build",
        "exec rm -rf build",
        "builtin eval 'rm -rf build'",
        "coproc rm -rf build",
        "repeat 2 rm -rf build",
        "noglob rm -rf build",
        "sudo --unsupported root rm -rf build",
        "env --unsupported NAME rm -rf build",
        "rm 2>out -rf build",
        "rm &>out -rf build",
        "rm &>>out -rf build",
    ] {
        assert_eq!(recursive_force_delete(command), None, "{command}");
    }
}

#[test]
fn dynamic_shell_arguments_never_prove_a_destructive_classification() {
    for command in [
        "rm $FLAGS build",
        "rm -rf $TARGET",
        "rm -rf \"$TARGET\"",
        "rm -rf ~/build",
        "rm -rf build*",
        "rm -rf build?",
        "rm -rf [ab]uild",
        "rm -rf {build,cache}",
    ] {
        assert_eq!(recursive_force_delete(command), None, "{command}");
    }

    assert_eq!(recursive_force_delete("rm -rf '$TARGET'"), Some(true));
    assert_eq!(recursive_force_delete("rm -rf \\$TARGET"), Some(true));
}

#[test]
fn unsupported_shell_structure_never_becomes_safety_evidence() {
    for command in [
        "bash -c \"bash -c 'bash -c \\\"bash -c \\\'bash -c \\\\\\\"rm -rf build\\\\\\\"\\\'\\\"'\"",
        "rm -rf build <<EOF",
        "rm 2 >out -rf build",
        "(rm -rf build)",
    ] {
        assert_eq!(recursive_force_delete(command), None, "{command}");
    }
    assert_eq!(recursive_force_delete("rm -rf build | cat"), Some(true));
    assert_eq!(
        recursive_force_delete("printf safe || rm -rf build"),
        Some(true)
    );
    assert_eq!(
        recursive_force_delete("printf 'a|b' && rm -rf build"),
        Some(true)
    );
}
