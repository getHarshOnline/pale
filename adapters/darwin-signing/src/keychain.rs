//! Native read-only lookup; this module has no key creation or update path.

use crate::{Refusal, Request};
use zeroize::Zeroizing;

#[cfg(feature = "qualification-fixture")]
pub(super) fn read_seed(request: &Request) -> Result<Zeroizing<Vec<u8>>, Refusal> {
    if request.service != "qualification-only" || request.account != "public-rfc8032-vector-1" {
        return Err(Refusal::KeyUnavailable);
    }
    // Published RFC 8032 section 7.1 test vector, never a production credential.
    Ok(Zeroizing::new(vec![
        0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec, 0x2c,
        0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03, 0x1c, 0xae,
        0x7f, 0x60,
    ]))
}

#[cfg(all(target_os = "macos", not(feature = "qualification-fixture")))]
pub(super) fn read_seed(request: &Request) -> Result<Zeroizing<Vec<u8>>, Refusal> {
    use security_framework::item::{CloudSync, ItemClass, ItemSearchOptions, SearchResult};
    let mut query = ItemSearchOptions::new();
    query
        .class(ItemClass::generic_password())
        .service(&request.service)
        .account(&request.account)
        .cloud_sync(CloudSync::MatchSyncNo)
        .skip_authenticated_items(true)
        .load_data(true)
        .limit(2);
    if let Some(group) = &request.access_group {
        query.access_group(group);
    }
    let results = query.search().map_err(|_| Refusal::KeyUnavailable)?;
    let mut seeds = Vec::new();
    let mut unexpected = false;
    for result in results {
        match result {
            SearchResult::Data(bytes) => seeds.push(Zeroizing::new(bytes)),
            _ => unexpected = true,
        }
    }
    if unexpected || seeds.len() != 1 {
        return Err(Refusal::KeyUnavailable);
    }
    seeds.pop().ok_or(Refusal::KeyUnavailable)
}

#[cfg(all(not(target_os = "macos"), not(feature = "qualification-fixture")))]
pub(super) fn read_seed(_: &Request) -> Result<Zeroizing<Vec<u8>>, Refusal> {
    Err(Refusal::KeyUnavailable)
}
