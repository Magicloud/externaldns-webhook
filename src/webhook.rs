use crate::{
    MEDIATYPE, domain_filter::DomainFilter, endpoint::Endpoint, provider::Provider, status::Status,
    webhook_json::WebhookJson,
};
use actix_web::{
    App, HttpServer, get,
    guard::GuardContext,
    http::{StatusCode, header::Accept},
    middleware::Logger,
    post,
    web::{Data, Json},
};
use logcall::logcall;
use serde_json::{Value, from_value};
use std::sync::Arc;

/// Setup of the HTTP server
/// The listening addresses and ports are specified in ExternalDNS,
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
    pub fn new(dns_manager: Arc<dyn Provider>, status: Arc<dyn Status>) -> Webhook {
        // As much as the http values are customizable, those are the value asked in ExternalDNS doc.
        Webhook {
            provider_address: "127.0.0.1".to_string(),
            provider_port: 8888,
            dns_manager,
            exposed_address: "0.0.0.0".to_string(),
            exposed_port: 8080,
            status,
        }
    }

    /// Start the webhook server, and healthz web server.
    #[logcall(ok = "debug", err = "error")]
    pub async fn start(&self) -> anyhow::Result<()> {
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
async fn get_root(dns_manager: Data<Arc<dyn Provider>>) -> WebhookJson<DomainFilter> {
    WebhookJson(Json(dns_manager.domain_filter().await))
}

// Returns the current records.
// Returns 200/500
#[logcall("debug")]
#[get("/records", guard = "media_type_guard")]
async fn get_records(dns_manager: Data<Arc<dyn Provider>>) -> WebhookJson<Vec<Endpoint>> {
    WebhookJson(Json(dns_manager.records().await))
}

// Applies the changes.
// Returns 204/500
#[logcall("debug")]
#[post("/records")]
async fn post_records(
    dns_manager: Data<Arc<dyn Provider>>,
    changes: Json<Value>,
) -> (String, StatusCode) {
    let json = changes.into_inner();
    if let Ok(changes) = from_value(json.clone()) {
        match dns_manager.apply_changes(changes).await {
            Ok(_) => (String::new(), StatusCode::NO_CONTENT),
            Err(e) => {
                log::warn!("{e:?}");
                (String::new(), StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        log::warn!("{json}");
        (String::new(), StatusCode::INTERNAL_SERVER_ERROR)
    }
}

// Executes the AdjustEndpoints method.
// Returns 200/500
#[logcall("debug")]
#[post("/adjustendpoints", guard = "media_type_guard")]
async fn post_adjustendpoints(
    dns_manager: Data<Arc<dyn Provider>>,
    endpoints: Json<Value>,
) -> (Json<Vec<Endpoint>>, StatusCode) {
    let json = endpoints.into_inner();
    if let Ok(endpoints) = from_value(json.clone()) {
        match dns_manager.adjust_endpoints(endpoints).await {
            Ok(x) => (Json(x), StatusCode::OK),
            Err(e) => {
                log::warn!("{e:?}");
                (Json(vec![]), StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        log::warn!("{json}");
        (Json(vec![]), StatusCode::INTERNAL_SERVER_ERROR)
    }
}

// Only takes and gives `MEDIATYPE`, why guard.
fn media_type_guard(ctx: &GuardContext<'_>) -> bool {
    ctx.header::<Accept>()
        .map_or(false, |h| h.preference() == MEDIATYPE)
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
