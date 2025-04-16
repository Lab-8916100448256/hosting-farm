#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use crate::models::_entities::user_ssh_keys::{self, ActiveModel, Entity};
use crate::models::users;
use axum::debug_handler;
use axum::routing::{delete, get, post};
use axum::{
    extract::{Path, State},
    Json,
};
use loco_rs::prelude::*;
use loco_rs::Error;
use sea_orm::entity::prelude::Uuid;
use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, ModelTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct AddSshKeyPayload {
    pub name: String,
    pub key: String,
}

#[derive(Serialize)]
pub struct SshKeyResponse {
    pub pid: Uuid,
    pub name: String,
    pub key: String,
}

async fn load_user_key(ctx: &AppContext, user_id: i32, pid: &str) -> Result<user_ssh_keys::Model> {
    let pid = Uuid::parse_str(pid)
        .map_err(|_| loco_rs::Error::BadRequest("Invalid key PID format".to_string()))?;
    user_ssh_keys::Entity::find()
        .filter(user_ssh_keys::Column::Pid.eq(pid))
        .filter(user_ssh_keys::Column::UserId.eq(user_id))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::string("SSH Key not found"))
}

/// List all SSH keys for the authenticated user
#[debug_handler]
pub async fn list(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let _user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?; // Keep user load for auth check
                                                                             // let keys = user.get_ssh_keys(&ctx.db).await?; // Temporarily commented out
    let key_responses: Vec<SshKeyResponse> = vec![]; // Temporarily return empty list
                                                     /* // Original mapping code
                                                     let key_responses = keys
                                                         .iter()
                                                         .map(|k| SshKeyResponse {
                                                             pid: k.pid,
                                                             name: k.name.clone(),
                                                             key: k.key.clone(),
                                                         })
                                                         .collect::<Vec<_>>();
                                                     */
    format::json(key_responses)
}

/// Add a new SSH key for the authenticated user
#[debug_handler]
pub async fn add(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Json(params): Json<AddSshKeyPayload>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;

    let new_key = ActiveModel {
        user_id: ActiveValue::set(user.id),
        name: ActiveValue::set(params.name),
        key: ActiveValue::set(params.key),
        pid: ActiveValue::Set(Uuid::new_v4()), // Generate PID here
        ..Default::default()
    };

    let created_key = new_key.insert(&ctx.db).await?;

    format::json(SshKeyResponse {
        pid: created_key.pid,
        name: created_key.name,
        key: created_key.key,
    })
}

/// Delete an SSH key by its PID
#[debug_handler]
pub async fn delete_key(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let key = load_user_key(&ctx, user.id, &pid).await?;

    key.delete(&ctx.db).await?;

    format::empty()
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/user_ssh_keys")
        .add("/", get(list).post(add))
        .add("/:pid", delete(self::delete_key))
}
