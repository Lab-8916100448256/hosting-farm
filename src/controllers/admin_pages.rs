use crate::{
    mailers::auth::AuthMailer,
    middleware::auth_no_error::JWTWithUserOpt,
    models::{
        _entities::users as user_entity,
        users::{
            ActiveModel as UserActiveModel, LayoutData, Model as UserModel, UpdateDetailsParams,
        },
    },
    views::{error_fragment, error_page, redirect, render_template},
};
use axum::{
    debug_handler,
    extract::{Form, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::{EntityTrait, IntoActiveModel, PaginatorTrait, QueryOrder};
use serde::Deserialize;
use tera::Context;
use tracing::{error, info, warn};

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
    12
}

/// Refactored admin check helper using LayoutData
async fn check_admin_privileges(
    layout_data: &LayoutData,
    v: &TeraView,
    headers: &HeaderMap,
) -> Result<(), Response> {
    if !layout_data.is_admin {
        let user_pid_str = layout_data
            .user
            .as_ref()
            .map(|u| u.pid.to_string())
            .unwrap_or_else(|| "None".to_string());
        tracing::warn!(user_pid = %user_pid_str, "Admin privileges check failed: User not admin.");

        // Return an Err containing the Response to be sent
        Err(if layout_data.user.is_none() {
            // Not logged in -> redirect. Use unwrap_or_else for robustness as in original code.
            redirect("/auth/login?next=/admin/users", headers.clone()).unwrap_or_else(|e| {
                tracing::error!(error = ?e, "Failed to create redirect response for admin check");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            })
        } else {
            // Logged in but not admin -> error page response. Use unwrap_or_else.
            error_page(
                v,
                "You are not authorized to perform this action.",
                Some(Error::Unauthorized("Admin privileges required".to_string())),
                layout_data.clone(), // Clone layout_data to pass ownership
            )
            .unwrap_or_else(|e| {
                tracing::error!(error = ?e, "Failed to create error page response for admin check");
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            })
        })
    } else {
        Ok(()) // User is admin, continue execution
    }
}

/// Handler for the main user management page.
#[debug_handler]
async fn manage_users_page(
    auth: JWTWithUserOpt<UserModel>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Query(pagination): Query<PaginationParams>,
) -> Result<Response> {
    // 1. Get LayoutData
    let layout_data = UserModel::create_layout_data(auth.user, &ctx).await?;

    // 2. Check for admin privileges
    if !layout_data.is_admin {
        let user_pid_str = layout_data
            .user
            .as_ref()
            .map(|u| u.pid.to_string())
            .unwrap_or_else(|| "None".to_string());
        tracing::warn!(user_pid = %user_pid_str, "Admin page access denied: User not admin.");

        return if layout_data.user.is_none() {
            // Not logged in -> redirect to login
            redirect("/auth/login?next=/admin/users", headers.clone()).map_err(Into::into)
        } else {
            // Logged in but not admin -> show error page
            error_page(
                &v,
                "You are not authorized to access this page.",
                Some(Error::Unauthorized("Admin privileges required".to_string())),
                layout_data, // Pass layout_data for layout rendering
            )
        };
    }
    // User is admin if we reach here

    // 3. Prepare pagination logic
    let page = pagination.page.max(1);
    let page_size = pagination.page_size.max(1).min(100);
    let paginator = user_entity::Entity::find()
        .order_by_asc(user_entity::Column::Name)
        .paginate(&ctx.db, page_size);
    let num_pages = paginator.num_pages().await?;
    let user_list_fragment_url = format!(
        "/admin/users/fragment?page={}&page_size={}",
        page, page_size
    );

    // 4. Create page-specific context
    let mut page_context = Context::new();
    page_context.insert("current_page", &page);
    page_context.insert("total_pages", &num_pages);
    page_context.insert("page_size", &page_size);
    page_context.insert("user_list_fragment_url", &user_list_fragment_url);

    // 5. Call render_template view helper
    render_template(
        &v,
        "admin/manage_users.html",
        Some("admin_users"), // Active page identifier
        layout_data,         // Pass common layout data
        page_context,        // Pass page-specific data
    )
}

/// Handler for the HTMX user list fragment (table body + pagination).
#[debug_handler]
async fn get_user_list_fragment(
    auth: JWTWithUserOpt<UserModel>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Query(pagination): Query<PaginationParams>,
) -> Result<Response> {
    // 1. Get LayoutData first for the admin check
    let layout_data = UserModel::create_layout_data(auth.user, &ctx).await?;

    // 2. Use the refactored admin check
    if let Err(response) = check_admin_privileges(&layout_data, &v, &headers).await {
        // For a fragment, returning the full error page/redirect might break the UI.
        // Original logic returned empty HTML on error. Let's stick to that for fragments.
        tracing::debug!("Admin check failed for fragment request, returning empty response.");
        return Ok(Html("<!-- Admin privileges required -->".to_string()).into_response());
        // Alternatively, could use error_fragment helper if a specific error display area exists
        // return error_fragment(&v, "Admin privileges required", "#some-error-container");
    }
    // User is admin

    // 3. Proceed with fragment logic
    let page = pagination.page.max(1);
    let page_size = pagination.page_size.max(1).min(100);

    let paginator = user_entity::Entity::find()
        .order_by_asc(user_entity::Column::Name)
        .paginate(&ctx.db, page_size);

    let num_pages = paginator.num_pages().await?;
    let users = paginator.fetch_page(page - 1).await?;

    // Calculate pagination URLs
    let base_url = "/admin/users/fragment";
    let prev_page_url = if page > 1 {
        Some(format!(
            "{}?page={}&page_size={}",
            base_url,
            page - 1,
            page_size
        ))
    } else {
        None
    };
    let next_page_url = if page < num_pages {
        Some(format!(
            "{}?page={}&page_size={}",
            base_url,
            page + 1,
            page_size
        ))
    } else {
        None
    };
    let page_url_base = format!("{}?page=", base_url);
    let page_size_suffix = format!("&page_size={}", page_size);

    // Render the fragment - does not need full LayoutData
    format::render().view(
        &v,
        "admin/_user_list.html",
        data!({
            "users": &users,
            "current_page": page,
            "total_pages": num_pages,
            "page_size": page_size,
            "edit_url_base": "/admin/users/",
            "reset_password_url_base": "/admin/users/",
            "prev_page_url": &prev_page_url,
            "next_page_url": &next_page_url,
            "page_url_base": &page_url_base,
            "page_size_suffix": &page_size_suffix,
        }),
    )
}

/// Handler to return the edit form for a user as an HTMX fragment.
#[debug_handler]
async fn get_user_edit_form(
    auth: JWTWithUserOpt<UserModel>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(user_pid): Path<String>,
) -> Result<Response> {
    // 1. Get LayoutData
    let layout_data = UserModel::create_layout_data(auth.user, &ctx).await?;

    // 2. Check admin privileges
    if let Err(response) = check_admin_privileges(&layout_data, &v, &headers).await {
        tracing::debug!("Admin check failed for edit form fragment, returning empty response.");
        // Return empty HTML fragment as per previous pattern for admin check failure on fragments
        return Ok(Html("<!-- Admin privileges required -->".to_string()).into_response());
    }
    // User is admin

    // 3. Find the target user (using UserModel)
    match UserModel::find_by_pid(&ctx.db, &user_pid).await {
        Ok(user) => {
            let update_url = format!("/admin/users/{}", user_pid);
            // URL to fetch the row view again after canceling edit
            let cancel_url = format!("/admin/users/{}/row", user_pid);

            // Render the edit form fragment
            format::render().view(
                &v,
                "admin/_user_edit_form.html",
                data!({
                    "user": &user,
                    "update_url": &update_url,
                    "cancel_url": &cancel_url,
                    "error_message": Option::<String>::None // Ensure error_message is optional and initially None
                }),
            )
        }
        Err(e) => {
            error!("Failed to find user {} for editing: {}", user_pid, e);
            // Return an error message within the fragment target area
            Ok(Html(format!(
                "<div class=\"text-red-500 p-4\">Error loading edit form: Could not find user {}.</div>",
                user_pid
            ))
            .into_response())
        }
    }
}

/// Handler to return the standard view row for a user (after canceling edit).
#[debug_handler]
async fn get_user_row_view(
    auth: JWTWithUserOpt<UserModel>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(user_pid): Path<String>,
) -> Result<Response> {
    // 1. Get LayoutData
    let layout_data = UserModel::create_layout_data(auth.user, &ctx).await?;

    // 2. Check admin privileges
    if let Err(response) = check_admin_privileges(&layout_data, &v, &headers).await {
        tracing::debug!("Admin check failed for row view fragment, returning empty response.");
        return Ok(Html("<!-- Admin privileges required -->".to_string()).into_response());
    }
    // User is admin

    // 3. Find the target user
    match UserModel::find_by_pid(&ctx.db, &user_pid).await {
        Ok(user) => {
            // Construct URLs needed by the _user_row_view template
            let edit_url = format!("/admin/users/{}/edit", user_pid); // URL to get edit form
            let reset_password_url = format!("/admin/users/{}/reset-password", user_pid); // URL for POST reset

            // Render the row view fragment
            format::render().view(
                &v,
                "admin/_user_row_view.html",
                data!({
                    "user": &user,
                    "edit_url": &edit_url,
                    "reset_password_url": &reset_password_url,
                }),
            )
        }
        Err(e) => {
            error!("Failed to find user {} for row view: {}", user_pid, e);
            // Return an error message within the fragment target area (a table row)
            Ok(Html(format!(
                "<tr id=\"user-row-error-{}\"><td colspan=\"3\" class=\"border-dashed border-t border-gray-200 px-6 py-3 text-red-500\">Error loading user data: {}. Refresh page.</td></tr>",
                user_pid, e
            ))
            .into_response())
        }
    }
}

/// Handler for updating user details by an admin.
#[debug_handler]
async fn update_user_details_admin(
    auth: JWTWithUserOpt<UserModel>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(user_pid): Path<String>,
    Form(params): Form<UpdateDetailsParams>,
) -> Result<Response> {
    // 1. Get LayoutData & Check Admin Privileges (This replaces the old require_admin call)
    let layout_data = UserModel::create_layout_data(auth.user, &ctx).await?;
    check_admin_privileges(&layout_data, &v, &headers).await?;
    // User is admin if we reach here

    // 3. Find the target user
    let target_user = match UserModel::find_by_pid(&ctx.db, &user_pid).await {
        Ok(user) => user,
        Err(e) => {
            error!("Admin Update: Failed to find user {}: {}", user_pid, e);
            return error_fragment(
                &v,
                &format!("Error updating user: User {} not found.", user_pid),
                "#admin-user-messages",
            );
        }
    };

    // 4. Attempt to update profile details
    let original_email = target_user.email.clone();
    match target_user.update_profile_details(&ctx.db, &params).await {
        Ok(updated_user) => {
            let email_changed = updated_user.email != original_email;
            info!(
                "Admin updated profile for user {}. Email changed: {}",
                user_pid, email_changed
            );

            if email_changed {
                let user_clone_for_token = updated_user.clone();
                let ctx_clone = ctx.clone();
                tokio::spawn(async move {
                    // Fix: Convert UserModel to UserActiveModel before calling methods
                    let mut active_user_for_token: UserActiveModel =
                        user_clone_for_token.into_active_model();
                    match active_user_for_token
                        .generate_email_verification_token(&ctx_clone.db) // Call on ActiveModel
                        .await
                    {
                        Ok(user_with_token_model) => {
                            // This returns a UserModel
                            let mailer = AuthMailer::new(ctx_clone.clone());
                            let user_mailer_clone = user_with_token_model.clone(); // Clone the returned Model
                            if let Err(e) = mailer.send_verification_email(&user_mailer_clone).await
                            {
                                error!(user_pid = %user_mailer_clone.pid, error = ?e, "Admin Update: Failed to send verification email");
                            } else {
                                // Fix: Convert the returned Model to ActiveModel again
                                let mut active_user_for_sent: UserActiveModel =
                                    user_with_token_model.into_active_model();
                                if let Err(e) = active_user_for_sent
                                    .set_email_verification_sent(&ctx_clone.db) // Call on ActiveModel
                                    .await
                                {
                                    error!(user_pid = %user_pid, error = ?e, "Admin Update: Failed to set email verification sent timestamp");
                                } else {
                                    info!(user_pid = %user_pid, "Admin Update: Verification email sent and timestamp updated for changed email.");
                                }
                            }
                        }
                        Err(e) => {
                            error!(user_pid = %user_pid, error = ?e, "Admin Update: Failed to generate verification token after email change");
                        }
                    }
                });
            }

            // 5. Success Response: Render the updated user row view fragment
            let edit_url = format!("/admin/users/{}/edit", user_pid);
            let reset_password_url = format!("/admin/users/{}/reset-password", user_pid);
            format::render().view(
                &v,
                "admin/_user_row_view.html",
                data!({ "user": &updated_user, "edit_url": &edit_url, "reset_password_url": &reset_password_url }),
            )
        }
        Err(e) => {
            // 6. Validation Failure Response: Re-render edit form
            warn!(
                "Admin update failed for user {}: {}. Submitted data: {:?}",
                user_pid, e, params
            );
            let update_url = format!("/admin/users/{}", user_pid);
            let cancel_url = format!("/admin/users/{}/row", user_pid);
            let error_message = e.to_string();
            format::render().view(
                &v,
                "admin/_user_edit_form.html",
                data!({ "user": &target_user, "update_url": &update_url, "cancel_url": &cancel_url, "error_message": Some(error_message), "form_data": &params }),
            )
        }
    }
}

/// Handler to trigger a password reset email for a user by an admin.
#[debug_handler]
async fn trigger_password_reset_admin(
    auth: JWTWithUserOpt<UserModel>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Path(user_pid): Path<String>,
) -> Result<Response> {
    // 1. Get LayoutData & Check Admin Privileges
    let layout_data = UserModel::create_layout_data(auth.user, &ctx).await?;
    // The '?' operator will return the error response directly if check fails
    check_admin_privileges(&layout_data, &v, &headers).await?;
    // User is admin

    // Extract admin user details for logging (optional but good practice)
    let admin_user_pid = layout_data
        .user
        .map(|u| u.pid.to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    // 3. Find the target user
    let target_user = match UserModel::find_by_pid(&ctx.db, &user_pid).await {
        Ok(user) => user,
        Err(e) => {
            error!(error = ?e, user_pid = user_pid, "Admin Reset PW: Failed to find target user");
            return error_fragment(&v, "Target user not found.", "#admin-user-messages");
        }
    };

    // 4. Initiate password reset (generates token) - This is on Model, returns Model
    let user_with_token = match target_user.initiate_password_reset(&ctx.db).await {
        Ok(u) => u, // Returns a Model
        Err(e) => {
            error!(user_email = %target_user.email, error = ?e, "Admin Reset PW: Failed to initiate password reset (token generation/save)");
            return error_fragment(
                &v,
                "Failed to prepare password reset for user.",
                "#admin-user-messages",
            );
        }
    };

    // 5. Send the password reset email
    let user_mailer_clone = user_with_token.clone();
    let ctx_clone = ctx.clone();
    let target_email_clone = user_with_token.email.clone(); // Clone email for logging

    if let Err(e) = AuthMailer::forgot_password(&ctx_clone, &user_mailer_clone).await {
        error!(user_email = %target_email_clone, error = ?e, "Admin Reset PW: Failed to send forgot password email");
        return error_fragment(
            &v,
            "Failed to send password reset email.",
            "#admin-user-messages",
        );
    }

    // 6. Update the 'sent_at' timestamp
    // Fix: Convert Model to ActiveModel before calling set_forgot_password_sent
    let mut active_user_for_sent: UserActiveModel = user_with_token.into_active_model();
    if let Err(e) = active_user_for_sent
        .set_forgot_password_sent(&ctx_clone.db)
        .await
    {
        // Log the error but proceed with success message to admin, as email was likely sent
        error!(user_email = %target_email_clone, error = ?e, "Admin Reset PW: Failed to set forgot password sent timestamp after sending email");
    }

    // 7. Success Response
    tracing::info!(admin_user_pid=%admin_user_pid, target_user_pid=%user_pid, "Password reset email sent by admin.");
    format::render().view(
        &v,
        "fragments/success_message.html", // Render success fragment
        data!({
            "message": format!("Password reset email sent to {}.", target_email_clone)
        }),
    )
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
