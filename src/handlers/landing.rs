use axum::extract::State;
use axum::response::Html;

use crate::constants::VERSION;
use crate::AppState;

/// GET / - Landing page with API documentation
pub async fn root(State(state): State<AppState>) -> Html<String> {
    let auth_valid = state.auth_manager.auth_status.valid;
    let auth_method = &state.auth_manager.auth_method;
    let status_color = if auth_valid { "#22c55e" } else { "#ef4444" };
    let status_text = if auth_valid { "Connected" } else { "Not Connected" };

    let html = include_str!("../assets/landing.html")
        .replace("{{VERSION}}", VERSION)
        .replace("{{STATUS_COLOR}}", status_color)
        .replace("{{STATUS_TEXT}}", status_text)
        .replace("{{AUTH_METHOD}}", auth_method);

    Html(html)
}
