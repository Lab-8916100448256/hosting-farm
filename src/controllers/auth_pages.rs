use crate::{
    mailers::auth::AuthMailer,
    middleware::auth_no_error::JWTWithUserOpt,
    models::{
        _entities::users as user_entity, // Use alias for entity if needed elsewhere, currently only in handle_register admin team logic
        teams,                           // Keep teams import if needed for admin team creation
        users::{
            ActiveModel as UserActiveModel, ForgotPasswordParams, LayoutData, LoginParams,
            Model as UserModel, RegisterParams, ResetPasswordParams,
        },
    },
    views::{error_fragment, error_page, redirect, render_template}, // Ensure all view helpers are imported
};
use tracing::{debug, error, info}; // Import tracing macros

use axum::http::{header::HeaderValue, HeaderMap};
use axum::{
    debug_handler,
    extract::{Form, Path, Query, State},
};
use loco_rs::prelude::*;
use sea_orm::{EntityTrait, IntoActiveModel, PaginatorTrait}; // Import IntoActiveModel
use std::collections::HashMap;
use tera::Context; // Import Context

/// Helper function to get the admin team name from config
fn get_admin_team_name(ctx: &AppContext) -> String {
    ctx.config
        .settings
        .as_ref() // Get Option<&Value>
        .and_then(|settings| settings.get("app")) // Get Option<&Value> for "app" key
        .and_then(|app_settings| app_settings.get("admin_team_name")) // Get Option<&Value> for "admin_team_name"
        .and_then(|value| value.as_str()) // Get Option<&str>
        .map(|s| s.to_string()) // Convert to Option<String>
        .unwrap_or_else(|| {
            tracing::warn!(
                "'app.admin_team_name' not found or not a string in config, using default 'Administrators'"
            );
            "Administrators".to_string()
        })
}

/// Renders the registration page
#[debug_handler]
async fn register(
    auth: JWTWithUserOpt<UserModel>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    if auth.user.is_some() {
        tracing::info!("User is already authenticated, redirecting to home");
        return redirect("/home", headers);
    }
    // Create layout data (user will be None)
    let layout_data = UserModel::create_layout_data(None, &ctx).await?;
    // Fix: Ensure Context::new() is passed for empty page-specific context
    render_template(&v, "auth/register.html", None, layout_data, Context::new())
}

/// Handles the registration form submission
#[debug_handler]
async fn handle_register(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(form): Form<RegisterParams>,
) -> Result<Response> {
    // Construct default layout data needed *only* if error_page is called
    let default_layout_data = LayoutData {
        user: None,
        is_admin: false,
    };

    if form.password != form.password_confirmation {
        return error_fragment(
            &v,
            "Password and password confirmation do not match",
            "#error-container",
        );
    }

    let params = RegisterParams {
        name: form.name.trim().to_string(),
        email: form.email,
        password: form.password,
        password_confirmation: form.password_confirmation,
    };

    match UserModel::create_with_password(&ctx.db, &params).await {
        Ok(user) => {
            // user is UserModel
            // --- Admin Team Creation Logic ---
            match user_entity::Entity::find().count(&ctx.db).await {
                // Use user_entity alias here
                Ok(user_count) if user_count == 1 => {
                    info!(
                        "First user registered ({}), creating default admin team.",
                        user.email
                    );
                    let admin_team_name = get_admin_team_name(&ctx); // Assuming this helper is still here
                    let team_params = teams::CreateTeamParams {
                        name: admin_team_name.clone(),
                        description: Some("Default administrators team.".to_string()),
                    };
                    // Use create_team defined on the teams Model (adjust path if needed)
                    match teams::Model::create_team(&ctx.db, user.id, &team_params).await {
                        Ok(team) => {
                            info!("Created admin team '{}' (ID: {})", admin_team_name, team.id)
                        }
                        Err(e) => {
                            error!("Failed to create admin team '{}': {:?}", admin_team_name, e)
                        }
                    }
                }
                Ok(count) => debug!(
                    "Not first user (count: {}), skipping admin team creation.",
                    count
                ),
                Err(e) => error!(
                    "Failed to query user count: {:?}. Skipping admin team check.",
                    e
                ),
            }
            // --- End Admin Team Creation ---

            // Fix: Convert Model to ActiveModel before calling ActiveModel methods
            let mut active_user: UserActiveModel = user.clone().into_active_model();
            match active_user.generate_email_verification_token(&ctx.db).await {
                Ok(user_with_token_model) => {
                    // returns Model
                    match AuthMailer::send_welcome(&ctx, &user_with_token_model).await {
                        Ok(_) => {
                            // Fix: Convert Model to ActiveModel
                            let mut active_user_sent: UserActiveModel =
                                user_with_token_model.clone().into_active_model();
                            match active_user_sent.set_email_verification_sent(&ctx.db).await {
                                Ok(_) => redirect("/auth/login?registered=true", headers),
                                Err(err) => {
                                    tracing::error!(error=%err, email=%user_with_token_model.email, "Failed to set email verification status");
                                    // Fix: Pass default_layout_data to error_page
                                    error_page(
                                        &v,
                                        "Account registered, but failed to finalize setup.",
                                        Some(loco_rs::Error::Model(err)),
                                        default_layout_data,
                                    )
                                }
                            }
                        }
                        Err(err) => {
                            tracing::error!(error=%err, email=%user_with_token_model.email, "Failed to send welcome email");
                            // Fix: Pass default_layout_data to error_page
                            error_page(
                                &v,
                                "Could not send welcome email.",
                                Some(loco_rs::Error::wrap(err)),
                                default_layout_data,
                            )
                        }
                    }
                }
                Err(err) => {
                    tracing::error!(error=%err, email=%user.email, "Failed to generate email verification token");
                    // Fix: Pass default_layout_data to error_page
                    error_page(
                        &v,
                        "Account registered, but failed to finalize setup.",
                        Some(loco_rs::Error::Model(err)),
                        default_layout_data,
                    )
                }
            }
        }
        Err(err) => {
            error!(error = ?err, "Registration failed");
            error_fragment(&v, &err.to_string(), "#error-container") // Send specific error back
        }
    }
}

