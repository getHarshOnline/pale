+++
description = "When continuity explains a real relationship, such as an item moving from a list into a detail view."
uses = [{ scope = "local", resource_id = "web" }]
official_sources = [{ label = "Use a view transition without losing navigation", url = "https://www.w3.org/TR/css-view-transitions-2/" }]

[guide]
exposure = "internal"

[stack]
+++

# Use a view transition without losing navigation

Use a transition when continuity explains a real relationship, such as an item moving from a list into a detail view. A normal link should navigate with a real address, history, modifier clicks, Back and a usable result when the transition is unavailable. Existing application routing may already own those jobs. Keep the animation from accidentally taking them over.

## Worked example: keep an item recognizable across views

```css
@view-transition {
  navigation: auto;
}

.product-mark {
  view-transition-name: current-product-mark;
}

@media (prefers-reduced-motion: reduce) {
  ::view-transition-group(*) {
    animation: none;
  }
}
```

Use the same name for the one corresponding element in each document. Check what happens if two elements in a document accidentally share the name. This example depends on cross-document support and eligible same-origin navigation. Verify normal navigation and its focus and scroll result without the feature. Check the captured old and new states with real text and images. Keep content and controls usable while the visual transition runs. Preserve project preference paths that go beyond the system query shown here.

Prefetch only where the destination, privacy policy and budget justify it. A likely intent, such as focus on a link, can justify a request that crawling every visible link cannot. Keep origin restrictions explicit and inspect actual requests. A failed prefetch must still leave the actual link usable. Test opening in a new tab and returning with Back, including a page that restores a prior selection or scroll position.

When an existing router handles a link, leave modified activation to the browser. Its link filter can start with this check:

```js
function usesBrowserNavigation(event) {
  return (
    event.defaultPrevented ||
    event.button !== 0 ||
    event.metaKey ||
    event.ctrlKey ||
    event.shiftKey ||
    event.altKey
  );
}
```

Also leave explicit targets, downloads and destinations outside the router's scope to the browser. Check keyboard activation of the plain link and the same link with the enhancement unavailable.

## Inspect the arrival, not only the animation

For a static publication, adding a page-swapping router solely for an effect takes on history, scroll restoration, focus, head metadata, failures and module reinitialization that document navigation already supplies. Prefer a supported cross-document enhancement when it serves the treatment. A persistent player or application router can have a real reason to own navigation; document and exercise that contract instead of importing a universal ban from another site's architecture.

| Failure to look for | Check through the actual navigation |
| --- | --- |
| A transition covers content already ready | Make arrival usable promptly; show pending state only for work actually pending. |
| Back or Forward loses the reader's place | Compare history, scroll and selection with the promised restoration behavior. |
| The old title, language or metadata remains | Verify the destination's document identity as well as its visible heading. |
| Focus stays on a removed element | Establish a useful focus position on arrival and exercise the skip link. |
| Content waits for an animation-end callback | Keep a complete arrival when animation is skipped, interrupted or unavailable. |
| Reduced motion removes information | Remove travel while preserving the destination's identity or relationship through an immediate change of light, material or emphasis where needed. |

## Sources

- [MDN View Transition API](https://developer.mozilla.org/en-US/docs/Web/API/View_Transition_API): named elements and same-document versus cross-document mechanics.
- [MDN `@view-transition`](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@view-transition): document opt-in and eligible navigation.
- [MDN History API](https://developer.mozilla.org/en-US/docs/Web/API/History_API): the history and restoration duties a router takes over.
- [MDN Speculation Rules API](https://developer.mozilla.org/en-US/docs/Web/API/Speculation_Rules_API): prefetch versus prerender, eagerness and supported restrictions.
