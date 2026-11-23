+++
description = 'Build and review browser interfaces and content.'
uses = [{ scope = "local", resource_id = "change" }]
official_sources = [{ label = 'MDN Web Docs', url = 'https://developer.mozilla.org/en-US/docs/Web' }]

[guide]

[stack]
role_uses = [{ scope = "local", resource_id = "web" }]
+++

# Build web interfaces around browser behaviour

Apply the common coding method and these shared browser rules before framework-specific guidance.

Use semantic HTML: choose the element that performs the intended job. Provide accessible labels, roles, focus order and contrast. Make layouts responsive with explicit geometry, dimension limits and crop rules, so state changes do not unexpectedly move the content. Use CSS custom properties and design tokens for shared theme values.

## Check generated styles and browser defaults

When a stylesheet generator scans source, check its configured input roots and the complete class names it can find. Choosing between complete literal classes differs from assembling fragments whose final class never appears in those inputs. Inspect emitted CSS before reporting an interpolated class as broken. Put runtime values in supported styles, custom properties or an explicitly supported finite mapping.

Trace browser defaults, reset coverage, cascade order and the selected token source before adding compensating styles. Two styling systems may coexist during an intentional migration; the project owns their integration and component choices.

## Make rendering fast and content safe

Give the critical rendering path an explicit budget. Defer non-essential JavaScript, load below-the-fold assets lazily, choose image dimensions and formats for their use, and avoid repeated layout work during state changes. Respect reduced-motion preferences.

Encode interpolated content for the place receiving it and keep untrusted HTML out of direct injection. Check content-security policy and secure headers at the deployment that owns them. Keep secrets out of client bundles, logs and committed files, and validate external input where its owner receives it.

## Capture event inputs before their owner changes

Capture values needed by deferred work before clearing, replacing or reusing their DOM source. For a file input, retain the selected `File` references or a plain array of them before resetting the input or scheduling a state updater. Do not postpone that read through the event target or its `FileList`.

Check the number of items to determine whether a selection exists; an empty array is truthy. Keep item identity separate from the owner's rules for reselection or deduplication. Matching file metadata alone does not establish matching bytes.

## Check what Apply will commit

Establish whether Apply commits the whole selection or a supported subset. Derive readiness from the current items and their valid required assignments. A nonempty list, truthy mapping or absence of running work is not enough. Explain what needs correction and validate again in the handler or service that makes the change, including changes since preview. A warning tells the user about a problem; a disabled button prevents that particular activation.

For a full-selection action, preserve owned state and recovery inputs until every required item passes. For a supported partial commit, identify what committed and keep a correction path for the remainder. Handle repeated activation and late results according to the owner's ordering rules. Native services still check and authorize their own effects.

## Exercise the delivered interface

Cover launch-critical paths at the highest available level. Distinguish desktop, mobile, loading, empty, error and permission states. Check layout and loading on the delivered page. Use a real browser for behaviour that depends on its rendering or interaction engine.

For example, use a real `<button type="button">` to open a local panel and a real link for its help destination. Exercise Enter, Space, pointer and modifier-click on the appropriate controls. Inspect what the browser exposes and where focus lands.
