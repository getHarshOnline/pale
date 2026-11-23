//! Direct locks for the shared service-topology law.

use pale_core::records::{ServiceDefinition, ServiceDefinitionInput, ServiceLaunch};

use super::*;

fn service(name: &str, precedence: u32, ports: &[u16], dependencies: &[&str]) -> ServiceDefinition {
    ServiceDefinition {
        name: name.to_owned(),
        kind: "worker".to_owned(),
        precedence,
        launch: ServiceLaunch {
            program: "runner".to_owned(),
            args: vec![name.to_owned()],
        },
        ports: ports.to_vec(),
        depends_on: dependencies
            .iter()
            .map(|dependency| (*dependency).to_owned())
            .collect(),
    }
}

#[test]
fn normalization_is_deterministic_and_canonicalizes_local_axes() {
    let forward = ServiceDefinitionInput {
        services: vec![
            service("web", 20, &[8081, 8080], &["cache", "api"]),
            service("api", 10, &[4000], &[]),
            service("cache", 10, &[6379], &[]),
        ],
    };
    let mut reverse = forward.clone();
    reverse.services.reverse();

    let forward = normalize_service_definitions(&forward).unwrap();
    let reverse = normalize_service_definitions(&reverse).unwrap();
    assert_eq!(forward, reverse);
    assert_eq!(
        forward
            .services
            .iter()
            .map(|definition| definition.name.as_str())
            .collect::<Vec<_>>(),
        ["api", "cache", "web"]
    );
    assert_eq!(forward.services[2].depends_on, ["api", "cache"]);
    assert_eq!(forward.services[2].ports, [8080, 8081]);
}

#[test]
fn contradictions_retain_typed_causes() {
    let unknown = ServiceDefinitionInput {
        services: vec![service("web", 20, &[], &["api"])],
    };
    assert!(matches!(
        normalize_service_definitions(&unknown),
        Err(ServiceTopologyError::UnknownDependency { dependency, .. }) if dependency == "api"
    ));

    let collision = ServiceDefinitionInput {
        services: vec![
            service("api", 10, &[4000], &[]),
            service("web", 20, &[4000], &["api"]),
        ],
    };
    assert!(matches!(
        normalize_service_definitions(&collision),
        Err(ServiceTopologyError::PortCollision { port: 4000, .. })
    ));
}

#[test]
fn multibyte_text_is_bounded_by_utf8_bytes_after_schema_shape_admission() {
    let mut multibyte_kind = service("worker", 10, &[], &[]);
    multibyte_kind.kind = "é".repeat(SERVICE_DEFINITION_MAX_KIND_BYTES_V1);
    assert!(matches!(
        normalize_service_definitions(&ServiceDefinitionInput {
            services: vec![multibyte_kind],
        }),
        Err(ServiceTopologyError::InvalidToken { field: "kind", .. })
    ));

    let mut multibyte_program = service("worker", 10, &[], &[]);
    multibyte_program.launch.program = "é".repeat(SERVICE_DEFINITION_MAX_PROGRAM_BYTES_V1);
    assert!(matches!(
        normalize_service_definitions(&ServiceDefinitionInput {
            services: vec![multibyte_program],
        }),
        Err(ServiceTopologyError::InvalidToken {
            field: "launch program",
            ..
        })
    ));

    let mut multibyte_argument = service("worker", 10, &[], &[]);
    multibyte_argument.launch.args = vec!["é".repeat(SERVICE_DEFINITION_MAX_ARGUMENT_BYTES_V1)];
    assert!(matches!(
        normalize_service_definitions(&ServiceDefinitionInput {
            services: vec![multibyte_argument],
        }),
        Err(ServiceTopologyError::InvalidArgument { index: 0, .. })
    ));
}
