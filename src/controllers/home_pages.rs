use axum::debug_handler;
use loco_rs::prelude::*;
use crate::models::users;
use tera;
use crate::utils::template::render_template;

/// Renders the home page for non-authenticated users
#[debug_handler]
async fn index(State(ctx): State<AppContext>) -> Result<Response> {
    let mut context = tera::Context::new();
    context.insert("active_page", "home");
    context.insert("invitation_count", &0);
    render_template(&ctx, "home/index.html.tera", context)
}

/// Renders the home page for authenticated users
#[debug_handler]
async fn authenticated_index(
    State(ctx): State<AppContext>,
    auth: auth::JWT,
) -> Result<Response> {
    let mut context = tera::Context::new();
    
    if let Ok(user) = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await {
        context.insert("user", &user);
        context.insert("active_page", "home");
        context.insert("invitation_count", &0); // TODO: Get actual invitation count
    }
    
    render_template(&ctx, "home/index.html.tera", context)
}

/// Home page routes
pub fn routes() -> Routes {
    Routes::new()
        .add("/", get(index))
        .add("/home", get(authenticated_index))
} 