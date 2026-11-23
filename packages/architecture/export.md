+++
description = "Export only permitted members and metadata, with checked revisions, limits and delivery."
see_also = [{ scope = "local", resource_id = "contracts/translation" }]

[guide]
+++

# Check what an export includes and omits

<a id="check"></a>

Identify the owner, caller, purpose, source and fixed revision, requested scope and optional members. Check permission and source membership independently. Build the export from permitted members at that revision. A matching label, current head or store-wide default does not establish membership. Select members before calculating counts, applying limits or pagination, generating summaries or assembling archives. Keep data and metadata within the same permitted scope.

Specify which requests may be narrowed and which must be refused. An explicit empty selection stays empty. Refuse unknown revisions and unsupported required members. Report permitted optional omissions without revealing private names, counts or rejection details. Define ordering, tie-breaks, limits and the scope and revision a cursor identifies. In the owner's export record, keep effective scope, output identity, allowed omissions, retention and invalidation rules.

Test overlapping identifiers across two sources and members outside the caller's permission. Include wrong caller or purpose, unknown source, stale revision or cursor, empty scope, unsupported flag and changes outside selected scope. Check data, counts, errors and attachments against the permitted view. Repeat on the same fixed source to check deterministic order and limits. For staged delivery, test revoked permission or invalidated source between staging and inspection under the owner's rules.

Report source and output identities, effective scope, refusals exercised and gaps. The owner defines memberships, selectors, export records, visibility and delivery. Evidence that an export succeeded grants no additional downstream use.

## Before handing off

- [ ] Check caller, purpose, source revision and member scope separately.
- [ ] Filter members before counts, pages and archive assembly.
- [ ] Check an empty scope and a member outside the grant.
