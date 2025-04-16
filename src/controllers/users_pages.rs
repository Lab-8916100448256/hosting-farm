use crate::{
    middleware::auth_no_error::JWTWithUserOpt,
    models::{_entities::team_memberships, _entities::teams, users},
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

    // Fetch SSH keys for the user
    let ssh_keys_result = crate::models::ssh_public_keys::Entity::find()
        .filter(crate::models::_entities::ssh_public_keys::Column::UserId.eq(user.id))
        .all(&ctx.db)
        .await;
    let (ssh_keys, ssh_keys_error) = match ssh_keys_result {
        Ok(keys) => (keys, None),
        Err(e) => (vec![], Some(format!("Could not load SSH keys: {}", e))),
    };
    render_template(
        &v,
        "users/profile.html",
        data!({
            "user": &user,
            "teams": &teams,
            "active_page": "profile",
            "invitation_count": &invitations,
            "ssh_keys": &ssh_keys,
            "ssh_keys_error": ssh_keys_error,
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

/// User routes
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/users")
        .add("/profile", get(profile))
        .add("/invitations", get(invitations))
        .add("/invitation_count", get(get_invitation_count))
        .add("/me", put(update_profile))
        .add("/me/password", post(update_password))
}
