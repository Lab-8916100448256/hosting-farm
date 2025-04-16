pub mod auth;
pub mod teams;
pub mod users;

use axum::{
    http::{header::HeaderValue, HeaderMap},
    response::{IntoResponse, Redirect, Response},
};

use loco_rs::prelude::*;

//use loco_rs::{controller::format, Result};
use serde::Serialize;

// Path to view templates
// const VIEWS_DIR: &str = "assets/views";

/// Renders a template with the given context
pub fn render_template<V, S>(v: &V, template_name: &str, context: S) -> Result<Response>
where
    V: ViewRenderer,
    S: serde::Serialize,
{
    format::render().view(v, template_name, context)
}

// Build an error message fragment with HTMX headers to inject it into error container of a page
pub fn error_fragment(v: &TeraView, message: &str, target_selector: &str) -> Result<Response> {
    let res = format::render().view(
        v,
        "error_message.html",
        data!({
            "message": message
        }),
    );
    match res {
        Ok(mut view) => {
            view.headers_mut().append(
                "HX-Retarget",
                HeaderValue::from_str(target_selector).unwrap_or_else(|_| {
                    tracing::warn!("Invalid header value for HX-Retarget: {}", target_selector);
                    HeaderValue::from_static("#error-container")
                }),
            );
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

pub fn redirect(url: &str, headers: HeaderMap) -> Result<Response> {
    if headers.get("HX-Request").map_or(false, |v| v == "true") {
        // HTMX request: Use HX-Redirect header
        tracing::info!("Redirecting to: {} using HTMX", url);
        htmx_redirect(url)
    } else {
        // Standard request: Use HTTP redirect
        tracing::info!("Redirecting to: {} using HTTP", url);
        Ok(Redirect::to(url).into_response())
    }
}

pub fn htmx_redirect(url: &str) -> Result<Response> {
    let response = match Response::builder()
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
