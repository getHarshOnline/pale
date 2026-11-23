//! Laws for the closed command-line parser surface.

use super::*;

#[test]
fn only_the_attachment_qualified_parser_accepts_resident_syntax() {
    let arguments = [
        "pale",
        "resident",
        "delegate",
        "advance",
        "--operation-handle-record-ref",
        "operations/child",
        "--operation-handle-sha256",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "--signed-request",
        "request.json",
    ];
    let stateless = match Cli::try_parse_from(arguments) {
        Err(error) => error,
        Ok(_) => panic!("the stateless parser accepted attachment-only syntax"),
    };
    assert_eq!(stateless.exit_code(), 2);

    let parsed = AttachedCli::try_parse_from(arguments)
        .expect("the injected resident route has explicit nested syntax");

    let AttachedCmd::Resident {
        operation:
            ResidentCommand::Delegate {
                step: ResidentDelegateCommand::Advance(arguments),
            },
    } = parsed.cmd
    else {
        panic!("resident Delegate advancement parsed as an unrelated command")
    };
    assert_eq!(arguments.operation_handle_record_ref, "operations/child");
    assert_eq!(arguments.signed_request, PathBuf::from("request.json"));
}

#[test]
fn help_describes_only_the_commands_available_on_each_parser() {
    let stateless = match Cli::try_parse_from(["pale", "--help"]) {
        Err(help) => help.to_string(),
        Ok(_) => panic!("Clap did not render stateless help"),
    };
    assert!(!stateless
        .lines()
        .any(|line| line.trim_start().starts_with("resident ")));

    let attached = match AttachedCli::try_parse_from(["pale", "--help"]) {
        Err(help) => help.to_string(),
        Ok(_) => panic!("Clap did not render attachment-qualified help"),
    };
    assert!(attached
        .lines()
        .any(|line| line.trim_start().starts_with("resident ")));

    let resident = match AttachedCli::try_parse_from(["pale", "resident", "--help"]) {
        Err(help) => help.to_string(),
        Ok(_) => panic!("Clap did not render resident help"),
    };
    for description in [
        "Start or advance an Invoke",
        "Start or advance a Consult",
        "Start or advance a Delegate",
    ] {
        assert!(
            resident.contains(description),
            "resident help names each operation"
        );
    }
    assert!(resident.contains("Send Cancel without claiming that the operation is terminal"));
    for stale in [
        "Start or advance an invocation",
        "Start or advance a consultation",
        "Start or advance a delegation",
    ] {
        assert!(!resident.contains(stale));
    }
}

#[test]
fn read_only_attachment_parser_exposes_observe_but_not_operation_commands() {
    let observed = ObservationAttachedCli::try_parse_from([
        "pale",
        "resident",
        "observe",
        "--operation-handle-record-ref",
        "operations/root",
        "--operation-handle-sha256",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "--signed-request",
        "request.json",
    ])
    .expect("read-only attachment accepts Observe");
    assert!(matches!(
        observed.cmd,
        ObservationAttachedCmd::Resident {
            operation: ResidentObservationCommand::Observe(_)
        }
    ));

    for unavailable in ["invoke", "consult", "delegate", "cancel"] {
        let error = match ObservationAttachedCli::try_parse_from([
            "pale",
            "resident",
            unavailable,
            "--help",
        ]) {
            Err(error) => error,
            Ok(_) => panic!("read-only attachment accepted {unavailable}"),
        };
        assert_eq!(error.exit_code(), 2);
    }
}
