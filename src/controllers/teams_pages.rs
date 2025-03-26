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
    context.insert("teams", &teams_result);
    context.insert("active_page", "teams");
    context.insert("invitation_count", &invitations);
    
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
    
    tracing::info!("Accessing team details for pid: {}", team_pid);
    
    // Parse and validate PID
    let _pid = match Uuid::parse_str(&team_pid) {
        Ok(uuid) => uuid,
        Err(e) => {
            tracing::error!("Invalid UUID format for team_pid: {}, error: {}", team_pid, e);
            return Err(Error::string(&format!("Invalid UUID: {}", e)));
        }
    };
    
    // Find team - Use the Model::find_by_pid method which has improved error handling
    let team = match _entities::teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            return Err(Error::string("Team not found"));
        }
    };
    
    tracing::info!("Found team: {}, id: {}, pid: {}", team.name, team.id, team.pid);
    
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
    
    // Check if user is an admin or owner
    let is_admin = if let Some(membership) = &membership {
        membership.role == "Owner" || membership.role == "Administrator" 
    } else {
        false
    };
    
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
                "user_pid": member.pid.to_string(),
                "name": member.name,
                "email": member.email,
                "role": membership.role
            }));
        }
    }
    
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
    context.insert("team", &json!({
        "pid": team.pid.to_string(),
        "name": team.name,
        "description": team.description
    }));
    context.insert("members", &members);
    context.insert("active_page", "teams");
    context.insert("is_admin", &is_admin);
    context.insert("invitation_count", &invitations);
    
    render_template(&ctx, "teams/show.html.tera", context)
}

/// Invite member page
#[debug_handler]
async fn invite_member_page(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    tracing::info!("Accessing invite member page for team pid: {}", team_pid);
    
    // Find team
    let team = match _entities::teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            return Err(Error::string("Team not found"));
        }
    };
    
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
    context.insert("team", &json!({
        "pid": team.pid.to_string(),
        "name": team.name
    }));
    context.insert("active_page", "teams");
    context.insert("invitation_count", &invitations);
    
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
    
    tracing::info!("Accessing edit team page for team pid: {}", team_pid);
    
    // Find team
    let team = match _entities::teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            return Err(Error::string("Team not found"));
        }
    };
    
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
    
    tracing::info!("Creating team with name: {}", params.name);
    
    // Create the team
    let team = match _entities::teams::Model::create_team(&ctx.db, user.id, &params).await {
        Ok(team) => {
            tracing::info!("Team created successfully with id: {}, pid: {}", team.id, team.pid);
            team
        },
        Err(e) => {
            tracing::error!("Failed to create team: {:?}", e);
            return Err(Error::string(&format!("Failed to create team: {}", e)));
        }
    };
    
    // Verify team exists in database using find_by_pid for consistency
    match _entities::teams::Model::find_by_pid(&ctx.db, &team.pid.to_string()).await {
        Ok(_) => {
            tracing::info!("Team verification successful: {}", team.pid);
        },
        Err(e) => {
            tracing::error!("Team verification failed after creation: {:?}", e);
        }
    }
    
    // Redirect to the team details page with explicit .to_string() to ensure proper conversion
    let redirect_url = format!("/teams/{}", team.pid.to_string());
    tracing::info!("Redirecting to: {}", redirect_url);
    format::redirect(&redirect_url)
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