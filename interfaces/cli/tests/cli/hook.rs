//! Process-level fail-closed proof for rendered repository-hook contributions.

use super::*;

enum RenderedHookProcess {
    Claude { program: String, args: Vec<String> },
    Codex { command: String },
}

impl RenderedHookProcess {
    fn label(&self) -> &'static str {
        match self {
            Self::Claude { .. } => "Claude Code exec-form handler",
            Self::Codex { .. } => "Codex shell-form handler",
        }
    }

    fn run(&self, input: &[u8]) -> std::process::Output {
        let mut command = match self {
            Self::Claude { program, args } => {
                let mut command = Command::new(program);
                command.args(args);
                command
            }
            Self::Codex { command } => {
                let mut shell = Command::new("/bin/sh");
                shell.args(["-c", command]);
                shell
            }
        };
        let executable_directory = Path::new(bin()).parent().unwrap();
        let inherited_path = std::env::var_os("PATH").unwrap_or_default();
        let search_path = std::env::join_paths(
            std::iter::once(executable_directory.to_path_buf())
                .chain(std::env::split_paths(&inherited_path)),
        )
        .unwrap();
        let mut child = command
            .env("PATH", search_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        child.wait_with_output().unwrap()
    }
}

fn rendered_pre_tool_use_processes() -> [RenderedHookProcess; 2] {
    let claude: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/pale-application/tests/fixtures/hooks/claude-pre-tool-use-contribution.json"
    )))
    .unwrap();
    let claude = &claude["hooks"]["PreToolUse"][0]["hooks"][0];
    let claude_program = claude["command"].as_str().unwrap().to_owned();
    let claude_args = claude["args"]
        .as_array()
        .unwrap()
        .iter()
        .map(|argument| argument.as_str().unwrap().to_owned())
        .collect();

    let codex: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/pale-application/tests/fixtures/hooks/codex-pre-tool-use-contribution.json"
    )))
    .unwrap();
    let codex = &codex["hooks"]["PreToolUse"][0]["hooks"][0];
    let codex_command = codex["command"].as_str().unwrap().to_owned();

    [
        RenderedHookProcess::Claude {
            program: claude_program,
            args: claude_args,
        },
        RenderedHookProcess::Codex {
            command: codex_command,
        },
    ]
}

enum HookFailureInput {
    Exact(&'static [u8]),
    Oversized,
}

impl HookFailureInput {
    fn bytes(&self) -> Vec<u8> {
        match self {
            Self::Exact(bytes) => bytes.to_vec(),
            Self::Oversized => vec![b' '; pale_sdk::ResidentHookRequest::MAX_BYTES + 1],
        }
    }
}

struct HookFailureCase {
    label: &'static str,
    input: HookFailureInput,
    expected_code: &'static str,
}

fn assert_hook_process_blocks(
    label: &str,
    process: &RenderedHookProcess,
    input: &[u8],
    expected_code: &str,
) {
    let output = process.run(input);
    assert_eq!(
        output.status.code(),
        Some(2),
        "{label} must complete with the native denial status; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty(), "{label} must not report success");
    let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(diagnostic["diagnostic"]["code"], expected_code, "{label}");
}

#[test]
fn rendered_pre_tool_use_failures_return_each_harness_native_denial_status() {
    let cases = [
        HookFailureCase {
            label: "absent native event",
            input: HookFailureInput::Exact(b""),
            expected_code: "resident_hook_payload_absent",
        },
        HookFailureCase {
            label: "malformed native event",
            input: HookFailureInput::Exact(b"{"),
            expected_code: "resident_hook_payload_malformed",
        },
        HookFailureCase {
            label: "oversized native event",
            input: HookFailureInput::Oversized,
            expected_code: "resident_hook_payload_malformed",
        },
        HookFailureCase {
            label: "mismatched native event",
            input: HookFailureInput::Exact(br#"{"hook_event_name":"SessionStart"}"#),
            expected_code: "resident_hook_event_mismatch",
        },
        HookFailureCase {
            label: "unavailable resident carrier",
            input: HookFailureInput::Exact(
                br#"{"hook_event_name":"PreToolUse","session_id":"native-session","tool_name":"Read"}"#,
            ),
            expected_code: "resident_hook_transport_unavailable",
        },
    ];

    for process in rendered_pre_tool_use_processes() {
        for case in &cases {
            let input = case.input.bytes();
            assert_hook_process_blocks(
                &format!("{}: {}", process.label(), case.label),
                &process,
                &input,
                case.expected_code,
            );
        }
    }
}
