//! Check the host wall-clock and OS cryptographic-random implementations.

use std::time::{SystemTime, UNIX_EPOCH};

use pale_adapter_process::{OsCryptographicRandom, SystemWallClock};
use pale_ports::{CryptographicRandomSourcePort, WallClockSourcePort};

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("test host clock is after the Unix epoch")
        .as_secs()
}

#[test]
fn serves_each_caller_bounded_destination_from_the_os_source() {
    let mut empty = [];
    let mut producer_epoch_material = [0_u8; 16];
    let mut wider_domain_material = [0_u8; 32];

    OsCryptographicRandom
        .fill_bytes(&mut empty)
        .expect("OS cryptographic random source");
    OsCryptographicRandom
        .fill_bytes(&mut producer_epoch_material)
        .expect("OS cryptographic random source");
    OsCryptographicRandom
        .fill_bytes(&mut wider_domain_material)
        .expect("OS cryptographic random source");
}

#[test]
fn observes_whole_unix_seconds_within_the_call_interval() {
    let before = unix_seconds();
    let observed = SystemWallClock
        .observe_unix_time()
        .expect("portable wall-clock observation")
        .get();
    let after = unix_seconds();

    assert!(before <= observed);
    assert!(observed <= after);
}

#[test]
fn selected_sources_are_shareable_without_mutable_global_state() {
    fn assert_send_sync<T: Send + Sync>() {}

    assert_send_sync::<OsCryptographicRandom>();
    assert_send_sync::<SystemWallClock>();
}
