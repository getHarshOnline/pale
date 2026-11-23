+++
description = "Check a reference through editor navigation, incoming-reference search and the actual reader destination."
see_also = [{ scope = "local", resource_id = "documentation" }]

[guide]
+++

# Check a reference in editing and published output

Identify the authored occurrence, source set and generation, target identity, and intended publication or document output. In the check result, distinguish inputs available to resolve references from outputs chosen for generation. A shared input can resolve a sibling publication's destination even when only the referring publication is built.

Check each route separately:

- **Jump from the editor:** start at the actual occurrence and read back the target declaration or anchor with source identity and location. State the position convention and precision. A range covering a construct may locate it without identifying the editable token.
- **Find affected references:** start at the resolved target and query incoming occurrences in the chosen corpus. State whether the declaration is included and preserve each source revision and range. Treat absence as a finding only when results are complete for that corpus. Stale, refused, partial or limit-exhausted queries leave the question open.
- **Follow the reader's link:** use the actual build output and inspect the emitted address, target identity where available and fragment. Check how this output mode resolves aliases and base addresses. Where serving or interaction matters, follow the link in the intended presentation and observe the landing. Configuration, generated links and reachable destinations each need evidence.

Where the editor supports overlays, test a changed or missing target and stale source generation. Test a link with the correct label but wrong route or fragment. After a repair, repeat affected checks using the corrected inputs and keep editor, incoming-reference and output findings distinct. Use the owning tools for supported syntax and results. Editorial relationships, edit permissions and release approval stay with their owners.

## Before handing off

- [ ] Does the editor resolve the intended target from the authored occurrence?
- [ ] Are incoming references complete for the checked corpus, or is the limit visible?
- [ ] Does the actual reader link reach the intended destination and section?
