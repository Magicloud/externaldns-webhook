#![feature(try_blocks)]
#![feature(let_chains)]

use anyhow::Result;
use clap::Parser;
use core::fmt::Display;
use externaldns_webhook::endpoint::RecordType;
use externaldns_webhook::webhook::Webhook;
use externaldns_webhook::{
    changes::Changes, domain_filter::DomainFilter, endpoint::Endpoint, provider::Provider,
};
use itertools::Itertools;
use logcall::logcall;
use nonempty::NonEmpty;
use rocket::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::str::FromStr;
use std::sync::Arc;
use std::{net::IpAddr, path::PathBuf};
use surrealdb::engine::local::{Db, SurrealKV};
use surrealdb::{RecordIdKey, Surreal};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let args = Args::parse();

    let db = Surreal::new::<SurrealKV>(args.db_filename).await?;
    db.use_ns("dnsmasq").use_db("dnsmasq").await?;

    let provider = Dnsmasq {
        domain_name: args.domain_name,
        conf_filename: args.conf_filename,
        extra_db: db,
        override_a: args.override_ip,
        override_cname: args.override_host,
    };
    Webhook::new(Arc::new(provider)).start().await?;

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
    /// Extra data DB file path
    #[arg(long)]
    db_filename: PathBuf,
    /// Override the host target of External DNS CNAME request
    #[arg(long)]
    override_host: Option<String>,
    /// Override the IP target of External DNS A/AAAA request
    #[arg(long)]
    override_ip: Option<IpAddr>,
}

#[derive(Debug)]
struct Dnsmasq {
    domain_name: String,
    conf_filename: PathBuf,
    extra_db: Surreal<Db>,
    override_cname: Option<String>,
    override_a: Option<IpAddr>,
}
#[async_trait]
impl Provider for Dnsmasq {
    #[logcall("info")]
    async fn domain_filter(&self) -> DomainFilter {
        DomainFilter::Strings {
            include: Some(vec![self.domain_name.clone()]),
            exclude: None,
        }
    }

    #[logcall("info")]
    async fn records(&self) -> Vec<Endpoint> {
        self.extra_db
            .select("endpoint")
            .await
            .map(|x| x.into_iter().map(|y: EndpointDBItem| y.0).collect())
            .unwrap_or(vec![])
    }

