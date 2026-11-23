
The completion cannot be copied into two dispatch releases. Its candidate can
be borrowed or cloned as historical data; that data cannot reconstruct this
completion or its database evidence.

```compile_fail
use pale_application::resident::CompletedOperationObservationCustody;

fn duplicate(
    completed: CompletedOperationObservationCustody,
) -> (CompletedOperationObservationCustody, CompletedOperationObservationCustody) {
    (completed.clone(), completed)
}
```