/// Renders the login page
#[debug_handler]
async fn login(
    auth: JWTWithUserOpt<UserModel>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Result<Response> {
    if let Some(user) = auth.user {
        let next_url = params.get("next").map_or("/home", |n| n.as_str());
        tracing::info!(user_email = %user.email, next_url = next_url, "User already authenticated, redirecting.");
        return redirect(next_url, headers);
    }
    let layout_data = UserModel::create_layout_data(None, &ctx).await?;
    let mut page_context = Context::new();
    // Correct: Populate page_context from params
    if params.contains_key("registered") {
        page_context.insert("registered", &true);
    }
    if params.contains_key("verified") {
        page_context.insert("verified", &true);
    }
    if params.contains_key("reset") {
        page_context.insert("reset", &true);
    }
    if let Some(next) = params.get("next") {
        page_context.insert("next", next);
    }
    if let Some(error) = params.get("error") {
        page_context.insert("error", error);
    }
    // Correct: Pass the populated page_context
    render_template(&v, "auth/login.html", None, layout_data, page_context)
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
    let user_result = UserModel::find_by_email(&ctx.db, &params.email).await;

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
    auth: JWTWithUserOpt<UserModel>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let layout_data = UserModel::create_layout_data(auth.user, &ctx).await?;
    // Correct: Use Context::new()
    render_template(&v, "auth/forgot.html", None, layout_data, Context::new())
}

/// Renders the page shown after requesting a password reset email
#[debug_handler]
async fn render_reset_email_sent_page(
    auth: JWTWithUserOpt<UserModel>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let layout_data = UserModel::create_layout_data(auth.user, &ctx).await?;
    // Correct: Use Context::new()
    render_template(
        &v,
        "auth/reset_sent.html",
        None,
        layout_data,
        Context::new(),
    )
}

/// Handles the forgot password form submission
#[debug_handler]
async fn handle_forgot_password(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(form): Form<ForgotPasswordParams>,
) -> Result<Response> {
    match UserModel::find_by_email(&ctx.db, &form.email).await {
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
            } else {
                if let Err(e) = user_with_token
                    .clone()
                    .into_active_model()
                    .set_forgot_password_sent(&ctx.db)
                    .await
                {
                    error!(user_email = %form.email, error = ?e, "Failed to set forgot password sent timestamp");
                }
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

/// Renders the password reset form page if the token is valid
#[debug_handler]
async fn reset_password(
    auth: JWTWithUserOpt<UserModel>, // Add auth for layout data
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>, // Add ctx for layout data and DB access
    Path(token): Path<String>,
) -> Result<Response> {
    // Fetch layout data first (user likely None)
    let layout_data = UserModel::create_layout_data(auth.user, &ctx).await?;

    match UserModel::find_by_reset_token(&ctx.db, &token).await {
        Ok(_user) => {
            // Found user, token is valid (expiration check might be added in find_by_reset_token)
            // Render the reset form
            let mut page_context = Context::new();
            page_context.insert("token", &token);
            // Use render_template helper
            render_template(&v, "auth/reset.html", None, layout_data, page_context)
        }
        Err(e) => {
            error!(error = ?e, token = %token, "Password reset token invalid or expired");
            // Token invalid or DB error, render error page
            // Fix: Pass layout_data to error_page call
            error_page(&v, "Invalid or expired reset link.", Some(e), layout_data)
        }
    }
}

/// Verifies the user's email address using the token from the URL
#[debug_handler]
async fn verify_email(
    auth: JWTWithUserOpt<UserModel>, // Add auth for layout data
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>, // Add ctx for layout data and DB access
    Path(token): Path<String>,
) -> Result<Response> {
    // Fetch layout data first (user might be logged in already or not)
    let layout_data = UserModel::create_layout_data(auth.user, &ctx).await?;

    match UserModel::find_by_verification_token(&ctx.db, &token).await {
        Ok(user) => {
            // Found user by token
            // Convert to ActiveModel to call 'verified' method
            let mut active_user: UserActiveModel = user.clone().into_active_model(); // Clone user before converting
            match active_user.verified(&ctx.db).await {
                // Call verified on ActiveModel
                Ok(verified_user_model) => {
                    // verified returns Model on success
                    info!("User {} verified successfully.", verified_user_model.email);
                    // Render verification success page
                    // Use render_template helper
                    render_template(&v, "auth/verified.html", None, layout_data, Context::new())
                }
                Err(e) => {
                    error!(error = ?e, token = %token, "Failed to update user verification status");
                    // Fix: Pass layout_data to error_page call
                    error_page(
                        &v,
                        "Could not verify your email due to an internal error.",
                        Some(Error::Model(e)),
                        layout_data,
                    )
                }
            }
        }
        Err(e) => {
            // User not found by token
            error!(error = ?e, token = %token, "Email verification token invalid");
            // Fix: Pass layout_data to error_page call
            error_page(
                &v,
                "Invalid or expired verification link.",
                Some(e),
                layout_data,
            )
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

/// Handles the password reset form submission
#[debug_handler]
async fn handle_reset_password(
    auth: JWTWithUserOpt<UserModel>, // Add auth for layout data
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>, // Add ctx for layout data and DB access
    headers: HeaderMap,            // Keep headers for redirect
    Form(form): Form<ResetPasswordParams>,
) -> Result<Response> {
    // Fetch layout data first (user likely None, needed for potential error render)
    let layout_data = UserModel::create_layout_data(auth.user, &ctx).await?;

    if form.password != form.password_confirmation {
        return error_fragment(&v, "Passwords do not match.", "#error-container");
        // Target error display area in reset.html
    }

    match UserModel::find_by_reset_token(&ctx.db, &form.token).await {
        Ok(user) => {
            // Convert Model to ActiveModel to call reset_password
            let mut active_user: UserActiveModel = user.into_active_model();
            match active_user.reset_password(&ctx.db, &form.password).await {
                Ok(_) => {
                    info!("Password reset successfully for token: {}", form.token);
                    redirect("/auth/login?reset=true", headers) // Redirect to login page
                }
                Err(e) => {
                    error!(error = ?e, token = %form.token, "Failed to update password in database");
                    // Render the reset form again with a generic error
                    let mut page_context = Context::new();
                    page_context.insert("token", &form.token);
                    page_context.insert(
                        "error_message",
                        "Failed to reset password due to an internal error.",
                    );
                    // Fix: Use render_template and pass layout_data
                    render_template(&v, "auth/reset.html", None, layout_data, page_context)
                }
            }
        }
        Err(e) => {
            error!(error = ?e, token = %form.token, "Password reset token invalid during form submission");
            // Render the reset form again with token error
            let mut page_context = Context::new();
            page_context.insert("token", &form.token);
            page_context.insert("error_message", "Invalid or expired reset token.");
            // Fix: Use render_template and pass layout_data
            render_template(&v, "auth/reset.html", None, layout_data, page_context)
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
