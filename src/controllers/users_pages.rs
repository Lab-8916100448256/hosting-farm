use crate::mailers::gpg_verification::GpgVerificationMailer;
use crate::{
    middleware::auth_no_error::JWTWithUserOpt,
    models::{
        _entities::team_memberships, _entities::teams, _entities::users::Column as UserColumn,
        users,
    },
    views::error_fragment,
    views::error_page,
    views::redirect,
    views::render_template,
};
use axum::debug_handler;
use axum::extract::{Form, State};
use axum::http::header::HeaderMap;
use loco_rs::prelude::*;
use sea_orm::PaginatorTrait;
use sea_orm::{ActiveValue, ColumnTrait, EntityTrait, Set};
use serde::Deserialize;
use serde_json::json;

/// Params for updating user profile
#[derive(Debug, Deserialize)]
pub struct UpdateProfileParams {
    pub name: String,
    pub email: String,
}

/// Params for updating user password
#[derive(Debug, Deserialize)]
pub struct UpdatePasswordParams {
    pub current_password: String,
    pub password: String,
    pub password_confirmation: String,
}

/// Renders the user profile page
#[debug_handler]
async fn profile(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        // Redirect to login if not authenticated
        return redirect("/auth/login", headers);
    };

    // Get user's team memberships
    let teams_result = teams::Entity::find()
        .find_with_related(team_memberships::Entity)
        .all(&ctx.db)
        .await;

    let teams_data = match teams_result {
        Ok(data) => data,
        Err(e) => {
            tracing::error!("Failed to load teams for user {}: {}", user.id, e);
            return error_page(
                &v,
                "Could not load your team information. Please try again later.",
                Some(e.into()),
            );
        }
    };

    let teams = teams_data
        .into_iter()
        .filter_map(|(team, memberships)| {
            let is_member = memberships
                .iter()
                .any(|m| m.user_id == user.id && !m.pending);
            if is_member {
                // Find user's role in this team
                let role = memberships
                    .iter()
                    .find(|m| m.user_id == user.id && !m.pending)
                    .map(|m| m.role.clone())
                    .unwrap_or_else(|| "Unknown".to_string());

                Some(json!({
                    "pid": team.pid.to_string(),
                    "name": team.name,
                    "role": role
                }))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // Get pending invitations count
    let invitations_result = team_memberships::Entity::find()
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(true))
        .count(&ctx.db)
        .await;

    let invitations = match invitations_result {
        Ok(count) => count,
        Err(e) => {
            tracing::error!(
                "Failed to load invitation count for user {}: {}",
                user.id,
                e
            );
            // Call error_page with 3 arguments: engine, message, error
            return error_page(
                &v,
                "Could not load your invitation count. Please try again later.",
                Some(e.into()),
            );
        }
    };

    render_template(
        &v,
        "users/profile.html",
        data!({
            "user": &user,
            "teams": &teams,
            "active_page": "profile",
            "invitation_count": &invitations,
        }),
    )
}

/// Renders the user invitations page
#[debug_handler]
async fn invitations(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        // Redirect to login if not authenticated
        return redirect("/auth/login", headers);
    };

    // Get user's pending team invitations
    let invitations_result = team_memberships::Entity::find()
        .find_with_related(teams::Entity)
        .all(&ctx.db)
        .await;

    let invitations_data = match invitations_result {
        Ok(data) => data,
        Err(e) => {
            tracing::error!("Failed to load invitations for user {}: {}", user.id, e);
            // Return error page on failure
            return error_page(
                &v,
                "Could not load your invitations. Please try again later.",
                Some(e.into()),
            );
        }
    };

    let invitations = invitations_data
        .into_iter()
        .filter_map(|(membership, teams)| {
            if membership.user_id == user.id && membership.pending {
                // Handle potential None from teams.first() safely
                teams.first().map(|team| {
                    json!({
                        "team_name": team.name,
                        "team_description": team.description.clone(),
                        "token": membership.invitation_token,
                        "role": membership.role,
                        "sent_at": membership.created_at.format("%Y-%m-%d").to_string()
                    })
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // Get pending invitations count (now derived from the collected invitations)
    let invitation_count = invitations.len();

    render_template(
        &v,
        "users/invitations.html",
        data!({
            "user": &user,
            "invitations": &invitations,
            "active_page": "invitations",
            "invitation_count": &invitation_count,
        }),
    )
}

/// Returns the HTML fragment for the invitation count badge
#[debug_handler]
async fn get_invitation_count(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    // Get pending invitations count
    let invitation_count = team_memberships::Entity::find()
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(true))
        .count(&ctx.db)
        .await?;

    // Use render_template to render the fragment
    render_template(
        &v,
        "users/_invitation_count_badge.html",
        data!({
            "invitation_count": &invitation_count,
        }),
    )
}

/// Updates user profile information
#[debug_handler]
async fn update_profile(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(params): Form<UpdateProfileParams>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    // Check if email already exists (if it changed)
    if user.email != params.email {
        let existing_user = users::Model::find_by_email(&ctx.db, &params.email).await;
        match existing_user {
            Ok(_) => return error_fragment(&v, "Email already in use", "#error-container"),
            Err(e) => {
                tracing::error!("Database error checking email: {}", e);
                return error_fragment(
                    &v,
                    "Could not verify email availability. Please try again.",
                    "#error-container",
                );
            }
        }
    }

    // Update user profile
    let mut user_model: crate::models::_entities::users::ActiveModel = user.into();
    user_model.name = sea_orm::ActiveValue::set(params.name);
    user_model.email = sea_orm::ActiveValue::set(params.email);

    match user_model.update(&ctx.db).await {
        Ok(_updated_user) => {
            // Return a response that refreshes the page
            let response = Response::builder()
                .header("HX-Refresh", "true")
                .body(axum::body::Body::empty())?;

            Ok(response)
        }
        Err(e) => {
            tracing::error!("Failed to update user profile: {}", e);
            error_fragment(
                &v,
                "Could not update profile. Please try again.",
                "#error-container",
            )
        }
    }
}

/// Update user password
#[debug_handler]
async fn update_password(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(params): Form<UpdatePasswordParams>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    // Verify passwords match
    if params.password != params.password_confirmation {
        return error_fragment(&v, "Passwords do not match", "#password-error-container");
    }

    // Verify current password
    if !user.verify_password(&params.current_password) {
        return error_fragment(
            &v,
            "Current password is incorrect",
            "#password-error-container",
        );
    }

    // Update password - handle result with match
    match user
        .into_active_model()
        .reset_password(&ctx.db, &params.password)
        .await
    {
        Ok(_updated_user) => {
            // Return a response that refreshes the page on success
            let response = Response::builder()
                .header("HX-Refresh", "true")
                .body(axum::body::Body::empty())?;
            Ok(response)
        }
        Err(e) => {
            // Use error_fragment on failure, target password errors
            tracing::error!("Failed to update password: {}", e);
            error_fragment(
                &v,
                "Could not update password. Please try again.",
                "#password-error-container",
            )
        }
    }
}

/// Trigger sending the GPG verification email
#[debug_handler]
async fn trigger_gpg_verification(
    auth: JWTWithUserOpt<users::Model>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    ViewEngine(v): ViewEngine<TeraView>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    // Ensure email is verified
    if user.email_verified_at.is_none() {
        return error_fragment(
            &v,
            "Please verify your email address first.",
            "#gpg-error-container",
        ); // Use a specific error container
    }

    // Ensure GPG key is set
    if user.gpg_key.is_none() || user.gpg_key.as_ref().map_or(true, |k| k.is_empty()) {
        return error_fragment(&v, "Please set a GPG key first.", "#gpg-error-container");
    }

    // Send the verification email
    match GpgVerificationMailer::send_verification_email(&ctx, &user).await {
        Ok(_) => {
            // Optionally return a success message fragment
            render_template(&v, "users/_gpg_email_sent.html", data!({}))
        }
        Err(e) => {
            tracing::error!("Failed to send GPG verification email: {}", e);
            error_fragment(
                &v,
                "Failed to send verification email. Please try again later.",
                "#gpg-error-container",
            )
        }
    }
}

/// Handle the GPG verification link click
#[debug_handler]
async fn verify_gpg_key(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    axum::extract::Path(token): axum::extract::Path<String>,
) -> Result<Response> {
    // Find user by token
    let user_result = users::Entity::find()
        .filter(UserColumn::GpgKeyVerificationToken.eq(token))
        .one(&ctx.db)
        .await;

    match user_result {
        Ok(Some(user)) => {
            // Check if already verified
            if user.gpg_key_verified_at.is_some() {
                return render_template(
                    &v,
                    "users/gpg_verify.html",
                    data!({
                        "success": true,
                        "message": "Your GPG key has already been verified."
                    }),
                );
            }

            // Mark as verified
            let mut user_active: users::ActiveModel = user.into();
            user_active.gpg_key_verified_at = Set(Some(chrono::Local::now().into()));
            user_active.gpg_key_verification_token = Set(None); // Clear token

            match user_active.update(&ctx.db).await {
                Ok(_) => render_template(
                    &v,
                    "users/gpg_verify.html",
                    data!({
                        "success": true,
                        "message": "Your GPG key has been successfully verified."
                    }),
                ),
                Err(e) => {
                    tracing::error!("Failed to update GPG verification status: {}", e);
                    render_template(
                        &v,
                        "users/gpg_verify.html",
                        data!({
                            "success": false,
                            "message": "Failed to update verification status. Please try again."
                        }),
                    )
                }
            }
        }
        Ok(None) => {
            // Token not found
            render_template(
                &v,
                "users/gpg_verify.html",
                data!({
                    "success": false,
                    "message": "Invalid or expired GPG verification link."
                }),
            )
        }
        Err(e) => {
            tracing::error!("Database error during GPG verification: {}", e);
            render_template(
                &v,
                "users/gpg_verify.html",
                data!({
                    "success": false,
                    "message": "A database error occurred. Please try again later."
                }),
            )
        }
    }
}

/// User routes
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/users")
        .add("/profile", get(profile))
        .add("/invitations", get(invitations))
        .add("/invitation_count", get(get_invitation_count))
        .add("/me", put(update_profile))
        .add("/me/password", post(update_password))
        .add("/trigger-gpg-verification", post(trigger_gpg_verification))
        .add("/verify-gpg/:token", get(verify_gpg_key))
}
