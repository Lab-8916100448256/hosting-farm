#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use loco_rs::prelude::*;
use serde_json::json;
use axum::debug_handler;

#[debug_handler]
pub async fn get_gpg_key(State(_ctx): State<AppContext>) -> Result<Response> {
    // TODO: implement get logic
    format::json(json!({ "gpg_key": null, "verified": false }))
}

#[debug_handler]
pub async fn verify_gpg_key(State(_ctx): State<AppContext>, Json(_payload): Json<serde_json::Value>) -> Result<Response> {
    // TODO: implement verify logic
    format::json(json!({ "status": "verification_sent" }))
}

#[debug_handler]
pub async fn update_gpg_key(State(_ctx): State<AppContext>, Json(_payload): Json<serde_json::Value>) -> Result<Response> {
    // TODO: implement update logic
    format::json(json!({ "status": "updated" }))
}


pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/gpg_key/")
        .add("/{id}", get(get_gpg_key))
        .add("/verify", post(verify_gpg_key))
        .add("/{id}", put(update_gpg_key))
}
