+++
description = "Use it to choose a role for control motion from what the action communicates, so feedback is prompt and the interface stays usable while the drawing catches up."
uses = [{ scope = "local", resource_id = "animate" }]
see_also = [{ scope = "local", resource_id = "controls" }, { scope = "local", resource_id = "animate/expressive-motion" }, { scope = "local", resource_id = "animate/reduced-motion" }, { scope = "local", resource_id = "animate/motion-implementation" }, { scope = "local", resource_id = "art-direction" }, { scope = "local", resource_id = "web" }, { scope = "local", resource_id = "surfaces" }, { scope = "local", resource_id = "animate/feedback-roles" }, { scope = "local", resource_id = "animate/frequency-and-pending" }]

[guide]
exposure = "internal"
+++

# Give control motion a clear job

Choose the role from what the action communicates. Give prompt feedback and keep the interface usable while the drawing catches up.

| If you are doing this | Read this |
| --- | --- |
| Time a press, preview, disclosure or other control response | [Choose feedback roles and timing](feedback-roles.md) |
| Handle frequent input, work in progress and result feedback | [Keep repeated and pending responses useful](frequency-and-pending.md) |
