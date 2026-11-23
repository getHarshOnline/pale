//! Service-definition proofs for dependency order, port ownership, and cycle rejection.

use pale_core::records::{ServiceDefinition, ServiceLaunch};

use super::*;

fn service(name: &str, precedence: u32, ports: &[u16], deps: &[&str]) -> ServiceDefinition {
    ServiceDefinition {
        name: name.into(),
        kind: "web".into(),
        precedence,
        launch: ServiceLaunch {
            program: "runner".into(),
            args: vec![name.into()],
        },
        ports: ports.to_vec(),
        depends_on: deps.iter().map(|value| (*value).into()).collect(),
    }
}

#[test]
fn definitions_are_dependency_checked_and_canonically_ordered() {
    let input = ServiceDefinitionInput {
        services: vec![
            service("web", 20, &[8080], &["api"]),
            service("api", 10, &[4000], &[]),
        ],
    };
    let output = execute(&input).expect("valid definitions");
    assert_eq!(output.services[0].name, "api");
    assert_eq!(output.services[1].depends_on, vec!["api"]);
}

#[test]
fn port_collisions_and_backward_dependencies_fail_closed() {
    let collision = ServiceDefinitionInput {
        services: vec![
            service("api", 10, &[4000], &[]),
            service("web", 20, &[4000], &[]),
        ],
    };
    assert_eq!(
        execute(&collision).expect_err("port collision").code,
        ActionErrorCode::DuplicateIdentifier
    );
    let backward = ServiceDefinitionInput {
        services: vec![
            service("api", 10, &[], &["web"]),
            service("web", 20, &[], &[]),
        ],
    };
    assert_eq!(
        execute(&backward).expect_err("backward dependency").code,
        ActionErrorCode::MalformedInput
    );
}
