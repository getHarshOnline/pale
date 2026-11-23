# Check what the recorded results establish

Use this when the exact candidate and its check results are already available. If the local diff or pull request still needs identifying, start with the code-review task. Apply the shared review method composed with this task to the claims below.

Identify the candidate, baseline, acceptance claims and responsible owner. For each claim, read the original result: the subject tested, inputs, command, toolchain, output and coverage. Keep passed, failed, missing, unavailable and stale results distinct. A result from another candidate applies only if an explicit comparison establishes equivalence for the particular claim.

Matching output bytes can establish that output is identical. Source identity, index freshness, a test result or permission must each be established for the candidate being reviewed. For example, a wire-format comparison may show equal descriptors while documentation and generated files still require their own checks.

Keep the original result intact. In the reply, state what each claim establishes, the evidence for carrying any narrower claim forward, and the smallest check needed to settle an open question. A review leaves an unrun check pending and acceptance with the responsible owner.

## Before handing off

- [ ] Is the change named, with the claim each check supports?
- [ ] Have I read the actual result and a material failure case or limit?
- [ ] Are the remaining unverified claims visible?
