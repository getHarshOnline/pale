//! Behavioral coverage from untrusted host events through the mandatory authority boundary.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use pale_adapter_fs::LocalMachineFilesystem;
use pale_application::hooks::{
    deny_event, parse_stdin_event, target_path_facts, HookEvent, PreToolUseFacts, StdinEvent,
    StdinEventFields, TargetPresence, WriteBoundary,
};
use pale_application::terminal_adapters::{for_exact_binding, TerminalAdapterDescriptor};
use pale_core::records::{
    HostToolEffectBinding, ParticipantRefV1, ProviderBindingRefV1, ResponsibilityV1,
};
use pale_ports::{
    MachineFilesystemError, MachineFilesystemOperation, MachineFilesystemPort, MachinePathKind,
    MachinePathObservation,
};

#[path = "support/provider_binding.rs"]
mod provider_binding_fixtures;

// All parser journeys use one explicit input ceiling so failures cannot depend on
// a test-specific implicit default.
const LIMIT: usize = 1 << 20;

struct ScriptedFilesystem {
    present: PathBuf,
    missing: PathBuf,
    dangling: PathBuf,
    denied: PathBuf,
    protected: PathBuf,
    alias: PathBuf,
    alias_child: PathBuf,
    operations: Mutex<Vec<(MachineFilesystemOperation, PathBuf)>>,
}

impl ScriptedFilesystem {
    fn observation(
        canonical_path: PathBuf,
        requested_kind: MachinePathKind,
        canonical_kind: MachinePathKind,
    ) -> Result<MachinePathObservation, MachineFilesystemError> {
        Ok(MachinePathObservation::new(
            canonical_path,
            requested_kind,
            canonical_kind,
            None,
        ))
    }

    fn failure<T>(
        path: &Path,
        operation: MachineFilesystemOperation,
        kind: io::ErrorKind,
    ) -> Result<T, MachineFilesystemError> {
        Err(MachineFilesystemError::new(
            operation,
            path.to_path_buf(),
            io::Error::from(kind),
        ))
    }

    fn record(&self, operation: MachineFilesystemOperation, path: &Path) {
        self.operations
            .lock()
            .expect("operation trace lock")
            .push((operation, path.to_path_buf()));
    }

    fn take_operations(&self) -> Vec<(MachineFilesystemOperation, PathBuf)> {
        std::mem::take(&mut *self.operations.lock().expect("operation trace lock"))
    }
}

impl MachineFilesystemPort for ScriptedFilesystem {
    fn inspect_requested(&self, path: &Path) -> Result<MachinePathKind, MachineFilesystemError> {
        self.record(MachineFilesystemOperation::InspectRequested, path);
        if path == self.present {
            Ok(MachinePathKind::RegularFile)
        } else if path == self.missing || path == self.alias_child {
            Self::failure(
                path,
                MachineFilesystemOperation::InspectRequested,
                io::ErrorKind::NotFound,
            )
        } else if path == self.dangling || path == self.alias {
            Ok(MachinePathKind::Symlink)
        } else if path == self.denied {
            Self::failure(
                path,
                MachineFilesystemOperation::InspectRequested,
                io::ErrorKind::PermissionDenied,
            )
        } else if path == self.protected {
            Ok(MachinePathKind::Directory)
        } else {
            Self::failure(
                path,
                MachineFilesystemOperation::InspectRequested,
                io::ErrorKind::NotFound,
            )
        }
    }

    fn canonicalize(&self, path: &Path) -> Result<PathBuf, MachineFilesystemError> {
        self.record(MachineFilesystemOperation::Canonicalize, path);
        if path == self.dangling {
            Self::failure(
                path,
                MachineFilesystemOperation::Canonicalize,
                io::ErrorKind::NotFound,
            )
        } else if path == self.alias {
            Ok(self.protected.clone())
        } else {
            Ok(path.to_path_buf())
        }
    }

    fn observe(&self, path: &Path) -> Result<MachinePathObservation, MachineFilesystemError> {
        let requested_kind = self.inspect_requested(path)?;
        let canonical_path = self.canonicalize(path)?;
        self.record(
            MachineFilesystemOperation::InspectCanonical,
            &canonical_path,
        );
        let canonical_kind = if canonical_path == self.protected {
            MachinePathKind::Directory
        } else {
            requested_kind
        };
        Self::observation(canonical_path, requested_kind, canonical_kind)
    }
}

