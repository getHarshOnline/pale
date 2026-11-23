# Change code so it fits

Read the repository's `AGENTS.md` and `CONTRIBUTING.md`, then the code and its callers. Explain in three or four sentences what this code does, what calls it and why it is built this way. If you cannot, read more before changing it. Use [the shared engineering method](judgment/common.md) for structure, comments, boundaries and tests.

Work through these questions in order:

1. **What does it touch?** Trace the ordinary entry point through the change to its callers, stored data and outputs. Include callers a text search might miss: generated bindings, configuration, plugins and external users. Find the cause of a failure before choosing the edit.
2. **What comes in from outside?** Check input where it enters. Keep permissions, secrets and ownership with the component responsible for them; changing an adapter must not silently change who may act.
3. **What is the simplest design that does the whole job?** Compare the parts, types, paths and names a maintainer must understand. Reuse an existing responsibility when it fits, and separate a genuinely different one. Judge the design by the concepts it needs, never by the fewest changed lines.
4. **What would fail if this were wrong?** Run that check through the normal entry point and keep a useful regression test. Choose a check that reaches the changed behaviour; a command that succeeds without running it leaves the question open.
5. **Would someone who did not write it accept it?** Review the final change against its callers and requirements. Resolve actionable findings and explain any material uncertainty in the reply.
6. **What is left for later?** Put necessary follow-up work on the product's existing work list, with why it can wait and what would make it urgent. A missing requirement or failing check needed for this task is unfinished work, not “later”.

## Before handing off

- [ ] Does the result do the whole requested job through its ordinary caller?
- [ ] Can another maintainer understand its responsibilities and reasons without this conversation?
- [ ] Does the reply say what was checked, what failed and what remains unfinished?
