+++
description = "Define shared design roles and components; find the source of a rendered defect and carry its fix to the interfaces that use it."
uses = [{ scope = "local", resource_id = "design" }]
see_also = [{ scope = "local", resource_id = "web" }, { scope = "local", resource_id = "system/roles" }, { scope = "local", resource_id = "system/tokens" }, { scope = "local", resource_id = "system/states" }, { scope = "local", resource_id = "system/consumers" }, { scope = "local", resource_id = "inspect/design-tools" }, { scope = "local", resource_id = "system/change" }, { scope = "local", resource_id = "system/presentations" }]

[guide]
+++

# Form and repair a design system

Use this when forming shared design roles or fixing a repeated defect. Read the original direction, corrections and the representative tasks and states. Decide what the system should make consistent, where variation belongs and which source owns the correction.

| If you are doing this | Read this |
| --- | --- |
| Form roles, components and legitimate variants | [Form roles and components from recurring meanings](system/roles.md) |
| Add a token, alias, mode or unit conversion | [Resolve a token change through its build](system/tokens.md) |
| Compare variants, changes and real component behavior | [Audit a shared design through complete states](system/states.md) |
| Locate a defect or determine what a change reaches | [Trace a design problem to its owning source](system/consumers.md) |
| Plan a design repair or build a new part of the system | [Carry a design decision from brief to rendered work](system/change.md) |
| Choose light, dark, motion and input treatments | [Design each presentation and check how it is reached](system/presentations.md) |

## Before handing off

- [ ] Do shared roles and intentional variation follow from real content and accepted references?
- [ ] Is the producing token, component, template or composition identified?
- [ ] Does the correction reach its consumers and work in the affected tasks and states?