fn adapter(plugin_id: &str) -> (TerminalAdapterDescriptor, ProviderBindingRefV1) {
    let mut binding = provider_binding_fixtures::provider_binding(ParticipantRefV1::TerminalLeaf {
        terminal_leaf_id: "terminal-owner".into(),
    });
    binding.responsibility = ResponsibilityV1::TerminalHarnessExecution;
    binding.adapter_plugin.adapter_plugin_id = plugin_id.into();
    binding.binding_identity = binding.recompute_identity().unwrap();
    let reference = ProviderBindingRefV1 {
        provider_binding_id: binding.id.clone(),
        provider_binding_identity: binding.binding_identity.clone(),
    };
    (for_exact_binding(&reference, &binding).unwrap(), reference)
}

fn claude() -> TerminalAdapterDescriptor {
    adapter("pale.adapter.claude-code.v1").0
}

fn codex() -> TerminalAdapterDescriptor {
    adapter("pale.adapter.codex.v1").0
}

fn parsed(bytes: &[u8]) -> StdinEventFields {
    match parse_stdin_event(bytes, LIMIT) {
        StdinEvent::Fields(fields) => *fields,
        other => panic!("expected trusted fields, got {other:?}"),
    }
}

fn denied(reason: Option<String>) -> bool {
    reason.is_some()
}

fn enforce(
    roots: &[String],
    effects: &[HostToolEffectBinding],
    facts: PreToolUseFacts<'_>,
) -> Option<String> {
    WriteBoundary::new(claude(), roots, &[], effects).violation(&LocalMachineFilesystem, facts)
}

