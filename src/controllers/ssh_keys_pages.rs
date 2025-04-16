#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use loco_rs::prelude::*;
use axum::debug_handler;

pub fn routes() -> Routes {
    Routes::new()
        .prefix("profile/ssh-keys")
        .add("/", get(profile_ssh_keys_page))
        .add("/add", get(add_ssh_key_form))
        .add("/add", post(add_ssh_key))
        .add("/{id}/edit", get(edit_ssh_key_form))
        .add("/{id}/edit", post(edit_ssh_key))
        .add("/{id}/delete", post(delete_ssh_key))
}


use axum::{extract::Path, response::Response, http::HeaderMap};
use crate::middleware::auth_no_error::JWTWithUserOpt;
use crate::models::{users, ssh_public_keys};
use crate::views::render_template;
use crate::models::_entities::ssh_public_keys::Column;
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
use loco_rs::prelude::*;

pub async fn profile_ssh_keys_page(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        // Redirect to login if not authenticated
        return crate::views::redirect("/auth/login", headers);
    };
    let keys_result = ssh_public_keys::Entity::find()
        .filter(Column::UserId.eq(user.id))
        .all(&ctx.db)
        .await;
    match keys_result {
        Ok(keys) => {
            render_template(&v, "profile/ssh_keys.html", data!({
                "ssh_keys": &keys,
                "user": &user,
            }))
        }
        Err(e) => {
            render_template(&v, "error_page.html", data!({
                "error": format!("Could not load SSH keys: {}", e)
            }))
        }
    }
}

pub async fn add_ssh_key_form(
    ViewEngine(v): ViewEngine<TeraView>,
    State(_ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    headers: HeaderMap,
) -> Result<Response> {
    if auth.user.is_none() {
        return crate::views::redirect("/auth/login", headers);
    }
    render_template(&v, "profile/ssh_keys_add_form.html", data!({}))
}

pub async fn add_ssh_key(
    ViewEngine(v): ViewEngine<TeraView>,
    State(_ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    headers: HeaderMap,
    Form(_form): Form<serde_json::Value>
) -> Result<Response> {
    if auth.user.is_none() {
        return crate::views::redirect("/auth/login", headers);
    }
    // TODO: Handle add logic, return updated fragment or redirect
    render_template(&v, "profile/ssh_keys_list.html", data!({}))
}

pub async fn edit_ssh_key_form(
    ViewEngine(v): ViewEngine<TeraView>,
    State(_ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    headers: HeaderMap,
    Path(_id): Path<i32>
) -> Result<Response> {
    if auth.user.is_none() {
        return crate::views::redirect("/auth/login", headers);
    }
    // TODO: Render edit form fragment
    render_template(&v, "profile/ssh_keys_edit_form.html", data!({}))
}

pub async fn edit_ssh_key(
    ViewEngine(v): ViewEngine<TeraView>,
    State(_ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    headers: HeaderMap,
    Path(_id): Path<i32>,
    Form(_form): Form<serde_json::Value>
) -> Result<Response> {
    if auth.user.is_none() {
        return crate::views::redirect("/auth/login", headers);
    }
    // TODO: Handle edit logic, return updated fragment or redirect
    render_template(&v, "profile/ssh_keys_list.html", data!({}))
}

pub async fn delete_ssh_key(
    ViewEngine(v): ViewEngine<TeraView>,
    State(_ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    headers: HeaderMap,
    Path(_id): Path<i32>
) -> Result<Response> {
    if auth.user.is_none() {
        return crate::views::redirect("/auth/login", headers);
    }
    // TODO: Handle delete logic, return updated fragment or redirect
    render_template(&v, "profile/ssh_keys_list.html", data!({}))
}
