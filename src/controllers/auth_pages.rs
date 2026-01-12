use crate::{
    mailers::auth::AuthMailer,
    middleware::auth_no_error::JWTWithUserOpt,
    models::{
        users,
        users::{ForgotPasswordParams, LoginParams, RegisterParams, ResetPasswordParams},
    },
    views::render_template,
    views::*,
};
use tracing::{error, info}; // Import tracing macros

use axum::http::{HeaderMap, header::HeaderValue};
use axum::{
    debug_handler,
    extract::{Form, Path, Query, State},
};
use loco_rs::prelude::*;
use std::collections::HashMap;

use rand::Rng;
use rand_distr::Alphanumeric;

/// Helper function to get the oidc auth enable setting from config
fn is_oidc_auth(ctx: &AppContext) -> bool {
    ctx.config
        .settings
        .as_ref() // Get Option<&Value>
        .and_then(|settings| settings.get("app")) // Get Option<&Value> for "app" key
        .and_then(|app_settings| app_settings.get("oidc_auth")) // Get Option<&Value> for "oidc_auth"
        .and_then(|value| value.as_bool()) // Get Option<bool>
        .unwrap_or_else(|| {
            tracing::warn!(
                "'app.oidc_auth' not found or not a boolean in config, using default 'false'"
            );
            false
        })
}