#[test]
fn parser_preserves_structured_tool_and_prompt_expansion_facts() {
    let tool = parsed(
        br#"{"hook_event_name":"PostToolUse","cwd":"/work","agent_type":"explore","tool_name":"MultiEdit","tool_input":{"edits":[{"file_path":"/a"},{"file_path":"/b"}]},"tool_response":{"success":true},"stop_hook_active":false}"#,
    );
    assert_eq!(tool.hook_event_name(), Some("PostToolUse"));
    assert_eq!(tool.cwd(), Some("/work"));
    assert_eq!(tool.agent_type(), Some("explore"));
    assert_eq!(tool.structured_write_paths(claude(), &[]), ["/a", "/b"]);
    assert_eq!(
        tool.tool_response(),
        Some(&serde_json::json!({"success": true}))
    );
    assert_eq!(tool.stop_hook_active(), Some(false));

    let expansion = parsed(
        br#"{"hook_event_name":"UserPromptExpansion","session_id":"s1","expansion_type":"slash_command","command_name":"deploy","command_args":"prod","command_source":"project","prompt":"/deploy prod"}"#,
    );
    let (_, provider) = adapter("pale.adapter.claude-code.v1");
    let project = expansion
        .direct_owner_prompt(HookEvent::UserPromptExpansion, &provider, claude())
        .unwrap();
    assert_eq!(project.invocation(), "/deploy prod");
    assert_eq!(expansion.message(), Some("/deploy prod"));

    let forged = parsed(br#"{"command_name":"deploy other","prompt":"/deploy other"}"#);
    assert!(forged
        .direct_owner_prompt(HookEvent::UserPromptExpansion, &provider, claude())
        .is_none());

    for untrusted in [
        br#"{"expansion_type":"mcp_prompt","command_name":"deploy","command_source":"project"}"#
            .as_slice(),
        br#"{"expansion_type":"slash_command","command_name":"deploy","command_source":"plugin"}"#
            .as_slice(),
        br#"{"expansion_type":"slash_command","command_name":"deploy"}"#.as_slice(),
    ] {
        assert!(parsed(untrusted)
            .direct_owner_prompt(HookEvent::UserPromptExpansion, &provider, claude())
            .is_none());
    }
}

#[test]
fn codex_apply_patch_targets_are_complete_and_malformed_documents_fail_closed() {
    let patch = parsed(
        br#"{"hook_event_name":"PreToolUse","tool_name":"apply_patch","tool_input":{"command":"*** Begin Patch\n*** Update File: src/old.rs\n@@\n-old\n+new\n*** Move to: src/new.rs\n*** Add File: docs/note with spaces.md\n+note\n*** Delete File: obsolete.txt\n*** End Patch\n"}}"#,
    );
    assert_eq!(
        patch.structured_write_paths(codex(), &[]),
        [
            "docs/note with spaces.md",
            "obsolete.txt",
            "src/new.rs",
            "src/old.rs"
        ]
    );

    for input in [
        br#"{"tool_name":"apply_patch","tool_input":{"command":"*** Update File: src/lib.rs\n"}}"#.as_slice(),
        br#"{"tool_name":"apply_patch","tool_input":{"command":"*** Begin Patch\n*** End Patch\n"}}"#.as_slice(),
        br#"{"tool_name":"apply_patch","tool_input":{"command":"*** Begin Patch\n*** Add File: \n*** End Patch\n"}}"#.as_slice(),
        br#"{"tool_name":"apply_patch","tool_input":{}}"#.as_slice(),
    ] {
        assert!(matches!(
            parse_stdin_event(input, LIMIT),
            StdinEvent::Malformed(_)
        ));
    }
}

#[test]
fn file_targets_become_write_facts_only_for_write_capable_tools() {
    let read = parsed(br#"{"tool_name":"Read","tool_input":{"file_path":"Cargo.toml"}}"#);
    let write = parsed(br#"{"tool_name":"Write","tool_input":{"file_path":"Cargo.toml"}}"#);

    assert!(read.structured_write_paths(claude(), &[]).is_empty());
    assert_eq!(write.structured_write_paths(claude(), &[]), ["Cargo.toml"]);
}

#[test]
fn malformed_host_boundaries_are_rejected_as_one_class() {
    let oversized = vec![b'x'; LIMIT + 1];
    let cases: Vec<&[u8]> = vec![
        b"{broken",
        br#"{"tool_name":"Read"} trailing"#,
        &[b'{', 0xff, b'}'],
        br#"[]"#,
        br#"{"tool_input":"not-an-object"}"#,
        br#"{"tool_name":3}"#,
        oversized.as_slice(),
    ];
    for input in cases {
        assert!(matches!(
            parse_stdin_event(input, LIMIT),
            StdinEvent::Malformed(_)
        ));
    }
    assert!(matches!(
        parse_stdin_event(b"  \n", LIMIT),
        StdinEvent::Absent
    ));
}

#[test]
fn conflicting_tool_aliases_are_untrusted_and_deny_at_the_write_boundary() {
    let event = parse_stdin_event(
        br#"{"tool_name":"Read","tool":{"name":"Write"},"tool_input":{"file_path":"plan.md"}}"#,
        LIMIT,
    );
    assert!(matches!(event, StdinEvent::Malformed(_)));

    let temp = tempfile::tempdir().unwrap();
    let authority = temp.path().join("authority");
    fs::create_dir_all(&authority).unwrap();
    let roots = vec![fs::canonicalize(authority)
        .unwrap()
        .to_string_lossy()
        .into_owned()];
    assert!(denied(enforce(
        &roots,
        &[],
        PreToolUseFacts {
            malformed: true,
            ..Default::default()
        },
    )));
}

#[test]
fn structured_file_targets_are_all_confined_and_normalized() {
    let temp = tempfile::tempdir().unwrap();
    let readonly = temp.path().join("authority");
    let runtime_state = temp.path().join("runtime-state");
    let writable = temp.path().join("work");
    fs::create_dir_all(&readonly).unwrap();
    fs::create_dir_all(&runtime_state).unwrap();
    fs::create_dir_all(&writable).unwrap();
    let roots = vec![fs::canonicalize(&readonly)
        .unwrap()
        .to_string_lossy()
        .into_owned()];
    let inside = readonly.join("plan.md").to_string_lossy().into_owned();
    let outside = writable.join("code.rs").to_string_lossy().into_owned();
    let state_target = runtime_state
        .join("session.json")
        .to_string_lossy()
        .into_owned();
    let traversal = writable
        .join("../authority/plan.md")
        .to_string_lossy()
        .into_owned();

    assert!(denied(enforce(
        &roots,
        &[],
        PreToolUseFacts {
            tool_name: Some("Write"),
            write_paths: std::slice::from_ref(&inside),
            cwd: Some(writable.to_str().unwrap()),
            ..Default::default()
        },
    )));
    assert!(denied(
        WriteBoundary::new(
            claude(),
            &[],
            &[runtime_state.to_string_lossy().into_owned()],
            &[],
        )
        .violation(
            &LocalMachineFilesystem,
            PreToolUseFacts {
                tool_name: Some("Write"),
                write_paths: &[state_target],
                cwd: Some(writable.to_str().unwrap()),
                ..Default::default()
            }
        )
    ));
    assert!(denied(enforce(
        &roots,
        &[],
        PreToolUseFacts {
            tool_name: Some("MultiEdit"),
            write_paths: &[outside.clone(), traversal],
            cwd: Some(writable.to_str().unwrap()),
            ..Default::default()
        },
    )));
    assert!(enforce(
        &roots,
        &[],
        PreToolUseFacts {
            tool_name: Some("Edit"),
            write_paths: &[outside],
            cwd: Some(writable.to_str().unwrap()),
            ..Default::default()
        },
    )
    .is_none());
    assert!(denied(enforce(
        &roots,
        &[],
        PreToolUseFacts {
            tool_name: Some("Edit"),
            cwd: Some(writable.to_str().unwrap()),
            ..Default::default()
        },
    )));
}

#[test]
fn permission_request_uses_the_same_write_boundary_with_its_native_deny_shape() {
    let temp = tempfile::tempdir().unwrap();
    let readonly = temp.path().join("authority");
    fs::create_dir_all(&readonly).unwrap();
    let roots = vec![fs::canonicalize(&readonly)
        .unwrap()
        .to_string_lossy()
        .into_owned()];
    let target = readonly.join("plan.md").to_string_lossy().into_owned();
    let reason = WriteBoundary::new(claude(), &roots, &[], &[])
        .violation(
            &LocalMachineFilesystem,
            PreToolUseFacts {
                tool_name: Some("Write"),
                write_paths: &[target],
                cwd: temp.path().to_str(),
                ..Default::default()
            },
        )
        .expect("protected permission request must deny");
    let output = deny_event(claude(), HookEvent::PermissionRequest, &reason);
    let wire: serde_json::Value = serde_json::from_str(&output.stdout).unwrap();
    assert_eq!(
        wire["hookSpecificOutput"]["hookEventName"],
        "PermissionRequest"
    );
    assert_eq!(wire["hookSpecificOutput"]["decision"]["behavior"], "deny");
}

#[cfg(unix)]
#[test]
fn write_through_symlinked_parent_is_denied() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let readonly = temp.path().join("authority");
    let writable = temp.path().join("work");
    fs::create_dir_all(&readonly).unwrap();
    fs::create_dir_all(&writable).unwrap();
    symlink(&readonly, writable.join("alias")).unwrap();
    let roots = vec![fs::canonicalize(&readonly)
        .unwrap()
        .to_string_lossy()
        .into_owned()];
    let target = writable.join("alias/new.md").to_string_lossy().into_owned();

    assert!(denied(enforce(
        &roots,
        &[],
        PreToolUseFacts {
            tool_name: Some("Write"),
            write_paths: &[target],
            cwd: Some(writable.to_str().unwrap()),
            ..Default::default()
        },
    )));
}

#[cfg(unix)]
#[test]
fn symlink_resolution_precedes_parent_components_at_the_write_boundary() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let readonly = temp.path().join("authority");
    let nested = readonly.join("nested");
    let writable = temp.path().join("work");
    fs::create_dir_all(&nested).unwrap();
    fs::create_dir_all(&writable).unwrap();
    fs::write(readonly.join("plan.md"), b"retained").unwrap();
    symlink(&nested, writable.join("alias")).unwrap();
    let roots = vec![fs::canonicalize(&readonly)
        .unwrap()
        .to_string_lossy()
        .into_owned()];
    let target = writable
        .join("alias/../plan.md")
        .to_string_lossy()
        .into_owned();

    let existing_reason = enforce(
        &roots,
        &[],
        PreToolUseFacts {
            tool_name: Some("Write"),
            write_paths: &[target],
            cwd: Some(writable.to_str().unwrap()),
            ..Default::default()
        },
    )
    .expect("existing target reached through symlink and parent must deny");
    assert!(existing_reason.contains("a write resolves into sealed read-only path"));

    let missing_below_resolved_parent = writable
        .join("alias/../new.md")
        .to_string_lossy()
        .into_owned();
    let missing_reason = enforce(
        &roots,
        &[],
        PreToolUseFacts {
            tool_name: Some("Write"),
            write_paths: &[missing_below_resolved_parent],
            cwd: Some(writable.to_str().unwrap()),
            ..Default::default()
        },
    )
    .expect("nearest-ancestor resolution must preserve a symlink before `..`");
    assert!(missing_reason.contains("a write resolves into sealed read-only path"));

    let unresolved = writable
        .join("missing/../ordinary.md")
        .to_string_lossy()
        .into_owned();
    let unresolved_reason = enforce(
        &roots,
        &[],
        PreToolUseFacts {
            tool_name: Some("Write"),
            write_paths: &[unresolved],
            cwd: Some(writable.to_str().unwrap()),
            ..Default::default()
        },
    )
    .expect("a missing component before `..` must fail closed");
    assert!(unresolved_reason.contains("cannot be resolved against the sealed read-only roots"));
}

#[cfg(unix)]
#[test]
fn unreadable_target_ancestry_is_denied_as_unknown() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().unwrap();
    let readonly = temp.path().join("authority");
    let inaccessible = temp.path().join("inaccessible");
    fs::create_dir_all(&readonly).unwrap();
    fs::create_dir_all(&inaccessible).unwrap();
    let roots = vec![fs::canonicalize(&readonly)
        .unwrap()
        .to_string_lossy()
        .into_owned()];
    fs::set_permissions(&inaccessible, fs::Permissions::from_mode(0o000)).unwrap();
    let target = inaccessible.join("new.md").to_string_lossy().into_owned();
    let result = enforce(
        &roots,
        &[],
        PreToolUseFacts {
            tool_name: Some("Write"),
            write_paths: &[target],
            cwd: temp.path().to_str(),
            ..Default::default()
        },
    );
    fs::set_permissions(&inaccessible, fs::Permissions::from_mode(0o700)).unwrap();

    assert!(denied(result));
}

