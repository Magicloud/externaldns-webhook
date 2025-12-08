use crate::{
    MEDIATYPE, domain_filter::DomainFilter, endpoint::Endpoint, provider::Provider, status::Status,
    webhook_json::WebhookJson,
};
use actix_web::{
    App, HttpServer, ResponseError, get,
    guard::GuardContext,
    http::{StatusCode, header::Accept},
    middleware::Logger,
    post,
    web::{Data, Json},
};
use logcall::logcall;
use serde_json::{Value, from_value};
use std::{fmt::Display, sync::Arc};

/// Setup of the HTTP server
/// The listening addresses and ports are specified in External-DNS,
/// hence they are not exposed to be configurable.
#[derive(Debug)]
pub struct Webhook {
    provider_address: String,
    provider_port: u16,
    dns_manager: Arc<dyn Provider>,

    exposed_address: String,
    exposed_port: u16,
    status: Arc<dyn Status>,
}
impl Webhook {
    /// Constructor of `Webhook`.
    #[logcall("debug")]
    pub fn new(dns_manager: Arc<dyn Provider>, status: Arc<dyn Status>) -> Self {
        // As much as the http values are customizable, those are the value asked in ExternalDNS doc.
        Self {
            provider_address: "127.0.0.1".to_string(),
            provider_port: 8888,
            dns_manager,
            exposed_address: "0.0.0.0".to_string(),
            exposed_port: 8080,
            status,
        }
    }

    /// Start the webhook server, and healthz web server.
    /// # Errors
    ///
    /// any errors that could happen
    #[logcall(ok = "debug", err = "error")]
    pub async fn start(&self) -> eyre::Result<()> {
        let x = self.status.clone();
        let exposed = HttpServer::new(move || {
            App::new()
                .app_data(Data::new(x.clone()))
                .service(get_healthz)
                .service(get_metrics)
        })
        .workers(2)
        .bind((self.exposed_address.clone(), self.exposed_port))?
        .run();

        let x = self.dns_manager.clone();
        let provider = HttpServer::new(move || {
            App::new()
                .wrap(Logger::default())
                .app_data(Data::new(x.clone()))
                .service(get_root)
                .service(get_records)
                .service(post_records)
                .service(post_adjustendpoints)
        })
        .workers(4)
        .bind((self.provider_address.clone(), self.provider_port))?
        .run();

        tokio::spawn(exposed);
        provider.await?;

        Ok(())
    }
}

// Initialisation and negotiates headers and returns domain filter.
// Returns 200/500
#[logcall("debug")]
#[get("/", guard = "media_type_guard")]
async fn get_root(
    dns_manager: Data<Arc<dyn Provider>>,
) -> Result<WebhookJson<DomainFilter>, ErrorWraper> {
    dns_manager
        .domain_filter()
        .await
        .map(|x| WebhookJson(Json(x)))
        .map_err(ErrorWraper)
}

// Returns the current records.
// Returns 200/500
#[logcall("debug")]
#[get("/records", guard = "media_type_guard")]
async fn get_records(
    dns_manager: Data<Arc<dyn Provider>>,
) -> Result<WebhookJson<Vec<Endpoint>>, ErrorWraper> {
    dns_manager
        .records()
        .await
        .map(|x| WebhookJson(Json(x)))
        .map_err(ErrorWraper)
}

// Applies the changes.
// Returns 204/500
#[logcall("debug")]
#[post("/records")]
async fn post_records(
    dns_manager: Data<Arc<dyn Provider>>,
    changes: Json<Value>,
) -> Result<(), ErrorWraper> {
    let json = changes.into_inner();
    match from_value(json.clone()) {
        Ok(changes) => dns_manager
            .apply_changes(changes)
            .await
            .map_err(ErrorWraper),
        Err(e) => {
            log::warn!("{json}");
            Err(ErrorWraper(e.into()))
        }
    }
}

// Executes the AdjustEndpoints method.
// Returns 200/500
#[logcall("debug")]
#[post("/adjustendpoints", guard = "media_type_guard")]
async fn post_adjustendpoints(
    dns_manager: Data<Arc<dyn Provider>>,
    endpoints: Json<Value>,
) -> Result<Json<Vec<Endpoint>>, ErrorWraper> {
    let json = endpoints.into_inner();
    match from_value(json.clone()) {
        Ok(endpoints) => dns_manager
            .adjust_endpoints(endpoints)
            .await
            .map(Json)
            .map_err(ErrorWraper),
        Err(e) => {
            log::warn!("{json}");
            Err(ErrorWraper(e.into()))
        }
    }
}

// Only takes and gives `MEDIATYPE`, why guard.
fn media_type_guard(ctx: &GuardContext<'_>) -> bool {
    ctx.header::<Accept>()
        .is_some_and(|h| h.preference() == MEDIATYPE)
}

// #[logcall("debug")]
#[get("/healthz")]
async fn get_healthz(status: Data<Arc<dyn Status>>) -> (String, StatusCode) {
    status.healthz().await
}

// #[logcall("debug")]
#[get("/metrics")]
async fn get_metrics(status: Data<Arc<dyn Status>>) -> (String, StatusCode) {
    status.metrics().await
}

#[derive(Debug)]
struct ErrorWraper(eyre::Error);
impl Display for ErrorWraper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.0))
    }
}
impl ResponseError for ErrorWraper {}
