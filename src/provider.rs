use async_trait::async_trait;
use eyre::Result;
use std::fmt::Debug;

use crate::{changes::Changes, domain_filter::DomainFilter, endpoint::Endpoint};

/// Definition of the webhook interface.
/// This interface should be implemented by DNS service provider
/// that wish to communicate with External-DNS.
#[async_trait]
pub trait Provider: Send + Sync + Debug {
    /// Return the domains the provider could handle.
    /// async fn domain_filter(&self) -> Result<DomainFilter>;
    async fn domain_filter(&self) -> Result<DomainFilter>;
    /// Return existing (previously registered by External-DNS) DNS records.
    /// async fn records(&self) -> Result<Vec<Endpoint>>;
    async fn records(&self) -> Result<Vec<Endpoint>>;
    /// Make records changes asked by External-DNS.
    /// async fn apply_changes(&self, changes: Changes) -> Result<()>;
    async fn apply_changes(&self, changes: Changes) -> Result<()>;
    /// Confirmation by providers, if any records should be adjusted, before making changes.
    /// async fn adjust_endpoints(&self, endpoints: Vec<Endpoint>) -> Result<Vec<Endpoint>>;
    async fn adjust_endpoints(&self, endpoints: Vec<Endpoint>) -> Result<Vec<Endpoint>> {
        Ok(endpoints)
    }
}
