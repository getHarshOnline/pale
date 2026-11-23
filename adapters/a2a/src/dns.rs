//! Bound outstanding OS lookups without pretending they are abortable.

use std::{
    io,
    net::{SocketAddr, ToSocketAddrs},
    num::NonZeroUsize,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub(crate) struct BoundedResolver {
    permits: Arc<Semaphore>,
    outstanding: Arc<AtomicUsize>,
    maximum_addresses: NonZeroUsize,
}

impl BoundedResolver {
    pub(crate) fn new(
        jobs: NonZeroUsize,
        addresses: NonZeroUsize,
        outstanding: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            permits: Arc::new(Semaphore::new(jobs.get())),
            outstanding,
            maximum_addresses: addresses,
        }
    }

    // Both the OS lookup and deterministic tests use this physical-job path.
    // The closure selects work, not a different permit or cancellation policy.
    fn physical_lookup<F, I>(&self, work: F) -> Resolving
    where
        F: FnOnce() -> io::Result<I> + Send + 'static,
        I: Iterator<Item = SocketAddr>,
    {
        let acquired = self.permits.clone().try_acquire_owned();
        let outstanding = self.outstanding.clone();
        let maximum_addresses = self.maximum_addresses.get();
        Box::pin(async move {
            let permit = acquired.map_err(|_| io::Error::other("resolver work limit"))?;
            outstanding.fetch_add(1, Ordering::AcqRel);
            let permit = LookupPermit {
                _permit: permit,
                outstanding,
            };
            let lookup = tokio::task::spawn_blocking(move || -> io::Result<Addrs> {
                // The permit belongs to the physical job. Dropping the
                // request future must not release a still-running OS lookup.
                let _permit = permit;
                let mut addresses = Vec::new();
                for address in work()? {
                    if addresses.len() == maximum_addresses {
                        return Err(io::Error::other("resolver address limit"));
                    }
                    addresses.push(address);
                }
                if addresses.is_empty() {
                    return Err(io::Error::other("resolver returned no addresses"));
                }
                Ok(Box::new(addresses.into_iter()))
            });
            lookup
                .await
                .map_err(|_| io::Error::other("resolver worker unavailable"))?
                .map_err(|_| io::Error::other("resolver lookup failed").into())
        })
    }
}

struct LookupPermit {
    _permit: OwnedSemaphorePermit,
    outstanding: Arc<AtomicUsize>,
}

impl Drop for LookupPermit {
    fn drop(&mut self) {
        self.outstanding.fetch_sub(1, Ordering::AcqRel);
    }
}

impl Resolve for BoundedResolver {
    fn resolve(&self, name: Name) -> Resolving {
        self.physical_lookup(move || (name.as_str(), 0).to_socket_addrs())
    }
}

#[cfg(test)]
#[path = "../tests/internal/dns.rs"]
mod tests;
