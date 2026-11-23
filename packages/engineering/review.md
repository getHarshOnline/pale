+++
description = "Review a local change or pull request, establish each finding and explain what the code and checks support."
uses = [{ scope = "local", resource_id = "change" }]

[guide]
exposure = "internal"

[task]
roles = [{ scope = "local", resource_id = "review" }]
outputs = ["In the reply: findings ordered by consequence, with the supporting code or check and any uncertainty.", "Checks run, what was unavailable, and the conclusion supported by this review."]

[role]

[policy]
+++

# Review a change

Review the exact change against the requested outcome, the dependencies found during investigation, the guarantees it must keep and the checks claimed for it. Read the change and form your own conclusion. Check correctness, security and data protection before looking for unnecessary complexity. Keep the candidate unchanged during the review; the writer's conclusion does not substitute for checking the result.

## Establish what is being reviewed

Identify the revision or local diff, baseline, requested behaviour and applicable project instructions. Read complete changed files in context, following callers, tests and any connection whose meaning the diff cannot explain. Account for generated output, migrations and affected users as well as the edited source. For a broad change, state the order of review and the parts still unread.

Choose language and library guidance from the repository's declared membership and the code the task changes. Read its actual check commands. If a needed command cannot be found there, ask the requesting task for it. A missing observation limits the conclusion; it does not establish a passing result.

Check four aspects of the result:

- Every acceptance condition and affected user has been handled, and the change fixes its cause.
- Permission, data protection, failures, compatibility and recovery work as required.
- Each passing claim matches the check that produced it; unavailable checks remain visible.
- The design has no avoidable duplicate state, abstraction, speculative extension, unused interface or dependency.

Keep the review tied to the inspected version. A later edit may need another look at the behaviour it changes.

Before commissioning review, fix known defects and run the relevant checks within existing permission. Give the reviewer the remaining uncertainty and results, including failures. Review time is better spent finding what the writer missed than rediscovering a known failing check.

## Review a local change or pull request

For a local change, find the changed files from Git, then read the relevant source, callers, tests and conventions. If there is no change to review, say so. Return the findings and conclusion in the current reply; create a review file only if one was requested.

For a named pull request, use the available code-host reader to establish intent, metadata, changed files and the exact patch. Read the files at that revision and run the repository's checks for the risks being changed, within their existing permission. A draft gets findings for its writer to address. Publish a review only when that action is authorised; otherwise return it locally.

If the code-host reader is unavailable, use an exact supplied diff or checkout of the target revision and say which metadata or files were unavailable. An unusually broad change still needs an explicit reading order and an honest account of coverage.

## Check what recorded results establish

When the candidate and its check results are already available, examine the claims against those results. Otherwise establish the local change or pull request first. Identify the candidate, baseline, acceptance claims and responsible owner. For each claim, read the original result: the subject tested, inputs, command, toolchain, output and coverage. Keep passed, failed, missing, unavailable and stale results distinct. A result from another candidate applies only if an explicit comparison establishes equivalence for that particular claim.

Matching output bytes can establish that output is identical. Source identity, index freshness, a test result or permission must each be established for the candidate being reviewed. For example, a wire-format comparison may show equal descriptors while documentation and generated files still require their own checks.

Keep the original result intact. State what each claim establishes, the evidence for carrying any narrower claim forward, and the smallest check needed to settle an open question. Leave an unrun check pending and acceptance with the responsible owner.

## Establish a finding before reporting it

Ask four questions: Where is the exact line? Which input and state cause which failure? Have callers, imports and tests been read? Does the consequence justify the severity?

For a high or critical finding, include the snippet and location, the concrete failure scenario, and why existing types, validation or framework protections do not prevent it. Otherwise lower the severity or leave the question open.

Check common false positives before reporting:

- Errors or validation may already be handled by a caller or framework.
- Familiar constants and fixed test expectations can be intentional.
- Long switches, configurations, test tables and generated code can be straightforward.
- Internal helpers may explain themselves through their names and types.
- Reassignment can require `let`; preceding guards can rule out null.
- Fixed-size or batched queries need no N+1 finding.
- Logging, metrics and queued work may intentionally run without awaiting completion.
- Existing JavaScript need not become TypeScript.
- Non-secret randomness and explicit plugin code loading need their actual risks checked.

These checks adapt [ECC's review method](https://github.com/affaan-m/ECC/blob/591ab5cbd3f2f65860ea91c226e410b1502c8e2e/agents/code-reviewer.md).

## Match severity to the consequence

Use the project's severity definitions when supplied. Otherwise distinguish critical failures of a security or correctness guarantee, high-impact reachable failures of required behaviour, medium-impact defects with narrower consequences, and low-impact maintenance problems. Explain the affected use and reach so the label can be assessed. A style preference becomes a finding only when it breaks an applicable convention or has a concrete consequence.

Group symptoms of one cause in one finding. Keep contrary evidence and uncertainty beside the claim they change. Report no actionable findings when that is what the reading supports; a clean review applies to the inspected scope and checks.

## Return what the next person needs

In the requested review or reply, name the version and change reviewed, whether the acceptance conditions are met, checks run, findings with their status, work that must be preserved, remaining risk and who must act next. Include important clean areas when they help explain the result. Separate recommendations from checks that actually ran.

Use an independent reader when self-review would share a material blind spot. Give that reader the revision or exact local diff, requested behaviour and constraints, owner instructions, applicable language guidance and checks already run. Ask for independent investigation, concrete failure cases and contrary evidence. Label self-review honestly when another reader is unavailable.

When the task or workspace calls for a review across providers, run that review with its selected providers and permitted data. Discover the available route to each reviewer before dispatch. Give each the same original request, later corrections, fixed candidate and check results; ask each to investigate independently before seeing the others' conclusions. Keep model choice and the number of reviewers with the consumer. If a required reviewer is unavailable, leave that part of the review open and name what is missing. Another session on the writer's model does not satisfy a requirement for a different provider.

Give concurrent readers the same fixed candidate and keep their writes separate. Investigate findings as they arrive, but wait for those readers to finish before changing their shared checkout. To start a repair sooner, keep the reviewed version available in an isolated checkout and give the repair its own writer. Preserve every outstanding finding when joining results; a timeout or failed reviewer leaves coverage missing.

Set the review effort to the risk and agreed budget. After a repair, check the affected behaviour and unresolved findings; repeat the whole review only when a broad change or new evidence warrants it. Stop when the required checks and material findings are resolved. Do not keep changing working code to raise a score or obtain another clean verdict. When an iteration limit is reached with a required finding open, hand back that unfinished result rather than treating the limit as acceptance.

In the existing review discussion, give each finding a stable reference and record the decision with its evidence: repaired, disagreed, deferred outside this task or still open. Return changed behaviour and unresolved findings to the required reviewers, carrying their earlier objections forward. A material disagreement needs an answer from the source or a decision by the responsible owner; counting votes does not settle it. Report who reviewed which revision, what the review changed and what remains unresolved. Include scores or usage only when requested and actually returned.

For a task that spans several changes, return the current findings together with the next agreed piece of work, its dependencies and the checks it will need. At the next review, bring both the responses to those findings and the next change. This keeps the whole task moving while each change gets a rigorous review; revisit a settled finding when the code or evidence affecting it changes.

The review gives a supported conclusion to the task or owner that requested it. Further checks, repairs, publication of the review, approval, merging, release, activation and changes to work status each follow their own existing permission and responsible owner.
