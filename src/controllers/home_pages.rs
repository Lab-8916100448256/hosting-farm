use axum::debug_handler;
use loco_rs::prelude::*;
use tera::Context; // Import tera::Context

use crate::{
    middleware::auth_no_error::JWTWithUserOpt,
    models::users::{self, LayoutContext, USER_STATUS_NEW}, // Import LayoutContext and USER_STATUS_NEW
    views, // Import views module
};

#[debug_handler]
async fn home(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    match auth.user {
        Some(user) => {
            tracing::info!(
                message = "generating home page for authenticated user,",
                user_email = &user.email,
                user_status = &user.status,
            );

            // Call the new layout context builder
            let layout_context: LayoutContext = user
                .get_layout_context(&ctx, "home")
                .await
                .map_err(|e| {
                    // Log the error and return a generic internal error
                    // Avoid exposing detailed model errors directly
                    tracing::error!("Failed to get layout context for home page: {}", e);
                    Error::InternalServerError
                })?;

            // Convert LayoutContext to tera::Context
            let mut context = Context::from_serialize(&layout_context).map_err(|e| {
                tracing::error!("Failed to serialize layout context for home page: {}", e);
                Error::InternalServerError
            })?;

            // Add page-specific context: check if user is pending approval
            context.insert("is_pending_approval", &(layout_context.user.status == USER_STATUS_NEW));

            // Render the template using the prepared context
            v.render("home/index.html", context)
        }
        None => {
            // Render the index view, non-authenticated user parameters
            tracing::info!(
                message = "generating home page for non-authenticated user,",
            );
            // Use the view function for rendering (logged out state)
            // which sets its own context including active_page
            views::home::index_logged_out(&v)
        }
    }
}

/// Home page routes
pub fn routes() -> Routes {
    Routes::new()
      .add("/", get(home))
      .add("/home", get(home))
}
