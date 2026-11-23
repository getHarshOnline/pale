+++
description = "Trace untrusted input to protected data and actions; check permission, secrets, exports, browser output and the actual failure path."

[guide]

[task]
uses = [{ scope = "local", resource_id = "review" }]
roles = [{ scope = "local", resource_id = "security" }]
outputs = ["In the reply: the vulnerable code or interface, safe reproduction evidence, impact, uncertainty and proposed correction."]

[role]
extends = [{ scope = "local", resource_id = "review" }]
+++

# Trace a security risk to the action it can reach

Review a specific part of the system by following attacker- or environment-controlled input to protected data, decisions and actions. Establish identities, trust relationships, privileges, sensitive data, public entry points, dependencies and deployment assumptions. A finding needs a reachable path, consequence and missing protection.

For each path, check authentication and permission, input size and shape, parsing, normalization, query and command construction, file paths, output encoding, request integrity, replay, rate or resource exhaustion, and information disclosed on failure. Follow protected data through collection, use, logs, traces, caches, artifacts, backups and deletion or expiry. Secrets need a justified runtime source and must stay out of source code, client-visible output, diagnostics and generated evidence.

For dependencies, establish where the exact locked version came from and whether the affected component is reachable. A vulnerability report is a reason to investigate that path in this system.

## Check stack-specific protections

Where the application accepts passwords, follow storage and verification through the configured password-hashing method, such as Argon2 or bcrypt. Plaintext storage or a fast checksum is unsuitable for password verification. Where it accepts sessions or tokens, check their authenticity, expiry and continued validity at the protected action, including after logout or revocation. Using synthetic accounts in a permitted test environment, revoke one session and retry a protected route with it while confirming another valid session still works. A successful login alone leaves that later permission check untested.

Where the service parses XML, inspect the parser's external-entity configuration and whether untrusted input can make it read local files or contact other services. Where browsers send credentials across origins, check the configured origins, methods and credential policy together. Credentialed CORS needs explicit permitted origins; a wildcard or reflection of any supplied origin cannot establish that permission. Test a permitted origin and an untrusted origin, including the preflight and actual response where applicable. CORS does not replace authentication or permission checks on the route.

For a production deployment, inspect the effective configuration for enabled debug behaviour, unchanged default credentials and required security headers. A development configuration file alone does not show what is deployed. For browser output, check context-appropriate escaping and the framework's auto-escaping behaviour alongside the configured Content Security Policy. CSP adds protection when content reaches the browser; it does not make unsafe output construction safe. A finding should explain the reachable disclosure or action and the protection that failed, using the severity and permission judgment below.

## Check protected exports through delivery and cleanup

Follow export creation, data selection, retrieval or streaming, and cleanup separately. Establish the requester and permitted scope from trusted context before reading or writing protected data. Caller-supplied tenant and artifact identifiers select a target; the service must still check permission. Enforce scope at the query or storage layer and again at a later delivery point when permission can change. Keep the dataset version and expected membership with the export's test result.

Connect each artifact to its request or explicitly permitted reuse. Concurrent attempts need their own private files or streams, unless a shared artifact is demonstrably immutable and safe for every permitted reader. One request must not overwrite, append to or remove another request's output. Check temporary-file cleanup, source-record retention and expiry or deletion of other copies separately.

Using synthetic records, overlap requests for the same scope and different scopes. Read back exact membership, multiplicity and delivery status. Exercise an unauthenticated request, an authenticated requester using another scope, and a permitted empty export. Check denied reads and writes, interrupted delivery, and cleanup while another response is in flight. Confirm cleanup touched only its intended artifact and inspect the source rows that remain. Say which application, gateway, storage and retention layers were actually reached.

## Check what reaches the browser

Classify build inputs by who may see or use them: public browser configuration, server credentials and private data such as labels or local paths. Trace the build runner, arguments and environment, client imports, generated files and server runtime configuration. Treat a value delivered to a browser as available to its operator. Even an intentionally public value needs a justified scope and purpose when it resembles a credential.

Build the exact candidate in an isolated environment with unique synthetic markers. Compare builds with and without the tested input. Use a known public marker to confirm the scan reaches emitted bytes, and a separate runtime-only marker to test exclusion. Inventory and hash what is actually delivered. Search HTML, chunks, styles, source maps, manifests, static assets and precompressed copies when present. Check reachable encodings and separately delivered diagnostics. In the test result, name the matched class and file location without including a secret or private value. A clean replacement has its own bytes; check separately whether older artifacts remain available in release storage, caches or history.

Check request permission as a separate behaviour. In the approved environment, follow the browser action through its real route to the server-side change. Exercise a permitted identity, a denied identity and missing credentials or scope. Inspect the request, response and any unauthorised changes. The server should obtain privileged credentials where the protected action occurs. A literal scan leaves generated values, browser permissions and deployed copies to be checked through their own routes. Name the artifact, cases and layers reached.

## Report the reachable failure

Rank findings by achievable conditions, consequence, reach, existing guards, detectability and recovery. Keep an unestablished path as a hypothesis with the missing evidence. Strengthen a report with safe reproduction and source reading; do not expose a secret or demonstrate a real exploit just to make the report more persuasive.

In the review reply, explain the protected asset, trust relationship, reachable path, controls inspected, finding, contrary evidence, uncertainty, defensive change and the check needed. Permission to review does not by itself grant permission to scan, exploit, rotate credentials, patch or disclose.

Check the defensive change against permitted and denied cases. If those checks have not run, identify them as pending in the review reply.

## Revoke access during an export

Use a test project with two accounts and synthetic exports. Confirm each account can open its own export. Revoke one account's project access and try its download route again. Confirm the other account's in-progress export still completes. Inspect the returned file and cleanup separately.

## When reviewing as a specialist

Use this method with the dependency and security results supplied by the requesting task and its permitted read-only source access. Ask that task for missing checks instead of running or inventing a scanner.

Focus on identity, permission, secrets, queries, commands, files, uploads, outside URLs, payments, webhooks, parsing or deserialization, browser output and logs. For each finding, trace attacker-controlled input to an action it can reach, explain why existing guards fail, identify the impact and affected users, and propose the specific defensive change. For an exposed secret, recommend rotation or revocation to the task's owner without reproducing its value.

Example credentials, public keys, fast hashes used only for integrity and defensive patterns need an actual exploit path before becoming findings. Treat an established exploit, exposed credential or missing required permission check as critical; otherwise set severity from reachability and impact. This role assesses the supplied evidence. Scanning, fixing, rotating credentials and checking their effects stay with the requesting task and its permission.

## Audit a permitted target

Use this task when adversarial security analysis is needed beyond a general design review. Establish the exact permitted target, its reachable inputs and ways of gathering evidence, then apply the shared review method and security role. Trace the unsafe assumption to the affected operation. Keep confirmed paths separate from hypotheses and rank findings by the conditions needed to exploit them and their impact.

Keep the audit read-only. In the reply, identify the vulnerable code or interface, reproduction evidence obtained within the permission given, impact, uncertainty, the defensive change needed and a safe way to check it. Exercising a real exploit or changing the target requires its own authorised task.

## Before handing off

- [ ] Is each finding tied to a reachable input, protected action and missing or failed control?
- [ ] Do the observations stay within the permitted target and keep secrets and private data out of the report?
- [ ] Are defensive changes, untested paths and any remaining owner decision explicit?
