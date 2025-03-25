use axum::debug_handler;
use loco_rs::prelude::*;
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
use crate::models::{users, _entities};
use serde_json::json;
use uuid::Uuid;
use crate::utils::template::render_template;

type JWT = loco_rs::controller::middleware::auth::JWT;

/// Create team page
#[debug_handler]
async fn create_team_page(
    auth: JWT,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // Get pending invitations count
    let invitations = _entities::team_memberships::Entity::find()
        .find_with_related(_entities::teams::Entity)
        .all(&ctx.db)
        .await?
        .into_iter()
        .filter(|(membership, _)| membership.user_id == user.id && membership.pending)
        .count();
    
    let mut context = tera::Context::new();
    context.insert("user", &user);
    context.insert("active_page", "teams");
    context.insert("invitation_count", &invitations);
    
    render_template(&ctx, "teams/new.html.tera", context)
}

/// List teams page
#[debug_handler]
async fn list_teams(
    auth: JWT,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // Get all teams where the user is a member
    let teams_result = _entities::teams::Entity::find()
        .find_with_related(_entities::team_memberships::Entity)
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
                    "description": team.description,
                    "role": role
                }))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    
    let mut context = tera::Context::new();
    context.insert("user", &user);
    context.insert("teams", &teams_result);
    context.insert("active_page", "teams");
    
    render_template(&ctx, "teams/list.html.tera", context)
}

/// Team details page
#[debug_handler]
async fn team_details(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // Parse and validate PID
    let _pid = Uuid::parse_str(&team_pid)
        .map_err(|e| loco_rs::Error::string(&format!("Invalid UUID: {}", e)))?;
    
    // Find team
    let team = _entities::teams::Entity::find()
        .filter(_entities::teams::Column::Pid.eq(team_pid.clone()))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| loco_rs::Error::string("Team not found"))?;
    
    // Check if user is a member of this team
    let membership = _entities::team_memberships::Entity::find()
        .filter(_entities::team_memberships::Column::TeamId.eq(team.id))
        .filter(_entities::team_memberships::Column::UserId.eq(user.id))
        .filter(_entities::team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await?;
    
    if membership.is_none() {
        return unauthorized("You are not a member of this team");
    }
    
    // Get team members
    let memberships = _entities::team_memberships::Entity::find()
        .filter(_entities::team_memberships::Column::TeamId.eq(team.id))
        .filter(_entities::team_memberships::Column::Pending.eq(false))
        .all(&ctx.db)
        .await?;
    
    let mut members = Vec::new();
    for membership in memberships {
        let member = _entities::users::Entity::find_by_id(membership.user_id)
            .one(&ctx.db)
            .await?;
        
        if let Some(member) = member {
            members.push(json!({
                "id": member.id,
                "name": member.name,
                "email": member.email,
                "role": membership.role
            }));
        }
    }
    
    let mut context = tera::Context::new();
    context.insert("user", &user);
    context.insert("team", &json!({
        "pid": team.pid.to_string(),
        "name": team.name,
        "description": team.description
    }));
    context.insert("members", &members);
    context.insert("active_page", "teams");
    
    render_template(&ctx, "teams/details.html.tera", context)
}

/// Invite member page
#[debug_handler]
async fn invite_member_page(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // Parse and validate PID
    let _pid = Uuid::parse_str(&team_pid)
        .map_err(|e| loco_rs::Error::string(&format!("Invalid UUID: {}", e)))?;
    
    // Find team
    let team = _entities::teams::Entity::find()
        .filter(_entities::teams::Column::Pid.eq(team_pid.clone()))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| loco_rs::Error::string("Team not found"))?;
    
    // Check if user is an administrator of this team
    let membership = _entities::team_memberships::Entity::find()
        .filter(_entities::team_memberships::Column::TeamId.eq(team.id))
        .filter(_entities::team_memberships::Column::UserId.eq(user.id))
        .filter(_entities::team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await?;
    
    if let Some(membership) = membership {
        if membership.role != "Owner" && membership.role != "Administrator" {
            return unauthorized("Only team administrators can invite members");
        }
    } else {
        return unauthorized("You are not a member of this team");
    }
    
    let mut context = tera::Context::new();
    context.insert("user", &user);
    context.insert("team", &json!({
        "pid": team.pid.to_string(),
        "name": team.name
    }));
    context.insert("active_page", "teams");
    
    render_template(&ctx, "teams/invite.html.tera", context)
}

/// Edit team page
#[debug_handler]
async fn edit_team_page(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // Parse and validate PID
    let _pid = Uuid::parse_str(&team_pid)
        .map_err(|e| loco_rs::Error::string(&format!("Invalid UUID: {}", e)))?;
    
    // Find team
    let team = _entities::teams::Entity::find()
        .filter(_entities::teams::Column::Pid.eq(team_pid.clone()))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| loco_rs::Error::string("Team not found"))?;
    
    // Check if user is an owner of this team
    let membership = _entities::team_memberships::Entity::find()
        .filter(_entities::team_memberships::Column::TeamId.eq(team.id))
        .filter(_entities::team_memberships::Column::UserId.eq(user.id))
        .filter(_entities::team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await?;
    
    if let Some(membership) = membership {
        if membership.role != "Owner" {
            return unauthorized("Only team owners can edit team details");
        }
    } else {
        return unauthorized("You are not a member of this team");
    }
    
    let mut context = tera::Context::new();
    context.insert("user", &user);
    context.insert("team", &json!({
        "pid": team.pid.to_string(),
        "name": team.name,
        "description": team.description
    }));
    context.insert("active_page", "teams");
    
    render_template(&ctx, "teams/edit.html.tera", context)
}

/// Handle team creation form submission
#[debug_handler]
async fn create_team_handler(
    auth: JWT,
    State(ctx): State<AppContext>,
    Form(params): Form<crate::models::teams::CreateTeamParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // Create the team
    let team = _entities::teams::Model::create_team(&ctx.db, user.id, &params).await?;
    
    // Redirect to the team details page
    format::redirect(&format!("/teams/{}", team.pid))
}

/// Team routes
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/teams")
        .add("/", get(list_teams))
        .add("/new", get(create_team_page))
        .add("/new", post(create_team_handler))
        .add("/{team_pid}", get(team_details))
        .add("/{team_pid}/edit", get(edit_team_page))
        .add("/{team_pid}/invite", get(invite_member_page))
} 