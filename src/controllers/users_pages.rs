use crate::{
    middleware::auth_no_error::JWTWithUserOpt,
    models::{_entities::team_memberships, _entities::teams, users},
    utils::template::render_template,
    views::htmx_redirect,
};
use axum::debug_handler;
use axum::extract::Form;
use axum::response::Redirect;
use loco_rs::prelude::*;
use sea_orm::PaginatorTrait;
use serde::Deserialize;
use serde_json::json;
use tera;

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
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        // Redirect to login if not authenticated
        return Ok(Redirect::to("/auth/login").into_response());
    };

    // Get user's team memberships
    let teams = teams::Entity::find()
        .find_with_related(team_memberships::Entity)
        .all(&ctx.db)
        .await?
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
    let invitations = team_memberships::Entity::find()
        .find_with_related(teams::Entity)
        .all(&ctx.db)
        .await?
        .into_iter()
        .filter(|(membership, _)| membership.user_id == user.id && membership.pending)
        .count();

    let mut context = tera::Context::new();
    context.insert("user", &user);
    context.insert("teams", &teams);
    context.insert("active_page", "profile");
    context.insert("invitation_count", &invitations);

    render_template(&ctx, "users/profile.html", context)
}

/// Renders the user invitations page
#[debug_handler]
async fn invitations(
    auth: JWTWithUserOpt<users::Model>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        // Redirect to login if not authenticated
        return Ok(Redirect::to("/auth/login").into_response());
    };

    // Get user's pending team invitations
    let invitations = team_memberships::Entity::find()
        .find_with_related(teams::Entity)
        .all(&ctx.db)
        .await?
        .into_iter()
        .filter_map(|(membership, teams)| {
            if membership.user_id == user.id && membership.pending {
                let team = teams.first()?;
                Some(json!({
                    "team_name": team.name,
                    "team_description": team.description.clone(),
                    "token": membership.invitation_token,
                    "role": membership.role,
                    "sent_at": membership.created_at.format("%Y-%m-%d").to_string()
                }))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // Get pending invitations count (same as above but just the count)
    let invitation_count = invitations.len();

    let mut context = tera::Context::new();
    context.insert("user", &user);
    context.insert("invitations", &invitations);
    context.insert("active_page", "invitations");
    context.insert("invitation_count", &invitation_count);

    render_template(&ctx, "users/invitations.html", context)
}

/// Returns the HTML fragment for the invitation count badge
#[debug_handler]
async fn get_invitation_count(
    auth: JWTWithUserOpt<users::Model>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return htmx_redirect("/auth/login");
    };

    // Get pending invitations count
    let invitation_count = team_memberships::Entity::find()
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(true))
        .count(&ctx.db)
        .await?;

    let mut context = tera::Context::new();
    context.insert("invitation_count", &invitation_count);

    // Use render_template to render the fragment
    render_template(&ctx, "users/_invitation_count_badge.html", context)
}

/// Update user profile
#[debug_handler]
async fn update_profile(
    auth: JWTWithUserOpt<users::Model>,
    State(ctx): State<AppContext>,
    Form(params): Form<UpdateProfileParams>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return htmx_redirect("/auth/login");
    };

    // Check if email already exists (if it changed)
    if user.email != params.email {
        let existing_user = users::Model::find_by_email(&ctx.db, &params.email).await;
        if existing_user.is_ok() {
            return bad_request("Email already in use");
        }
    }

    // Update user profile
    let mut user_model: crate::models::_entities::users::ActiveModel = user.into();
    user_model.name = sea_orm::ActiveValue::set(params.name);
    user_model.email = sea_orm::ActiveValue::set(params.email);

    let _updated_user = user_model.update(&ctx.db).await?;

    // Return a response that refreshes the page
    let response = Response::builder()
        .header("HX-Refresh", "true")
        .body(axum::body::Body::empty())?;

    Ok(response)
}

/// Update user password
#[debug_handler]
async fn update_password(
    auth: JWTWithUserOpt<users::Model>,
    State(ctx): State<AppContext>,
    Form(params): Form<UpdatePasswordParams>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return htmx_redirect("/auth/login");
    };

    // Verify passwords match
    if params.password != params.password_confirmation {
        return bad_request("Passwords do not match");
    }

    // Verify current password
    if !user.verify_password(&params.current_password) {
        return bad_request("Current password is incorrect");
    }

    // Update password
    let _updated_user = user
        .into_active_model()
        .reset_password(&ctx.db, &params.password)
        .await?;

    // Return a response that refreshes the page
    let response = Response::builder()
        .header("HX-Refresh", "true")
        .body(axum::body::Body::empty())?;

    Ok(response)
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
