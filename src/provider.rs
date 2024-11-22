use crate::{changes::Changes, domain_filter::DomainFilter, endpoint::Endpoint};
use anyhow::Result;
use rocket::async_trait;
use std::fmt::Debug;

/// Definition of the webhook interface.
/// This interface should be implemented by DNS service provider
/// that wish to communicate with ExternalDNS.
#[async_trait]
pub trait Provider: Send + Sync + Debug {
    /// Return the domains the provider could handle.
    async fn domain_filter(&self) -> DomainFilter;
    /// Return existing (previously registered by ExternalDNS) DNS records.
    async fn records(&self) -> Vec<Endpoint>;
    /// Make records changes asked by ExternalDNS.
    async fn apply_changes(&self, changes: Changes) -> Result<()>;
    /// Confirmation by providers, if any records should be adjusted, before making changes.
    async fn adjust_endpoints(&self, endpoints: Vec<Endpoint>) -> Result<Vec<Endpoint>> {
        Ok(endpoints)
    }
}
