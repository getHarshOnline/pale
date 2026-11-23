+++
description = "Write and review Python code and its packaging."
uses = [{ scope = "local", resource_id = "change" }]
official_sources = [{ label = "Python documentation", url = "https://docs.python.org/3/" }]

[guide]

[role]
extends = [{ scope = "local", resource_id = "review" }]
applies_to = [{ scope = "local", resource_id = "python" }]

[stack]
role_uses = [{ scope = "local", resource_id = "python" }]
+++

# Write Python with explicit inputs and cleanup

Follow the repository's formatter, import and lint rules. Prefer immutable data where practical, including tuples and frozen dataclasses, and avoid mutable default arguments. Type public interfaces and structured data, then use the project's type checker to check that callers fit those interfaces.

## Keep data and resources understandable

Use protocols and structural behaviour where a concrete base class adds no useful contract. Keep functions small and composable. Dataclasses suit clear records, comprehensions suit readable transformations over a known collection, and generators help iterate without retaining the whole input. Use context managers for files, locks, connections and resources that need cleanup on every path.

Raise specific exceptions with useful context and catch them where recovery is meaningful. Avoid bare catches, silent failure and surprising work at import time. Inspect deserialization, subprocess arguments, import effects and dependency injection where dynamic values enter the system.

## Test the public behaviour

Use parameter sets for important input classes and narrowly scoped setup for resources. Replace clocks, networks, filesystems and other outside systems where the code calls them. Keep the behaviour under test real; a test that mocks all internal cooperation may only confirm its stubs.

Distinguish fast isolated checks from those that need an environment, including asynchronous cleanup and failure cases. Use coverage to locate meaningful gaps. In the reply, identify which kinds of check ran and any environment-dependent behaviour left untested.

## Parse an external record

Validate an external JSON value's fields before returning a typed record. Open the source file with a context manager. Test a valid record and a missing field, and distinguish a parse error from failure to access the file.

## When the application uses FastAPI

Construct the application in a factory such as `create_app()` so tests can create isolated instances. Keep routes focused on HTTP input and output, with business behaviour and persistence in services. Supply database sessions and the authenticated user through dependencies; creating sessions or long-lived clients inside a route hides their lifetime and makes substitution and cleanup harder. Do not call blocking database or HTTP clients directly on the event loop. Use asynchronous clients, synchronous routes or dependencies that FastAPI runs in a thread pool, or explicit thread offloading. A synchronous helper called directly from an asynchronous route is not automatically offloaded. See FastAPI's [execution rules](https://fastapi.tiangolo.com/async/#very-technical-details).

Define request, partial-update and response models separately. Their fields serve different purposes: a field accepted during registration need not be editable later or visible in a response. Use `response_model` for application data and model field constraints where they express the validation. Exclude passwords, password hashes and internal authentication state from response models. Keep access and refresh tokens out of ordinary data and profile responses; model an authentication endpoint's intended token response explicitly, as in FastAPI's [token-issuance example](https://fastapi.tiangolo.com/tutorial/security/oauth2-jwt/). For example, create a synthetic user with a password hash, fetch the user's public profile and inspect the actual response keys. A successful status with the hash still present is a disclosure, even if the ordinary client never displays it.

Keep database and authentication replacements tied to the exact callable passed to `Depends`. A test override for a similarly named wrapper may leave the real dependency active. Set overrides on the isolated application, exercise the route with an appropriate asynchronous client, and clear `app.dependency_overrides` during cleanup even if the test fails. Check that the next test uses its intended dependency rather than a previous test's identity or database.

When the service uses JWTs, verify expiry, issuer, audience and the permitted signing algorithm. An otherwise valid token for another service must fail this service's authentication check. Configure CORS for the environment's intended origins and apply the [credentialed browser request checks](security.md#check-stack-specific-protections). Check rate limits for authentication and write-heavy routes, and keep credentials, cookies, authorization headers and tokens out of logs.

## When reviewing as a specialist

Add Python-specific risks to the shared review method. Look for:

- untrusted data reaching queries, shells, paths, dynamic evaluation, deserialization or unsafe YAML parsing, and weak cryptography used for a security guarantee;
- broad catches or fallback values creating false success, lost exception context, leaked resources or corrupted state;
- mutable defaults or shared state leaking across calls, incorrectly mixed synchronous and asynchronous work, and unsynchronized concurrent writes;
- `Any`, casts or schema translations hiding a required contract or dropping data; and
- observed N+1 I/O, blocking work in an asynchronous path, or measured repeated allocation and computation.

Type coverage, formatter preferences and Pythonic style alone do not establish a defect. Ask the requesting task for the relevant configured type, lint, security and test results.
