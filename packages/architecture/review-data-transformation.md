# Review a record transformation

Identify the source version, proposed transformation, output contract and consumer. Apply the transformation and analytical-grain methods to the supplied inputs and observed results.

Return findings with their source or output locations, the cases checked, every write or delivery the transformation can cause and checks still needed. Put them in the review reply. Reviewing a pipeline neither runs it nor grants permission for database writes or downstream delivery.

## Before handing off

- [ ] Trace input rows and keys to the resulting records.
- [ ] Check missing, duplicate and changed records under the actual rule.
- [ ] Separate the observed output from downstream approval.
