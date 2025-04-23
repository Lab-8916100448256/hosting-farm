use loco_rs::model::ModelError;
use crate::{
    mailers::auth::AuthMailer,
    models::{
        _entities::users,
        // Import the custom Model struct, ActiveModel, and params from models::users
        users::{Model as UserModel, ActiveModel as UserActiveModel, LoginParams, RegisterParams, ForgotPasswordParams, ResetPasswordParams},
    },
    views::auth::{CurrentResponse, LoginResponse},
};
use axum::{
    debug_handler,
    extract::{Json, Path, State},
    routing::{get, post},
    response::{Response, IntoResponse}, // Import IntoResponse
    Router,
};
use loco_rs::prelude::*;
use regex::Regex;
// serde removed as Json extractor handles it
use std::sync::OnceLock;
use serde::Deserialize;

pub static EMAIL_DOMAIN_RE: OnceLock<Regex> = OnceLock::new();

fn get_allow_email_domain_re() -> &'static Regex {
    EMAIL_DOMAIN_RE.get_or_init(|| {
        Regex::new(r"@example\.com$|@gmail\.com$").expect("Failed to compile regex")
    })
}

// Structs moved to models::users.rs or handled by params types directly

/// Register function creates a new user with the given parameters and sends a
/// welcome email to the user
#[debug_handler]
async fn register(
    State(ctx): State<AppContext>,
    Json(params): Json<RegisterParams>, // Use RegisterParams from models::users
) -> Result<Response> { // Use loco_rs::prelude::Result
    // Use custom model's create_with_password, which now handles transactions internally
    let user = match UserModel::create_with_password(&ctx, &ctx.db, &params).await {
        Ok(user) => user,
        Err(err) => {
            tracing::info!(
                message = err.to_string(),
                user_email = &params.email,
                "could not register user",
            );
            // Return an appropriate error response, maybe bad request or conflict
            return Err(Error::Model(err));
        }
    };

    // Generate token *after* successful creation
    let token = match user.generate_email_verification_token(&ctx.db).await {
         Ok(token) => token,
         Err(e) => {
              error!(error = ?e, user_pid = %user.pid, "Failed to generate verification token after registration");
              // Decide how to handle this - user created but token failed
              // Maybe log and proceed, or return internal server error?
              return Err(Error::Model(e));
         }
    };

    // Refetch user to ensure we have the latest state with the token for the mailer
    let user_with_token = match UserModel::find_by_pid(&ctx.db, &user.pid.to_string()).await {
         Ok(u) => u,
         Err(e) => {
              error!(error = ?e, user_pid = %user.pid, "Failed to refetch user after generating verification token");
              return Err(Error::Model(e));
         }
    };

    AuthMailer::send_welcome(&ctx, &user_with_token).await?; // Pass token to mailer

    format::json(()) // Return empty JSON on success
}

/// Verify register user. if the user not verified his email, he can't login to
/// the system.
#[debug_handler]
async fn verify(State(ctx): State<AppContext>, Path(token): Path<String>) -> Result<Response> {
    let user = UserModel::find_by_email_verification_token(&ctx.db, &token).await?; // Use the correct find method

    if user.email_verified_at.is_some() {
        tracing::info!(pid = user.pid.to_string(), "user already verified");
    } else {
        // Use the verify_email method on the model instance
        let _verified_user = user.verify_email(&ctx.db, &token).await?; // Use the token passed to verify
        tracing::info!(pid = _verified_user.pid.to_string(), "user verified");
    }

    format::json(())
}

