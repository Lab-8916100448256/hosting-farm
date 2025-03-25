use axum::debug_handler;
use loco_rs::prelude::*;
use crate::models::{users, _entities::team_memberships, _entities::teams};
use serde_json::json;
use tera;
use crate::utils::template::render_template;

type JWT = loco_rs::controller::middleware::auth::JWT;

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
    
    let mut context = tera::Context::new();
    context.insert("user", &user);
    context.insert("teams", &teams);
    context.insert("active_page", "profile");
    
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
                    "token": membership.invitation_token,
                    "role": membership.role
                }))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    
    let mut context = tera::Context::new();
    context.insert("user", &user);
    context.insert("invitations", &invitations);
    context.insert("active_page", "invitations");
    
    render_template(&ctx, "users/invitations.html.tera", context)
}

/// User routes
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/users")
        .add("/profile", get(profile))
        .add("/invitations", get(invitations))
} 