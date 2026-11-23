# Review a local change or pull request

Use the shared review method composed with this task. Choose local review for the current checkout, or pull-request review when the request identifies a concrete pull request.

## Review a local change

Find the changed files from Git, then read each relevant file in full with its callers, tests and project conventions. If there is no change to review, say so. Return the findings and conclusion in the current reply; create a review file only if one was requested.

## Review a pull request

Use the available code-host reader to establish intent, metadata, changed files and the exact patch. Read the files at that revision and run the repository's checks for the risks being changed, within their existing permission. A draft gets findings for its writer to address. Publish a review only when that action is authorised; otherwise return it locally.

If the code-host reader is unavailable, use an exact supplied diff or checkout of the target revision and say which metadata or files were unavailable. An unusually broad change needs an explicit reading order and an honest account of coverage.

## Give another reviewer enough context

Pass the revision or exact local diff, requested behaviour and constraints, owner instructions, applicable language guidance and checks already run. Ask the reviewer to investigate independently, with concrete failure cases and contrary evidence included. Label self-review when an independent reader is unavailable. Separate recommendations from checks that actually ran.

## Before handing off

- [ ] Have I read the change and its actual entry points?
- [ ] Does each consequential finding have a reproducible failure or source support?
- [ ] Does the reply name the checks and limits of this review?
