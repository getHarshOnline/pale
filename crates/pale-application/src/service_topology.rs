//! Check a set of service definitions and put them in one dependable order, so the
//! native action and the resident supervisor agree.
//!
//! `actions` and `resident` build on it; it builds on no other module in this
//! crate.
//!
//! [`normalize_service_definitions`] is the single owner of the structural law
//! consumed by both the native normalization action and resident supervision.
//! For service set `S`, dependency edges `D`, declared ports `P`, and total
//! validated token/argument bytes `T`, the rendered law states the admission
//! bound under the bounded initial-v1 identity comparison cost. It returns one
//! deterministic precedence/name order and performs no executable resolution,
//! process action, readiness probe, provider admission, or native effect.

#![doc = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/docs/math/service-topology-complexity.display.mathml"
))]
use std::collections::{BTreeMap, BTreeSet};

use pale_core::records::{
    ServiceDefinition, ServiceDefinitionInput, ServiceDefinitionOutput,
    SERVICE_DEFINITION_MAX_ARGUMENTS_V1, SERVICE_DEFINITION_MAX_ARGUMENT_BYTES_V1,
    SERVICE_DEFINITION_MAX_DEPENDENCIES_V1, SERVICE_DEFINITION_MAX_KIND_BYTES_V1,
    SERVICE_DEFINITION_MAX_NAME_BYTES_V1, SERVICE_DEFINITION_MAX_PORTS_V1,
    SERVICE_DEFINITION_MAX_PROGRAM_BYTES_V1, SERVICE_DEFINITION_MAX_SERVICES_V1,
};

/// Typed reason a service topology cannot reach canonical admission.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ServiceTopologyError {
    /// The complete service-set size is outside the bounded initial-v1 domain.
    #[error("service topology contains {observed} definitions; expected 1..={maximum}")]
    ServiceCount {
        /// Number of supplied definitions.
        observed: usize,
        /// Maximum admitted definitions.
        maximum: usize,
    },
    /// A service name violates the shared lowercase identifier grammar.
    #[error("service name `{name}` must be a lowercase hyphenated identifier")]
    InvalidServiceName {
        /// Rejected name.
        name: String,
    },
    /// A bounded textual launch field is empty, padded, controlled, or too large.
    #[error(
        "service `{service}` {field} must contain 1..={maximum_bytes} control-free bytes with no surrounding whitespace"
    )]
    InvalidToken {
        /// Service owning the field.
        service: String,
        /// Stable field class.
        field: &'static str,
        /// Maximum admitted UTF-8 byte length.
        maximum_bytes: usize,
    },
    /// A service declares more process arguments than the initial-v1 bound.
    #[error("service `{service}` exceeds the {maximum}-argument bound")]
    ArgumentCount {
        /// Service owning the arguments.
        service: String,
        /// Maximum admitted arguments.
        maximum: usize,
    },
    /// One launch argument contains control text or exceeds the byte bound.
    #[error("service `{service}` has an invalid launch argument at index {index}")]
    InvalidArgument {
        /// Service owning the argument.
        service: String,
        /// Zero-based argument position.
        index: usize,
    },
    /// A service exceeds its bounded port inventory.
    #[error("service `{service}` exceeds the {maximum}-port bound")]
    PortCount {
        /// Service owning the ports.
        service: String,
        /// Maximum admitted ports.
        maximum: usize,
    },
    /// A service exceeds its bounded dependency inventory.
    #[error("service `{service}` exceeds the {maximum}-dependency bound")]
    DependencyCount {
        /// Service owning the dependencies.
        service: String,
        /// Maximum admitted dependencies.
        maximum: usize,
    },
    /// Two declarations use the same service identity.
    #[error("duplicate service name `{name}`")]
    DuplicateService {
        /// Repeated service identity.
        name: String,
    },
    /// A service repeats a port or declares port zero.
    #[error("service `{service}` has zero or duplicate port `{port}`")]
    InvalidPort {
        /// Service owning the invalid port.
        service: String,
        /// Invalid port number.
        port: u16,
    },
    /// Two services claim one global port.
    #[error("port `{port}` is claimed by both `{first}` and `{second}`")]
    PortCollision {
        /// Contended port.
        port: u16,
        /// First canonical claimant.
        first: String,
        /// Later claimant.
        second: String,
    },
    /// One service repeats the same dependency identity.
    #[error("service `{service}` repeats dependency `{dependency}`")]
    DuplicateDependency {
        /// Service owning the repeated edge.
        service: String,
        /// Repeated dependency.
        dependency: String,
    },
    /// One dependency target is absent from the complete service set.
    #[error("service `{service}` references unknown dependency `{dependency}`")]
    UnknownDependency {
        /// Service owning the edge.
        service: String,
        /// Missing dependency.
        dependency: String,
    },
    /// A dependency does not precede its dependent service.
    #[error("service `{service}` dependency `{dependency}` must have lower precedence")]
    InvalidDependencyOrder {
        /// Service owning the edge.
        service: String,
        /// Backward or self dependency.
        dependency: String,
    },
}

