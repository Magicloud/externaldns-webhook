#![feature(try_blocks)]

use serde::{Deserialize, Serialize};

pub mod changes;
pub mod domain_filter;
pub mod endpoint;
pub mod provider;
pub mod webhook;
mod webhook_json;

/// Container of either of the two items.
/// Rust `Result` works like `Either` in Haskell, but generally implies a good one or
/// a bad one. There is an `Either` crate, which is even weirder.
#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum IDoNotCareWhich<A, B> {
    One(A),
    Another(B),
}

const MEDIATYPE: &str = "application/external.dns.webhook+json;version=1";

pub use provider::Provider;
pub use webhook::Webhook;
