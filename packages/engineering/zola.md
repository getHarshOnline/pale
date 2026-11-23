+++
description = 'Build and review Zola sites with the template engine version they use.'
uses = [{ scope = "local", resource_id = "change" }]
official_sources = [{ label = 'Zola documentation', url = 'https://www.getzola.org/documentation/' }, { label = 'Zola template documentation', url = 'https://www.getzola.org/documentation/templates/overview/' }, { label = 'Tera v1 documentation used by Zola', url = 'https://github.com/Keats/tera/blob/v1/docs/content/docs/_index.md' }]

[guide]

[role]
extends = [{ scope = "local", resource_id = "review" }]
applies_to = [{ scope = "local", resource_id = "zola" }]

[stack]
role_uses = [{ scope = "local", resource_id = "zola" }]
+++

# Check Zola templates content and generated pages

Apply this guidance when Zola is selected, using the site's actual engine, templates and configuration.

Resolve template precedence across site overrides, theme templates, shortcodes, macros and built-ins. Check that every caller supplies the context its template expects. Check the selected engine's escaping defaults and each use of `safe`: the value must already be trusted markup.

Keep authored content, front matter, workspace policy, reusable theme presentation, colocated assets, static copying, stylesheet compilation and generated output with their respective owners. Follow the configured content and routing rules when interpreting a filename or rendered page.

Build and check representative pages and states with the repository's declared commands. Inspect rendered structure where composition or escaping matters, using the selected engine and site configuration. Check assets, links and language in generated output, and open the affected destination.

In the reply, explain the affected content and assets, which template and owner control the result, what context and trusted markup they need, and what the generated output and checks showed. State build and release status separately. Exact forks, versions, commands, domains and publication state remain facts supplied by the project.

## Follow a translated link

A translated page works in preview but includes the default language's route. Build with the real language and theme settings and open the emitted link. Fix the lookup where it is constructed, then check that the other language still uses its own route.

## Review Zola ownership and rendering failures

Apply the shared review method to the selected Zola repository. Establish whether a change belongs to workspace policy, site configuration, authored content, reusable theme code or generated output. Use the repository's declared verification commands and actual paths, flags and fork behaviour.

Look for:

- workspace or site configuration changing domain selection, ownership, routing, taxonomies or output beyond its authority;
- content or front matter breaking the declared content format, placing presentation policy in authored data or relying on a filename convention absent from configuration;
- templates, shortcodes, macros or assets bypassing intended overrides or expecting context their callers do not supply;
- Tera inheritance, escaping, filters or `safe` changing meaning, trust or failure behaviour without support from the engine's pinned template semantics; and
- source assets, compiled assets, static copies or generated output being confused with each other, including rendered output treated as authoritative source.

Report the broken ownership rule or rendered behaviour and its consequence. Static-site preferences alone do not establish a defect.
