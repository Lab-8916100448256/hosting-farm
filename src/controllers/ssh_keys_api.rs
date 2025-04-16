#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use loco_rs::prelude::*;

use axum::debug_handler;
use crate::middleware::auth_no_error::JWTWithUserOpt;
use crate::models::{users, ssh_public_keys};
use crate::models::_entities::ssh_public_keys::Column;
use serde_json::json;
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};

#[debug_handler]
pub async fn list_ssh_keys(
    State(ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
) -> Result<Response> {
    if let Some(user) = auth.user {
        let keys_result = ssh_public_keys::Entity::find()
            .filter(Column::UserId.eq(user.id))
            .all(&ctx.db)
            .await;
        match keys_result {
            Ok(keys) => format::json(json!({ "ssh_keys": keys })),

            Err(e) => Ok((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({"error": "Could not fetch SSH keys", "details": e.to_string()})),
            ).into_response()),
        }
    } else {
        // Not authenticated
        Ok((
            axum::http::StatusCode::UNAUTHORIZED,
            axum::Json(json!({"error": "Unauthorized"})),
        ).into_response())
    }
}


#[debug_handler]
pub async fn create_ssh_key(State(_ctx): State<AppContext>, Json(_payload): Json<serde_json::Value>) -> Result<Response> {
    // TODO: implement create logic
    format::json(json!({ "status": "created" }))
}

#[debug_handler]
pub async fn update_ssh_key(State(_ctx): State<AppContext>, Path(_id): Path<i32>, Json(_payload): Json<serde_json::Value>) -> Result<Response> {
    // TODO: implement update logic
    format::json(json!({ "status": "updated" }))
}

#[debug_handler]
pub async fn delete_ssh_key(State(_ctx): State<AppContext>, Path(_id): Path<i32>) -> Result<Response> {
    // TODO: implement delete logic
    format::json(json!({ "status": "deleted" }))
}


pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/ssh_keys/")
        .add("/", get(list_ssh_keys))
        .add("/", post(create_ssh_key))
        .add("/{id}", put(update_ssh_key))
        .add("/{id}", delete(delete_ssh_key))
}

