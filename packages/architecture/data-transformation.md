# Check a record transformation

Use this method for a dataset or record pipeline. File and image preparation have different checks. Identify the input version, transformation revision, output and downstream consumer. Keep original observations, normalised records, current views and delivery results distinguishable. The source owner decides meanings, permitted use and retention.

## Establish the output contract

Define what one record represents, its identity, field order, types, units, null and missing-value meanings, and time basis. Compare every source before combining them. Matching names or column counts do not prove matching positions, types or meanings. Map differences explicitly. Apply the analytical-grain method to joins, aggregations and the populations included.

Test the key function with permitted inputs, including embedded separators, null versus empty values, reused local IDs and distinct sources. A configured conflict key does not establish uniqueness or safe reruns. Changing encoding or deduplication may reinterpret stored records and requires the owner's compatibility and reconciliation decision.

## Separate update coverage from observed output

Identify the actual change signal, selection window, watermark and time zone for incremental work. Cover late arrivals, corrections to old records and removals. Show uncovered intervals explicitly. The newest booking or event time does not establish when data was acquired or which updates were covered.

When incremental and full recomputation are promised to agree, compare them on the same accepted source version. Compare keyed values, omissions and multiplicity, not just totals. Keep predecessor and current identities when changes trigger downstream work. Replacing a baseline or overlapping runs must neither invent nor lose a change occurrence.

## Check the complete execution boundary

Before running a command, inspect the transformation, inherited actions, wrappers, database hooks and downstream dispatch. Identify reads, writes, schema changes and outside effects. Use a permitted disposable target or read-only route capable of proving the intended claim. A successful SELECT or build does not establish hook completion, delivery or acceptance.

Choose relevant cases: reordered fields, key collisions, duplicate rows, unknown values, time-boundary records and interrupted updates. Verify coverage, meaning and effects separately. Identify which tests were selected and which assertions ran; success with no relevant checks is not behavioural evidence. Keep durable cases in the pipeline's tests. In the review reply, distinguish source and compiled-output inspection, executed checks, actual results and acceptance still needed.
