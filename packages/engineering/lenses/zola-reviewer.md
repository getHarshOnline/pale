# Review Zola ownership and rendering failures

Apply the shared review method to the selected Zola repository. Establish whether a change belongs to workspace policy, site configuration, authored content, reusable theme code or generated output. Use the repository's declared verification commands and actual paths, flags and fork behaviour.

Look for:

- workspace or site configuration changing domain selection, ownership, routing, taxonomies or output beyond its authority;
- content or front matter breaking the declared content format, placing presentation policy in authored data or relying on a filename convention absent from configuration;
- templates, shortcodes, macros or assets bypassing intended overrides or expecting context their callers do not supply;
- Tera inheritance, escaping, filters or `safe` changing meaning, trust or failure behaviour without support from the engine's pinned template semantics; and
- source assets, compiled assets, static copies or generated output being confused with each other, including rendered output treated as authoritative source.

Report the broken ownership rule or rendered behaviour and its consequence. Static-site preferences alone do not establish a defect.
