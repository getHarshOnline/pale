//! Cold Consult continuation through the public service composition.

use super::*;

#[test]
fn cold_consult_service_resumes_saved_start_without_reissuing_it() {
    let mut custody = None;
    exercise_consult_service_restart(|store, _, _, use_store| {
        let retained =
            custody.get_or_insert_with(|| ConformanceOperationCustody::new(store.clone()));
        use_store(retained);
    });
}
