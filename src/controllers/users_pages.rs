use axum::debug_handler;
use loco_rs::prelude::*;
use crate::models::{users, _entities::team_memberships, _entities::teams};
use serde_json::json;
use tera;
use crate::utils::template::render_template;
use serde::Deserialize;
use axum::extract::Form;

type JWT = loco_rs::controller::middleware::auth::JWT;

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
    auth: JWT,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // Get user's team memberships
    let teams = teams::Entity::find()
        .find_with_related(team_memberships::Entity)
        .all(&ctx.db)
        .await?
        .into_iter()
        .filter_map(|(team, memberships)| {
            let is_member = memberships.iter().any(|m| m.user_id == user.id && !m.pending);
            if is_member {
                // Find user's role in this team
                let role = memberships.iter()
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
    
    render_template(&ctx, "users/profile.html.tera", context)
}

/// Renders the user invitations page
#[debug_handler]
async fn invitations(
    auth: JWT,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
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
    
    render_template(&ctx, "users/invitations.html.tera", context)
}

/// Update user profile
#[debug_handler]
async fn update_profile(
    auth: JWT,
    State(ctx): State<AppContext>,
    Form(params): Form<UpdateProfileParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
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
    auth: JWT,
    State(ctx): State<AppContext>,
    Form(params): Form<UpdatePasswordParams>,
) -> Result<Response> {
    // Verify passwords match
    if params.password != params.password_confirmation {
        return bad_request("Passwords do not match");
    }
    
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // Verify current password
    if !user.verify_password(&params.current_password) {
        return bad_request("Current password is incorrect");
    }
    
    // Update password
    let _updated_user = user.into_active_model()
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
        .add("/me", put(update_profile))
        .add("/me/password", post(update_password))
} 