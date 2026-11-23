# Review the finished change

After implementation, review the exact change against the requested outcome, affected users, required guarantees and checks run. Read complete changed files in their owning context, and follow connections the diff cannot explain. Check correctness, security, data protection, compatibility and recovery before looking for unnecessary complexity.

Account for each acceptance condition, including generated output and migrations. Match every passing claim to the check that produced it, and leave unavailable checks visible. Look for duplicate state, unnecessary abstractions, speculative extensions, unused interfaces and avoidable dependencies.

Use the Engineering review method for individual findings: establish the location, concrete failure, surrounding protections and justified severity. The review tasks and reviewer roles compose that method with their project-specific instructions. Group symptoms of one cause, retain contrary evidence and accept a review with no actionable findings when the code supports it.

In the existing review or reply, give the revision or diff, whether the requested result is met, checks run, findings and their status, work to preserve, remaining risks and who acts next. Use an independent reader when self-review would share a material blind spot. Keep approval, merging, publication, activation and work-status changes with their responsible owner and existing permission.
