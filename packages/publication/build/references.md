+++
description = "Use it to check the bibliography with clean auxiliary state and fresh diagnostics, not a processor's success alone."
uses = [{ scope = "local", resource_id = "local-build" }]

[guide]
exposure = "internal"
+++

# Check the bibliography in the rendered document

Identify the master document, bibliography inputs and processor. Begin with clean auxiliary state or invalidate stale bibliography output. Use fresh auxiliary files and diagnostics to determine whether entries were requested, including explicit inclusion without an in-text citation. Accept an empty bibliography only when the candidate permits it and diagnostics contain only the expected empty-case messages.

Check status and diagnostics together: a processor can report success with a missing key, and an earlier PDF can remain after a later failure. Keep that PDF outside verified results. For a cited document, check resolution of required keys and the expected reference text in the final artifact. For a citation-free document, check for stale entries or an empty reference heading.

Exercise a valid citation, citation-free master, missing database or style, unresolved key and rerun with stale bibliography output. In the build result, distinguish each stage and final artifact readback from typography, visual review and publisher acceptance.

## Worked example

Build a cited document with clean auxiliary files. Remove a required database key and rerun while the earlier PDF remains. Check diagnostics, restore the key, rebuild and read the rendered references. Also build a citation-free document and verify that earlier entries and an empty heading are absent.
