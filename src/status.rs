use actix_web::http::StatusCode;
use async_trait::async_trait;
use std::fmt::Debug;

// TODO: Support Tracing

/// Definition of the Status interface.
/// This interface should be implemented by DNS service provider application
/// to give healthz and metrics information
#[async_trait]
pub trait Status: Send + Sync + Debug {
    /// Return if the service is healthy in general
    async fn healthz(&self) -> (String, StatusCode) {
        ("OK".to_string(), StatusCode::OK)
    }
    /// Return metrics data for Prometheus
    async fn metrics(&self) -> (String, StatusCode) {
        let report = prometheus::TextEncoder::new()
            .encode_to_string(&prometheus::default_registry().gather());
        match report {
            Ok(x) => (x, StatusCode::OK),
            Err(e) => (format!("{e:?}"), StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}
