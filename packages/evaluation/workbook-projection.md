# Check a workbook export or reconstructed view

Identify the source revision, exporter and options, exact output and receipt, intended consumer and calculation engine. State which properties the decision requires you to preserve. Keep original bytes and observations separate from new exports or reconstructed views.

| Question | Check |
| --- | --- |
| Are these the recorded workbook bytes? | Compare the entire file digest with the exact output named by the receipt. |
| Does a candidate preserve the workbook model? | Compare the required sheets, typed cells, formulas, formats and resolved named ranges or object references. Include sheet order, visibility, chart inputs and validation when they affect the intended use. Resolve package relationships to their targets before comparing features. |
| What did the saved file contain? | Read stored results independently of formula expressions. Record absent caches, errors, blanks and zero separately. |
| What will the reader's engine calculate and show? | Recalculate selected inputs in a disposable copy using the intended engine. Check the resulting values against their source, and observe the affected presentation and controls when the claim depends on them. |

When rebuilding a historical view, link each source revision or retained prefix to its own receipt. A prefix may recover the rows in that view; exact recovery of the historical file requires its recorded digest. Repacking or exporting again may preserve the workbook model while changing archive metadata, relationship identifiers or serialization. Report exactly what was compared and keep the rebuilt artifact separately identified.

Test a repack with identical parts but a different hash, a substituted formula with a plausible saved result, and a changed input with stale or missing saved output. Inspect the exact exported workbook. Report which properties differ, unchecked engine behaviour and the decision still needed from the owner. The owning system determines the export profile, authoritative calculation engine, receipt and release policy.

## Compare exports under a declared interpretation

For comparisons across locations or providers, identify each output by the source identity and revision issued by its owner, exporter, options and receipt. Use the same accepted interpretation profile or a verified mapping between profiles. A current filename, location, timestamp or provider checksum does not establish the source revision or that mapping. If the exporter cannot identify the source revision for every selected part, leave that comparison unresolved.

State which cells, types and features the comparison preserves and which serialization differences it may ignore. Include date basis, locale, time zone, numeric precision and calculation behaviour where they affect the result. Settle interpretation before computing a normalized digest. Keep original bytes and artifact digests with their receipts. Equivalent output establishes only equivalence of the compared model or view; it does not change source identity, custody or canonical location.

Test an equivalent repack and a stored value whose meaning changes under another profile, such as a date serial interpreted with a different date basis. Inspect the typed value or view in the approved reader. Distinguish equal bytes, equivalence within scope, differences and unavailable comparisons. If a required feature or interpretation is unsupported, leave the claim open and name the exact missing source or reader check. The source owner sets interpretation, comparison scope and permission to repair.

## Reconcile package parts between receipt-bound exports

Verify retained input and output digests for each interval. Before interpreting changes, inventory every package part by path, type and digest. For an append, identify the recorded prefix and expected cell, table-range and filter-range changes. Compare adjacent retained exports separately. If an intermediate output is missing, locate the change within the smallest interval bounded by verified copies; leave attribution to a particular append unresolved.

Compare relationship IDs together with the attributes that refer to them and the targets they resolve to. Identify each reference's owning part, relationship type, target mode and exact target. Coordinated renumbering is equivalent only for the references actually checked. Report dangling references, changed targets or modes and unresolved parts. Do not globally remove identifiers or treat equal XML counts as proof of equivalence.

For a changed style or conditional format, compare the rule, affected range, priority, formula and referenced style data. Show actual formatting differences alongside intended cell and range changes. Different saved style data establishes a model change; check its display separately in the named reader when that matters to the decision.

Give every changed part an explicit result. Test harmless renumbering, an unchanged ID pointing to a different target, and a formatting change alongside a valid append. Report the verified interval, extent of structural comparison, untested reader behaviour and exact missing file or receipt needed for stronger recovery.

## Before handing off

- [ ] Identify the workbook and supported model features.
- [ ] Recalculate with the actual engine and inspect key outputs.
- [ ] Distinguish file identity, formula behavior and displayed values.
