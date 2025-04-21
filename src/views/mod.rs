use axum::{
    body::Body,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use axum_template::Key;
use loco_rs::{app::AppContext, errors::Error, prelude::*};
use serde_json::json;
use tera::Context;

pub mod admin; // Added admin views module
pub mod auth;
pub mod home;
pub mod teams;
pub mod users;

/// Renders the given template with the provided context.
///
/// # Errors
///
/// Returns a `loco_rs::Error::View` if the template cannot be rendered.
pub fn render(ctx: &AppContext, template_name: &str, context: Context) -> Result<Response> {
    ctx.view_engine
       .render(template_name, context)
       .map(|resp| resp.into_response())
       .map_err(|e| {
           // Log the error
           if let axum_template::Error::Tera(ref tera_err) = e {
               tracing::error!(tera_error = ?tera_err, "Tera template rendering error");
           } else {
               tracing::error!(error = ?e, "Template rendering error");
           }
           // Explicitly map axum_template::Error to loco_rs::Error::View
           Error::View(e)
       })
}

pub fn render_template<S: Into<String>>(
    v: &TeraView,
    template: S,
    context: serde_json::Value,
) -> Result<Response> {
    v.render(&template.into(), context)
        .map(|html| Html(html).into_response()) // Map success case
        .map_err(|e| Error::View(e)) // Explicitly map the axum_template::Error to loco_rs::Error::View
}

/// Renders an error page with the given message and optional error details.
///
/// # Errors
///
/// Returns a `Error::TeraError` if the error page template cannot be rendered.
pub fn error_page(v: &TeraView, message: &str, err: Option<Error>) -> Result<Response> {
    let mut context = Context::new();
    context.insert("message", message);
    if let Some(e) = err {
        context.insert("error", &e.to_string());
    }
    v.render("error_page.html", context)
        .map(|html| (StatusCode::INTERNAL_SERVER_ERROR, Html(html)).into_response())
        .map_err(|e| Error::View(e)) // Explicitly map the axum_template::Error to loco_rs::Error::View
}

/// Renders an inline error fragment for HTMX responses.
///
/// # Errors
///
/// Returns a `Error::TeraError` if the inline error template cannot be rendered.
pub fn error_fragment(v: &TeraView, message: &str, target_id: &str) -> Result<Response> {
    let context = json!({ "message": message });
    let html_result = v.render("_inline_error.html", context);

    match html_result {
        Ok(html) => {
            let mut headers = axum::http::HeaderMap::new();
            // Safely parse headers or return Error::BadRequest
            let hx_retarget = target_id.parse().map_err(|_| Error::BadRequest("Invalid target_id format".to_string()))?;
            let hx_reswap = "innerHTML".parse().map_err(|_| Error::InternalServerError)?; // Should not fail

            headers.insert("HX-Retarget", hx_retarget);
            headers.insert("HX-Reswap", hx_reswap);
            Ok((StatusCode::UNPROCESSABLE_ENTITY, headers, Html(html)).into_response())
        }
        Err(template_err) => {
            // Log and return the template error
            if let axum_template::Error::Tera(ref tera_err) = template_err {
                tracing::error!(tera_error = ?tera_err, "Tera template rendering error for _inline_error.html");
            } else {
                tracing::error!(error = ?template_err, "Template rendering error for _inline_error.html");
            }
            Err(Error::View(template_err)) // Map to loco_rs::Error::View
        }
    }
}

/// Creates a redirect response, optionally adding HX-Redirect for HTMX requests.
/// For non-HTMX requests, returns a standard 302 redirect.
pub fn redirect(uri: &str, headers: axum::http::HeaderMap) -> Result<Response> {
    let mut response_headers = axum::http::HeaderMap::new();
    if headers.contains_key("HX-Request") {
        response_headers.insert("HX-Redirect", uri.parse().map_err(|e| Error::Any(e))?);
        // Return 200 OK for HTMX redirects, the client handles the navigation
        Ok((StatusCode::OK, response_headers, Body::empty()).into_response())
    } else {
        response_headers.insert(axum::http::header::LOCATION, uri.parse().map_err(|e| Error::Any(e))?);
        Ok((StatusCode::FOUND, response_headers, Body::empty()).into_response())
    }
}