/// In case the user forgot his password  this endpoints generate a forgot token
/// and send email to the user. In case the email not found in our DB, we are
/// returning a valid request for for security reasons (not exposing users DB
/// list).
#[debug_handler]
async fn forgot(
    State(ctx): State<AppContext>,
    Json(params): Json<ForgotPasswordParams>, // Use ForgotPasswordParams from models::users
) -> Result<Response> {
    let Ok(user) = UserModel::find_by_email(&ctx.db, &params.email).await else {
        // we don't want to expose our users email. if the email is invalid we still
        // returning success to the caller
        tracing::debug!(email = params.email, "Password reset requested for non-existent email");
        return format::json(());
    };

    // Generate token using the model method
    let token = match user.generate_reset_token(&ctx.db).await {
         Ok(t) => t,
         Err(e) => {
             error!(error = ?e, user_pid = %user.pid, "Failed to generate password reset token");
             return Err(Error::Model(e));
         }
    };

    // Refetch user to get updated state with token
    let user_with_token = match UserModel::find_by_pid(&ctx.db, &user.pid.to_string()).await {
         Ok(u) => u,
         Err(e) => {
             error!(error = ?e, user_pid = %user.pid, "Failed to refetch user after generating reset token");
             return Err(Error::Model(e));
         }
    };

    AuthMailer::forgot_password(&ctx, &user_with_token).await?; // Pass token to mailer

    format::json(())
}

/// reset user password by the given parameters
#[debug_handler]
async fn reset(State(ctx): State<AppContext>, Json(params): Json<ResetPasswordParams>) -> Result<Response> {
    // Call the static model method for password reset
    match UserModel::reset_password(&ctx.db, &params).await {
        Ok(_) => {
            // Check if token length allows slicing before logging prefix
            let token_prefix = if params.reset_token.len() >= 5 { &params.reset_token[..5] } else { params.reset_token.as_str() };
            info!(token_prefix = token_prefix, "Password reset successful");
            format::json(())
        }
        Err(ModelError::Message(msg)) if msg == "invalid reset token" || msg == "token expired" => {
            let token_prefix = if params.reset_token.len() >= 5 { &params.reset_token[..5] } else { params.reset_token.as_str() };
            warn!(token_prefix = token_prefix, message = msg, "Password reset failed (token invalid/expired)");
            // Don't reveal specific reason to client for security
            format::json(()) // Return success-like response
        }
        Err(e @ ModelError::Validation(_)) => {
            let token_prefix = if params.reset_token.len() >= 5 { &params.reset_token[..5] } else { params.reset_token.as_str() };
            warn!(error = ?e, token_prefix = token_prefix, "Password reset failed (validation)");
             Err(Error::Model(e)) // Return validation error
        }
        Err(e) => {
            let token_prefix = if params.reset_token.len() >= 5 { &params.reset_token[..5] } else { params.reset_token.as_str() };
            error!(error = ?e, token_prefix = token_prefix, "Password reset failed (internal error)");
            Err(Error::Model(e)) // Return internal error
        }
    }
}

/// Creates a user login and returns a token
#[debug_handler]
async fn login(State(ctx): State<AppContext>, Json(params): Json<LoginParams>) -> Result<Response> { // Use LoginParams
    // Use the model method which includes password verification and status check
    let user = match UserModel::find_by_email_and_password(&ctx.db, &params).await {
        Ok(u) => u,
        Err(ModelError::EntityNotFound) | Err(ModelError::Message(_)) => {
            // Treat not found and invalid password/status the same way
            return Err(Error::Unauthorized("Invalid email or password.".to_string()));
        }
        Err(e) => return Err(Error::Model(e)), // Other errors
    };

    // If find_by_email_and_password succeeded, user is valid and approved
    let jwt_secret = ctx.config.get_jwt_config()?;

    let token = user
        .generate_token(&jwt_secret.secret, Some(chrono::Duration::seconds(jwt_secret.expiration as i64))).await // Use generate_token from Authenticable
        .map_err(|_| Error::InternalServerError)?;// Convert JWTError to InternalServerError

    format::json(LoginResponse::new(&user, &token)) // Use LoginResponse view
}

#[debug_handler]
async fn current(auth: auth::JWTWithUser<UserModel>, State(_ctx): State<AppContext>) -> Result<Response> { // Use JWTWithUser, mark ctx as unused
    // auth.user is already the UserModel instance loaded by the middleware
    format::json(CurrentResponse::new(&auth.user)) // Use CurrentResponse view
}

