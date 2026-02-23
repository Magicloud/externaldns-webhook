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

use std::future::ready;
use std::str::FromStr;
use std::sync::Arc;
use std::{collections::HashSet, path::PathBuf};

use async_trait::async_trait;
use clap::Parser;
use core::fmt::Display;
use externaldns_webhook::{
    Provider, Status, Webhook,
    changes::Changes,
    domain_filter::DomainFilter,
    endpoint::{Endpoint, RecordType},
};
use eyre::{Result, eyre};
use opentelemetry::global;
use opentelemetry::metrics::Gauge;
use opentelemetry::trace::TracerProvider;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::MetricExporter;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::metrics::exporter::PushMetricExporter;
use opentelemetry_sdk::{Resource, logs::SdkLoggerProvider, trace::SdkTracerProvider};
use tokio::{
    fs,
    io::{AsyncBufReadExt, BufReader},
};
use tracing::{error, info, instrument, level_filters::LevelFilter, warn};
use tracing_error::ErrorLayer;
use tracing_subscriber::{
    EnvFilter, Layer,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let log_provider = match opentelemetry_otlp::LogExporter::builder()
        .with_tonic()
        .build()
    {
        Ok(log_exporter) => SdkLoggerProvider::builder()
            .with_resource(Resource::builder().with_service_name("e_d").build())
            .with_batch_exporter(log_exporter)
            .build(),
        Err(e) => {
            eprintln!("Cannot initialize OTLP log exporter: {e:?}");
            SdkLoggerProvider::builder()
                .with_batch_exporter(opentelemetry_stdout::LogExporter::default())
                .build()
        }
    };

    let trace_provider = match opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .build()
    {
        Ok(trace_exporter) => Some(
            SdkTracerProvider::builder()
                .with_resource(Resource::builder().with_service_name("e_d").build())
                .with_batch_exporter(trace_exporter)
                .build()
                .tracer("e_d"),
        ),
        Err(e) => {
            warn!(target: "OTLP", message = format!("Failed to initialize trace exporter: {e:?}"));
            None
        }
    };

    let r = tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_span_events(FmtSpan::NONE)
                .with_filter(EnvFilter::from_default_env()),
        )
        .with(ErrorLayer::default())
        .with(OpenTelemetryTracingBridge::new(&log_provider).with_filter(LevelFilter::INFO));
    if let Some(tp) = trace_provider {
        r.with(
            tracing_opentelemetry::layer()
                .with_tracer(tp)
                .with_filter(LevelFilter::INFO),
        )
        .try_init()?;
    } else {
        r.try_init()?;
    }

    color_eyre::install()?;

    let metric_exporter = MetricExporter::builder().with_tonic().build()?;
    // let metric_exporter = DebugMetricExporter;
    let metric_provider = SdkMeterProvider::builder()
        .with_periodic_exporter(metric_exporter)
        .with_resource(Resource::builder().with_service_name("e_d").build())
        .build();
    global::set_meter_provider(metric_provider);

    let meter = global::meter("e_d");
    let gauge = meter
        .u64_gauge("record_count") // Some docs uses "record.count", which would be dropped by Alloy.
        .with_description("Total records held by DnsMasq")
        .with_unit("records")
        .build();

    let args = Args::parse();

    let provider = Arc::new(Dnsmasq {
        domain_name: args.domain_name,
        conf_filename: args.conf_filename,
        gauge_record_count: gauge,
    });
    Webhook::new(provider.clone(), provider).start().await?;

    // log_provider.shutdown()?;
    // metric_provider.shutdown()?;

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
    gauge_record_count: Gauge<u64>,
}
#[async_trait]
impl Provider for Dnsmasq {
    #[instrument(skip_all)]
    async fn domain_filter(&self) -> Result<DomainFilter> {
        Ok(DomainFilter::Strings {
            include: Some(vec![self.domain_name.clone()]),
            exclude: None,
        })
    }

    #[instrument(skip_all)]
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

        self.gauge_record_count
            .record(result.len().try_into()?, &[]);

        Ok(result)
    }

    #[instrument(skip_all)]
    async fn apply_changes(&self, changes: Changes) -> Result<()> {
        let simple_show = |ep: &Endpoint| {
            let empty_string = String::new();
            let empty_vec = Vec::new();
            format!(
                "{}/{}",
                ep.dns_name.as_ref().unwrap_or(&empty_string),
                ep.targets.as_ref().unwrap_or(&empty_vec).join(",")
            )
        };
        let mut endpoints: HashSet<Endpoint> = self.records().await?.into_iter().collect();
        for i in changes.create {
            info!(target: "e_d report", message = format!("Insert: {}", simple_show(&i)));
            endpoints.insert(i);
        }
        for i in changes.delete {
            info!(target: "e_d report", message = format!("Delete: {}", simple_show(&i)));
            endpoints.remove(&i);
        }
        for i in changes.update {
            info!(target: "e_d report", message = format!("Update: {} -> {}", simple_show(&i.from), simple_show(&i.to)));
            endpoints.remove(&i.from);
            endpoints.insert(i.to);
        }

        self.gauge_record_count
            .record(endpoints.len().try_into()?, &[]);

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
                    Err(eyre!("Unsupported ExternalDNS endpoint: {endpoint:?}"))?;
                }
            }
            f.write_str("\n")?;
            Ok(())
        };
        let y: eyre::Result<()> = try_block();
        match y {
            Ok(()) => Ok(()),
            Err(e) => {
                error!(target: "EndpointED: Display", message = format!("{e:?}"));
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

#[allow(dead_code)]
#[derive(Debug)]
struct DebugMetricExporter;
impl PushMetricExporter for DebugMetricExporter {
    fn export(
        &self,
        metrics: &opentelemetry_sdk::metrics::data::ResourceMetrics,
    ) -> impl std::future::Future<Output = opentelemetry_sdk::error::OTelSdkResult> + Send {
        eprintln!("{metrics:?}");
        ready(Ok(()))
    }

    fn force_flush(&self) -> opentelemetry_sdk::error::OTelSdkResult {
        Ok(())
    }

    fn shutdown_with_timeout(
        &self,
        _timeout: std::time::Duration,
    ) -> opentelemetry_sdk::error::OTelSdkResult {
        Ok(())
    }

    fn temporality(&self) -> opentelemetry_sdk::metrics::Temporality {
        opentelemetry_sdk::metrics::Temporality::LowMemory
    }
}
