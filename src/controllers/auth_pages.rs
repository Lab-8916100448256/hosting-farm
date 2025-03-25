use axum::debug_handler;
use loco_rs::prelude::*;
use tera;
use crate::{
    mailers::auth::AuthMailer,
    models::{
        _entities::users,
        users::{LoginParams, RegisterParams},
    },
    utils::template::render_template,
    views::auth::LoginResponse,
};
use serde::Deserialize;
use axum::response::Redirect;
use axum::extract::{Form, Query};
use std::collections::HashMap;

/// Form data for user registration
#[derive(Debug, Deserialize)]
pub struct RegisterFormData {
    pub name: String,
    pub email: String,
    pub password: String,
    pub password_confirmation: String,
}

/// Form data for user login
#[derive(Debug, Deserialize)]
pub struct LoginFormData {
    pub email: String,
    pub password: String,
}

/// Renders the registration page
#[debug_handler]
async fn register(
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let context = tera::Context::new();
    render_template(&ctx, "auth/register.html.tera", context)
}

/// Handles the registration form submission
#[debug_handler]
async fn handle_register(
    State(ctx): State<AppContext>,
    Form(form): Form<RegisterFormData>,
) -> Result<Response> {
    // Check if passwords match
    if form.password != form.password_confirmation {
        let mut context = tera::Context::new();
        context.insert("error", "Passwords do not match");
        context.insert("name", &form.name);
        context.insert("email", &form.email);
        return render_template(&ctx, "auth/register.html.tera", context);
    }
    
    // Convert form data to RegisterParams
    let params = RegisterParams {
        name: form.name,
        email: form.email,
        password: form.password,
    };
    
    // Create user account
    let res = users::Model::create_with_password(&ctx.db, &params).await;
    
    match res {
        Ok(user) => {
            // Send verification email
            let user = user
                .into_active_model()
                .set_email_verification_sent(&ctx.db)
                .await?;
                
            AuthMailer::send_welcome(&ctx, &user).await?;
            
            // Redirect to login page with success message
            Ok(Redirect::to("/auth/login?registered=true").into_response())
        },
        Err(err) => {
            tracing::info!(
                message = err.to_string(),
                user_email = &params.email,
                "could not register user",
            );
            
            let mut context = tera::Context::new();
            context.insert("error", "Registration failed. Email may already be in use.");
            context.insert("name", &params.name);
            context.insert("email", &params.email);
            render_template(&ctx, "auth/register.html.tera", context)
        }
    }
}

/// Renders the login page
#[debug_handler]
async fn login(
    State(ctx): State<AppContext>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response> {
    let mut context = tera::Context::new();
    
    // Check if coming from registration success
    if params.get("registered") == Some(&"true".to_string()) {
        context.insert("registered", &true);
    }
    
    render_template(&ctx, "auth/login.html.tera", context)
}

/// Handles the login form submission
#[debug_handler]
async fn handle_login(
    State(ctx): State<AppContext>,
    Form(form): Form<LoginFormData>,
) -> Result<Response> {
    // Convert form data to login params
    let params = LoginParams {
        email: form.email,
        password: form.password,
    };
    
    // Try to login
    let user_result = users::Model::find_by_email(&ctx.db, &params.email).await;
    
    match user_result {
        Ok(user) => {
            let valid = user.verify_password(&params.password);
            
            if !valid {
                let mut context = tera::Context::new();
                context.insert("error", "Invalid email or password");
                context.insert("email", &params.email);
                return render_template(&ctx, "auth/login.html.tera", context);
            }
            
            let jwt_secret = ctx.config.get_jwt_config()?;
            
            let token = match user.generate_jwt(&jwt_secret.secret, &jwt_secret.expiration) {
                Ok(token) => token,
                Err(_) => {
                    let mut context = tera::Context::new();
                    context.insert("error", "Authentication error");
                    context.insert("email", &params.email);
                    return render_template(&ctx, "auth/login.html.tera", context);
                }
            };
            
            // Set token in session and redirect to home
            // For HTMX, we'll use a special response with HX-Redirect header
            let response = Response::builder()
                .header("HX-Redirect", "/home")
                .header("Set-Cookie", format!("auth_token={}; Path=/", token))
                .body(axum::body::Body::empty())?;
                
            Ok(response)
        },
        Err(_) => {
            let mut context = tera::Context::new();
            context.insert("error", "Invalid email or password");
            context.insert("email", &params.email);
            render_template(&ctx, "auth/login.html.tera", context)
        }
    }
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
        .add("/register", post(handle_register))
        .add("/login", get(login))
        .add("/login", post(handle_login))
        .add("/forgot-password", get(forgot_password))
        .add("/reset-password/{token}", get(reset_password))
} 