+++
description = "When search needs grouping, a no-match state or an honest unavailable response."
uses = [{ scope = "local", resource_id = "controls" }]

[guide]
exposure = "internal"
+++

# Explain what a search result can do

Use this when search needs grouping, a no-match state or an honest unavailable response.

Show empty, no-match, pending, unavailable, failed and complete states where they can occur. A count can be a status message without turning the whole list into repeated announcements or moving focus on every query. Verify actual announcement and input behavior. Project search fields, weighting, expansion, index limits and provider configuration remain native inputs.

Group results by a relation the person can predict: destinations, documents or actions, for example. Show enough context to identify the destination and effect. If several result kinds appear, preserve the actual labels and one active item. Do not invent ranks, results or provider counts. An unavailable provider can still leave useful local results visible. Use a status message for the count where it helps, and announce changes without rereading every result.

## Worked example: local results while a provider is unavailable

Typing “report” matches two documents in the local index and the remote lookup fails. Show the two real document links and a short message saying the remote search is unavailable. Keep the input so the person can retry or narrow the query. Never turn a provider failure into “No results” for the whole search.
