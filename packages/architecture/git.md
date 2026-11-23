+++
description = "Choose a Git workflow, reassess older changes and follow pull requests through checks and integration."
see_also = [{ scope = "local", resource_id = "repo/strategy" }, { scope = "local", resource_id = "repo/staging" }, { scope = "local", resource_id = "repo/delivery" }]

[guide]
+++

# Choose a version-control workflow

Use this method to choose how to base, record, integrate, publish or consume a change. Version control preserves collaboration and release history as well as files. Read the repository's actual branch and remote roles, protections, release channels, submodule pins and contributor permissions. A common convention or example cannot override those facts. Put the recommended action, relevant refs and remaining risks in the change review or handoff.

| If you need to do this | Read this |
| --- | --- |
| Choose a Git strategy, refresh an older change or follow a pull request | [Read the guide](repo/strategy.md) |
| Verify what the staged change contains | [Read the guide](repo/staging.md) |
| Check fork lineage and what a recipient can reach | [Read the guide](repo/delivery.md) |

## Before handing off

- [ ] Check the branch, remote and actual staged change.
- [ ] Choose the least disruptive history operation that serves the task.
- [ ] Verify the destination and what it will receive.
