use axum::debug_handler;
use loco_rs::prelude::*;
use crate::models::users;

/// Renders the home page
#[debug_handler]
async fn index(
    State(ctx): State<AppContext>,
    auth: auth::JWTAuth,
) -> Result<Response> {
    let mut context = tera::Context::new();
    
    // If user is logged in, add user information to the context
    if let auth::JWTAuth::JWT(jwt) = auth {
        let user = users::Model::find_by_pid(&ctx.db, &jwt.claims.pid).await?;
        context.insert("user", &user);
    }
    
    format::render_template(&ctx, "home/index.html.tera", context)
}

/// Home routes
pub fn routes() -> Routes {
    Routes::new()
        .add("/", get(index))
} 