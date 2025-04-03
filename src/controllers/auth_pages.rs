use crate::{
    mailers::auth::AuthMailer,
    middleware::auth_no_error::JWTWithUserOpt,
    models::{
        users,
        users::{LoginParams, RegisterParams, ForgotPasswordParams, ResetPasswordParams},
    },
    utils::template::render_template,
};
use axum::debug_handler;
use axum::extract::{Form, Query};
use axum::http::HeaderMap;
use loco_rs::prelude::*;
use std::collections::HashMap;


/// Renders the registration page
#[debug_handler]
async fn register(State(ctx): State<AppContext>) -> Result<Response> {
    let context = tera::Context::new();
    render_template(&ctx, "auth/register.html", context)
}

/// Handles the registration form submission
#[debug_handler]
async fn handle_register(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Form(form): Form<RegisterParams>,
) -> Result<Response> {
    // Check if passwords match
    if form.password != form.password_confirmation {
        return format::render().view(
            &v,
            "error.html",
            data!({"message": "Password and password confirmation do not match"}),
        );
    }

    // Convert form data to RegisterParams
    let params = RegisterParams {
        name: form.name,
        email: form.email,
        password: form.password,
        password_confirmation: form.password_confirmation,
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
            let response = Response::builder()
                .header("HX-Redirect", "/auth/login?registered=true")
                .body(axum::body::Body::empty())?;
            Ok(response)
        }
        Err(err) => {
            tracing::info!(
                message = err.to_string(),
                user_email = &params.email,
                "could not register user",
            );

            let err_message = match err {
                ModelError::EntityAlreadyExists => format!("Account already exists"),
                ModelError::EntityNotFound => format!("Entity not found"),
                ModelError::Validation(err) => format!("Validation error: {}", err),
                ModelError::Jwt(err) => format!("JWT error: {}", err),
                ModelError::DbErr(err) => format!("Database error: {}", err),
                ModelError::Any(err) => format!("{}", err),
                ModelError::Message(err) => format!("{}", err),
            };
            format::render().view(
                &v,
                "error.html",
                data!({"message": format!("Could not register account: {}", err_message)}),
            )
        }
    }
}