/// Magic link authentication provides a secure and passwordless way to log in to the application.
// Define MagicLinkParams locally if not imported
#[derive(Debug, Deserialize, Validate)]
struct MagicLinkParams {
    #[validate(email(message = "Invalid email format."))]
    pub email: String,
}

#[debug_handler]
async fn magic_link(
    State(ctx): State<AppContext>,
    Json(params): Json<MagicLinkParams>,
) -> Result<Response> {
    let email_regex = get_allow_email_domain_re();
    if !email_regex.is_match(&params.email) {
        tracing::debug!(
            email = params.email,
            "The provided email is invalid or does not match the allowed domains"
        );
        return Err(Error::BadRequest("Invalid email domain.".to_string()));
    }

    let Ok(user) = UserModel::find_by_email(&ctx.db, &params.email).await else {
        tracing::debug!(email = params.email, "Magic link requested for non-existent email");
        return format::json(()); // Return success-like response
    };

    // Check if user is approved
    if user.status != users::USER_STATUS_APPROVED {
        tracing::warn!(user_pid = %user.pid, "Magic link requested for non-approved user");
        return format::json(()); // Return success-like response, don't reveal status
    }

    // Generate magic link token using model method
    let token = match user.generate_magic_link_token(&ctx.db).await {
        Ok(t) => t,
        Err(e) => {
             error!(error = ?e, user_pid = %user.pid, "Failed to generate magic link token");
             return Err(Error::Model(e));
        }
    };

     // Refetch user to get updated state with token
    let user_with_token = match UserModel::find_by_pid(&ctx.db, &user.pid.to_string()).await {
         Ok(u) => u,
         Err(e) => {
             error!(error = ?e, user_pid = %user.pid, "Failed to refetch user after generating magic link token");
             return Err(Error::Model(e));
         }
    };

    AuthMailer::send_magic_link(&ctx, &user_with_token).await?;

    format::json(())
}

/// Verifies a magic link token and authenticates the user.
#[debug_handler]
async fn magic_link_verify(
    Path(token): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    // Use the find_by method that also checks expiration
    let user = match UserModel::find_by_magic_link_token(&ctx.db, &token).await {
         Ok(u) => u,
         Err(ModelError::EntityNotFound) | Err(ModelError::Message(_)) => {
             // Treat not found and expired/invalid token the same
             return Err(Error::Unauthorized("Invalid or expired magic link.".to_string()));
         }
         Err(e) => return Err(Error::Model(e)), // Other errors
    };

    // Clear the magic link token after successful verification
    let mut user_am: UserActiveModel = user.inner.clone().into();
    user_am.magic_link_token = Set(None);
    user_am.magic_link_expiration = Set(None);
    let user_entity = user_am.update(&ctx.db).await.map_err(ModelError::from)?;
    let updated_user = UserModel::from(user_entity); // Get the updated model

    let jwt_secret = ctx.config.get_jwt_config()?;

    let jwt_token = updated_user
        .generate_token(&jwt_secret.secret, Some(chrono::Duration::seconds(jwt_secret.expiration as i64))).await
        .map_err(|_| Error::InternalServerError)?;

    format::json(LoginResponse::new(&updated_user, &jwt_token))
}

/// Handles user logout by clearing the auth token cookie
#[debug_handler]
async fn logout() -> Result<Response> {
    // API logout is typically handled client-side by discarding the token.
    // If server-side token revocation is needed, it requires a different mechanism (e.g., blacklist).
    // For now, return success.
    tracing::info!("API logout endpoint called (client should discard token)");
    format::json(())
}

pub fn routes() -> Router<AppContext> { // Use Router<AppContext>
    Router::new() // Use Router::new()
        .route("/register", post(register))
        .route("/verify/:token", get(verify))
        .route("/login", post(login))
        .route("/forgot", post(forgot))
        .route("/reset", post(reset))
        .route("/current", get(current))
        .route("/magic-link", post(magic_link))
        .route("/magic-link/:token", get(magic_link_verify))
        .route("/logout", post(logout))
}