#[test]
fn obvious_shell_write_destinations_are_checked_without_blocking_reads() {
    let temp = tempfile::tempdir().unwrap();
    let readonly = temp.path().join("authority");
    let writable = temp.path().join("work");
    fs::create_dir_all(&readonly).unwrap();
    fs::create_dir_all(&writable).unwrap();
    let roots = vec![fs::canonicalize(&readonly)
        .unwrap()
        .to_string_lossy()
        .into_owned()];
    let ro = readonly.to_string_lossy();
    let cwd = writable.to_str().unwrap();
    for command in [
        format!("echo changed > '{ro}/plan.md'"),
        format!("rm '{ro}/plan.md'"),
        format!("cd '{ro}' && touch plan.md"),
        format!("env MODE=test rm '{ro}/plan.md'"),
        format!("sudo -u root rm '{ro}/plan.md'"),
        format!("sudo MODE=checked rm '{ro}/plan.md'"),
        format!("env -u NAME rm '{ro}/plan.md'"),
        format!("rm \\\n'{ro}/plan.md'"),
        format!("rm \\\r\n'{ro}/plan.md'"),
        format!("rm >'{}/output.log' '{ro}/plan.md'", writable.display()),
        format!("sh -c \"rm '{ro}/plan.md'\""),
        "custom-writer ../authority/plan.md".to_string(),
        "custom-writer --output=../authority/plan.md".to_string(),
    ] {
        assert!(
            denied(enforce(
                &roots,
                &[],
                PreToolUseFacts {
                    tool_name: Some("Bash"),
                    cwd: Some(cwd),
                    command: Some(&command),
                    ..Default::default()
                },
            )),
            "must deny {command}"
        );
    }
    let read = format!("cat '{ro}/plan.md'");
    assert!(enforce(
        &roots,
        &[],
        PreToolUseFacts {
            tool_name: Some("Bash"),
            cwd: Some(cwd),
            command: Some(&read),
            ..Default::default()
        },
    )
    .is_none());

    let opaque = format!("python3 -c 'open(\"{ro}/plan.md\", \"w\").write(\"x\")'");
    assert!(denied(enforce(
        &roots,
        &[],
        PreToolUseFacts {
            tool_name: Some("Bash"),
            cwd: Some(cwd),
            command: Some(&opaque),
            ..Default::default()
        },
    )));

    for command in [
        format!("cat harmless\nrm '{ro}/plan.md'"),
        format!("cat harmless & rm '{ro}/plan.md'"),
        "cat $(printf dynamic-path)".to_string(),
        "cat `printf dynamic-path`".to_string(),
        "touch $TARGET".to_string(),
        "touch \"$TARGET\"".to_string(),
        "echo ~".to_string(),
        "echo *.md".to_string(),
        "echo {safe,dynamic}".to_string(),
        format!("! rm '{ro}/plan.md'"),
        format!("time rm '{ro}/plan.md'"),
        format!("exec rm '{ro}/plan.md'"),
        format!("builtin eval \"rm '{ro}/plan.md'\""),
        format!("coproc rm '{ro}/plan.md'"),
        format!("repeat 2 rm '{ro}/plan.md'"),
        format!("noglob rm '{ro}/plan.md'"),
        format!("sudo --unsupported root rm '{ro}/plan.md'"),
        format!("env --unsupported NAME rm '{ro}/plan.md'"),
    ] {
        assert!(denied(enforce(
            &roots,
            &[],
            PreToolUseFacts {
                tool_name: Some("Bash"),
                cwd: Some(cwd),
                command: Some(&command),
                ..Default::default()
            },
        )));
    }
}

