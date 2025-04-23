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

use loco_rs::{controller::views::TeraView, prelude::*};
use tera::Context;

use crate::models::users::LayoutData;

// Path to view templates
// const VIEWS_DIR: &str = "assets/views";

/// Renders a template with the given context, automatically including layout data.
///
/// This function takes the TeraView engine, the template name, an optional active page identifier,
/// the layout data (containing user and admin status), and page-specific context.
/// It merges these into a single context before rendering the view.
pub fn render_template<V>(
    v: &V,
    template_name: &str,
    active_page: Option<&str>,
    layout_data: LayoutData,
    mut context: Context,
) -> Result<Response>
where
    V: ViewRenderer,
{
    context.insert("user", &layout_data.user);
    context.insert("is_admin", &layout_data.is_admin);

    if let Some(ap) = active_page {
        context.insert("active_page", ap);
    }

    format::render().view(v, template_name, context)
}

// Build an error message fragment with HTMX headers to inject it into error container of a page
pub fn error_fragment(v: &TeraView, message: &str, target_selector: &str) -> Result<Response> {
    let res = format::render().view(
        v,
        "_inline_error.html",
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

// Build a full page with an error message, using the base layout
pub fn error_page(
    v: &TeraView,
    message: &str,
    e: Option<Error>,
    layout_data: LayoutData,
) -> Result<Response> {
    let error_details = if let Some(ref err) = e {
        format!("Error details : {}", err)
    } else {
        String::new()
    };

    let mut context = Context::new();
    context.insert("error_message", message);
    context.insert("error_details", &error_details);
    context.insert("user", &layout_data.user);
    context.insert("is_admin", &layout_data.is_admin);

    format::render().view(v, "error_page.html", context)
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
