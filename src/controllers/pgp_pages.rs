// src/controllers/pgp_pages.rs
use crate::{
    middleware::auth_no_error::JWTWithUserOpt, // Re-use existing auth middleware
    models::_entities::users::Column,
    models::users,
    views::{error_page, redirect}, // Use existing view helpers
};
use axum::http::HeaderMap;
use axum::{
    debug_handler,
    extract::{Path, State},
    response::Response,
    routing::get,
};
use loco_rs::prelude::*;
use loco_rs::prelude::{AppContext, TeraView, ViewEngine};

async fn find_user_by_pgp_token(db: &DatabaseConnection, token: &str) -> ModelResult<users::Model> {
    let user = users::Entity::find()
        .filter(Column::PgpVerificationToken.eq(token))
        .one(db)
        .await?;
    user.map(users::Model::from) // Convert entity to custom model
        .ok_or_else(|| ModelError::EntityNotFound)
}

#[debug_handler]
async fn verify_pgp_token(
    Path(token): Path<String>,
    State(ctx): State<AppContext>,
    ViewEngine(v): ViewEngine<TeraView>, // Needed for error_page
    auth: JWTWithUserOpt<users::Model>,  // Get current user for context/potential checks
    headers: HeaderMap,
) -> Result<Response> {
    match find_user_by_pgp_token(&ctx.db, &token).await {
        Ok(user) => {
            // Enforce that the token belongs to the currently logged-in user
            if let Some(current_user) = auth.user {
                if current_user.id != user.id {
                    tracing::warn!(
                        "PGP token {} belongs to user {}, but accessed by user {}",
                        token,
                        user.id,
                        current_user.id
                    );
                    // Return an error page if users don't match
                    return error_page(
                        &v,
                        "Verification token mismatch. Are you logged in as the correct user?",
                        None,
                    );
                }
            } else {
                // If no user is logged in at all, this link shouldn't work either.
                tracing::warn!(
                    "Attempted to use PGP token {} without being logged in.",
                    token
                );
                // Redirect to login or show an error
                return error_page(&v, "Please log in to verify your PGP key.", None);
            }

            // Call the verify_pgp method which handles token check and update
            match user.verify_pgp(&ctx.db, &token).await {
                Ok(_) => {
                    tracing::info!("PGP email verified successfully for token: {}", token);
                    // Redirect to profile with a success flash message (TODO: Implement flash message)
                    redirect("/users/profile?pgp_verified=true", headers) // Simple query param for now
                }
                Err(e) => {
                    tracing::error!("Failed to mark PGP as verified for token {}: {}", token, e);
                    error_page(
                        &v,
                        "Failed to verify PGP. The link may be invalid or expired.",
                        Some(Error::Model(e)),
                    )
                }
            }
        }
        Err(ModelError::EntityNotFound) => {
            tracing::warn!(
                "Invalid or expired PGP verification token received: {}",
                token
            );
            error_page(&v, "Invalid or expired PGP verification link.", None)
        }
        Err(e) => {
            tracing::error!("Database error verifying PGP token {}: {}", token, e);
            error_page(
                &v,
                "Could not verify PGP due to a server error.",
                Some(Error::Model(e)),
            )
        }
    }
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/pgp") // Prefix for PGP related actions
        .add("/verify/{token}", get(verify_pgp_token)) // Use curly braces {} for path parameters
}
