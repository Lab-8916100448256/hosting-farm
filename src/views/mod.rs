pub mod auth;
pub mod teams;
pub mod users;

use axum::http::header::HeaderValue;

use loco_rs::prelude::*;

// Build an error message fragment with HTMX headers to inject it into error container of a page
pub fn error_fragment(v: &TeraView, message: &str) -> Result<Response> {
    let res = format::render().view(
        v,
        "error_message.html",
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

// Build a full page with an error message
pub fn error_page(v: &TeraView, message: &str, e: Option<Error>) -> Result<Response> {
    let error_details = if let Some(e) = &e {
        format!("Error details : {}", e)
    } else {
        "".to_string()
    };
    format::render().view(
        v,
        "error_page.html",
        data!({
            "error_message": message,
            "error_details": error_details,
        }),
    )
}

pub fn htmx_redirect(url: &str) -> Result<Response> {
    tracing::info!("Redirecting to: {}", url);
    let response = match Response::builder()
        // TODO: Use HX-Location instead of HX-Redirect
        .header("HX-Redirect", url)
        .body(axum::body::Body::empty())
    {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("Failed to create redirect response: {}", e);
            return Err(e.into());
        }
    };

    Ok(response)
}
