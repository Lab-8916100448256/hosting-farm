use axum::debug_handler;
use loco_rs::prelude::*;
use crate::models::users;
use serde_json::json;
use tera;
use crate::utils::template::render_template;

/// Renders the home page for non-authenticated users
#[debug_handler]
async fn index(State(ctx): State<AppContext>) -> Result<Response> {
    let context = tera::Context::new();
    render_template(&ctx, "home/index.html.tera", context)
}

/// Renders the home page for authenticated users
#[debug_handler]
async fn authenticated_index(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    let mut context = tera::Context::new();
    context.insert("user", &user);
    
    render_template(&ctx, "home/index.html.tera", context)
}

/// Home routes
pub fn routes() -> Routes {
    Routes::new()
        .add("/", get(index))
        .add("/home", get(authenticated_index))
} 