+++
description = "Find the cause of a failed build, review the explanation and correct it within the agreed work."
uses = [{ scope = "local", resource_id = "change" }]

[guide]
exposure = "internal"

[task]
uses = [{ scope = "local", resource_id = "error-handling" }]
roles = [{ scope = "local", resource_id = "build" }]
outputs = ["In the reply: the cause and supporting diagnostic, the correction or unapplied proposal, checks run and remaining failures."]

[role]
+++

# Find and fix the cause of a failed build

Identify the failing command, what it was meant to build, the checkout and revision, toolchain, target and permitted changes. Keep the first diagnostic with its source location and exit status in the existing failure report or working analysis. Group later diagnostics behind the earliest cause that explains them. Follow errors in generated code back to the generator or declared input, and use the language guidance supported by the actual build.

For an intermittent failure, obtain a comparable reproduction or say that it remains intermittent. Compare the suspected cause with the dependency path and contrary evidence. When using the build reviewer, pass those sources and ask which observation would disprove the explanation and which part belongs to another owner.

## Review the explanation independently

As the build reviewer, read the exact failure independently. Distinguish the earliest causal diagnostic from the failures it produces later. Check the proposed explanation against the source, build dependencies, declared toolchain and contrary evidence.

Prefer a correction at the established cause. Identify any dependency, configuration, generator, environment, architecture, behaviour or security problem that needs another owner or specialist. In the reply, give the cause, its source location, explanations ruled out, the smallest proposed correction and the command or observation that could disprove it.

Keep this review read-only. The build task owns permitted edits, fetching, cleaning, generation, installation and execution.

## Correct the cause and check the result

When carrying out the build task, make the smallest correction that fixes the producing cause within the agreed work. Run the same build again and broaden the check when an affected caller or artifact requires it. Check adjacent failures where the change can reach them. Keep a repaired build distinct from behaviour the build never exercises.

In the reply, give the supported cause, correction or unapplied proposal, the project's check command, comparable results when run, and remaining failures. Reconsider the cause after repeated failed hypotheses. Resolve missing project facts, necessary scope changes or a proposed weakening of the system's guarantees before proceeding with that part.

## Before handing off

- [ ] Does the explanation identify the producing cause rather than a later diagnostic?
- [ ] Does the correction stay within the agreed work and preserve the system’s required behaviour?
- [ ] Did the same build run again, and are failures or behaviour it did not check still visible?
