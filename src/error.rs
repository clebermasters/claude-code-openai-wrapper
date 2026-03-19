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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use http::Response;

    fn status_of(err: AppError) -> StatusCode {
        let resp = err.into_response();
        resp.status()
    }

    #[test]
    fn test_bad_request_status() {
        assert_eq!(status_of(AppError::BadRequest("x".into())), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_unauthorized_status() {
        assert_eq!(status_of(AppError::Unauthorized("x".into())), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_not_found_status() {
        assert_eq!(status_of(AppError::NotFound("x".into())), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_payload_too_large_status() {
        assert_eq!(status_of(AppError::PayloadTooLarge("x".into())), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[test]
    fn test_rate_limited_status() {
        assert_eq!(status_of(AppError::RateLimited("x".into())), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn test_validation_error_status() {
        assert_eq!(
            status_of(AppError::ValidationError { message: "x".into(), details: vec![] }),
            StatusCode::UNPROCESSABLE_ENTITY,
        );
    }

    #[test]
    fn test_service_unavailable_status() {
        assert_eq!(status_of(AppError::ServiceUnavailable("x".into())), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn test_internal_status() {
        assert_eq!(status_of(AppError::Internal("x".into())), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
        let app_err = AppError::from(io_err);
        assert!(matches!(app_err, AppError::Internal(_)));
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", AppError::BadRequest("test".into())), "Bad Request: test");
        assert_eq!(format!("{}", AppError::Internal("boom".into())), "Internal Error: boom");
        assert_eq!(
            format!("{}", AppError::ValidationError { message: "bad".into(), details: vec![] }),
            "Validation Error: bad",
        );
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
