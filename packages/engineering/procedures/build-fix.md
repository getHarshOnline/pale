# Find and fix the cause of a failed build

Identify the failing command, what it was meant to build, the checkout and revision, toolchain, target and permitted changes. Keep the first diagnostic with its source location and exit status in the existing failure report or working analysis. Group later diagnostics behind the earliest cause that explains them. Follow errors in generated code back to the generator or declared input, and use the language guidance supported by the actual build.

For an intermittent failure, obtain a comparable reproduction or say that it remains intermittent. Compare the suspected cause with the dependency path and contrary evidence. When using the build reviewer, pass those sources and ask which observation would disprove the explanation and which part belongs to another owner.

Make the smallest correction that fixes the producing cause within the agreed work. Run the same build again and broaden the check when an affected caller or artifact requires it. Keep a repaired build distinct from behaviour the build never exercises.

In the reply, give the supported cause, correction or unapplied proposal, the project's check command, comparable results when run, and remaining failures. Reconsider the cause after repeated failed hypotheses. Resolve missing project facts, necessary scope changes or a proposed weakening of the system's guarantees before proceeding with that part.

## Before handing off

- [ ] Was the failing build reproduced with its real inputs, or is the intermittent failure still clear?
- [ ] Did the correction address the producing cause, and did the relevant build run again?
- [ ] Were adjacent failures checked where the change can reach them?
