#![feature(try_blocks)]

use serde::{Deserialize, Serialize};

pub mod changes;
pub mod domain_filter;
pub mod endpoint;
pub mod provider;
pub mod webhook;
mod webhook_json;

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum IDoNotCareWhich<A, B> {
    One(A),
    Another(B),
}

const MEDIATYPE: &str = "application/external.dns.webhook+json;version=1";
