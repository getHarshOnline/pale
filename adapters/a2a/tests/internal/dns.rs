//! Physical-job cancellation tests, not whole-host shutdown qualification.

use std::{
    future::poll_fn,
    sync::{mpsc, Condvar, Mutex},
    task::Poll,
    time::{Duration, Instant},
};

use super::*;

struct ReleaseOnDrop(Arc<(Mutex<bool>, Condvar)>);

impl ReleaseOnDrop {
    fn release(&self) {
        *self.0 .0.lock().unwrap() = true;
        self.0 .1.notify_all();
    }
}

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.release();
    }
}

fn resolver() -> BoundedResolver {
    BoundedResolver::new(
        NonZeroUsize::new(1).unwrap(),
        NonZeroUsize::new(2).unwrap(),
        Arc::new(AtomicUsize::new(0)),
    )
}

fn addresses() -> std::vec::IntoIter<SocketAddr> {
    vec![SocketAddr::from(([127, 0, 0, 1], 0))].into_iter()
}

#[test]
fn cancelled_waiter_retains_physical_job_permit_until_work_returns() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    // Declared after the runtime: failure releases the job before runtime Drop.
    let release = ReleaseOnDrop(Arc::new((Mutex::new(false), Condvar::new())));
    let barrier = release.0.clone();
    let (entered, entry) = mpsc::sync_channel(1);
    let resolver = resolver();
    let mut pending = resolver.physical_lookup(move || {
        entered.send(()).unwrap();
        let (released, timeout) = barrier
            .1
            .wait_timeout_while(
                barrier.0.lock().unwrap(),
                Duration::from_secs(3),
                |released| !*released,
            )
            .unwrap();
        if timeout.timed_out() && !*released {
            return Err(io::Error::other("test barrier watchdog"));
        }
        Ok(addresses())
    });
    runtime.block_on(poll_fn(|context| {
        assert!(pending.as_mut().poll(context).is_pending());
        Poll::Ready(())
    }));
    entry.recv_timeout(Duration::from_secs(2)).unwrap();
    assert_eq!(resolver.outstanding.load(Ordering::Acquire), 1);
    drop(pending);
    for _ in 0..3 {
        assert!(runtime
            .block_on(resolver.physical_lookup(|| Ok(addresses())))
            .is_err());
        assert_eq!(resolver.outstanding.load(Ordering::Acquire), 1);
        assert_eq!(resolver.permits.available_permits(), 0);
    }
    release.release();
    let deadline = Instant::now() + Duration::from_secs(2);
    while resolver.outstanding.load(Ordering::Acquire) != 0
        || resolver.permits.available_permits() != 1
    {
        assert!(
            Instant::now() < deadline,
            "physical lookup completion watchdog"
        );
        std::thread::yield_now();
    }
    let result = runtime
        .block_on(resolver.physical_lookup(|| Ok(addresses())))
        .unwrap();
    assert_eq!(result.collect::<Vec<_>>(), addresses().collect::<Vec<_>>());
}

#[test]
fn physical_lookup_errors_and_address_limits_release_the_same_permit() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let resolver = resolver();
    for count in [0, 3] {
        let result = runtime.block_on(resolver.physical_lookup(move || {
            Ok(vec![SocketAddr::from(([127, 0, 0, 1], 0)); count].into_iter())
        }));
        assert!(result.is_err());
        assert_eq!(resolver.outstanding.load(Ordering::Acquire), 0);
        assert_eq!(resolver.permits.available_permits(), 1);
    }
    let failure = resolver.physical_lookup(|| -> io::Result<std::vec::IntoIter<SocketAddr>> {
        Err(io::Error::other("private resolver detail must not escape"))
    });
    let error = match runtime.block_on(failure) {
        Ok(_) => panic!("failed lookup returned addresses"),
        Err(error) => error,
    };
    assert_eq!(error.to_string(), "resolver lookup failed");
    assert_eq!(resolver.outstanding.load(Ordering::Acquire), 0);
    assert_eq!(resolver.permits.available_permits(), 1);
}

#[test]
fn unpolled_lookup_does_not_start_physical_work() {
    let resolver = resolver();
    let pending = resolver.physical_lookup(|| -> io::Result<std::vec::IntoIter<SocketAddr>> {
        panic!("an unpolled lookup must not execute");
    });
    assert_eq!(resolver.outstanding.load(Ordering::Acquire), 0);
    assert_eq!(resolver.permits.available_permits(), 0);
    drop(pending);
    assert_eq!(resolver.permits.available_permits(), 1);
}