/// Renders the registration page
#[debug_handler]
async fn register(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(_ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    match auth.user {
        Some(_user) => {
            // User is already authenticated, redirect to home using standard redirect
            tracing::info!("User is already authenticated, redirecting to home");
            redirect("/home", headers)
        }
        None => render_template(&v, "auth/register.html", data!({})),
    }
}

/// Handles the registration form submission
#[debug_handler]
async fn handle_register(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(form): Form<RegisterParams>,
) -> Result<Response> {
    // Check if passwords match
    if form.password != form.password_confirmation {
        return error_fragment(
            &v,
            "Password and password confirmation do not match",
            "#error-container",
        );
    }

    // Convert form data to RegisterParams, trimming the name
    let params = RegisterParams {
        name: form.name.trim().to_string(),
        email: form.email, // Email validation should handle trimming if necessary
        password: form.password,
        password_confirmation: form.password_confirmation,
    };

    // Create user account
    let res = users::Model::create_with_password(&ctx.db, &params).await;

    match res {
        Ok(user) => {
            // Auto-create admin team for the first user if needed
            if let Err(e) = user.create_admin_team_if_needed(&ctx.db, &ctx).await {
                tracing::error!(
                    error = ?e,
                    user_email = user.email,
                    "Failed to create administrators team during registration"
                );
                return error_page(&v, "Failed to create administrators team.", Some(e.into()));
            }

            // Generate email verification token
            match user
                .clone()
                .into_active_model()
                .generate_email_verification_token(&ctx.db)
                .await
            {
                Ok(user_with_token) => {
                    // Send verification email first
                    match AuthMailer::send_welcome(&ctx, &user_with_token).await {
                        Ok(_) => {
                            // Email sent successfully, now update verification status
                            match user_with_token
                                .clone()
                                .into_active_model()
                                .set_email_verification_sent(&ctx.db)
                                .await
                            {
                                Ok(_) => {
                                    // All good, redirect to login page with success message
                                    redirect("/auth/login?registered=true", headers)
                                }
                                Err(err) => {
                                    tracing::error!(
                                        message = "Failed to set email verification status after sending email",
                                        user_email = &user_with_token.email, // use user_with_token here
                                        error = err.to_string(),
                                    );
                                    // Although the email was sent, the DB update failed.
                                    // This is an internal error state, show error page.
                                    error_page(
                                        &v,
                                        "Account registered, but failed to finalize setup. Please contact support.",
                                        Some(loco_rs::Error::Model(err)),
                                    )
                                }
                            }
                        }
                        Err(err) => {
                            tracing::error!(
                                message = "Failed to send welcome email",
                                user_email = &user_with_token.email, // use user_with_token here
                                error = err.to_string(),
                            );
                            // Don't proceed to update verification status if email failed.
                            error_page(
                                &v,
                                "Could not send welcome email.",
                                Some(loco_rs::Error::wrap(err)),
                            )
                        }
                    }
                }
                Err(err) => {
                    tracing::error!(
                        message = "Failed to generate email verification token",
                        user_email = &user.email, // user is still available here from initial Ok(user)
                        error = err.to_string(),
                    );
                    // Error generating token after user creation.
                    // This is an internal error state, show error page.
                    error_page(
                        &v,
                        "Account registered, but failed to finalize setup. Please contact support.",
                        Some(loco_rs::Error::Model(err)),
                    )
                }
            }
        }
        Err(err) => {
            // Log the specific error details
            tracing::info!(
                error = err.to_string(), // Log the actual error string
                user_email = &params.email,
                user_name = &params.name, // Also log the username attempt
                "Could not register user",
            );

            // Handle specific ModelError variants
            match err {
                // Use the message directly from the ModelError for uniqueness errors
                ModelError::Message(msg) => error_fragment(&v, &msg, "#error-container"),
                // Handle validation errors
                ModelError::Validation(validation_err) => error_fragment(
                    &v,
                    &format!("Validation error: {}", validation_err),
                    "#error-container",
                ),
                // Handle other potential errors generically
                _ => error_fragment(
                    &v,
                    "Could not register account due to an unexpected error.",
                    "#error-container",
                ),
            }
        }
    }
}

/// Renders the login page
#[debug_handler]
async fn login(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Response> {
    if let Some(oidc_header) = headers.get("X-Oidc-Roles").and_then(|h| h.to_str().ok()) {
        tracing::info!("X-Oidc-Roles = {}", oidc_header);
    } else {
        tracing::info!("X-Oidc-Roles header found");
    }

    match auth.user {
        Some(_user) => {
            // User is already authenticated, redirect to home using standard redirect
            tracing::debug!("User is already authenticated, redirecting to home");
            redirect("/home", headers)
        }
        None => {
            // ToDo : Use X-Oidc-* auth headers only if enabled by settings.app.oidc_auth
            // ToDo : Add a user name field to user entity that would be the unique identifier instead of email, to support e-mail modification on OIDC side
            // ToDo : Add a auth type field to user entity (or to session?) to disable local modification of identity related profile information when authenticated by OIDC, and link to IPM profile page for that (or use settings.app.oidc_auth, but in that case it would be all iodc or all local users?)
            if let Some(oidc_email) = headers.get("X-Oidc-Email").and_then(|h| h.to_str().ok()) {
                tracing::debug!("X-Oidc-Email = {}", oidc_email);
                let user_result = users::Model::find_by_email(&ctx.db, oidc_email).await;

                match user_result {
                    Ok(user) => {
                        // Get JWT secret, handling potential error
                        let jwt_secret = match ctx.config.get_jwt_config() {
                            Ok(config) => config,
                            Err(err) => {
                                tracing::error!(
                                    message = "Failed to get JWT configuration,",
                                    error = err.to_string(),
                                );
                                // Use error_fragment for user-facing error
                                return error_fragment(
                                    &v,
                                    "Log in failed: Server configuration error.",
                                    "#error-container",
                                );
                            }
                        };

                        let token =
                            match user.generate_jwt(&jwt_secret.secret, &jwt_secret.expiration) {
                                Ok(token) => token,
                                Err(err) => {
                                    tracing::error!(
                                        message = "Failed to generate JWT token,",
                                        user_email = &oidc_email,
                                        error = err.to_string(),
                                    );
                                    return error_fragment(
                                        &v,
                                        "Log in failed: Failed to generate JWT token",
                                        "#error-container",
                                    );
                                }
                            };

                        // Redirect to home with cookies
                        let mut response = redirect("/home", headers.clone())?;

                        // Add Set-Cookie header for the auth token
                        response.headers_mut().append(
                            axum::http::header::SET_COOKIE,
                            HeaderValue::from_str(&format!("auth_token={}; Path=/", token))?,
                        );

                        tracing::debug!(
                            message = "User login from OIDC auth successful,",
                            user_email = &oidc_email,
                        );
                        return Ok(response);
                    }
                    Err(_) => {
                        tracing::debug!(
                            message = "Unknown user login attempt,",
                            user_email = &oidc_email,
                        );

                        if let Some(oidc_name) = headers
                            .get("X-Oidc-Displayname")
                            .and_then(|h| h.to_str().ok())
                        {
                            tracing::debug!("X-Oidc-Displayname = {}", oidc_name);

                            let random_password: String = (0..32)
                                .map(|_| rand::rng().sample(Alphanumeric) as char)
                                .collect();

                            // Convert form data to RegisterParams, trimming the name
                            let user_params = RegisterParams {
                                name: oidc_name.trim().to_string(),
                                email: oidc_email.trim().to_string(),
                                password: random_password.clone(),
                                password_confirmation: random_password.clone(),
                            };

                            // Create user account
                            let res =
                                users::Model::create_with_password(&ctx.db, &user_params).await;

                            match res {
                                Ok(user) => {
                                    // Auto-create admin team for the first user if needed
                                    if let Err(e) =
                                        user.create_admin_team_if_needed(&ctx.db, &ctx).await
                                    {
                                        tracing::error!(
                                            error = ?e,
                                            user_email = user.email,
                                            "Failed to create administrators team during OIDC registration"
                                        );
                                        return error_page(
                                            &v,
                                            "Failed to create administrators team.",
                                            Some(e.into()),
                                        );
                                    }

                                    // Generate email verification token
                                    match user
                                        .clone()
                                        .into_active_model()
                                        .generate_email_verification_token(&ctx.db)
                                        .await
                                    {
                                        Ok(user_with_token) => {
                                            // Send verification email first
                                            match AuthMailer::send_welcome(&ctx, &user_with_token)
                                                .await
                                            {
                                                Ok(_) => {
                                                    // Email sent successfully, now update verification status
                                                    match user_with_token
                                                        .clone()
                                                        .into_active_model()
                                                        .set_email_verification_sent(&ctx.db)
                                                        .await
                                                    {
                                                        Ok(_) => {
                                                            // All good, redirect to login page with success message
                                                            return redirect(
                                                                "/auth/login?registered=true",
                                                                headers,
                                                            );
                                                        }
                                                        Err(err) => {
                                                            tracing::error!(
                                                                message = "Failed to set email verification status after sending email",
                                                                user_email = &user_with_token.email, // use user_with_token here
                                                                error = err.to_string(),
                                                            );
                                                            // Although the email was sent, the DB update failed.
                                                            // This is an internal error state, show error page.
                                                            return error_page(
                                                                &v,
                                                                "Account registered, but failed to finalize setup. Please contact support.",
                                                                Some(loco_rs::Error::Model(err)),
                                                            );
                                                        }
                                                    }
                                                }
                                                Err(err) => {
                                                    tracing::error!(
                                                        message = "Failed to send welcome email",
                                                        user_email = &user_with_token.email, // use user_with_token here
                                                        error = err.to_string(),
                                                    );
                                                    // Don't proceed to update verification status if email failed.
                                                    return error_page(
                                                        &v,
                                                        "Could not send welcome email.",
                                                        Some(loco_rs::Error::wrap(err)),
                                                    );
                                                }
                                            }
                                        }
                                        Err(err) => {
                                            tracing::error!(
                                                message =
                                                    "Failed to generate email verification token",
                                                user_email = &user.email, // user is still available here from initial Ok(user)
                                                error = err.to_string(),
                                            );
                                            // Error generating token after user creation.
                                            // This is an internal error state, show error page.
                                            return error_page(
                                                &v,
                                                "Account registered, but failed to finalize setup. Please contact support.",
                                                Some(loco_rs::Error::Model(err)),
                                            );
                                        }
                                    }
                                }
                                Err(err) => {
                                    // Log the specific error details
                                    tracing::info!(
                                        error = err.to_string(), // Log the actual error string
                                        user_email = &user_params.email,
                                        user_name = &user_params.name, // Also log the username attempt
                                        "Could not register user",
                                    );

                                    // Handle specific ModelError variants
                                    match err {
                                        // Use the message directly from the ModelError for uniqueness errors
                                        ModelError::Message(msg) => {
                                            return error_fragment(&v, &msg, "#error-container");
                                        }
                                        // Handle validation errors
                                        ModelError::Validation(validation_err) => {
                                            return error_fragment(
                                                &v,
                                                &format!("Validation error: {}", validation_err),
                                                "#error-container",
                                            );
                                        }
                                        // Handle other potential errors generically
                                        _ => {
                                            return error_fragment(
                                                &v,
                                                "Could not register account due to an unexpected error.",
                                                "#error-container",
                                            );
                                        }
                                    }
                                }
                            }
                        } else {
                            tracing::info!("no X-Oidc-Displayname header found");
                        }
                    }
                }
            } else {
                tracing::info!("no X-Oidc-Email header found");
            }

            let registered = params.get("registered") == Some(&"true".to_string());
            let mut email = "";
            // Parse cookies from headers
            if let Some(cookie_header) = headers.get("cookie").and_then(|h| h.to_str().ok()) {
                // Check if user has a "remember me" cookie
                for cookie in cookie_header.split(';').map(|s| s.trim()) {
                    if let Some(remembered_email) = cookie.strip_prefix("remembered_email=") {
                        email = remembered_email;
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
    headers: HeaderMap,
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
                return error_fragment(
                    &v,
                    "Log in failed: Invalid email or password",
                    "#error-container",
                );
            };

            // Get JWT secret, handling potential error
            let jwt_secret = match ctx.config.get_jwt_config() {
                Ok(config) => config,
                Err(err) => {
                    tracing::error!(
                        message = "Failed to get JWT configuration,",
                        error = err.to_string(),
                    );
                    // Use error_fragment for user-facing error
                    return error_fragment(
                        &v,
                        "Log in failed: Server configuration error.",
                        "#error-container",
                    );
                }
            };

            let token = match user.generate_jwt(&jwt_secret.secret, &jwt_secret.expiration) {
                Ok(token) => token,
                Err(err) => {
                    tracing::error!(
                        message = "Failed to generate JWT token,",
                        user_email = &params.email,
                        error = err.to_string(),
                    );
                    return error_fragment(
                        &v,
                        "Log in failed: Failed to generate JWT token",
                        "#error-container",
                    );
                }
            };

            // Redirect to home with cookies
            let mut response = redirect("/home", headers)?;

            // Add Set-Cookie header for the auth token
            response.headers_mut().append(
                axum::http::header::SET_COOKIE,
                HeaderValue::from_str(&format!("auth_token={}; Path=/", token))?,
            );

            // If remember me is checked, set a persistent cookie with email
            if params.remember_me.is_some() {
                // Add Set-Cookie header for the remember me email
                match HeaderValue::from_str(&format!(
                    "remembered_email={}; Path=/; Max-Age=31536000",
                    form.email
                )) {
                    Ok(header_value) => {
                        response
                            .headers_mut()
                            .append(axum::http::header::SET_COOKIE, header_value);
                    }
                    Err(e) => {
                        // Log the error but continue processing as this is not critical
                        tracing::error!(
                            "Failed to create header value for remember_me cookie for user {}: {}",
                            &form.email,
                            e
                        );
                    }
                }
            }

            tracing::info!(
                message = "User login successful,",
                user_email = &params.email,
            );
            Ok(response)
        }
        Err(_) => {
            tracing::info!(
                message = "Unknown user login attempt,",
                user_email = &params.email,
            );
            error_fragment(
                &v,
                "Log in failed: Invalid email or password",
                "#error-container",
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
    render_template(&v, "auth/forgot_password.html", data!({}))
}

/// Renders the reset email sent confirmation page
#[debug_handler]
async fn render_reset_email_sent_page(
    ViewEngine(v): ViewEngine<TeraView>,
    State(_ctx): State<AppContext>, // May not need ctx if just rendering static page
) -> Result<Response> {
    render_template(&v, "auth/reset_email_sent.html", data!({}))
}

/// Handles the forgot password form submission
#[debug_handler]
async fn handle_forgot_password(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(form): Form<ForgotPasswordParams>,
) -> Result<Response> {
    match users::Model::find_by_email(&ctx.db, &form.email).await {
        Ok(user) => {
            let user_with_token = match user.initiate_password_reset(&ctx.db).await {
                Ok(u) => u,
                Err(e) => {
                    error!(user_email = %form.email, error = ?e, "Failed to initiate password reset (token generation/save)");
                    return redirect("/auth/reset-email-sent", headers);
                }
            };

            // Correct mailer function name: forgot_password
            if let Err(e) = AuthMailer::forgot_password(&ctx, &user_with_token).await {
                error!(user_email = %form.email, error = ?e, "Failed to send forgot password email");
            } else if let Err(e) = user_with_token
                .clone()
                .into_active_model()
                .set_forgot_password_sent(&ctx.db)
                .await
            {
                error!(user_email = %form.email, error = ?e, "Failed to set forgot password sent timestamp");
            }
        }
        Err(ModelError::EntityNotFound) => {
            info!(user_email = %form.email, "Forgot password requested for non-existent user");
        }
        Err(e) => {
            error!(user_email = %form.email, error = ?e, "Database error finding user for password reset");
        }
    }

    redirect("/auth/reset-email-sent", headers)
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

/// Handles the email verification link
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
                return render_template(
                    &v,
                    "auth/verify.html",
                    data!({
                        "success": true,
                        "message": "Your email has already been verified.",
                    }),
                );
            }

            let user_active_model = user.clone().into_active_model();
            match user_active_model.verified(&ctx.db).await {
                Ok(verified_user_model) => {
                    // Now try to fetch and update the PGP key using the new model method
                    let fetch_result = verified_user_model
                        .clone()
                        .into_active_model()
                        .fetch_and_update_pgp_key(&ctx.db)
                        .await;

                    let message = match fetch_result {
                        Ok(final_user_model) => {
                            if final_user_model.pgp_key.is_some() {
                                "Your email is now verified. We found and saved a PGP key for your account.Go to the your profile page to review it".to_string()
                            } else {
                                "Your email is now verified. We could not find a PGP key for your email. Ensure your PGP key is published on public PGP servers and try again to fetch it from your profile page".to_string()
                            }
                        }
                        Err(e) => {
                            // Log the error, but proceed with verification success message
                            tracing::error!(user_email = %verified_user_model.email, error = ?e, "Failed during PGP key fetch/update after verification.");
                            "Your email is now verified, but there was an issue checking for your PGP key on public PGP key servers. You can re-try to fetch it from you profile page.".to_string()
                        }
                    };

                    render_template(
                        &v,
                        "auth/verify.html",
                        data!({ "success": true, "message": message }),
                    )
                }
                Err(err) => {
                    tracing::error!(
                        message = "Failed to mark email as verified",
                        user_email = &user.email,
                        error = err.to_string(),
                    );
                    render_template(
                        &v,
                        "auth/verify.html",
                        data!({
                            "success": false,
                            "message": "Email verification failed. Please try again or contact support.",
                        }),
                    )
                }
            }
        }
        Err(err) => {
            tracing::error!(
                message = "Invalid or expired email verification token",
                token = token,
                error = err.to_string(),
            );
            render_template(
                &v,
                "auth/verify.html",
                data!({
                    "success": false,
                    "message": "Invalid or expired email verification link.",
                }),
            )
        }
    }
}

/// Handles user logout by clearing the auth token cookie
#[debug_handler]
async fn handle_logout(State(ctx): State<AppContext>) -> Result<Response> {
    // TODO: Implement server-side JWT invalidation (e.g., token blacklist)
    // This implementation only clears the client-side cookie. The JWT itself
    // remains valid until it expires. For a fully secure logout, the token
    // should be invalidated on the server-side as well.
    let redirect_url = match is_oidc_auth(&ctx) {
        true => "/oidc/logout",
        false => "/auth/login",
    };
    let response = Response::builder()
        .header("HX-Redirect", redirect_url)
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
    headers: HeaderMap,
    Form(form): Form<ResetPasswordParams>,
) -> Result<Response> {
    // Check if passwords match
    if form.password != form.password_confirmation {
        return error_fragment(
            &v,
            "New password and confirmation do not match.",
            "#error-container",
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
                        redirect("/auth/login?reset=success", headers)
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to update password for token {}: {}",
                            &form.token,
                            e
                        );
                        error_fragment(
                            &v,
                            "Failed to reset password. Please try again.",
                            "#error-container",
                        )
                    }
                }
            } else {
                // Token mismatch or already cleared - treat as invalid
                error_fragment(
                    &v,
                    "Invalid or expired password reset link.",
                    "#error-container",
                )
            }
        }
        Err(_) => {
            // User not found for the token
            error_fragment(
                &v,
                "Invalid or expired password reset link.",
                "#error-container",
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
        .add("/reset-email-sent", get(render_reset_email_sent_page))
        .add("/reset-password/{token}", get(reset_password))
        .add("/reset-password", post(handle_reset_password))
        .add("/verify/{token}", get(verify_email))
        .add("/logout", post(handle_logout))
}
