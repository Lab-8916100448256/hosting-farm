use crate::{
    mailers,
    middleware::auth_no_error::JWTWithUserOpt,
    models::users::{self, ForgotPasswordParams, LoginParams, RegisterParams, ResetPasswordParams},
    views::render_template,
    views::{error_fragment, error_page, redirect},
};
use axum::{
    debug_handler,
    extract::{Form, Path, Query, State},
    http::header::HeaderMap,
    http::StatusCode,
    response::{Html, IntoResponse, Response}, // Added Html and Response
    routing::get,
};
use axum_extra::extract::cookie::{Cookie, SameSite}; // Added Cookie

use loco_rs::{
    app::AppContext,
    config::{self, ConfigExt},
    prelude::*,
}; // Added ConfigExt
use serde::Deserialize;
use serde_json::json;
use tera::Tera;
use tracing;

// Authentication pages handlers

#[debug_handler]
async fn login_page(
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<users::Model>,
) -> Result<impl IntoResponse> {
    // Redirect logged in user to home
    if auth.user.is_some() {
        return Ok(redirect("/", HeaderMap::new())); // Use empty headers
    }

    render_template(
        &v,
        "auth/login.html",
        json!({ "title": "Login", "error": Option::<String>::None }),
    )
}

#[debug_handler]
async fn handle_login(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(params): Form<LoginParams>,
) -> Result<Response> {
    tracing::info!(email = params.email, "Attempting login");

    let user = match users::Model::find_by_email_and_password(&ctx.db, &params).await {
        Ok(user) => user,
        Err(e) => {
            tracing::warn!("Login failed for {}: {}", params.email, e);
            let error_message = match e {
                ModelError::Message(msg) => msg,
                ModelError::Unauthorized(_) => "Invalid email or password.".to_string(),
                _ => "Login failed due to an unexpected error.".to_string(),
            };
            return error_fragment(&v, &error_message, "#error-container");
        }
    };

    // Generate JWT token
    let token = user.generate_jwt(&ctx).await?;

    // Create cookie
    let cookie = Cookie::build(("auth_token", token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        // Use config for secure flag (defaults to false if not found or parse error)
        .secure(
            loco_rs::config::ConfigExt::get::<bool>(&ctx.config, "auth.cookie.secure")
                .unwrap_or(false),
        )
        .finish();

    let mut response_headers = HeaderMap::new();
    response_headers.insert(axum::http::header::SET_COOKIE, cookie.to_string().parse()?);
    response_headers.insert("HX-Redirect", "/".parse()?); // Redirect to home after successful login

    // Return Ok response with headers for HTMX redirect
    Ok((StatusCode::OK, response_headers).into_response()) // Use IntoResponse
}

#[debug_handler]
async fn register_page(
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<users::Model>,
) -> Result<impl IntoResponse> {
    // Redirect logged in user to home
    if auth.user.is_some() {
        return Ok(redirect("/", HeaderMap::new())); // Use empty headers
    }

    render_template(
        &v,
        "auth/register.html",
        json!({ "title": "Register", "error": Option::<String>::None }),
    )
}

#[debug_handler]
async fn handle_register(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(mut params): Form<RegisterParams>,
) -> Result<impl IntoResponse> {
    tracing::info!(
        email = params.email,
        name = params.name,
        "Attempting registration"
    );

    // Trim whitespace from username and email
    params.name = params.name.trim().to_string();
    params.email = params.email.trim().to_lowercase();

    let user = match users::Model::create_with_password(&ctx.db, &params).await {
        Ok(user) => {
            tracing::info!(
                "User {} ({}) registered successfully.",
                user.name,
                user.email
            );
            user
        }
        Err(err) => {
            tracing::error!("Registration failed: {}", err);
            let error_message = match err {
                ModelError::Message(msg) => msg, // Use the message directly for uniqueness or other model errors
                ModelError::Validation(validation_err) => {
                    let mut messages = Vec::new();
                    if let Some(errors) = validation_err.field_errors().get("name") {
                        messages.extend(
                            errors
                                .iter()
                                .filter_map(|e| e.message.as_ref().map(|m| m.to_string())),
                        );
                    }
                    if let Some(errors) = validation_err.field_errors().get("email") {
                        messages.extend(
                            errors
                                .iter()
                                .filter_map(|e| e.message.as_ref().map(|m| m.to_string())),
                        );
                    }
                    if let Some(errors) = validation_err.field_errors().get("password") {
                        messages.extend(
                            errors
                                .iter()
                                .filter_map(|e| e.message.as_ref().map(|m| m.to_string())),
                        );
                    }
                    if let Some(errors) = validation_err.field_errors().get("password_confirmation")
                    {
                        messages.extend(
                            errors
                                .iter()
                                .filter_map(|e| e.message.as_ref().map(|m| m.to_string())),
                        );
                    }
                    if messages.is_empty() {
                        "Validation failed.".to_string()
                    } else {
                        messages.join(" ")
                    }
                }
                _ => "Could not register account due to an unexpected error.".to_string(),
            };
            return error_fragment(&v, &error_message, "#error-container");
        }
    };

    // Send verification email if user is not automatically approved (not the first user)
    if user.status != users::USER_STATUS_APPROVED {
        tracing::info!("Sending verification email to new user: {}", user.email);
        let token = user.generate_email_verification_token(&ctx.db).await?;
        match mailers::auth::AuthMailer::send_verification(&ctx, &user, &token).await {
            Ok(_) => tracing::info!("Verification email sent successfully to {}", user.email),
            Err(e) => tracing::error!("Failed to send verification email to {}: {}", user.email, e),
        }
    }

    // Redirect to login page with a success message (or potentially a verify email page)
    let redirect_url = if user.status == users::USER_STATUS_APPROVED {
        "/auth/login?success=Registration successful! You can now log in."
    } else {
        "/auth/login?success=Registration successful! Please check your email to verify your account."
    };
    redirect(redirect_url, headers)
}

#[debug_handler]
async fn forgot_password_page(
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<users::Model>,
) -> Result<impl IntoResponse> {
    // Redirect logged in user to home
    if auth.user.is_some() {
        return Ok(redirect("/", HeaderMap::new()));
    }

    render_template(
        &v,
        "auth/forgot-password.html",
        json!({ "title": "Forgot Password", "error": Option::<String>::None }),
    )
}

#[debug_handler]
async fn handle_forgot_password(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(params): Form<ForgotPasswordParams>,
) -> Result<impl IntoResponse> {
    tracing::info!(email = params.email, "Forgot password request");

    match users::Model::find_by_email(&ctx.db, &params.email).await {
        Ok(user) => {
            let token = user.generate_reset_token(&ctx.db).await?;
            match mailers::auth::AuthMailer::forgot_password(&ctx, &user, &token).await {
                Ok(_) => tracing::info!("Password reset email sent to {}", user.email),
                Err(e) => tracing::error!(
                    "Failed to send password reset email to {}: {}",
                    user.email,
                    e
                ),
            }
            // Always redirect to success page to avoid leaking user existence
            redirect("/auth/reset-email-sent", headers)
        }
        Err(ModelError::EntityNotFound) => {
            tracing::warn!(
                "Password reset requested for non-existent email: {}",
                params.email
            );
            // User not found, but act as if successful to prevent email enumeration
            redirect("/auth/reset-email-sent", headers)
        }
        Err(e) => {
            tracing::error!("Error during forgot password process: {}", e);
            error_fragment(&v, "An unexpected error occurred.", "#error-container")
        }
    }
}

#[debug_handler]
async fn reset_email_sent_page(
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<users::Model>,
) -> Result<impl IntoResponse> {
    // Redirect logged in user to home
    if auth.user.is_some() {
        return Ok(redirect("/", HeaderMap::new()));
    }

    render_template(
        &v,
        "auth/reset-email-sent.html",
        json!({ "title": "Reset Email Sent" }),
    )
}

#[derive(Deserialize)]
pub struct ResetTokenQuery {
    token: String,
}

#[debug_handler]
async fn reset_password_page(
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<users::Model>,
    Query(query): Query<ResetTokenQuery>,
) -> Result<impl IntoResponse> {
    // Redirect logged in user to home
    if auth.user.is_some() {
        return Ok(redirect("/", HeaderMap::new()));
    }

    // TODO: Optionally validate the token here to show an error if invalid?
    //       Or just let the form handler deal with it.
    render_template(
        &v,
        "auth/reset-password.html",
        json!({ "title": "Reset Password", "token": query.token, "error": Option::<String>::None }),
    )
}

#[debug_handler]
async fn handle_reset_password(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(params): Form<ResetPasswordParams>,
) -> Result<impl IntoResponse> {
    tracing::info!(token = params.reset_token, "Attempting password reset");

    match users::Model::reset_password(&ctx.db, &params).await {
        Ok(_) => {
            tracing::info!("Password reset successful for token {}", params.reset_token);
            // Redirect to login page with success message
            redirect("/auth/login?success=Password reset successful. You can now log in with your new password.", headers)
        }
        Err(e) => {
            tracing::error!(
                "Password reset failed for token {}: {}",
                params.reset_token,
                e
            );
            let error_message = match e {
                ModelError::Message(msg) => msg, // Use Message for token errors too
                ModelError::Validation(err) => {
                    // Extract validation messages
                    err.field_errors()
                        .values()
                        .flatten()
                        .filter_map(|e| e.message.as_ref().map(|m| m.to_string()))
                        .collect::<Vec<_>>()
                        .join(" ")
                }
                _ => "Password reset failed due to an unexpected error.".to_string(),
            };
            // Re-render the reset page with the token and error
            let context = json!({
                "title": "Reset Password",
                "token": params.reset_token,
                "error": error_message
            });
            match v.render("auth/reset-password.html", context) {
                Ok(html) => Ok(Html(html).into_response()),
                Err(tera_err) => {
                    tracing::error!(
                        "Failed to render reset password page after error: {}",
                        tera_err
                    );
                    error_page(
                        &v,
                        "An unexpected error occurred.",
                        Some(Error::View(tera_err)), // Map Tera error to loco Error::View
                    )
                }
            }
        }
    }
}


#[derive(Deserialize)]
pub struct VerifyTokenQuery {
    token: String,
}

#[debug_handler]
async fn verify_email_page(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Query(query): Query<VerifyTokenQuery>,
) -> Result<impl IntoResponse> {
    tracing::info!(token = query.token, "Attempting email verification");

    let user = match users::Entity::find()
        .filter(users::Column::EmailVerificationToken.eq(query.token.clone()))
        .one(&ctx.db)
        .await
    {
        Ok(Some(user)) => users::Model::from(user),
        Ok(None) => {
            tracing::warn!(
                "Invalid or expired email verification token: {}",
                query.token
            );
            return render_template(
                &v,
                "auth/verify.html",
                json!({ "title": "Email Verification Failed", "success": false, "message": "Invalid or expired verification link." }),
            );
        }
        Err(e) => {
            tracing::error!("Database error during email verification: {}", e);
            return error_page(
                &v,
                "An unexpected error occurred during verification.",
                Some(Error::from(e)),
            );
        }
    };

    match user.verify_email(&ctx.db, &query.token).await {
        Ok(_) => {
            tracing::info!("Email verified successfully for user {}", user.email);
            render_template(
                &v,
                "auth/verify.html",
                json!({ "title": "Email Verified", "success": true, "message": "Your email has been verified successfully!" }),
            )
        }
        Err(e) => {
            tracing::error!("Email verification failed for user {}: {}", user.email, e);
            let message = match e {
                ModelError::Message(msg) => msg, // Use Message for token errors too
                _ => "Email verification failed due to an unexpected error.".to_string(),
            };
            render_template(
                &v,
                "auth/verify.html",
                json!({ "title": "Email Verification Failed", "success": false, "message": message }),
            )
        }
    }
}

#[debug_handler]
async fn logout(State(ctx): State<AppContext>, headers: HeaderMap) -> Result<Response> {
    tracing::info!("Logging out user");
    let cookie = Cookie::build(("auth_token", ""))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        // Use config for secure flag
        .secure(
            loco_rs::config::ConfigExt::get::<bool>(&ctx.config, "auth.cookie.secure")
                .unwrap_or(false),
        )
        .max_age(time::Duration::ZERO) // Expire the cookie
        .finish();

    let mut response_headers = HeaderMap::new();
    response_headers.insert(axum::http::header::SET_COOKIE, cookie.to_string().parse()?);
    response_headers.insert("HX-Redirect", "/auth/login".parse()?);

    // Return Ok response with headers
    Ok((StatusCode::OK, response_headers).into_response()) // Use IntoResponse
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/auth")
        .add("/", get(login_page).post(handle_login))
        .add("/login", get(login_page).post(handle_login))
        .add("/register", get(register_page).post(handle_register))
        .add("/logout", post(logout))
        .add(
            "/forgot-password",
            get(forgot_password_page).post(handle_forgot_password),
        )
        .add("/reset-email-sent", get(reset_email_sent_page))
        .add(
            "/reset-password",
            get(reset_password_page).post(handle_reset_password),
        )
        .add("/verify", get(verify_email_page))
}
