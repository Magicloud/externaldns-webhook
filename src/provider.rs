use crate::{changes::Changes, domain_filter::DomainFilter, endpoint::Endpoint};
use anyhow::Result;
use rocket::async_trait;
use std::fmt::Debug;

#[async_trait]
pub trait Provider: Send + Sync + Debug {
    async fn domain_filter(&self) -> DomainFilter;
    async fn records(&self) -> Vec<Endpoint>;
    async fn apply_changes(&self, changes: Changes) -> Result<()>;
    async fn adjust_endpoints(&self, endpoints: Vec<Endpoint>) -> Result<Vec<Endpoint>> {
        Ok(endpoints)
    }
}
