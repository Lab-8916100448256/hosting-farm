#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use axum::{debug_handler, extract::State, Json};
use loco_rs::prelude::*;
use sea_orm::{ActiveValue, Set};
use serde::{Deserialize, Serialize};

use crate::models::users;

#[derive(Deserialize)]
pub struct UpdateGpgKeyPayload {
    pub gpg_key: Option<String>, // Allow clearing the key
}

#[derive(Serialize)]
pub struct GpgKeyResponse {
    pub gpg_key: Option<String>,
}

/// Update the GPG key for the authenticated user
#[debug_handler]
pub async fn update(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Json(params): Json<UpdateGpgKeyPayload>,
) -> Result<Response> {
    let mut user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid)
        .await?
        .into_active_model();

    // TODO: Add validation to check if the provided string is a valid GPG key

    user.gpg_key = ActiveValue::set(params.gpg_key);

    let updated_user = user.update(&ctx.db).await?;

    format::json(GpgKeyResponse {
        gpg_key: updated_user.gpg_key,
    })
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/user_gpg_key") // Singular prefix
        .add("/", put(update))
}
