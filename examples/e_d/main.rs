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

use eyre::{Result, eyre};
use async_trait::async_trait;
use clap::Parser;
use core::fmt::Display;
use dashmap::DashSet;
use externaldns_webhook::{
    Provider, Status, Webhook,
    changes::Changes,
    domain_filter::DomainFilter,
    endpoint::{Endpoint, RecordType},
};
use logcall::logcall;
use prometheus::Gauge;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use tokio::{
    fs,
    io::{AsyncBufReadExt, BufReader},
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    env_logger::init();
    let recorder = metrics_prometheus::install();
    let g = Gauge::new("total_records", "Total number of current holding FQDNs.")?;
    recorder.register_metric(g.clone());
    let args = Args::parse();

    let provider = Arc::new(Dnsmasq {
        domain_name: args.domain_name,
        conf_filename: args.conf_filename,
        gauge_record_count: g,
    });
    Webhook::new(provider.clone(), provider).start().await?;

    Ok(())
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Domain name
    #[arg(long)]
    domain_name: String,
    /// Dnsmasq configuration file path
    #[arg(long)]
    conf_filename: PathBuf,
}

#[derive(Debug)]
struct Dnsmasq {
    domain_name: String,
    conf_filename: PathBuf,
    gauge_record_count: Gauge,
}
#[async_trait]
impl Provider for Dnsmasq {
    #[logcall("info")]
    async fn domain_filter(&self) -> Result<DomainFilter> {
        Ok(DomainFilter::Strings {
            include: Some(vec![self.domain_name.clone()]),
            exclude: None,
        })
    }

    #[logcall("info")]
    async fn records(&self) -> Result<Vec<Endpoint>> {
        let file = fs::OpenOptions::new()
            .read(true)
            .open(&self.conf_filename)
            .await?;
        let mut conf = BufReader::new(file).lines();
        let mut buf = Vec::new();
        let mut result = Vec::new();
        while let Some(l) = conf.next_line().await? {
            if l.is_empty() {
                if !buf.is_empty() {
                    result.push(EndpointED::from_str(&buf.join("\n"))?.0);
                }
                buf = vec![];
            } else {
                buf.push(l);
            }
        }
        if !buf.is_empty() {
            result.push(EndpointED::from_str(&buf.join("\n"))?.0);
        }
        Ok(result)
    }

    #[logcall("info")]
    async fn apply_changes(&self, changes: Changes) -> Result<()> {
        let endpoints: DashSet<Endpoint> =
            self.records().await?.into_iter().collect::<DashSet<_>>();
        for i in changes.create {
            endpoints.insert(i);
        }
        for i in changes.delete {
            endpoints.remove(&i);
        }
        for i in changes.update {
            endpoints.remove(&i.from);
            endpoints.insert(i.to);
        }

        self.gauge_record_count
            .set(f64::from(u32::try_from(endpoints.len())?));

        fs::write(
            &self.conf_filename,
            endpoints
                .into_iter()
                .map(|x| format!("{}", EndpointED(x)))
                .collect::<Vec<String>>()
                .join("\n"),
        )
        .await?;
        Ok(())
    }
}

#[async_trait]
impl Status for Dnsmasq {}

// address=/domain[/domain]/ip
// cname=cname[,cname],target[,ttl]
// txt-record=name[,"text"]*
// ptr-record=name[,target]
struct EndpointED(Endpoint);
impl Display for EndpointED {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let endpoint = self.0.clone();
        let mut try_block = || {
            if endpoint.dns_name.is_none() || endpoint.targets.is_none() {
                Err(eyre::eyre!("ExternalDNS did not give enough data"))?;
            }
            let targets = endpoint.targets.as_ref().unwrap();
            let dns_name = endpoint.dns_name.as_ref().unwrap();

            f.write_fmt(format_args!("# {}\n", serde_json::to_string(&endpoint)?))?;
            match endpoint.record_type {
                Some(RecordType::A) => {
                    for target in targets {
                        f.write_fmt(format_args!("address=/{dns_name}/{target}\n"))?;
                    }
                }
                Some(RecordType::CNAME) => {
                    let ttl = endpoint
                        .record_ttl
                        .map(|ttl| format!(",{ttl}"))
                        .unwrap_or_default();
                    for target in targets {
                        f.write_fmt(format_args!("cname={dns_name},{target}{ttl}\n"))?;
                    }
                }
                Some(RecordType::TXT) => {
                    let targets = targets
                        .iter()
                        .map(|t| format!(",{t}"))
                        .collect::<Vec<_>>()
                        .concat();
                    f.write_fmt(format_args!("txt-record={dns_name}{targets}"))?;
                }
                Some(RecordType::PTR) => f.write_fmt(format_args!(
                    "ptr-record={},{}",
                    dns_name,
                    targets
                        .first()
                        .ok_or_else(|| eyre!("No target found in PTR request"))?
                ))?,
                _ => {
                    log::info!("Unsupported ExternalDNS endpoint: {endpoint:?}");
                }
            }
            f.write_str("\n")?;
            Ok(())
        };
        let y: eyre::Result<()> = try_block();
        match y {
            Ok(()) => Ok(()),
            Err(e) => {
                log::error!("{e:?}");
                Err(std::fmt::Error)
            }
        }
    }
}
impl FromStr for EndpointED {
    type Err = eyre::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let mut lines = s.lines();
        let first_line = lines
            .next()
            .and_then(|l| l.strip_prefix("# "))
            .ok_or_else(|| eyre!("Input does not contain a commented first line: {s}"))?;
        let endpoint = serde_json::from_str(first_line)?;
        Ok(Self(endpoint))
    }
}
