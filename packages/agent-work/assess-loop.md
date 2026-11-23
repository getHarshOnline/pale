# Check whether a loop should continue

Identify the proposed or running loop, its permissions, observed runtime state, budgets, progress signal, intervention owner and safe stopping path. Use the loop guide to recommend start, continue, intervene, stop or leave unresolved. In the review, give the supporting evidence and conditions for resuming. Wakeups, activity and elapsed time alone cannot establish progress; controlling or storing the process remains the runtime's job.

## Before handing off

- [ ] Are the goal, progress signal, budget and permitted effects clear?
- [ ] Have the conditions for stopping or asking for help been tested?
- [ ] Is the next permitted step clear, with its reason?
