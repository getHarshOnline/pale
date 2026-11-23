+++
description = 'Review a Spring Framework or Spring Boot service using its actual dependencies and configuration.'
uses = [{ scope = "local", resource_id = "java" }]
official_sources = [{ label = 'Spring Framework reference; resolve the consuming version', url = 'https://docs.spring.io/spring-framework/reference/' }, { label = 'Spring Boot reference; resolve the consuming version', url = 'https://docs.spring.io/spring-boot/reference/' }]

[guide]

[stack]
extends = [{ scope = "local", resource_id = "java" }]
role_uses = [{ scope = "local", resource_id = "spring" }]
+++

# Check the Spring service that actually runs

Apply this guidance when Spring is selected, after the Java and common methods. Identify the repository's Framework and Boot versions, dependencies, profiles, configuration sources and native checks. Check which declared behaviour is active in the running service.

## Resolve dependencies and configuration

Use constructor dependencies for required collaborators. Define what happens when an optional or gated bean is absent, ambiguous or disabled. Choose the provider or optional mechanism by how it resolves those cases. Inspect generated constructors and annotation-copy settings to see which annotations reach the injection point. A lazy dependency can postpone failure while leaving a responsibility cycle unresolved.

Read effective property-source precedence and binding. Distinguish an absent value, an explicitly empty value and a valid default: a higher-priority empty binding can hide a lower-level default. Keep the default and its validation together. Check actual conditions, bean scopes and lifecycle before concluding that a module or scheduler is disabled; profile names and status metadata alone do not determine the active object graph.

## Follow input validation and errors

Trace deserialization, binding, validation groups, controller invocation and exception translation. Check runtime input even where static nullness annotations exist. Follow the selected MVC argument, method-validation or proxy route and inspect its error response, using the project's actual version. Preserve meaningful missing and failure states, protect client responses and retain the diagnostics needed to investigate the cause.

## Follow proxy calls and transactions

Trace interception through actual calls, including self-invocation, visibility and generated proxies. Establish the transaction manager, propagation, rollback rules, thread or reactive context and persistence operations. Check that the call reaches the required transaction; an annotation or bulk-update marker alone does not establish it. Multiple stores and outside requests need their own consistency and recovery arrangement. Limit locks and network work accordingly.

For a changed dependency, configuration, validation or transaction path, exercise the relevant bean states, invalid requests, proxy and direct calls, and committed or rolled-back data. In the reply, distinguish compilation, context startup, real persistence and outside integration results. Use the repository's commands, test-writing rules, entity conventions and business requirements.

## Check self-invocation

A method calls another method on the same bean expecting a new transaction. Trace whether the call crosses the proxy. Exercise an exception and read back stored state. Put the transactional call at the owning interface if the application requires that separation.

## Versioned references

- [Spring Framework proxying, 6.2](https://docs.spring.io/spring-framework/reference/6.2/core/aop/proxying.html)
- [Spring transaction annotations, 6.2](https://docs.spring.io/spring-framework/reference/6.2/data-access/transaction/declarative/annotations.html)
- [Spring MVC validation, 6.2](https://docs.spring.io/spring-framework/reference/6.2/web/webmvc/mvc-controller/ann-validation.html)
- [Spring Boot property binding, 3.5](https://docs.spring.io/spring-boot/3.5/reference/features/external-config.html)

These describe the examined version families. Check the consuming project's selected version where behaviour differs.
