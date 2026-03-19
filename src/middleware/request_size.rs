use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// Middleware that limits request body size.
pub async fn request_size_middleware(
    request: Request<Body>,
    next: Next,
    max_size: usize,
) -> Response {
    if let Some(content_length) = request
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<usize>().ok())
    {
        if content_length > max_size {
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                axum::Json(json!({
                    "error": {
                        "message": format!("Request body too large. Maximum size is {max_size} bytes."),
                        "type": "request_too_large",
                        "code": 413,
                    }
                })),
            )
                .into_response();
        }
    }

    next.run(request).await
}
