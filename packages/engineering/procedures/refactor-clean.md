# Simplify code while keeping its behaviour

Identify the code being considered, who owns it, its present behaviour, callers, dynamic or generated users, permitted changes and the project's check command. Use the solution and review methods to decide what can be removed, merged or must remain. Where permission to change or check it is missing, return the specific proposal and what it still needs.

Search both the implementation and its stable public names. Include package exports, callbacks, registries, configuration, generated users, serialization and migration compatibility, and project integration entry points. If a likely user lies outside the available sources, name that dependency and keep the required behaviour until its owner can establish whether it is still used.

Move users to the surviving implementation before removing the old one. Compare the same callers and outputs before and after, including ordinary entry points and failure cases. Keep unrelated rewrites separate so the comparison still answers whether this cleanup preserved behaviour.

In the reply, explain what was removed, merged, retained or remains uncertain; why; the compatibility kept; checks run; and users that could not be examined.

## Before handing off

- [ ] Are the callers and the behaviour they need understood?
- [ ] Does the surviving path serve them before duplication is removed?
- [ ] Do the checks cover normal entry points and failure cases?