    #[logcall("info")]
    async fn apply_changes(&self, changes: Changes) -> Result<()> {
        log::info!("{} items to delete", changes.delete.len());
        for i in changes.delete {
            let _: Option<EndpointDBItem> = self
                .extra_db
                .delete(("endpoint", EndpointDBItem(i)))
                .await?;
        }
        log::info!("{} items to update", changes.update.len());
        for i in changes.update {
            let _: Option<EndpointDBItem> = self
                .extra_db
                .update(("endpoint", EndpointDBItem(i.from)))
                .content(EndpointDBItem(i.to))
                .await?;
        }
        log::info!("{} items to create", changes.create.len());
        for i in changes.create {
            let _: Option<EndpointDBItem> = self
                .extra_db
                .create(("endpoint", EndpointDBItem(i.clone())))
                .content(EndpointDBItem(i))
                .await?;
        }

        let mut lines = vec![];
        let endpoints: Vec<EndpointDBItem> = self.extra_db.select("endpoint").await?;
        for (k, endpoints) in endpoints
            .into_iter()
            .map(|x| x.0.clone())
            .chunk_by(|x| x.record_type.clone())
            .into_iter()
        {
            match k {
                Some(RecordType::A) | Some(RecordType::AAAA) => {
                    if let Some(ip) = self.override_a {
                        let domains = NonEmpty::from_vec(
                            endpoints
                                .into_iter()
                                .filter_map(|endpoint| endpoint.dns_name)
                                .collect(),
                        );
                        if let Some(domains) = domains {
                            lines.push(format!("{}", Record::Address { domains, ip }));
                        }
                    } else {
                        // Since Dnsmasq 2.86, multiple `address` records with the
                        // same FQDN and different IPs is suported, I guess it is
                        // not necessary to reorganize External-DNS requests to
                        // have unique IP per record.
                        // The following code turns "one fqdn to multiple target"
                        // to "multiple fqdn to one target"
                        for (target, record) in endpoints
                            .into_iter()
                            .filter_map(|x| {
                                let x: Option<_> = try { (x.dns_name?, x.targets?) };
                                x.map(|(dns_name, targets)| {
                                    targets
                                        .into_iter()
                                        .map(|target| (dns_name.clone(), target))
                                        .collect::<Vec<_>>()
                                })
                            })
                            .flatten()
                            .chunk_by(|(_dns_name, target)| target.clone())
                            .into_iter()
                        {
                            let ip = IpAddr::from_str(&target);
                            let domains = NonEmpty::from_vec(
                                record.into_iter().map(|(dns_name, _)| dns_name).collect(),
                            );
                            if let Ok(ip) = ip
                                && let Some(domains) = domains
                            {
                                lines.push(format!("{}", Record::Address { domains, ip }));
                            }
                        }
                    }
                }
                Some(RecordType::CNAME) => {
                    if let Some(host) = &self.override_cname {
                        for (ttl, endpoints) in endpoints
                            .into_iter()
                            .chunk_by(|endpoint| endpoint.record_ttl)
                            .into_iter()
                        {
                            let cnames = NonEmpty::from_vec(
                                endpoints
                                    .into_iter()
                                    .filter_map(|endpoint| endpoint.dns_name)
                                    .collect(),
                            );
                            if let Some(cnames) = cnames {
                                lines.push(format!(
                                    "{}",
                                    Record::Cname {
                                        cnames,
                                        target: host.clone(),
                                        ttl
                                    }
                                ));
                            }
                        }
                    } else {
                        for ((target, ttl), record) in endpoints
                            .into_iter()
                            .filter_map(|x| {
                                let x: Option<_> = try { (x.dns_name?, x.targets?, x.record_ttl) };
                                x.map(|(dns_name, targets, record_ttl)| {
                                    targets
                                        .into_iter()
                                        .map(|target| (dns_name.clone(), (target, record_ttl)))
                                        .collect::<Vec<_>>()
                                })
                            })
                            .flatten()
                            .chunk_by(|(_dns_name, target_record_ttl)| target_record_ttl.clone())
                            .into_iter()
                        {
                            let cnames = NonEmpty::from_vec(
                                record.into_iter().map(|(cname, _)| cname).collect(),
                            );
                            if let Some(cnames) = cnames {
                                lines.push(format!(
                                    "{}",
                                    Record::Cname {
                                        cnames,
                                        target,
                                        ttl
                                    }
                                ));
                            }
                        }
                    }
                }
                Some(RecordType::TXT) => {
                    for endpoint in endpoints {
                        let name = endpoint.dns_name;
                        let texts = endpoint.targets;
                        if let Some(name) = name {
                            lines.push(format!("{}", Record::TxtRecord { name, texts }));
                        }
                    }
                }
                Some(RecordType::PTR) => {
                    for endpoint in endpoints {
                        let name = endpoint.dns_name;
                        let target = endpoint
                            .targets
                            .and_then(|targets| targets.first().cloned());
                        if let Some(name) = name {
                            lines.push(format!("{}", Record::PtrRecord { name, target }));
                        }
                    }
                }
                _ => (),
            }
        }

        log::info!("{} lines to sync", lines.len());
        tokio::fs::write(&self.conf_filename, lines.join("\n").as_bytes()).await?;

        let mut hasher = Sha512::new();
        hasher.update(tokio::fs::read(&self.conf_filename).await?);
        let result = hasher.finalize().to_vec();
        let _: Option<Vec<u8>> = self
            .extra_db
            .upsert(("dnsmasq-digest", "dnsmasq-digest"))
            .content(result)
            .await?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct EndpointDBItem(Endpoint);
impl Into<RecordIdKey> for EndpointDBItem {
    fn into(self) -> RecordIdKey {
        let str = format!(
            "{}{:?}{:?}{}",
            self.0.dns_name.unwrap_or_default(),
            self.0.targets.unwrap_or_default(),
            self.0
                .record_type
                .unwrap_or(externaldns_webhook::endpoint::RecordType::NS),
            self.0.record_ttl.unwrap_or_default()
        );
        RecordIdKey::from(str)
    }
}

// address=/domain[/domain]/ip
// cname=cname[,cname],target[,ttl]
// txt-record=name[,"text"]*
// ptr-record=name[,target]

enum Record {
    Address {
        domains: NonEmpty<String>,
        ip: IpAddr,
    },
    Cname {
        cnames: NonEmpty<String>,
        target: String,
        ttl: Option<u32>,
    },
    TxtRecord {
        name: String,
        texts: Option<Vec<String>>,
    },
    PtrRecord {
        name: String,
        target: Option<String>,
    },
}
impl Display for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Record::Address { domains, ip } => {
                f.write_str("address=")?;
                for i in domains {
                    f.write_str(&format!("/{i}"))?;
                }
                f.write_str(&format!("{ip}"))?;
            }
            Record::Cname {
                cnames,
                target,
                ttl,
            } => {
                f.write_str("cname=")?;
                for i in cnames {
                    f.write_str(&format!("{i},"))?;
                }
                f.write_str(&target)?;
                if let Some(ttl) = ttl {
                    f.write_str(&format!(",{ttl}"))?;
                }
            }
            Record::TxtRecord { name, texts } => {
                f.write_str("txt-record=")?;
                f.write_str(name)?;
                if let Some(texts) = texts {
                    for i in texts {
                        f.write_str(&format!(",\"{i}\""))?;
                    }
                }
            }
            Record::PtrRecord { name, target } => {
                f.write_str("ptr_record=")?;
                f.write_str(name)?;
                if let Some(target) = target {
                    f.write_str(target)?;
                }
            }
        }
        Ok(())
    }
}
