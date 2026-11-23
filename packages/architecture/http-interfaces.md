# Design an HTTP interface

Start with the resource and state change the interface exposes. Identify clients, trust, consistency, scale, compatibility period and failure behaviour before choosing URLs and representations.

Give resources stable identities and make each method match the observation or change it performs. Define request and response forms, content types, validation, permissions, idempotency, conditional updates, pagination and errors together. A success status carrying an error body, a retry that duplicates an effect or a stack trace exposed to clients is a contract defect.

Choose collection pagination for the mutation rate, ordering stability, access path and navigation needs. Specify filters and sorting, and reject queries exceeding permitted limits instead of relying on clients to restrain them. For asynchronous or long-running work, expose durable progress and terminal outcomes rather than leaving a request ambiguously open.

Test compatibility with actual consumers: whether they ignore new fields, accept new enum values, depend on order or defaults, and work alongside the proposed server. Deprecation needs an owner, observed remaining use, a successor and a removal condition. A version label alone creates no compatibility.

In the interface design or review, describe resources, state changes, representations, validation, failures, compatibility, access and workload controls, and tests required. The consuming service owns framework APIs, routes, schemas, deployment and running state.

## Before handing off

- [ ] Name the resource and each method’s effect.
- [ ] Check pagination, failures and caller compatibility.
- [ ] Verify the interface against actual consumer requests.
