use crate::{
    mailers::auth::AuthMailer,
    middleware::auth_no_error::JWTWithUserOpt,
    models::{_entities::users, users::UpdateDetailsParams},
    views::{error_fragment, error_page, redirect, render_template},
};
use axum::{
    debug_handler,
    extract::{Form, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post, put},
};
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::{EntityTrait, PaginatorTrait, QueryOrder};
use serde::Deserialize;
use tracing::error;

/// Struct for pagination query parameters
#[derive(Deserialize, Debug)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    page: u64,
    #[serde(default = "default_page_size")]
    page_size: u64,
}

fn default_page() -> u64 {
    1
}

fn default_page_size() -> u64 {
    25
}

/// Helper to check for admin privileges
async fn require_admin(
    auth: &JWTWithUserOpt<users::Model>,
    ctx: &AppContext,
    v: &TeraView,
    headers: &HeaderMap,
) -> Result<users::Model, Response> {
    let user = match &auth.user {
        Some(user) => user,
        None => {
            tracing::debug!("Admin check failed: User not authenticated.");
            return Err(
                redirect("/auth/login?next=/admin/users", headers.clone()).unwrap_or_else(|e| {
                    tracing::error!(error = ?e, "Failed to create redirect response");
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }),
            );
        }
    };

    match user.is_admin(&ctx.db, ctx).await {
        Ok(true) => Ok(user.clone()),
        Ok(false) => {
            tracing::warn!(
                user_pid = user.pid.to_string(),
                "Admin check failed: User not admin."
            );
            Err(error_page(
                v,
                "You are not authorized to access this page.",
                Some(Error::Unauthorized("Admin privileges required".to_string())),
            )
            .unwrap_or_else(|e| {
                tracing::error!(error = ?e, "Failed to create error page response");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }))
        }
        Err(e) => {
            tracing::error!(error = ?e, user_pid = user.pid.to_string(), "Admin check failed: DB error.");
            Err(error_page(
                v,
                "Could not verify your permissions. Please try again later.",
                Some(Error::Model(e)),
            )
            .unwrap_or_else(|e| {
                tracing::error!(error = ?e, "Failed to create error page response");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }))
        }
    }
}

/// Handler for the main user management page.
#[debug_handler]
async fn manage_users_page(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Query(pagination): Query<PaginationParams>,
) -> Result<Response> {
    let admin_user = match require_admin(&auth, &ctx, &v, &headers).await {
        Ok(user) => user,
        Err(response) => return Ok(response),
    };

    let page = pagination.page.max(1);
    let page_size = pagination.page_size.max(1).min(100);

    let paginator = users::Entity::find()
        .order_by_asc(users::Column::Name)
        .paginate(&ctx.db, page_size);

    let num_pages = paginator.num_pages().await?;
    // Fetching the first page here might be redundant if HTMX loads it immediately,
    // but it could be useful if HTMX fails or for initial state.
    // Let's keep it for now, but remove the users_list from the context as it's loaded via HTMX.
    // let users_list = paginator.fetch_page(page - 1).await?;

    let is_admin = admin_user.is_admin(&ctx.db, &ctx).await.unwrap_or(false);

    // Construct the URL for the initial fragment load
    let user_list_fragment_url = format!(
        "/admin/users/fragment?page={}&page_size={}",
        page, page_size
    );

    render_template(
        &v,
        "admin/manage_users.html",
        data!({
            // "users": &users_list, // Remove this, it will be loaded via HTMX
            "current_page": page, // Still needed for potential non-HTMX fallback or context
            "total_pages": num_pages, // Still needed for context/UI elements outside the list
            "page_size": page_size, // Still needed for context/UI elements
            "user": &admin_user,
            "invitation_count": 0, // TODO: Add actual invitation count
            "active_page": "admin_users",
            "is_admin": is_admin,
            "user_list_fragment_url": &user_list_fragment_url // Add the fragment URL
        }),
    )
}

