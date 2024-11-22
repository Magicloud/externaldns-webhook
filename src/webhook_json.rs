use rocket::{
    response::{Responder, Result},
    serde::json::Json,
    Request,
};
use serde::Serialize;

use crate::MEDIATYPE;

// ContentType::new("application", "external.dns.webhook+json").with_params(("version", "1"))

/// A patch for returned content type.
/// Because external-dns just comparing the string of content type header, not parsing to be flexible.
/// And Rocket result has a space after the semicolon, which is allowed in spec.

#[derive(Debug)]
pub struct WebhookJson<T>(pub Json<T>)
where
    T: Serialize;
impl<'r, T> Responder<'r, 'static> for WebhookJson<T>
where
    T: Serialize,
{
    fn respond_to(self, request: &'r Request<'_>) -> Result<'static> {
        self.0.respond_to(request).map(|mut r| {
            r.set_raw_header("Content-Type", MEDIATYPE); // Sadly, I cannot use this const in `post` attr.
            r
        })
    }
}
