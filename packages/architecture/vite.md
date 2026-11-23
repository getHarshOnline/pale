# Check Vite inputs and browser output

Use this guidance when the project builds with Vite. Read its configuration and the applicable official documentation for plugin APIs, environment prefixes, server rendering and output behaviour.

Configuration and plugins execute code in the build. Identify who owns each transformation, which files enter the module graph and which values become visible to browsers during compilation. An environment prefix controls disclosure as well as naming.

Compare development and production behaviour. Aliases, proxies, asset URLs, module conditions, dynamic imports and server/client entry points can differ even when hot module replacement appears to work. HMR must invalidate state affected by a module's changed meaning. Preserving stale singletons or application state can hide a failure until a full reload.

For application, library or SSR output, identify entries, external dependencies, chunk and CSS ownership, runtime target, source-map exposure and what the consumer expects. Measure changes to the module graph and delivered payload before changing chunking based on intuition.

In the build review, report plugin and disclosure risks, development/production differences, expected output and checks still needed. Configuration and built artifacts remain with the project.

## Worked example

A site works in the dev server and fails below a subpath after build. Build with the actual `base`, inspect the emitted entry and asset URLs, then open the site at that subpath. Check that no private setting was exposed through `import.meta.env`.

## Before handing off

- [ ] Check what the project sends to the browser.
- [ ] Resolve build inputs, imports and public settings from the actual output.
- [ ] Open the result and verify the affected route.
