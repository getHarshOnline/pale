//! Verifies process-port diagnostics redact captured bytes while preserving typed operating-system sources.

use super::*;

#[test]
fn probe_diagnostics_redact_captured_bytes() {
    let observation = ProbeObservation {
        stdout: b"private-output".to_vec(),
    };
    assert!(!format!("{observation:?}").contains("private-output"));

    let error = ProcessProbeError::NonZero {
        code: Some(7),
        stderr: ": private-error".to_string(),
    };
    assert!(!format!("{error:?}").contains("private-error"));
    assert!(!error.to_string().contains("private-error"));
    assert!(matches!(
        error,
        ProcessProbeError::NonZero { ref stderr, .. } if stderr.contains("private-error")
    ));
}

#[test]
fn operating_system_failures_remain_typed_sources() {
    let termination =
        ProcessTerminationUncertainty::both_unproven(vec![ProcessTerminationIssue::new(
            ProcessTerminationStage::ForcedGroupSignal,
            io::Error::from(io::ErrorKind::PermissionDenied),
        )]);
    let failures = [
        ProcessProbeError::Spawn(io::Error::from(io::ErrorKind::NotFound)),
        ProcessProbeError::Wait(io::Error::from(io::ErrorKind::Interrupted)),
        ProcessProbeError::Capture {
            stream: ProbeStream::Stdout,
            source: io::Error::from(io::ErrorKind::BrokenPipe),
        },
        ProcessProbeError::Terminate(termination),
    ];

    for failure in &failures[..3] {
        let source = std::error::Error::source(failure)
            .and_then(|source| source.downcast_ref::<io::Error>());
        assert!(source.is_some(), "typed source missing from {failure:?}");
    }
    let ProcessProbeError::Terminate(termination) = &failures[3] else {
        panic!("fourth failure lost its termination type");
    };
    assert_eq!(
        termination.issues()[0].stage(),
        ProcessTerminationStage::ForcedGroupSignal
    );
    assert_eq!(
        termination.issues()[0].source_error().kind(),
        io::ErrorKind::PermissionDenied
    );
}

#[test]
fn supervised_process_debug_output_redacts_literal_invocation_data() {
    let digest = pale_core::records::ExecutableDigest::hash_bytes(b"executable");
    let arguments = ["private-argument"];
    let environment = [ProcessEnvironmentEntry {
        name: "TOKEN",
        value: "private-value",
    }];
    let request = SupervisedProcessRequest {
        executable: "/absolute/executable",
        expected_digest: &digest,
        arguments: &arguments,
        environment: &environment,
        fallback_termination: ProcessTerminationPolicy::immediate(Duration::from_secs(1)),
    };

    let debug = format!("{request:?}");
    assert!(!debug.contains("private-argument"));
    assert!(!debug.contains("private-value"));
    assert!(debug.contains("argument_count: 1"));
    assert!(debug.contains("environment_entry_count: 1"));
    assert!(!format!("{:?}", environment[0]).contains("private-value"));
}
