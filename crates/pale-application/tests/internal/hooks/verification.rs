//! Repository-verifier proofs for exact command identity and native host evidence.

use super::*;
use crate::terminal_adapters::TerminalAdapterDescriptor;
use pale_core::records::{CapabilityIdV1, HostCommandConstraint};

fn verifier() -> RepositoryVerifierBinding {
    RepositoryVerifierBinding {
        capability_id: CapabilityIdV1::parse("project-test").unwrap(),
        tool_name: "Bash".into(),
        command: HostCommandConstraint {
            executable: "cargo".into(),
            argv_prefix: vec!["test".into(), "--locked".into()],
            allow_trailing_args: false,
        },
    }
}

#[test]
fn verifier_requires_exact_command_and_host_proof() {
    let bindings = [verifier()];
    let claude_response = serde_json::json!({
        "stdout": "ok",
        "stderr": "",
        "interrupted": false,
        "isImage": false
    });
    assert_eq!(
        classify_repository_verifier(
            TerminalAdapterDescriptor::claude_code(),
            &bindings,
            Some("Bash"),
            Some("cargo test --locked"),
            Some(&claude_response),
        ),
        VerificationEvidence::ProvenSuccess
    );
    assert_eq!(
        classify_repository_verifier(
            TerminalAdapterDescriptor::codex(),
            &bindings,
            Some("Bash"),
            Some("cargo test --locked"),
            Some(&serde_json::json!("Process exited with code 0")),
        ),
        VerificationEvidence::CompletedWithoutStatus,
        "Codex output text cannot forge an exit status"
    );
    assert_eq!(
        classify_repository_verifier(
            TerminalAdapterDescriptor::claude_code(),
            &bindings,
            Some("Bash"),
            Some("cargo test --locked"),
            None,
        ),
        VerificationEvidence::CompletedWithoutStatus
    );
    assert_eq!(
        classify_repository_verifier(
            TerminalAdapterDescriptor::claude_code(),
            &bindings,
            Some("Bash"),
            Some("cargo test --locked"),
            Some(&serde_json::json!({"interrupted": false})),
        ),
        VerificationEvidence::CompletedWithoutStatus,
        "an incomplete host response cannot prove success"
    );
    for (tool, command) in [
        (Some("shell"), Some("cargo test --locked")),
        (Some("Bash"), Some("cargo test")),
        (Some("Bash"), Some("cargo test --locked --all")),
        (Some("Bash"), Some("cargo test --locked; touch forged")),
        (None, Some("cargo test --locked")),
    ] {
        assert_eq!(
            classify_repository_verifier(
                TerminalAdapterDescriptor::claude_code(),
                &bindings,
                tool,
                command,
                Some(&claude_response),
            ),
            VerificationEvidence::Irrelevant
        );
    }
}