/// Handler for the HTMX user list fragment (table body + pagination).
#[debug_handler]
async fn get_user_list_fragment(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Query(pagination): Query<PaginationParams>,
) -> Result<Response> {
    if require_admin(&auth, &ctx, &v, &headers).await.is_err() {
        return Ok(Html("".to_string()).into_response());
    }

    let page = pagination.page.max(1);
    let page_size = pagination.page_size.max(1).min(100);

    let paginator = users::Entity::find()
        .order_by_asc(users::Column::Name)
        .paginate(&ctx.db, page_size);

    let num_pages = paginator.num_pages().await?;
    let users = paginator.fetch_page(page - 1).await?;

    format::render().view(
        &v,
        "admin/_user_list.html",
        data!({
            "users": &users,
            "current_page": page,
            "total_pages": num_pages,
            "page_size": page_size,
            "edit_url_base": "/admin/users/",
            "reset_password_url_base": "/admin/users/"
        }),
    )
}

/// Handler to return the editable form row for a user.
#[debug_handler]
async fn get_user_edit_form(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(user_pid): Path<String>,
) -> Result<Response> {
    if require_admin(&auth, &ctx, &v, &headers).await.is_err() {
        return Ok(Html("<p>Unauthorized</p>".to_string()).into_response());
    }

    match users::Model::find_by_pid(&ctx.db, &user_pid).await {
        Ok(user) => {
            let update_url = format!("/admin/users/{}", user_pid);
            let cancel_url = update_url.clone(); // Same URL for cancel (GET)
            format::render().view(
                &v,
                "admin/_user_edit_form.html",
                data!({
                    "user": &user,
                    "update_url": &update_url,
                    "cancel_url": &cancel_url
                }),
            )
        }
        Err(e) => {
            tracing::error!(error = ?e, user_pid = user_pid, "Failed to find user for editing");
            Ok(Html(format!(
                "<tr id=\"user-row-error-{}\"><td colspan=\"3\" class=\"text-red-500 p-4\">Error loading user data: {}</td></tr>",
                user_pid, e
            )).into_response())
        }
    }
}

/// Handler to return the read-only row for a user (used for cancelling edit).
#[debug_handler]
async fn get_user_row_view(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(user_pid): Path<String>,
) -> Result<Response> {
    if require_admin(&auth, &ctx, &v, &headers).await.is_err() {
        return Ok(Html("<p>Unauthorized</p>".to_string()).into_response());
    }

    match users::Model::find_by_pid(&ctx.db, &user_pid).await {
        Ok(user) => {
            // Construct URLs needed by the _user_row_view template
            let edit_url = format!("/admin/users/{}/edit", user.pid);
            let reset_password_url = format!("/admin/users/{}/reset-password", user.pid);
            format::render().view(
                &v,
                "admin/_user_row_view.html",
                data!({
                    "user": &user,
                    "edit_url": &edit_url,
                    "reset_password_url": &reset_password_url
                }),
            )
        }
        Err(e) => {
            tracing::error!(error = ?e, user_pid = user_pid, "Failed to find user for display row");
            Ok(Html(format!(
                "<tr id=\"user-row-error-{}\"><td colspan=\"3\" class=\"text-red-500 p-4\">Error loading user data: {}</td></tr>",
                user_pid, e
            )).into_response())
        }
    }
}