#[test]
fn unavailable_effect_facts_fail_closed_but_no_boundary_stays_inert() {
    let temp = tempfile::tempdir().unwrap();
    let readonly = temp.path().join("authority");
    fs::create_dir_all(&readonly).unwrap();
    let roots = vec![fs::canonicalize(&readonly)
        .unwrap()
        .to_string_lossy()
        .into_owned()];

    for (malformed, tool) in [
        (true, None),
        (false, None),
        (false, Some("mcp__filesystem__write_file")),
    ] {
        assert!(denied(enforce(
            &roots,
            &[],
            PreToolUseFacts {
                malformed,
                tool_name: tool,
                cwd: temp.path().to_str(),
                ..Default::default()
            },
        )));
    }
    assert!(enforce(
        &[],
        &[],
        PreToolUseFacts {
            malformed: true,
            ..Default::default()
        },
    )
    .is_none());
}

#[test]
fn native_session_controls_do_not_require_filesystem_effect_bindings() {
    let temp = tempfile::tempdir().unwrap();
    let protected = temp.path().join("authority");
    fs::create_dir_all(&protected).unwrap();
    let roots = vec![fs::canonicalize(&protected)
        .unwrap()
        .to_string_lossy()
        .into_owned()];

    for tool_name in ["Agent", "Skill", "TaskUpdate"] {
        assert!(enforce(
            &roots,
            &[],
            PreToolUseFacts {
                tool_name: Some(tool_name),
                cwd: temp.path().to_str(),
                ..Default::default()
            },
        )
        .is_none());
    }
}

