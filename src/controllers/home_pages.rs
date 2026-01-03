use axum::debug_handler;
use loco_rs::prelude::*;

use crate::{middleware::auth_no_error::JWTWithUserOpt, models::users};
use axum::http::HeaderMap;

#[debug_handler]
async fn home(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    if let Some(oidc_header) = headers.get("X-Oidc-Roles").and_then(|h| h.to_str().ok()) {
        tracing::info!("X-Oidc-roRolesles = {}", oidc_header);
    } else {
        tracing::info!("no X-Oidc-Roles header found");
    }
    match auth.user {
        Some(user) => {
            tracing::info!(
                message = "generating home page for authenticated user,",
                user_email = &user.email,
            );

            let layout_context = user.get_base_layout_context(&ctx.db, &ctx).await;

            // Render the index view, authenticated user parameters
            format::render().view(
                &v,
                "home/index.html",
                data!({
                    "user": user,
                    "active_page": "home",
                    "invitation_count": layout_context.invitation_count,
                    "pending_user_count": layout_context.pending_user_count,
                    "is_app_admin": layout_context.is_app_admin,
                }),
            )
        }
        None => {
            // Render the index view, non-authenticated user parameters
            tracing::info!(message = "generating home page for non-authenticated user,",);
            format::render().view(
                &v,
                "home/index.html",
                data!({
                "active_page": "home"}),
            )
        }
    }
}

/// Home page routes
pub fn routes() -> Routes {
    Routes::new().add("/", get(home)).add("/home", get(home))
}
