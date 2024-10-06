use crate::{
    changes::Changes, domain_filter::DomainFilter, endpoint::Endpoint, provider::Provider,
    webhook_json::WebhookJson, IDoNotCareWhich,
};
use core::net::IpAddr;
use core::str::FromStr;
use logcall::logcall;
use rocket::{
    async_trait, get,
    http::{MediaType, Status},
    post,
    request::{FromRequest, Outcome},
    routes,
    serde::json::Json,
    Request, State,
};
use std::sync::Arc;

#[derive(Debug)]
pub struct Webhook {
    pub provider_address: String,
    pub provider_port: u16,
    pub dns_manager: Arc<dyn Provider>,

    pub exposed_address: String,
    pub exposed_port: u16,
}
impl Webhook {
    #[logcall("debug")]
    pub fn new(dns_manager: Arc<dyn Provider>) -> Webhook {
        Webhook {
            provider_address: "127.0.0.1".to_string(),
            provider_port: 8888,
            dns_manager,
            exposed_address: "0.0.0.0".to_string(),
            exposed_port: 8080,
        }
    }

    #[logcall(ok = "debug", err = "error")]
    pub async fn start(&self) -> anyhow::Result<()> {
        let exposed = rocket::custom(rocket::Config {
            address: IpAddr::from_str(&self.exposed_address)?,
            port: self.exposed_port,
            ..rocket::Config::default()
        })
        .mount("/", routes![get_healthz]) // get_metrics
        .launch();

        let provider = rocket::custom(rocket::Config {
            address: IpAddr::from_str(&self.provider_address)?,
            port: self.provider_port,
            ..rocket::Config::default()
        })
        .manage(self.dns_manager.clone())
        .mount(
            "/",
            routes![get_root, get_records, post_records, post_adjustendpoints,],
        )
        .launch();

        tokio::spawn(exposed);
        provider.await?;

        Ok(())
    }
}

// Negotiate `DomainFilter`
#[logcall("debug")]
#[get("/")]
async fn get_root(
    dns_manager: &State<Arc<dyn Provider>>,
    _header_check: GetHeadersCheck,
) -> WebhookJson<DomainFilter> {
    WebhookJson(Json(dns_manager.domain_filter().await))
}

// Get records
#[logcall("debug")]
#[get("/records")]
async fn get_records(
    dns_manager: &State<Arc<dyn Provider>>,
    _header_check: GetHeadersCheck,
) -> WebhookJson<Vec<Endpoint>> {
    WebhookJson(Json(dns_manager.records().await))
}

// Apply record
#[logcall("debug")]
#[post(
    "/records",
    format = "application/external.dns.webhook+json;version=1",
    data = "<changes>"
)]
// #[suppress(unknown_format)]
async fn post_records(
    dns_manager: &State<Arc<dyn Provider>>,
    changes: Json<Changes>,
) -> (Status, String) {
    match dns_manager.apply_changes(changes.0).await {
        Ok(_) => (Status::Ok, "".to_string()),
        Err(e) => (Status::InternalServerError, format!("{e:?}")),
    }
}

// Provider specific adjustments of records
#[logcall("debug")]
#[post(
    "/adjustendpoints",
    format = "application/external.dns.webhook+json;version=1",
    data = "<endpoints>"
)]
async fn post_adjustendpoints(
    dns_manager: &State<Arc<dyn Provider>>,
    endpoints: Json<Vec<Endpoint>>,
    _header_check: GetHeadersCheck,
) -> (Status, Json<IDoNotCareWhich<Vec<Endpoint>, String>>) {
    match dns_manager.adjust_endpoints(endpoints.0).await {
        Ok(x) => (Status::Ok, Json(IDoNotCareWhich::One(x))),
        Err(e) => (
            Status::InternalServerError,
            Json(IDoNotCareWhich::Another(format!("{e:?}"))),
        ),
    }
}

// The interfaces should return what is `Accept`ed. Giving no other formats are needed.
// Just check external-dns accepts what I can give.
#[derive(Debug)]
enum GetHeadersCheckError {
    NoAcceptHeader,
    OnlyAcceptMediaVersion1,
}

#[derive(Debug)]
struct GetHeadersCheck {}
#[async_trait]
impl<'r> FromRequest<'r> for GetHeadersCheck {
    type Error = GetHeadersCheckError;

    #[logcall("debug")]
    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let accepted_media_type = MediaType::new("application", "external.dns.webhook+json")
            .with_params(("version", "1"));
        let ret: Result<(), GetHeadersCheckError> = try {
            request
                .accept()
                .ok_or(Self::Error::NoAcceptHeader)?
                .media_types()
                .find(|x| **x == accepted_media_type)
                .ok_or(Self::Error::OnlyAcceptMediaVersion1)?;
        };
        match ret {
            Ok(_) => Outcome::Success(GetHeadersCheck {}),
            Err(e) => Outcome::Error((Status::BadRequest, e)),
        }
    }
}

// #[logcall("debug")]
#[get("/healthz")]
async fn get_healthz() -> (Status, String) {
    (Status::Ok, "OK".to_string())
}
