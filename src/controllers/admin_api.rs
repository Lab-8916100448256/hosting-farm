use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use loco_rs::{
    app::AppContext,
    controller::format, // Import format module for rendering
    prelude::*,
};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use tracing::{info, warn};

use crate::{
    middleware::auth_no_error::JWTWithUserOpt, // Use JWTWithUserOpt to handle no-login gracefully
    models::users::{Model as UserModel, USER_STATUS_NEW}, // Import UserModel and status constant
};

#[debug_handler]
async fn get_pending_users_count(
    auth: JWTWithUserOpt<UserModel>, // Use JWTWithUserOpt
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = match auth.user {
        Some(u) => u,
        None => {
            // If not logged in, definitely not an admin, return empty string or FORBIDDEN?
            // Let's return Forbidden for clarity, even though client-side won't show the element.
            warn!("Attempt to access pending user count by unauthenticated user.");
            return Ok(StatusCode::FORBIDDEN.into_response());
        }
    };

    info!(user_pid = %user.pid, "Checking admin status for pending user count");

    // Check if the user is a system admin
    let is_admin = match UserModel::is_system_admin(&ctx, &user).await {
        Ok(is_admin) => is_admin,
        Err(e) => {
            warn!(error = ?e, user_pid = %user.pid, "Failed to check admin status for user");
            // Treat errors checking admin status as forbidden
            return Ok(StatusCode::FORBIDDEN.into_response());
        }
    };

    if !is_admin {
        warn!(user_pid = %user.pid, "Non-admin user attempted to access pending user count.");
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    // Query the count of users with status 'new'
    let count = match crate::models::_entities::users::Entity::find()
        .filter(crate::models::_entities::users::Column::Status.eq(USER_STATUS_NEW))
        .count(&ctx.db)
        .await
    {
        Ok(c) => c,
        Err(e) => {
            error!(error = ?e, user_pid = %user.pid, "Failed to query pending user count");
            // Return empty response on DB error, avoids breaking the UI
            return Ok(Html("".to_string()).into_response());
        }
    };

    info!(user_pid = %user.pid, pending_count = count, "Admin fetched pending user count");

    // Format the response as an HTML badge or empty string
    let badge_html = if count > 0 {
        format!(
            r#"<span class="inline-flex items-center rounded-md bg-red-50 px-2 py-1 text-xs font-medium text-red-700 ring-1 ring-inset ring-red-600/10 ml-2">{} Pending</span>"#,
            count
        )
    } else {
        "".to_string()
    };

    Ok(Html(badge_html).into_response())
}

pub fn routes() -> Router<AppContext> {
    Router::new().route("/pending-users/count", get(get_pending_users_count))
}
