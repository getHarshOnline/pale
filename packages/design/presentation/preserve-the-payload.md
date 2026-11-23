+++
description = "Use it when a fact appears in a new form, to keep its identity, values, units, qualifications, source and the distinct roles around it."
uses = [{ scope = "local", resource_id = "presentation" }]

[guide]
exposure = "internal"
+++

# Keep facts and relationships through a change of form

## Keep facts and relationships intact

For the fields a form shows, preserve identity, kind, values, units, supplied precision, qualifications, source, lifecycle and destination. Preserve distinct roles such as author, publisher, operator, client, rights holder and ecosystem participant wherever the source distinguishes them. Do not merge several known roles into an ambiguous byline or infer an unknown role from a shared engine, contact or folder. A compact form may omit irrelevant detail, but must retain what makes its displayed claim accurate and provide the appropriate full source or an honest indication of omission.

A figure needs a measure and unit, the population or scope that defines it, relevant date, uncertainty and source. Keep the necessary qualifiers with the value; a distant footnote cannot always repair an unqualified headline. If the form cannot carry a material qualification, show a suitably bounded statement or omit the figure and link to its proper account. A large number is not merely decorative material.

Put meaningful labels, units, status and qualifications in the document. Inspect `content:` rules on `::before` and `::after` for words or figures that exist only in styling. Generated content can be exposed differently across reading and copying paths; it must not be the sole source of a required fact. Check the document with styles disabled and copy the rendered measurement to confirm the unit and qualification travel with it. Use `<figure>` and `<figcaption>` where a self-contained figure and caption form the evidence unit, while preserving table semantics for tabular data.

Spell out an unfamiliar abbreviation on first use. An `<abbr title="…">` can supplement that expansion, but a reader without hover or reliable title exposure still needs to understand the measure. Keep the supplied study or dataset identity when citing evidence; shortening it to an organization and year can remove the information needed to locate the source.

Distinguish a supplied comparison from two independent observations. A slash can suggest a ratio that the source never asserted. Keep the stated relation, sequence and each labeled step readable. Do not invent a difference, trend, sample size or interval to make a composition stronger. If an authorized derivation or conversion is needed, make it an explicit content transformation with its own inputs, units and review; it is not an invisible theme calculation. Locale formatting must preserve the accepted value and precision. A missing day in a month-level date cannot be supplied by the build date.

Separate three states that often look alike: current navigation or selection, the reader's visited/saved/history state, and the object's published lifecycle. Opening a closed project's chapter does not revive the project. Marking an article read does not make its findings current. Give the relevant states separate labels and accessible expression; the product owns their persistence and transition rules. The browser's history is not a content-status registry.

Preserve the kind and direction of supplied relations. Subject, passing mention, membership, editorial relatedness and derivation answer different questions. Do not promote a citation into ownership, a mention into subject authority or recent items into an editorial recommendation merely to fill a slot. Link appearance may distinguish a destination's environment without changing its object relation. Whether an editorial relation is symmetric, how membership is ordered and what constitutes derivation follow the owner's actual model.

Where the page publishes structured relations, generate them from the same supplied records as its visible labels. In a schema.org model, `about`, `mentions`, `isPartOf` and `isBasedOn` serve different jobs; use only the mappings the actual record supports. Compare the objects and relation types in the structured output with the page. A hand-written JSON-LD block can drift even while the visible template is correct. Keep author, publisher and rights-holder fields separate there too.

## Match the amount shown to the job of the form

| Form | What it should carry |
| --- | --- |
| Index row | Identity, kind, what distinguishes the item and a path to its full record. |
| Card or related item | Enough detail to explain its role in the collection, with a path to the source. |
| Citation | The identity, source and destination needed to support the sentence. |
| Preview | A short summary tied to the actual target. The original link still works without the preview. |
| Search result | The match and its context, with a reachable result. |
| Staged object | A visible or accessible identity, kind and destination beside the rendering. |
| Embed | The parts its placement needs, with a path to the object and its complete context. |

Show a unit beside its value even if the form has little space. If the reader can only recover a qualification by following a link, keep it in the form whenever leaving it out would change the claim. In a pair, put the comparison into words or structure. A ladder keeps its steps and labels. If color favors one figure without a reason in the source, apply emphasis to the relation or its label instead.

Keep machine-readable and visible precision together when the source has a value and a month, for example:

```html
<data value="42">42%</data> of respondents,
<time datetime="2024-05">May 2024</time>
```

Use the measure and relevant source beside the example on a real page. Preserve month precision rather than making up a day in the attribute.

Validate the machine-readable representation against the format it claims to use. `<data value>` can carry a supplied value while its text carries the permitted local formatting and unit. `<time datetime>` must use an accepted HTML date or time syntax at the supplied precision. If the source has no suitable date value, retain an accurate textual label rather than inventing a date to satisfy the markup.

## Worked example

A comparison appears as two columns at a desk and two stacked items on a phone. Keep the values, units, sample definitions, date range and source with the same observations. A slash between numbers can turn a comparison into a ratio, so give them labels and show the relation explicitly. Compare the actual text and figure in both views. A published status comes from the record. A reading treatment follows the audience’s task and cannot change that status.

## Sources

- [HTML data](https://html.spec.whatwg.org/multipage/text-level-semantics.html#the-data-element), [time](https://html.spec.whatwg.org/multipage/text-level-semantics.html#the-time-element) and [figure](https://html.spec.whatwg.org/multipage/grouping-content.html#the-figure-element): machine values, valid date syntax and a figure's relationship to its caption.
- [MDN abbreviation element](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/abbr): visible expansion and the limits of relying on a title.
- [schema.org about](https://schema.org/about), [mentions](https://schema.org/mentions), [isPartOf](https://schema.org/isPartOf) and [isBasedOn](https://schema.org/isBasedOn): distinguish subject, reference, membership and derivation when mapping supplied relations.
