use crate::MEDIATYPE;
use actix_web::{
    HttpRequest, HttpResponse, Responder, body::EitherBody, error::JsonPayloadError, web::Json,
};
use serde::Serialize;

// ContentType::new("application", "external.dns.webhook+json").with_params(("version", "1"))

/// A patch for returned content type.
/// Because external-dns just comparing the string of content type header, not parsing to be flexible.
/// And Actix result has a space after the semicolon, which is allowed in spec.

#[derive(Debug)]
pub struct WebhookJson<T>(pub Json<T>)
where
    T: Serialize;
impl<T> Responder for WebhookJson<T>
where
    T: Serialize,
{
    type Body = EitherBody<String>;

    fn respond_to(self, _: &HttpRequest) -> HttpResponse<Self::Body> {
        match serde_json::to_string(&self.0) {
            Ok(body) => match HttpResponse::Ok()
                .insert_header(("Content-Type", MEDIATYPE))
                .message_body(body)
            {
                Ok(res) => res.map_into_left_body(),
                Err(err) => HttpResponse::from_error(err).map_into_right_body(),
            },

            Err(err) => {
                HttpResponse::from_error(JsonPayloadError::Serialize(err)).map_into_right_body()
            }
        }
    }
}