/// Handler to update a user's details.
#[debug_handler]
async fn update_user_details_admin(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(user_pid): Path<String>,
    Form(params): Form<UpdateDetailsParams>,
) -> Result<Response> {
    let admin_user = match require_admin(&auth, &ctx, &v, &headers).await {
        Ok(user) => user,
        Err(response) => return Ok(response),
    };

    let target_user = match users::Model::find_by_pid(&ctx.db, &user_pid).await {
        Ok(user) => user,
        Err(e) => {
            tracing::error!(error = ?e, user_pid = user_pid, "Failed to find user for update");
            return error_fragment(
                &v,
                "Target user not found.",
                &format!("#edit-row-error-{}", user_pid),
            );
        }
    };

    let original_email = target_user.email.clone();

    match target_user.update_profile_details(&ctx.db, &params).await {
        Ok(updated_user) => {
            let email_changed = updated_user.email != original_email;
            let mut final_user_state = updated_user.clone();

            if email_changed {
                let user_clone_for_token = updated_user.clone();
                let user_with_token_result = users::ActiveModel::from(user_clone_for_token)
                    .generate_email_verification_token(&ctx.db)
                    .await;

                match user_with_token_result {
                    Ok(user_with_token) => {
                        let user_with_token_clone = user_with_token.clone();
                        if let Err(e) = AuthMailer::send_welcome(&ctx, &user_with_token).await {
                            tracing::error!(user_pid = user_with_token.pid.to_string(), error = ?e, "Admin Update: Failed to send verification email");
                        } else {
                            if let Err(e) = users::ActiveModel::from(user_with_token_clone)
                                .set_email_verification_sent(&ctx.db)
                                .await
                            {
                                tracing::error!(user_pid = updated_user.pid.to_string(), error = ?e, "Admin Update: Failed to set email verification sent timestamp");
                            }
                        }
                        final_user_state = user_with_token;
                    }
                    Err(e) => {
                        tracing::error!(user_pid = updated_user.pid.to_string(), error = ?e, "Admin Update: Failed to generate email verification token");
                    }
                }
            }

            let edit_url = format!("/admin/users/{}/edit", updated_user.pid);
            let reset_password_url = format!("/admin/users/{}/reset-password", updated_user.pid);

            format::render().view(
                &v,
                "admin/_user_row_view.html",
                data!({
                   "user": &final_user_state,
                   "edit_url": &edit_url,
                   "reset_password_url": &reset_password_url
                }),
            )
        }
        Err(e) => {
            tracing::error!(error = ?e, user_pid = user_pid, "Failed to update user details by admin {}", admin_user.pid);
            let error_message = e.to_string();
            let update_url = format!("/admin/users/{}", user_pid);
            let cancel_url = update_url.clone();
            format::render().view(
                &v,
                "admin/_user_edit_form.html",
                data!({
                    "user": &target_user,
                    "error_message": &error_message,
                    "update_url": &update_url,
                    "cancel_url": &cancel_url
                }),
            )
        }
    }
}

/// Handler to trigger a password reset email for a user.
#[debug_handler]
async fn trigger_password_reset_admin(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(user_pid): Path<String>,
) -> Result<Response> {
    let admin_user = match require_admin(&auth, &ctx, &v, &headers).await {
        Ok(user) => user,
        Err(response) => return Ok(response),
    };

    let target_user = match users::Model::find_by_pid(&ctx.db, &user_pid).await {
        Ok(user) => user,
        Err(e) => {
            tracing::error!(error = ?e, user_pid = user_pid, "Admin Reset PW: Failed to find user");
            return error_fragment(&v, "User not found", "#admin-user-messages");
        }
    };

    let user_with_token = match target_user.initiate_password_reset(&ctx.db).await {
        Ok(u) => u,
        Err(e) => {
            error!(user_email = %target_user.email, error = ?e, "Admin Reset PW: Failed to initiate password reset (token generation/save)");
            return error_fragment(
                &v,
                "Failed to prepare password reset for user.",
                "#admin-user-messages",
            );
        }
    };

    if let Err(e) = AuthMailer::forgot_password(&ctx, &user_with_token).await {
        error!(user_email = %target_user.email, error = ?e, "Admin Reset PW: Failed to send forgot password email");
        return error_fragment(
            &v,
            "Failed to send password reset email.",
            "#admin-user-messages",
        );
    } else {
        if let Err(e) = user_with_token
            .clone()
            .into_active_model()
            .set_forgot_password_sent(&ctx.db)
            .await
        {
            error!(user_email = %target_user.email, error = ?e, "Admin Reset PW: Failed to set forgot password sent timestamp");
        }
        tracing::info!(admin_user_pid=%admin_user.pid, target_user_pid=%target_user.pid, "Password reset email sent by admin.");
        format::render().view(
            &v,
            "fragments/success_message.html",
            data!({
                "message": format!("Password reset email sent to {}.", target_user.email)
            }),
        )
    }
}

/// Admin routes
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/admin")
        .add("/users", get(manage_users_page))
        .add("/users/fragment", get(get_user_list_fragment))
        .add("/users/{user_pid}/edit", get(get_user_edit_form))
        .add("/users/{user_pid}", post(update_user_details_admin))
        .add("/users/{user_pid}", get(get_user_row_view))
        .add(
            "/users/{user_pid}/reset-password",
            post(trigger_password_reset_admin),
        )
}
