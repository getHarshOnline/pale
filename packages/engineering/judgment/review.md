# Review the finished change

Read the finished change against the requested outcome, the dependencies found during investigation, the guarantees it must keep and the checks claimed for it. Review correctness and security before looking for unnecessary complexity. Keep the review separate from implementation so the writer's conclusion does not substitute for checking the result.

## Establish what is being reviewed

Identify the exact revision or local diff, baseline, requested behaviour and applicable project instructions. Read the complete changed files in context, following callers, tests and any connection whose meaning the diff alone cannot explain. Account for generated output, migrations and affected users as well as the edited source. For a broad change, state the order of review and the parts still unread.

Check four aspects of the result:

- Every acceptance condition and affected user has been handled.
- Permission, data protection, failures, compatibility and recovery work as required.
- Each passing claim matches the check that produced it; unavailable checks remain visible.
- The design has no avoidable duplicate state, abstraction, speculative extension, unused interface or dependency.

Use the language and domain guidance that fits the actual project. Keep this review tied to its inspected version; a later edit may need another look at the behaviour it changes.

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

Use the project's severity definitions when supplied. Otherwise distinguish critical failures of a security or correctness guarantee, high-impact reachable failures of required behaviour, medium-impact defects with narrower consequences, and low-impact maintenance problems. Explain the affected use and reach, so the label can be assessed. A style preference becomes a finding only when it breaks an applicable convention or has a concrete consequence.

Group symptoms of one cause in one finding. Keep contrary evidence and uncertainty beside the claim they change. Report no actionable findings when that is what the reading supports; a clean review applies to the inspected scope and checks.

## Return what the next person needs

In the requested review or reply, name the version and change reviewed, whether the acceptance conditions are met, checks run, findings with their status, work that must be preserved, remaining risk and who must act next. Include important clean areas when they help explain the result. Use an independent reader when self-review would share a material blind spot; label self-review honestly when one is unavailable.

The review gives a supported conclusion for the owner to use. Approval, merging, publication, activation and changes to work status each follow their own existing permission and responsible owner.
