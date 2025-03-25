use axum::debug_handler;
use loco_rs::prelude::*;
use tera;
use crate::utils::template::render_template;

/// Renders the registration page
#[debug_handler]
async fn register(
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let context = tera::Context::new();
    render_template(&ctx, "auth/register.html.tera", context)
}

/// Renders the login page
#[debug_handler]
async fn login(
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let context = tera::Context::new();
    render_template(&ctx, "auth/login.html.tera", context)
}

/// Renders the forgot password page
#[debug_handler]
async fn forgot_password(
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let context = tera::Context::new();
    render_template(&ctx, "auth/forgot-password.html.tera", context)
}

/// Renders the reset password page
#[debug_handler]
async fn reset_password(
    State(ctx): State<AppContext>,
    Path(token): Path<String>,
) -> Result<Response> {
    let mut context = tera::Context::new();
    context.insert("token", &token);
    render_template(&ctx, "auth/reset-password.html.tera", context)
}

/// Authentication page routes
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/auth")
        .add("/register", get(register))
        .add("/login", get(login))
        .add("/forgot-password", get(forgot_password))
        .add("/reset-password/:token", get(reset_password))
} 