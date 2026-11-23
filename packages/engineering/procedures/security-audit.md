# Review a specific target for security failures

Use this when a task requires adversarial security analysis beyond a general design review. Establish the exact permitted target and ways of gathering evidence, then use the shared review method and security-reviewer role. Find reachable weaknesses with concrete consequences, keep confirmed paths separate from hypotheses and rank findings by the conditions needed to exploit them and their impact.

Keep the audit read-only. In the reply, identify the vulnerable code or interface, reproduction evidence obtained within the permission given, impact, uncertainty and the defensive change needed. Exercising a real exploit or changing the target requires its own authorised task.

## Before handing off

- [ ] Are the target and its reachable inputs identified?
- [ ] Has the unsafe assumption been traced to the affected operation?
- [ ] Does the reply explain the defensive correction and a safe way to check it?
