# A2A transport adapter

Send one prepared JSON-RPC request over verified HTTPS and read bounded A2A evidence. The first carrier supports text messages, task reads and cancellation requests. It does not coordinate operations or grant authority to dispatch.

The application calls `prepare` before durable dispatch custody. This returns the exact encoded JSON body and its digest. After custody succeeds, it calls `start`, which verifies the body against the request and sends those unchanged bytes. The route, request context, exact limits and TLS trust input remain separately bound. The body is not an HTTP or TLS transcript.

One native service owns the request queue, executor and bounded resolver jobs. Each exchange has byte, event, queue and elapsed-time limits. The total deadline includes queue residence and waits for a slow consumer. Redirects, proxies, automatic retries, connection reuse and compression are disabled.

Local cancellation stops HTTP observation. It does not send `CancelTask` or prove remote cancellation. Shutdown rejects new work and reports draining until physical resolver jobs return. Dropping the service does not block the caller, but repeated construction and abandonment is not resource-bounded. A single retained SDK host lifetime still needs qualification before public eligibility.

Response facts remain untrusted. Raw error text stays in confined evidence, not diagnostics. Task state never becomes a Pale result, effect or settlement. Binary/URL/data parts and metadata/extensions are rejected in the selected response profile. Structured input and client credentials are not implemented.
