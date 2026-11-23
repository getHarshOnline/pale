//! Analysis-generation debounce laws.

use super::*;

#[test]
fn debounce_and_stale_coordinates_are_monotone() {
    let start = Instant::now();
    let mut clock = AnalysisClock::default();
    let first = clock.schedule(start).unwrap();
    assert!(!clock.claim(first, start + Duration::from_millis(149)));
    let second = clock.schedule(start).unwrap();
    assert!(!clock.claim(first, start + Duration::from_millis(200)));
    assert!(clock.claim(second, start + Duration::from_millis(200)));
}
