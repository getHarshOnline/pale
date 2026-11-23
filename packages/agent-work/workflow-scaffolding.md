# Design a temporary workflow for one task

A temporary workflow coordinates repeated steps within one objective. Use one when manual work would cause meaningful inconsistency, missed evidence or avoidable coordination effort.

## Decide whether the workflow helps

Compare four options: perform the steps directly, use an existing Package or system procedure, create a temporary workflow for this task, or propose a shared reusable method. A temporary workflow earns its place when the sequence repeats, inputs can follow a consistent form, outputs can be checked, and the workflow is easier to understand and retire than repeated manual coordination.

Before using it, define the objective, accepted inputs, output format, step order, things it may change, effect limits, verification, failure behaviour, budgets, handoff and completion condition. Distinguish deterministic mechanics from steps needing expert judgment. Keep that judgment in its owning Package guidance; the workflow chooses and orders the relevant methods.

## Run within the task's permissions

Use the smallest representation the available tools support. The engine or harness owns execution, temporary files, process state, dispatch and result records. Check tool availability and permission for each consequential action. Installation, persistent learning and broader access each need their own basis.

At a phase change, keep the input version reached, completed checks, current output, unresolved failures and next step in the task's existing handoff. If the assumptions change, stop and resolve them before altering the sequence.

## Finish the task or propose reuse

Separate the finished task artifact from knowledge that might help other work. Remove disposable mechanics within the task's cleanup permission, or leave their disposition with the owner. Propose a shared method when the pattern has recurred across contexts, its stable decision is understood, consumer-specific facts can stay outside it and someone owns maintenance. One successful task cannot establish that it generalises.

In the plan or reply, explain the chosen option, workflow requirements, Package methods used, supporting checks, final state and whether reuse is justified. Stop developing the temporary workflow when an existing procedure already owns the problem, the sequence cannot be tested, state or permissions are unclear, expert judgment would be hidden or maintenance would cost more than the task.
