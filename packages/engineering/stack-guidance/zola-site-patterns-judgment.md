# Check Zola templates content and generated pages

Apply this guidance when Zola is selected, using the site's actual engine, templates and configuration.

Resolve template precedence across site overrides, theme templates, shortcodes, macros and built-ins. Check that every caller supplies the context its template expects. Check the selected engine's escaping defaults and each use of `safe`: the value must already be trusted markup.

Keep authored content, front matter, workspace policy, reusable theme presentation, colocated assets, static copying, stylesheet compilation and generated output with their respective owners. Follow the configured content and routing rules when interpreting a filename or rendered page.

Build and check representative pages and states with the repository's declared commands. Inspect rendered structure where composition or escaping matters, using the selected engine and site configuration.

In the reply, explain the affected content and assets, which template and owner control the result, what context and trusted markup they need, and what the generated output and checks showed. Exact forks, versions, commands, domains and publication state remain facts supplied by the project.

## Follow a translated link

A translated page works in preview but includes the default language's route. Build with the real language and theme settings and open the emitted link. Fix the lookup where it is constructed, then check that the other language still uses its own route.

## Before handing off

- [ ] Are content, theme and template inputs correct for the intended site?
- [ ] Were assets, links and language checked in generated output?
- [ ] Was the affected destination opened, with build and release status stated separately?