/// Renders the login page
#[debug_handler]
async fn login(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(_ctx): State<AppContext>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Response> {
    match auth.user {
        Some(_user) => {
            // User is already authenticated, redirect to home
            let response = Response::builder()
                .header("HX-Redirect", "/home")
                .body(axum::body::Body::empty())?;
            return Ok(response);
        }
        None => {
            let registered = params.get("registered") == Some(&"true".to_string());
            let mut email = "";
            // Parse cookies from headers
            if let Some(cookie_header) = headers.get("cookie").and_then(|h| h.to_str().ok()) {
                // Check if user has a "remember me" cookie
                for cookie in cookie_header.split(';').map(|s| s.trim()) {
                    if cookie.starts_with("remembered_email=") {
                        email = &cookie["remembered_email=".len()..];
                        break;
                    }
                }
            }

            format::render().view(
                &v,
                "auth/login.html",
                data!({
                "registered": registered, 
                "email": email}),
            )
        }
    }
}

/// Handles the login form submission
#[debug_handler]
async fn handle_login(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Form(form): Form<LoginParams>,
) -> Result<Response> {
    // Convert form data to login params
    let params = LoginParams {
        email: form.email.clone(),
        password: form.password,
        remember_me: form.remember_me,
    };

    // Try to login
    let user_result = users::Model::find_by_email(&ctx.db, &params.email).await;

    match user_result {
        Ok(user) => {
            let valid = user.verify_password(&params.password);

            if !valid {
                tracing::info!(
                    message = "Invalid password in login attempt,",
                    user_email = &params.email,
                );             
                return format::render().view(
                    &v,
                    "error.html",
                    data!({"message": "Log in failed: Invalid email or password"})
                );
            };

            let jwt_secret = ctx.config.get_jwt_config()?;

            let token = match user.generate_jwt(&jwt_secret.secret, &jwt_secret.expiration) {
                Ok(token) => token,
                Err(err) => {
                    tracing::error!(
                        message = "Failed to generate JWT token,",
                        user_email = &params.email,
                        error = err.to_string(),
                    );
                    return format::render().view(
                        &v,
                        "error.html",
                        data!({"message": "Log in failed: Failed to generate JWT token"})
                    );                    
                }
            };

            // Prepare response with auth token cookie and redirect
            let mut response_builder = Response::builder()
                .header("HX-Redirect", "/home")
                .header("Set-Cookie", format!("auth_token={}; Path=/", token));

            // If remember me is checked, set a persistent cookie with email
            if params.remember_me.is_some() {
                // Set a permanent cookie (Max-Age = 1 year)
                response_builder = response_builder.header(
                    "Set-Cookie",
                    format!("remembered_email={}; Path=/; Max-Age=31536000", form.email),
                );
            }

            let response = response_builder.body(axum::body::Body::empty())?;
            Ok(response)
        }
        Err(_) => {
            tracing::info!(
                message = "Unknown user login attempt,",
                user_email = &params.email,
            );
            format::render().view(
                &v,
                "error.html",
                data!({
                "message": "Log in failed: Invalid email or password", 
                "email": &params.email}),
            )
        }
    }
}

/// Renders the forgot password page
#[debug_handler]
async fn forgot_password(
    ViewEngine(v): ViewEngine<TeraView>,
    State(_ctx): State<AppContext>,
) -> Result<Response> {
    format::render().view(
        &v,
        "auth/forgot-password.html",
        data!({}),
    )
}

#[debug_handler]
async fn handle_forgot_password(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Form(form): Form<ForgotPasswordParams>,
) -> Result<Response> {
    let params = ForgotPasswordParams {
        email: form.email.clone(),
    };

    // Find user by email
    match users::Model::find_by_email(&ctx.db, &params.email).await {
        Ok(user) => {
            // User found, generate reset token and send email
            match user
                .into_active_model()
                .set_forgot_password_sent(&ctx.db)
                .await
            {
                Ok(updated_user) => {
                    // Send forgot password email
                    if let Err(e) = AuthMailer::forgot_password(&ctx, &updated_user).await {
                        // Log error but proceed to render confirmation page
                        tracing::error!(
                            "Failed to send forgot password email to {}: {}",
                            &params.email,
                            e
                        );
                    } else {
                         tracing::info!("Forgot password email sent to {}", &params.email);
                    }
                }
                Err(e) => {
                    // Log error but proceed to render confirmation page
                    tracing::error!(
                        "Failed to set forgot password token for {}: {}",
                        &params.email,
                        e
                    );
                }
            }
        }
        Err(ModelError::EntityNotFound) => {
            // User not found, log but proceed as if successful to prevent email enumeration
             tracing::info!("Forgot password attempt for non-existent user: {}", &params.email);
        }
        Err(e) => {
            // Other DB error, log but proceed
            tracing::error!("Database error during forgot password for {}: {}", &params.email, e);
        }
    }

    // Always render the confirmation page, regardless of whether the user was found or email sent successfully.
    format::render().view(
        &v,
        "auth/reset-email-sent.html",
        data!({}),
    )
}

/// Renders the reset password page
#[debug_handler]
async fn reset_password(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Path(token): Path<String>,
) -> Result<Response> {
    // Attempt to find the user by the reset token
    match users::Model::find_by_reset_token(&ctx.db, &token).await {
        Ok(user) => {
            // Check if the token is still valid (e.g., within an expiration timeframe)
            // Assuming the find_by_reset_token implicitly checks validity or that validity check happens on POST.
            // For now, just render the form if the token leads to a user.
            if user.reset_token.is_some() && user.reset_sent_at.is_some() {
                 // Optional: Add explicit time validation if needed
                 // let expiry_duration = Duration::hours(1); // Example: 1 hour validity
                 // if user.reset_sent_at.unwrap() + expiry_duration < chrono::Utc::now().naive_utc() { ... handle expired ... }

                format::render().view(
                    &v,
                    "auth/reset-password.html",
                    data!({ "token": token }), // Pass only token when valid
                )
            } else {
                // Token exists but seems invalid (e.g., already used)
                 format::render().view(
                    &v,
                    "auth/reset-password.html",
                    data!({ "error": "Invalid or expired reset link." }), // Pass only error when invalid
                )
            }
        }
        Err(_) => {
            // Token not found or other DB error
            format::render().view(
                &v,
                "auth/reset-password.html",
                data!({ "error": "Invalid or expired reset link." }), // Pass only error when invalid
            )
        }
    }
}

/// Renders the email verification page
#[debug_handler]
async fn verify_email(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Path(token): Path<String>,
) -> Result<Response> {
    let user_result = users::Model::find_by_verification_token(&ctx.db, &token).await;

    match user_result {
        Ok(user) => {
            if user.email_verified_at.is_some() {
                tracing::info!(pid = user.pid.to_string(), "user already verified");
                return format::render().view(
                    &v,
                    "auth/verify.html",
                    data!({
                        "success": true,
                        "message": "Your email has already been verified.",
                    }),
                );
            } else {
                let active_model = user.into_active_model();
                let _user = active_model.verified(&ctx.db).await?;
                tracing::info!(pid = _user.pid.to_string(), "user verified");
                return format::render().view(
                    &v,
                    "auth/verify.html",
                    data!({
                        "success": true,
                        "message": "Your email has been verified.",
                    }),
                );
            }
        }
        Err(_) => {
            return format::render().view(
                &v,
                "auth/verify.html",
                data!({
                    "success": false,
                    "message": "Invalid or expired verification token.",
                }),
            );
        }
    }
}

/// Handles user logout by clearing the auth token cookie
#[debug_handler]
async fn handle_logout() -> Result<Response> {
    // TODO: Implement server-side JWT invalidation (e.g., token blacklist)
    // This implementation only clears the client-side cookie. The JWT itself
    // remains valid until it expires. For a fully secure logout, the token
    // should be invalidated on the server-side as well.
    let response = Response::builder()
        .header("HX-Redirect", "/auth/login")
        .header(
            "Set-Cookie",
            "auth_token=; Path=/; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
        )
        .body(axum::body::Body::empty())?;
    Ok(response)
}

/// Handles the reset password form submission
#[debug_handler]
async fn handle_reset_password(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Form(form): Form<ResetPasswordParams>,
) -> Result<Response> {
    // Check if passwords match
    if form.password != form.password_confirmation {
        return format::render().view(
            &v,
            "error.html",
            data!({ "message": "New password and confirmation do not match." }),
        );
    }

    // Find user by reset token
    match users::Model::find_by_reset_token(&ctx.db, &form.token).await {
        Ok(user) => {
            // Check if token is actually associated with this user and potentially check expiry
            if user.reset_token.as_deref() == Some(&form.token) && user.reset_sent_at.is_some() {
                
                // Use the reset_password method on ActiveModel
                match user
                    .into_active_model()
                    .reset_password(&ctx.db, &form.password)
                    .await
                {
                    Ok(_) => {
                        // Redirect to login page with success message
                        let response = Response::builder()
                            .header("HX-Redirect", "/auth/login?reset=success")
                            .body(axum::body::Body::empty())?;
                        Ok(response)
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to update password for token {}: {}",
                            &form.token, e
                        );
                        format::render().view(
                            &v,
                            "error.html",
                            data!({
                                "message": "Failed to reset password. Please try again."
                            }),
                        )
                    }
                }
            } else {
                // Token mismatch or already cleared - treat as invalid
                format::render().view(
                    &v,
                    "error.html",
                    data!({
                        "message": "Invalid or expired password reset link."
                    }),
                )
            }
        }
        Err(_) => {
            // User not found for the token
            format::render().view(
                &v,
                "error.html",
                data!({
                    "message": "Invalid or expired password reset link."
                }),
            )
        }
    }
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
        .add("/forgot-password", post(handle_forgot_password))
        .add("/reset-password/{token}", get(reset_password))
        .add("/reset-password", post(handle_reset_password))
        .add("/verify/{token}", get(verify_email))
        .add("/logout", post(handle_logout))
}
