pub mod auth;
pub mod teams;
pub mod users;

use axum::http::header::HeaderValue;

use loco_rs::prelude::*;

// Build an error message fragment with HTMX headers to inject it into error container of a page
pub fn error_fragment(v: TeraView, message: &str) -> Result<Response> {
    let res = format::render().view(
        &v,
        "error.html",
        data!({
            "message": message
        }),
    );
    match res {
        Ok(mut view) => {
            view.headers_mut()
                .append("HX-Retarget", HeaderValue::from_static("#error-container"));
            view.headers_mut()
                .append("HX-Swap", HeaderValue::from_static("innerHTML"));
            Ok(view)
        }
        Err(e) => {
            tracing::error!("Failed to render error fragment: {}", e);
            Err(e)
        }
    }
}
