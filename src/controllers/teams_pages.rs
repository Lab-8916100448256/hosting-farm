use axum::debug_handler;
use loco_rs::prelude::*;
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};
use crate::models::{users, _entities};
use crate::models::_entities::{teams, team_memberships};
use serde_json::json;
use tera;
use crate::utils::{template::render_template, middleware};

type JWT = loco_rs::controller::middleware::auth::JWT;

/// Renders the teams list page
#[debug_handler]
async fn index(
    auth: JWT,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // Get all teams where the user is a member
    let teams_result = teams::Entity::find()
        .find_with_related(team_memberships::Entity)
        .all(&ctx.db)
        .await?
        .into_iter()
        .filter_map(|(team, memberships)| {
            let is_member = memberships.iter().any(|m| m.user_id == user.id && !m.pending);
            if is_member {
                Some(tera::Context::from_serialize(json!({
                    "pid": team.pid.to_string(),
                    "name": team.name,
                    "description": team.description
                })).unwrap())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    
    let mut context = tera::Context::new();
    context.insert("user", &user);
    context.insert("teams", &teams_result);
    context.insert("active_page", "teams");
    
    render_template(&ctx, "teams/index.html.tera", context)
}

/// Renders the new team page
#[debug_handler]
async fn new(
    auth: JWT,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    let mut context = tera::Context::new();
    context.insert("user", &user);
    
    render_template(&ctx, "teams/new.html.tera", context)
}

/// Renders the team details page
#[debug_handler]
async fn show(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // Find team by pid
    let parse_uuid = uuid::Uuid::parse_str(&team_pid)
        .map_err(|e| Error::msg(format!("Invalid UUID: {}", e)))?;
    
    let team = teams::Entity::find()
        .filter(teams::Column::Pid.eq(parse_uuid))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::msg("Team not found"))?;
    
    // Check if user is a member of this team
    let membership = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await?;
    
    let has_access = membership.is_some();
    if !has_access {
        return unauthorized("You are not a member of this team");
    }
    
    // Check if user is an admin or owner
    let is_admin = membership.map(|m| {
        m.role == "Owner" || m.role == "Administrator"
    }).unwrap_or(false);
    
    // Get team members
    let memberships = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .find_with_related(users::Entity)
        .all(&ctx.db)
        .await?;
    
    let members = memberships
        .into_iter()
        .filter_map(|(membership, users)| {
            if let Some(user) = users.first() {
                Some(tera::Context::from_serialize(json!({
                    "user_pid": user.pid.to_string(),
                    "name": user.name,
                    "email": user.email,
                    "role": membership.role
                })).unwrap())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    
    let mut context = tera::Context::new();
    context.insert("user", &user);
    context.insert("team", &tera::Context::from_serialize(json!({
        "pid": team.pid.to_string(),
        "name": team.name,
        "description": team.description
    })).unwrap());
    context.insert("members", &members);
    context.insert("is_admin", &is_admin);
    context.insert("active_page", "teams");
    
    render_template(&ctx, "teams/show.html.tera", context)
}

/// Renders the edit team page
#[debug_handler]
async fn edit(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // Find team by pid
    let parse_uuid = uuid::Uuid::parse_str(&team_pid)
        .map_err(|e| Error::msg(format!("Invalid UUID: {}", e)))?;
    
    let team = teams::Entity::find()
        .filter(teams::Column::Pid.eq(parse_uuid))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::msg("Team not found"))?;
    
    // Check if user is an owner of this team
    let is_owner = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Role.eq("Owner"))
        .filter(team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await?
        .is_some();
    
    if !is_owner {
        return unauthorized("Only team owners can edit team details");
    }
    
    let mut context = tera::Context::new();
    context.insert("user", &user);
    context.insert("team", &tera::Context::from_serialize(json!({
        "pid": team.pid.to_string(),
        "name": team.name,
        "description": team.description
    })).unwrap());
    
    render_template(&ctx, "teams/edit.html.tera", context)
}

/// Renders the invite team member page
#[debug_handler]
async fn invite(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // Find team by pid
    let parse_uuid = uuid::Uuid::parse_str(&team_pid)
        .map_err(|e| Error::msg(format!("Invalid UUID: {}", e)))?;
    
    let team = teams::Entity::find()
        .filter(teams::Column::Pid.eq(parse_uuid))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::msg("Team not found"))?;
    
    // Check if user is an admin of this team
    let is_admin = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await?
        .map(|m| m.role == "Owner" || m.role == "Administrator")
        .unwrap_or(false);
    
    if !is_admin {
        return unauthorized("Only team administrators can invite members");
    }
    
    let mut context = tera::Context::new();
    context.insert("user", &user);
    context.insert("team", &tera::Context::from_serialize(json!({
        "pid": team.pid.to_string(),
        "name": team.name
    })).unwrap());
    
    render_template(&ctx, "teams/invite.html.tera", context)
}

/// Team page routes
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/teams")
        .add("/", get(index).layer(middleware::auth()))
        .add("/new", get(new).layer(middleware::auth()))
        .add("/:team_pid", get(show).layer(middleware::auth()))
        .add("/:team_pid/edit", get(edit).layer(middleware::auth()))
        .add("/:team_pid/invite", get(invite).layer(middleware::auth()))
} 