impl ServiceTopologyError {
    /// Whether this error belongs to the duplicate-identity action category.
    pub(crate) const fn is_duplicate(&self) -> bool {
        matches!(
            self,
            Self::DuplicateService { .. }
                | Self::PortCollision { .. }
                | Self::DuplicateDependency { .. }
        )
    }
}

/// Validate and canonically order one complete provider-neutral service set.
///
/// Dependencies must name declared lower-precedence services. Ports are unique
/// across the complete set. Each service's ports and dependencies are sorted,
/// then services are sorted by `(precedence, name)`. The input is borrowed and
/// never mutated.
///
/// # Errors
///
/// Returns [`ServiceTopologyError`] at the first structural contradiction.
/// Independent runtime readiness, process identity, and owner availability are
/// deliberately outside this normalization boundary.
pub fn normalize_service_definitions(
    input: &ServiceDefinitionInput,
) -> Result<ServiceDefinitionOutput, ServiceTopologyError> {
    normalize_owned_service_definitions(input.clone())
}

/// Validate and canonically order one owned provider-neutral service set.
///
/// This consuming path shares the exact admission owner with
/// [`normalize_service_definitions`] while allowing compiler phases that already
/// own their input to avoid cloning complete launch and dependency payloads.
pub(crate) fn normalize_owned_service_definitions(
    input: ServiceDefinitionInput,
) -> Result<ServiceDefinitionOutput, ServiceTopologyError> {
    if input.services.is_empty() || input.services.len() > SERVICE_DEFINITION_MAX_SERVICES_V1 {
        return Err(ServiceTopologyError::ServiceCount {
            observed: input.services.len(),
            maximum: SERVICE_DEFINITION_MAX_SERVICES_V1,
        });
    }

    {
        let mut names = BTreeMap::new();
        let mut ports = BTreeMap::new();
        for service in &input.services {
            admit_definition(service, &mut names, &mut ports)?;
        }
        for service in &input.services {
            admit_dependencies(service, &names)?;
        }
    }
    let mut normalized = input.services;
    for service in &mut normalized {
        service.depends_on.sort();
        service.ports.sort_unstable();
    }
    normalized.sort_by(|left, right| {
        left.precedence
            .cmp(&right.precedence)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(ServiceDefinitionOutput {
        services: normalized,
    })
}

/// Admit one definition into the shared name and port namespaces.
fn admit_definition<'a>(
    service: &'a ServiceDefinition,
    names: &mut BTreeMap<&'a str, u32>,
    ports: &mut BTreeMap<u16, &'a str>,
) -> Result<(), ServiceTopologyError> {
    if !valid_service_name(&service.name) {
        return Err(ServiceTopologyError::InvalidServiceName {
            name: service.name.clone(),
        });
    }
    require_token(
        service,
        "kind",
        &service.kind,
        SERVICE_DEFINITION_MAX_KIND_BYTES_V1,
    )?;
    require_token(
        service,
        "launch program",
        &service.launch.program,
        SERVICE_DEFINITION_MAX_PROGRAM_BYTES_V1,
    )?;
    if service.launch.args.len() > SERVICE_DEFINITION_MAX_ARGUMENTS_V1 {
        return Err(ServiceTopologyError::ArgumentCount {
            service: service.name.clone(),
            maximum: SERVICE_DEFINITION_MAX_ARGUMENTS_V1,
        });
    }
    if let Some(index) = service
        .launch
        .args
        .iter()
        .position(|argument| invalid_argument(argument))
    {
        return Err(ServiceTopologyError::InvalidArgument {
            service: service.name.clone(),
            index,
        });
    }
    if service.ports.len() > SERVICE_DEFINITION_MAX_PORTS_V1 {
        return Err(ServiceTopologyError::PortCount {
            service: service.name.clone(),
            maximum: SERVICE_DEFINITION_MAX_PORTS_V1,
        });
    }
    if service.depends_on.len() > SERVICE_DEFINITION_MAX_DEPENDENCIES_V1 {
        return Err(ServiceTopologyError::DependencyCount {
            service: service.name.clone(),
            maximum: SERVICE_DEFINITION_MAX_DEPENDENCIES_V1,
        });
    }
    if names
        .insert(service.name.as_str(), service.precedence)
        .is_some()
    {
        return Err(ServiceTopologyError::DuplicateService {
            name: service.name.clone(),
        });
    }

    let mut local_ports = BTreeSet::new();
    for &port in &service.ports {
        if port == 0 || !local_ports.insert(port) {
            return Err(ServiceTopologyError::InvalidPort {
                service: service.name.clone(),
                port,
            });
        }
        if let Some(first) = ports.insert(port, service.name.as_str()) {
            return Err(ServiceTopologyError::PortCollision {
                port,
                first: first.to_owned(),
                second: service.name.clone(),
            });
        }
    }
    Ok(())
}

/// Validate every dependency edge owned by one service.
fn admit_dependencies(
    service: &ServiceDefinition,
    names: &BTreeMap<&str, u32>,
) -> Result<(), ServiceTopologyError> {
    let mut dependencies = BTreeSet::new();
    for dependency in &service.depends_on {
        if !dependencies.insert(dependency.as_str()) {
            return Err(ServiceTopologyError::DuplicateDependency {
                service: service.name.clone(),
                dependency: dependency.clone(),
            });
        }
        let dependency_precedence = names.get(dependency.as_str()).ok_or_else(|| {
            ServiceTopologyError::UnknownDependency {
                service: service.name.clone(),
                dependency: dependency.clone(),
            }
        })?;
        if dependency == &service.name || dependency_precedence >= &service.precedence {
            return Err(ServiceTopologyError::InvalidDependencyOrder {
                service: service.name.clone(),
                dependency: dependency.clone(),
            });
        }
    }
    Ok(())
}

/// Whether one service identity obeys the shared lexical contract.
fn valid_service_name(value: &str) -> bool {
    let mut bytes = value.bytes();
    value.len() <= SERVICE_DEFINITION_MAX_NAME_BYTES_V1
        && matches!(bytes.next(), Some(b'a'..=b'z'))
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

/// Validate one bounded token while retaining its service owner.
fn require_token(
    service: &ServiceDefinition,
    field: &'static str,
    value: &str,
    maximum_bytes: usize,
) -> Result<(), ServiceTopologyError> {
    if value.is_empty()
        || value.len() > maximum_bytes
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(ServiceTopologyError::InvalidToken {
            service: service.name.clone(),
            field,
            maximum_bytes,
        });
    }
    Ok(())
}

/// Whether one literal launch argument violates the shared byte/text bound.
fn invalid_argument(argument: &str) -> bool {
    argument.len() > SERVICE_DEFINITION_MAX_ARGUMENT_BYTES_V1
        || argument.chars().any(char::is_control)
}

#[cfg(test)]
#[path = "../tests/internal/service_topology.rs"]
mod tests;
