# Automation

Package ID: `pale.automation` · [Manifest](package.toml)

Build and compare workflows, check their effects and retries, and bring in human review when a decision needs it.

A workspace selects entries from this family for the duties its people and agents perform. To read it directly, open the page for your task and follow its reading links.

## Guides

- [Admit a signal as work only when it asks for a business action](<admit-signals.md>) · `admit-signals` · `doctrine`
  Decide whether an incoming signal asks for a business action, and give admitted work an owner, a deadline and an escalation.
  Read first: [Trace workflow identity, effects and replay](<workflow.md>).
  Read as needed: [Check operator review and permission to act](<human-review.md>).
- [Check an Appsmith application and its operator journey](<appsmith.md>) · `appsmith` · `doctrine`
  Check an Appsmith application, its saved revision and the operator journey.
  Read as needed: [Check operator review and permission to act](<human-review.md>).
- [Check an n8n workflow before deployment and after failure](<n8n.md>) · `n8n` · `doctrine`
  Use it to compare the authored workflow, generated export, imported revision and active revision, and to test worker and webhook routing and queue recovery.
  Read as needed: [Trace workflow identity, effects and replay](<workflow.md>).
- [Check callbacks and destination identity](<callbacks.md>) · `callbacks` · `doctrine`
  Check callbacks, destination identity and refreshed authentication.
  Read first: [Trace workflow identity, effects and replay](<workflow.md>).
- [Check operator review and permission to act](<human-review.md>) · `human-review` · `doctrine`
  Check human-in-the-loop (HITL) decisions, usable reviews and the result actually applied.
- [Check the applied rule, schedule and caller contract](<rules.md>) · `rules` · `doctrine`
  Check the rule applied, scheduled runs and replacement behaviour.
  Read first: [Trace workflow identity, effects and replay](<workflow.md>).
- [Commit results once and retry from durable state](<retry.md>) · `retry` · `doctrine`
  Use it to apply the command-retry method to queue expiry, final results, queued prerequisites, reporting and a shared allowance for provider calls.
  Read first: [Trace workflow identity, effects and replay](<workflow.md>), [Check commands that may be retried](<../architecture/retry.md>).
  Read as needed: [Keep model-assisted work within its value and budget](<../model-work/cost.md>).
- [Deliver committed occurrences and recover history](<delivery.md>) · `delivery` · `doctrine`
  Use it when delivery depends on a source commit, to record the occurrence in that transaction and keep the source record, attempt, acknowledgement and effect distinct.
  Read first: [Trace workflow identity, effects and replay](<workflow.md>).
- [Trace workflow identity, effects and replay](<workflow.md>) · `workflow` · `doctrine`
  Trace a workflow from request to recorded result, including retries and replay.
  Read as needed: [Check callbacks and destination identity](<callbacks.md>), [Deliver committed occurrences and recover history](<delivery.md>), [Commit results once and retry from durable state](<retry.md>), [Check the applied rule, schedule and caller contract](<rules.md>).

## Tasks

- [Check a workflow change before it goes live](<check.md>) · `check` · `operation`
  Check that a workflow change does what it should, including when things go wrong, before it goes live.
  Read first: [Trace workflow identity, effects and replay](<workflow.md>), [Check operator review and permission to act](<human-review.md>).
- [Check delegated work across independently owned systems](<delegation.md>) · `delegation` · `operation`
  Check dispatch prerequisites and each owner’s result in a delegated workflow.
  Read first: [Trace workflow identity, effects and replay](<workflow.md>).
- [Compare saved workflow and application exports](<exports.md>) · `exports` · `operation`
  Compare saved n8n workflows, Appsmith unpublished exports or saved-import receipts with the configured verifier.
  Read first: [Establish what actually ran](<../engineering/execution.md>).

## Stacks and tools

- [Check an Appsmith application and its operator journey](<appsmith.md>) · `appsmith` · `stack`
  Check an Appsmith application, its saved revision and the operator journey.
  Read as needed: [Check operator review and permission to act](<human-review.md>).
- [Check an n8n workflow before deployment and after failure](<n8n.md>) · `n8n` · `stack`
  Use it to compare the authored workflow, generated export, imported revision and active revision, and to test worker and webhook routing and queue recovery.
  Read as needed: [Trace workflow identity, effects and replay](<workflow.md>).
- [verify-exports](<verify-exports.toml>) · `verify-exports` · `tool_recipe`
  Compare the selected saved workflow or application artifacts and report coverage and differences.

## Policies and other resources

- [export-policy](<export-policy.toml>) · `export-policy` · `capability_policy`
