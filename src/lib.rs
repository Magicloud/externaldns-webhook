#![warn(clippy::cargo)]
#![warn(clippy::complexity)]
#![warn(clippy::correctness)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::perf)]
#![warn(clippy::style)]
#![warn(clippy::suspicious)]
#![allow(clippy::future_not_send)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::wildcard_dependencies)]
#![doc = include_str!("../README.md")]

pub mod changes;
pub mod domain_filter;
pub mod endpoint;
mod provider;
mod status;
mod webhook;
mod webhook_json;

const MEDIATYPE: &str = "application/external.dns.webhook+json;version=1";

pub use provider::Provider;
pub use status::Status;
pub use webhook::Webhook;
