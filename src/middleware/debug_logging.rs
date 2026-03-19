use axum::body::Body;
use axum::http::Request;
use axum::middleware::Next;
use axum::response::Response;
use tracing::debug;

/// Middleware for debug/verbose request logging.
pub async fn debug_logging_middleware(
    request: Request<Body>,
    next: Next,
    debug_enabled: bool,
) -> Response {
    if !debug_enabled {
        return next.run(request).await;
    }

    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = std::time::Instant::now();

    debug!("Incoming request: {method} {uri}");

    let response = next.run(request).await;

    let duration = start.elapsed();
    debug!(
        "Response: {} in {:.2}ms",
        response.status(),
        duration.as_secs_f64() * 1000.0,
    );

    response
}
