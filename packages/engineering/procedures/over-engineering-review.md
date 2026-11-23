# Check whether the design needs its complexity

Establish the behaviour the change must preserve, constraints it must satisfy now and maintenance burden it adds. For each new concept, dependency, intermediate layer or configuration option, identify the present requirement it serves and the failure it helps prevent. Something without a demonstrated need is a candidate for simplification; establish the consequence before calling it a defect.

In the review reply, rank the useful simplifications by their ongoing cost and value. Name the affected code, the requirement it serves or fails to serve, the smallest alternative that keeps the behaviour, and the uncertainty in removing it. Use the correctness, security and performance methods for their respective claims. Return recommendations from this review; actual edits follow the implementation task's permission.

## Before handing off

- [ ] Is the purpose of each added layer understood?
- [ ] Would the proposed removal preserve required behaviour and ownership?
- [ ] Has the simpler path been traced through a real caller?
