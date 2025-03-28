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
};
use serde::Deserialize;
use axum::response::Redirect;
use axum::extract::{Form, Query};
use axum::http::HeaderMap;
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
    #[serde(rename = "remember-me")]
    pub remember_me: Option<String>,
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
    headers: axum::http::HeaderMap,
) -> Result<Response> {
    let mut context = tera::Context::new();
    
    // Check if coming from registration success
    if params.get("registered") == Some(&"true".to_string()) {
        context.insert("registered", &true);
    }
    
    // Parse cookies from headers
    let cookie_header = headers.get(axum::http::header::COOKIE)
        .and_then(|value| value.to_str().ok());
    
    if let Some(cookie_str) = cookie_header {
        // Check if user is already authenticated
        if cookie_str.contains("auth_token=") {
            for cookie in cookie_str.split(';') {
                let cookie = cookie.trim();
                if cookie.starts_with("auth_token=") && !cookie.eq("auth_token=") {
                    // User is already authenticated, redirect to home
                    return Ok(Redirect::to("/home").into_response());
                }
            }
        }
        
        // Check for remembered email
        for cookie in cookie_str.split(';') {
            let cookie = cookie.trim();
            if cookie.starts_with("remembered_email=") {
                let email = cookie["remembered_email=".len()..].to_string();
                if !email.is_empty() {
                    context.insert("email", &email);
                }
                break;
            }
        }
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
        email: form.email.clone(),
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
            
            // Prepare response with auth token cookie and redirect
            let mut response_builder = Response::builder()
                .header("HX-Redirect", "/home")
                .header("Set-Cookie", format!("auth_token={}; Path=/", token));
                
            // If remember me is checked, set a persistent cookie with email
            if form.remember_me.is_some() {
                // Set a permanent cookie (Max-Age = 1 year)
                response_builder = response_builder.header(
                    "Set-Cookie", 
                    format!("remembered_email={}; Path=/; Max-Age=31536000", form.email)
                );
            }
            
            let response = response_builder.body(axum::body::Body::empty())?;    
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

/// Renders the email verification page
#[debug_handler]
async fn verify_email(
    State(ctx): State<AppContext>,
    Path(token): Path<String>,
) -> Result<Response> {
    let user_result = users::Model::find_by_verification_token(&ctx.db, &token).await;
    
    let mut context = tera::Context::new();
    
    match user_result {
        Ok(user) => {
            if user.email_verified_at.is_some() {
                tracing::info!(pid = user.pid.to_string(), "user already verified");
                context.insert("success", &true);
                context.insert("error_message", "Your email is already verified.");
            } else {
                let active_model = user.into_active_model();
                let _user = active_model.verified(&ctx.db).await?;
                tracing::info!(pid = _user.pid.to_string(), "user verified");
                context.insert("success", &true);
                context.insert("error_message", "");
            }
        }
        Err(_) => {
            context.insert("success", &false);
            context.insert("error_message", "Invalid or expired verification token.");
        }
    }
    
    render_template(&ctx, "auth/verify.html.tera", context)
}

/// Handles user logout by clearing the auth token cookie
#[debug_handler]
async fn handle_logout() -> Result<Response> {
    let response = Response::builder()
        .header("Set-Cookie", "auth_token=; Path=/; Expires=Thu, 01 Jan 1970 00:00:00 GMT")
        .header("HX-Redirect", "/auth/login")
        .body(axum::body::Body::empty())?;
    Ok(response)
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
        .add("/reset-password", get(reset_password))
        .add("/verify/{token}", get(verify_email))
        .add("/logout", post(handle_logout))
} 