#[test]
fn filesystem_observation_stages_preserve_presence_and_write_boundary_policy() {
    let root = tempfile::tempdir().unwrap();
    let present = root.path().join("present");
    let missing = root.path().join("missing");
    let dangling = root.path().join("dangling");
    let denied = root.path().join("denied");
    let protected = root.path().join("protected");
    let alias = root.path().join("alias");
    let alias_child = alias.join("new.md");
    let filesystem = ScriptedFilesystem {
        present: present.clone(),
        missing: missing.clone(),
        dangling: dangling.clone(),
        denied: denied.clone(),
        protected: protected.clone(),
        alias,
        alias_child: alias_child.clone(),
        operations: Mutex::new(Vec::new()),
    };
    let paths = [&present, &missing, &dangling, &denied]
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect::<Vec<_>>();

    let facts = target_path_facts(&filesystem, &paths, None);
    assert_eq!(
        facts.iter().map(|fact| fact.presence).collect::<Vec<_>>(),
        [
            TargetPresence::Present,
            TargetPresence::Missing,
            TargetPresence::Present,
            TargetPresence::Unknown,
        ]
    );
    assert_eq!(
        filesystem.take_operations(),
        paths
            .iter()
            .map(|path| {
                (
                    MachineFilesystemOperation::InspectRequested,
                    PathBuf::from(path),
                )
            })
            .collect::<Vec<_>>(),
        "presence classification must not canonicalize or inspect a target"
    );

    let protected_roots = [protected.to_string_lossy().into_owned()];
    let target = [alias_child.to_string_lossy().into_owned()];
    assert!(
        WriteBoundary::new(claude(), &protected_roots, &[], &[])
            .violation(
                &filesystem,
                PreToolUseFacts {
                    tool_name: Some("Write"),
                    write_paths: &target,
                    ..Default::default()
                },
            )
            .is_some(),
        "a missing leaf below a symlinked parent must resolve into the protected root"
    );
    assert_eq!(
        filesystem.take_operations(),
        [
            (
                MachineFilesystemOperation::InspectRequested,
                protected.clone(),
            ),
            (MachineFilesystemOperation::Canonicalize, protected.clone(),),
            (
                MachineFilesystemOperation::InspectRequested,
                alias_child.clone(),
            ),
            (
                MachineFilesystemOperation::InspectRequested,
                filesystem.alias.clone(),
            ),
            (
                MachineFilesystemOperation::Canonicalize,
                filesystem.alias.clone(),
            ),
        ],
        "the protected root costs two operations and one missing target leaf costs k + 2"
    );

    let denied_child = denied.join("new.md");
    let target = [denied_child.to_string_lossy().into_owned()];
    assert!(
        WriteBoundary::new(claude(), &protected_roots, &[], &[])
            .violation(
                &filesystem,
                PreToolUseFacts {
                    tool_name: Some("Write"),
                    write_paths: &target,
                    ..Default::default()
                },
            )
            .is_some(),
        "a denied ancestor observation must fail closed"
    );
    assert_eq!(
        filesystem.take_operations(),
        [
            (
                MachineFilesystemOperation::InspectRequested,
                protected.clone(),
            ),
            (MachineFilesystemOperation::Canonicalize, protected.clone(),),
            (MachineFilesystemOperation::InspectRequested, denied_child,),
            (MachineFilesystemOperation::InspectRequested, denied),
        ],
        "permission failure must stop without canonicalization"
    );

    let target = [dangling.to_string_lossy().into_owned()];
    assert!(
        WriteBoundary::new(claude(), &protected_roots, &[], &[])
            .violation(
                &filesystem,
                PreToolUseFacts {
                    tool_name: Some("Write"),
                    write_paths: &target,
                    ..Default::default()
                },
            )
            .is_some(),
        "a dangling symlink must fail closed"
    );
    assert_eq!(
        filesystem.take_operations(),
        [
            (
                MachineFilesystemOperation::InspectRequested,
                protected.clone(),
            ),
            (MachineFilesystemOperation::Canonicalize, protected,),
            (
                MachineFilesystemOperation::InspectRequested,
                dangling.clone(),
            ),
            (MachineFilesystemOperation::Canonicalize, dangling),
        ],
        "a canonicalization failure must stop before target inspection"
    );
}

#[cfg(unix)]
#[test]
fn target_presence_distinguishes_missing_entries_from_symlinks() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let missing = "missing/.eslintrc.js".to_string();
    let link = temp.path().join(".eslintrc.js");
    symlink(temp.path().join("absent-target"), &link).unwrap();
    let paths = vec![missing, link.to_string_lossy().into_owned()];
    let facts = target_path_facts(&LocalMachineFilesystem, &paths, temp.path().to_str());

    assert_eq!(facts[0].presence, TargetPresence::Missing);
    assert_eq!(facts[1].presence, TargetPresence::Present);
}
