+++
description = "Distinguish permitted tuning from contract changes and test refresh and deadline behaviour."
see_also = [{ scope = "local", resource_id = "contracts/profiles" }]

[guide]
+++

# Separate tuning controls from contract changes

<a id="check"></a>

Classify a setting by the meanings and effects it changes, not whether it is numeric or stored in an environment variable. The owner defines the permitted tuning range. Changes to input/output shapes, required instructions, acceptance or permissions still need a contract decision. Document parsing, allowed values, omitted versus explicit zero, defaults, invalid-input handling and effective configuration revision.

Trace when the consumer reads a value: build time, startup, per request or explicit refresh. Reading the process environment per request does not prove an upstream configuration source can update that process. Test the real refresh or recreation route and inspect the effective value. Rebuild browser-visible constants when their input is fixed at build time.

For deadlines, distinguish total work time, idle time, transport limits and how long the client waits. If the server must return a structured timeout before the caller gives up, test that ordering with delivery margin and intervening proxy limits. Include a long but active response and a stalled one. Simply setting the client timer higher than the server's does not reconcile different timer meanings. Keep configured limits, observed cancellation and remote outcomes separate.

## Before handing off

- [ ] Name the settings operators can change and their allowed range.
- [ ] Compare behavior at the extremes and when a setting is absent.
- [ ] Check that tuning preserves the promised meaning and results.
