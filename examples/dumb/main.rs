use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashSet;
use externaldns_webhook::{
    Provider, Status, Webhook, changes::Changes, domain_filter::DomainFilter, endpoint::Endpoint,
};
use logcall::logcall;
use metrics::Gauge;
use std::sync::Arc;

#[logcall(ok = "debug", err = "error")]
#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let _recorder = metrics_prometheus::install();
    let g = metrics::gauge!("dns_dumb_total_records", "kind" => "dns_provider");
    metrics::describe_gauge!(
        "dns_dumb_total_records",
        "How many records (of all kinds) are held by this DNS provider."
    );
    // recorder.register_metric(g.clone());

    let x = Arc::new(DumbDns {
        domain_filter: DomainFilter::Strings {
            include: None,
            exclude: None,
        },
        fqdns: DashSet::new(),
        gauge_record_count: g,
    });
    Webhook::new(x.clone(), x).start().await?;
    Ok(())
}

#[derive(Debug)]
struct DumbDns {
    domain_filter: DomainFilter,
    fqdns: DashSet<Endpoint>,
    gauge_record_count: Gauge,
}
#[async_trait]
impl Provider for DumbDns {
    #[logcall("debug")]
    async fn domain_filter(&self) -> Result<DomainFilter> {
        Ok(self.domain_filter.clone())
    }

    #[logcall("debug")]
    async fn records(&self) -> Result<Vec<Endpoint>> {
        Ok(self.fqdns.iter().map(|x| x.clone()).collect())
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
        self.gauge_record_count
            .set(f64::from(i32::try_from(self.fqdns.len()).unwrap_or(-1)));

        Ok(())
    }
}
#[async_trait]
impl Status for DumbDns {}
