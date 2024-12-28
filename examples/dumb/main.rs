use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashSet;
use externaldns_webhook::{
    changes::Changes, domain_filter::DomainFilter, endpoint::Endpoint, provider::Provider,
    webhook::Webhook,
};
use logcall::logcall;
use std::sync::Arc;

#[logcall(ok = "debug", err = "error")]
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    Webhook::new(Arc::new(DumbDns {
        domain_filter: DomainFilter::Strings {
            include: None,
            exclude: None,
        },
        fqdns: DashSet::new(),
    }))
    .start()
    .await?;
    Ok(())
}

#[derive(Debug)]
struct DumbDns {
    domain_filter: DomainFilter,
    fqdns: DashSet<Endpoint>,
}
#[async_trait]
impl Provider for DumbDns {
    #[logcall("debug")]
    async fn domain_filter(&self) -> DomainFilter {
        self.domain_filter.clone()
    }

    #[logcall("debug")]
    async fn records(&self) -> Vec<Endpoint> {
        self.fqdns.iter().map(|x| x.clone()).collect()
    }

    #[logcall("debug")]
    async fn apply_changes(&self, changes: Changes) -> Result<()> {
        for i in changes.create {
            self.fqdns.insert(i);
        }

        for i in changes.delete {
            self.fqdns.remove(&i);
        }

        for i in changes.update {
            // No locking since it is dumb.
            self.fqdns.remove(&i.from);
            self.fqdns.insert(i.to);
        }

        Ok(())
    }
}
