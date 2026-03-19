use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Unauthorized(String),
    NotFound(String),
    PayloadTooLarge(String),
    RateLimited(String),
    ValidationError { message: String, details: Vec<serde_json::Value> },
    ServiceUnavailable(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, message, extra) = match self {
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, "invalid_request_error", msg, None),
            Self::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "authentication_error", msg, None),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found_error", msg, None),
            Self::PayloadTooLarge(msg) => (StatusCode::PAYLOAD_TOO_LARGE, "request_too_large", msg, None),
            Self::RateLimited(msg) => (StatusCode::TOO_MANY_REQUESTS, "rate_limit_error", msg, None),
            Self::ValidationError { message, details } => {
                let body = json!({
                    "error": {
                        "message": message,
                        "type": "validation_error",
                        "code": "invalid_request_error",
                        "details": details,
                    }
                });
                return (StatusCode::UNPROCESSABLE_ENTITY, axum::Json(body)).into_response();
            }
            Self::ServiceUnavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, "service_unavailable", msg, None),
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", msg, None),
        };

        let mut body = json!({
            "error": {
                "message": message,
                "type": error_type,
                "code": status.as_u16().to_string(),
            }
        });

        if let Some(extra) = extra {
            body["error"]["extra"] = extra;
        }

        (status, axum::Json(body)).into_response()
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::Internal(e.to_string())
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadRequest(msg) => write!(f, "Bad Request: {msg}"),
            Self::Unauthorized(msg) => write!(f, "Unauthorized: {msg}"),
            Self::NotFound(msg) => write!(f, "Not Found: {msg}"),
            Self::PayloadTooLarge(msg) => write!(f, "Payload Too Large: {msg}"),
            Self::RateLimited(msg) => write!(f, "Rate Limited: {msg}"),
            Self::ValidationError { message, .. } => write!(f, "Validation Error: {message}"),
            Self::ServiceUnavailable(msg) => write!(f, "Service Unavailable: {msg}"),
            Self::Internal(msg) => write!(f, "Internal Error: {msg}"),
        }
    }
}
