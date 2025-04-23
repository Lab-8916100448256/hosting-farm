use axum::debug_handler;
use loco_rs::prelude::*;
use tera::Context;

use crate::{middleware::auth_no_error::JWTWithUserOpt, models::users, views::render_template};

#[debug_handler]
async fn home(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    // Create layout data regardless of auth status
    let layout_data = users::Model::create_layout_data(auth.user, &ctx).await?;

    // Prepare page-specific context (empty for this page)
    let page_context = Context::new();

    if let Some(ref user) = layout_data.user {
        tracing::info!(
            message = "generating home page for authenticated user,",
            user_email = &user.email,
        );
        // Invitation count calculation removed as it's handled by HTMX badge in layout
    } else {
        tracing::info!(message = "generating home page for non-authenticated user,",);
    }

    // Render the template using the helper function
    render_template(
        &v,
        "home/index.html",
        Some("home"),
        layout_data,
        page_context,
    )
}

/// Home page routes
pub fn routes() -> Routes {
    Routes::new().add("/", get(home)).add("/home", get(home))
}
