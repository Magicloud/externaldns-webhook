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
