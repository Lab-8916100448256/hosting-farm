#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use axum::debug_handler;
use loco_rs::prelude::*;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("profile/gpg-key")
        .add("/{id}", get(profile_gpg_key_page))
        .add("/verify", post(verify_gpg_key))
        .add("/update", post(update_gpg_key))
}

use crate::views::render_template;
use axum::response::Response;

pub async fn profile_gpg_key_page(
    ViewEngine(v): ViewEngine<TeraView>,
    State(_ctx): State<AppContext>,
) -> Result<Response> {
    // TODO: Render the GPG key management section (HTML or HTMX fragment)
    render_template(&v, "profile/gpg_key.html", data!({}))
}

pub async fn verify_gpg_key(
    ViewEngine(v): ViewEngine<TeraView>,
    State(_ctx): State<AppContext>,
) -> Result<Response> {
    // TODO: Handle verification logic, return updated fragment or status
    render_template(&v, "profile/gpg_key_status.html", data!({}))
}

pub async fn update_gpg_key(
    ViewEngine(v): ViewEngine<TeraView>,
    State(_ctx): State<AppContext>,
) -> Result<Response> {
    // TODO: Handle update logic, return updated fragment or status
    render_template(&v, "profile/gpg_key_status.html", data!({}))
}
