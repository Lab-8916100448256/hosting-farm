use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use loco_rs::controller::middleware::auth;
use loco_rs::prelude::*;
use serde::{Deserialize, Serialize};

use crate::models::_entities::{ssh_keys, users};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SshKeyPayload {
    pub public_key: String,
    // Maybe add a name/label field later?
}

// Basic validation for SSH public key format
fn is_valid_ssh_public_key(key: &str) -> bool {
    let parts: Vec<&str> = key.trim().split_whitespace().collect();
    if parts.len() < 2 {
        return false;
    }
    match parts[0] {
        "ssh-rsa"
        | "ssh-dss"
        | "ecdsa-sha2-nistp256"
        | "ecdsa-sha2-nistp384"
        | "ecdsa-sha2-nistp521"
        | "ssh-ed25519" => {}
        _ => return false,
    }
    // Basic check for Base64 encoding, not a full validation
    BASE64_STANDARD.decode(parts[1]).is_ok()
}

async fn list_keys(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;

    let keys = ssh_keys::Entity::find()
        .filter(ssh_keys::Column::UserId.eq(user.id))
        .all(&ctx.db)
        .await?;

    format::json(keys)
}

async fn add_key(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Form(params): Form<SshKeyPayload>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;

    // Validate the input
    if params.public_key.is_empty() {
        return Err(Error::BadRequest("Public key cannot be empty".to_string()));
    }

    if !is_valid_ssh_public_key(&params.public_key) {
        return Err(Error::BadRequest(
            "Invalid SSH public key format".to_string(),
        ));
    }

    // Check if key already exists for this user
    let existing_key = ssh_keys::Entity::find()
        .filter(ssh_keys::Column::UserId.eq(user.id))
        .filter(ssh_keys::Column::PublicKey.eq(params.public_key.clone()))
        .one(&ctx.db)
        .await?;

    if existing_key.is_some() {
        return Err(Error::BadRequest(
            "SSH Key already exists for this user".to_string(),
        ));
    }

    let key = ssh_keys::ActiveModel {
        user_id: ActiveValue::Set(user.id),
        public_key: ActiveValue::Set(params.public_key.clone()),
        ..Default::default()
    };
    key.insert(&ctx.db).await?;

    // Retrieve the inserted key to return it (optional, but good practice)
    // Note: Inserting doesn't automatically populate the model with ID etc.
    // We query it back, although we could construct the response manually if needed.
    let inserted_key = ssh_keys::Entity::find()
        .filter(ssh_keys::Column::UserId.eq(user.id))
        .filter(ssh_keys::Column::PublicKey.eq(params.public_key))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::InternalServerError)?; // Should exist after insert

    format::json(inserted_key)
}

async fn delete_key(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(key_id): Path<i32>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;

    let key = ssh_keys::Entity::find_by_id(key_id)
        .filter(ssh_keys::Column::UserId.eq(user.id)) // Ensure the key belongs to the user
        .one(&ctx.db)
        .await?;

    match key {
        Some(key) => {
            let key: ssh_keys::ActiveModel = key.into();
            key.delete(&ctx.db).await?;
            format::empty()
        }
        None => Err(Error::NotFound), // Key not found or doesn't belong to user
    }
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/user/ssh_keys")
        .add("/", get(list_keys).post(add_key))
        .add("/{id}", delete(delete_key))
}
