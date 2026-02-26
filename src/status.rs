use actix_web::http::StatusCode;
use async_trait::async_trait;
use std::fmt::Debug;

/// Definition of the Status interface.
/// This interface should be implemented by DNS service provider application
/// to give healthz and metrics information
#[async_trait]
pub trait Status: Send + Sync + Debug {
    /// Return if the service is healthy in general
    async fn healthz(&self) -> (String, StatusCode) {
        ("OK".to_string(), StatusCode::OK)
    }
    // Return metrics data for Prometheus
    // Removed since OTLP uses push mode.
}
