#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use loco_rs::prelude::*;
use axum::{
    debug_handler,
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::models::{teams, team_members, team_invitations, users, auth};

#[derive(Debug, Serialize)]
struct TeamResponse {
    pid: String,
    name: String,
    description: Option<String>,
    owner_id: i32,
    created_at: String,
    updated_at: String,
}

impl From<&teams::Model> for TeamResponse {
    fn from(team: &teams::Model) -> Self {
        Self {
            pid: team.pid.to_string(),
            name: team.name.clone(),
            description: team.description.clone(),
            owner_id: team.owner_id,
            created_at: team.created_at.to_string(),
            updated_at: team.updated_at.to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
struct TeamMemberResponse {
    pid: String,
    user_id: i32,
    role: String,
    user_name: String,
    user_email: String,
}

#[derive(Debug, Serialize)]
struct TeamInvitationResponse {
    pid: String,
    email: String,
    role: String,
    status: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct CreateTeamParams {
    name: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateTeamParams {
    name: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InviteTeamMemberParams {
    email: String,
    role: String,
}

#[derive(Debug, Deserialize)]
struct UpdateRoleParams {
    role: String,
}

#[debug_handler]
async fn index(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let teams = teams::Model::find_by_user(&ctx.db, user.id).await?;
    
    let response: Vec<TeamResponse> = teams.iter().map(TeamResponse::from).collect();
    format::json(response)
}

#[debug_handler]
async fn create(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateTeamParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    let create_params = teams::CreateParams {
        name: params.name,
        description: params.description,
    };
    
    let team = teams::Model::create(&ctx.db, user.id, &create_params).await?;
    
    format::json(TeamResponse::from(&team))
}

#[debug_handler]
async fn show(
    auth: auth::JWT,
    Path(team_pid): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    
    // Check if user is a member of the team
    let _member = team_members::Model::find_by_team_and_user(&ctx.db, team.id, user.id)
        .await
        .map_err(|_| format::unauthorized("You are not a member of this team"))?;
    
    format::json(TeamResponse::from(&team))
}

#[debug_handler]
async fn update(
    auth: auth::JWT,
    Path(team_pid): Path<String>,
    State(ctx): State<AppContext>,
    Json(params): Json<UpdateTeamParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    
    // Check if user is the owner or admin
    let member = team_members::Model::find_by_team_and_user(&ctx.db, team.id, user.id)
        .await
        .map_err(|_| format::unauthorized("You are not a member of this team"))?;
    
    if !member.is_team_admin(&ctx.db).await? {
        return format::unauthorized("You don't have permission to update this team");
    }
    
    let update_params = teams::UpdateParams {
        name: params.name,
        description: params.description,
    };
    
    let updated_team = team.update(&ctx.db, &update_params).await?;
    
    format::json(TeamResponse::from(&updated_team))
}

#[debug_handler]
async fn delete(
    auth: auth::JWT,
    Path(team_pid): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    
    // Check if user is the owner
    if team.owner_id != user.id {
        return format::unauthorized("Only the team owner can delete the team");
    }
    
    team.delete(&ctx.db).await?;
    
    format::empty_json()
}

#[debug_handler]
async fn members(
    auth: auth::JWT,
    Path(team_pid): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    
    // Check if user is a member of the team
    let _member = team_members::Model::find_by_team_and_user(&ctx.db, team.id, user.id)
        .await
        .map_err(|_| format::unauthorized("You are not a member of this team"))?;
    
    let members = team_members::Model::find_by_team(&ctx.db, team.id).await?;
    
    let mut response = Vec::new();
    for member in members {
        let user = users::Model::find_by_id(&ctx.db, member.user_id).await?;
        response.push(TeamMemberResponse {
            pid: member.pid.to_string(),
            user_id: member.user_id,
            role: member.role.clone(),
            user_name: user.name.clone(),
            user_email: user.email.clone(),
        });
    }
    
    format::json(response)
}

#[debug_handler]
async fn update_member_role(
    auth: auth::JWT,
    Path((team_pid, member_pid)): Path<(String, String)>,
    State(ctx): State<AppContext>,
    Json(params): Json<UpdateRoleParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    
    // Check if user is an admin or owner
    let current_user_member = team_members::Model::find_by_team_and_user(&ctx.db, team.id, user.id)
        .await
        .map_err(|_| format::unauthorized("You are not a member of this team"))?;
    
    if !current_user_member.can_manage_members(&ctx.db).await? {
        return format::unauthorized("You don't have permission to update member roles");
    }
    
    // Get the member to update
    let member = team_members::Model::find_by_pid(&ctx.db, &member_pid).await?;
    
    // Prevent changing the owner's role
    if member.user_id == team.owner_id {
        return format::bad_request("Cannot change the role of the team owner");
    }
    
    // Prevent non-owners from changing admin roles
    if member.role == "admin" && team.owner_id != user.id {
        return format::unauthorized("Only the team owner can change an admin's role");
    }
    
    let update_params = team_members::UpdateRoleParams {
        role: params.role,
    };
    
    let updated_member = member.update_role(&ctx.db, &update_params).await?;
    
    let member_user = users::Model::find_by_id(&ctx.db, updated_member.user_id).await?;
    
    format::json(TeamMemberResponse {
        pid: updated_member.pid.to_string(),
        user_id: updated_member.user_id,
        role: updated_member.role.clone(),
        user_name: member_user.name.clone(),
        user_email: member_user.email.clone(),
    })
}

#[debug_handler]
async fn remove_member(
    auth: auth::JWT,
    Path((team_pid, member_pid)): Path<(String, String)>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    
    // Check if user is an admin or owner
    let current_user_member = team_members::Model::find_by_team_and_user(&ctx.db, team.id, user.id)
        .await
        .map_err(|_| format::unauthorized("You are not a member of this team"))?;
    
    if !current_user_member.can_manage_members(&ctx.db).await? {
        return format::unauthorized("You don't have permission to remove members");
    }
    
    // Get the member to remove
    let member = team_members::Model::find_by_pid(&ctx.db, &member_pid).await?;
    
    // Prevent removing the owner
    if member.user_id == team.owner_id {
        return format::bad_request("Cannot remove the team owner");
    }
    
    // Prevent non-owners from removing admins
    if member.role == "admin" && team.owner_id != user.id {
        return format::unauthorized("Only the team owner can remove an admin");
    }
    
    member.delete(&ctx.db).await?;
    
    format::empty_json()
}

#[debug_handler]
async fn invite(
    auth: auth::JWT,
    Path(team_pid): Path<String>,
    State(ctx): State<AppContext>,
    Json(params): Json<InviteTeamMemberParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    
    // Check if user is an admin or owner
    let current_user_member = team_members::Model::find_by_team_and_user(&ctx.db, team.id, user.id)
        .await
        .map_err(|_| format::unauthorized("You are not a member of this team"))?;
    
    if !current_user_member.can_manage_members(&ctx.db).await? {
        return format::unauthorized("You don't have permission to invite members");
    }
    
    // Check if the role is valid
    if !["admin", "member"].contains(&params.role.as_str()) {
        return format::bad_request("Invalid role. Must be 'admin' or 'member'");
    }
    
    // Create invitation
    let invite_params = team_invitations::CreateParams {
        team_id: team.id,
        email: params.email,
        role: params.role,
        invited_by_id: user.id,
    };
    
    let invitation = team_invitations::Model::create(&ctx.db, &invite_params).await?;
    
    // TODO: Send invitation email
    
    format::json(TeamInvitationResponse {
        pid: invitation.pid.to_string(),
        email: invitation.email.clone(),
        role: invitation.role.clone(),
        status: "pending".to_string(),
        created_at: invitation.created_at.to_string(),
    })
}

#[debug_handler]
async fn invitations(
    auth: auth::JWT,
    Path(team_pid): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    
    // Check if user is an admin or owner
    let current_user_member = team_members::Model::find_by_team_and_user(&ctx.db, team.id, user.id)
        .await
        .map_err(|_| format::unauthorized("You are not a member of this team"))?;
    
    if !current_user_member.can_manage_members(&ctx.db).await? {
        return format::unauthorized("You don't have permission to view invitations");
    }
    
    let invitations = team_invitations::Model::find_by_team(&ctx.db, team.id).await?;
    
    let mut response = Vec::new();
    for invitation in invitations {
        let status = if invitation.accepted_at.is_some() {
            "accepted"
        } else if invitation.rejected_at.is_some() {
            "rejected"
        } else if invitation.is_expired().await {
            "expired"
        } else {
            "pending"
        };
        
        response.push(TeamInvitationResponse {
            pid: invitation.pid.to_string(),
            email: invitation.email.clone(),
            role: invitation.role.clone(),
            status: status.to_string(),
            created_at: invitation.created_at.to_string(),
        });
    }
    
    format::json(response)
}

#[debug_handler]
async fn user_invitations(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    let invitations = team_invitations::Model::find_pending_by_email(&ctx.db, &user.email).await?;
    
    let mut response = Vec::new();
    for invitation in invitations {
        let team = teams::Model::find_by_id(&ctx.db, invitation.team_id).await?;
        let inviter = users::Model::find_by_id(&ctx.db, invitation.invited_by_id).await?;
        
        response.push(serde_json::json!({
            "pid": invitation.pid.to_string(),
            "team": {
                "pid": team.pid.to_string(),
                "name": team.name,
            },
            "role": invitation.role,
            "invited_by": {
                "name": inviter.name,
                "email": inviter.email,
            },
            "created_at": invitation.created_at.to_string(),
        }));
    }
    
    format::json(response)
}

#[debug_handler]
async fn accept_invitation(
    auth: auth::JWT,
    Path(invitation_pid): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let invitation = team_invitations::Model::find_by_pid(&ctx.db, &invitation_pid).await?;
    
    // Check if invitation is for this user
    if invitation.email != user.email {
        return format::unauthorized("This invitation is not for you");
    }
    
    // Accept invitation
    invitation.accept(&ctx.db, user.id).await?;
    
    format::empty_json()
}

#[debug_handler]
async fn reject_invitation(
    auth: auth::JWT,
    Path(invitation_pid): Path<String>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let invitation = team_invitations::Model::find_by_pid(&ctx.db, &invitation_pid).await?;
    
    // Check if invitation is for this user
    if invitation.email != user.email {
        return format::unauthorized("This invitation is not for you");
    }
    
    // Reject invitation
    invitation.reject(&ctx.db).await?;
    
    format::empty_json()
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/teams")
        .add("/", get(index))
        .add("/", post(create))
        .add("/:team_pid", get(show))
        .add("/:team_pid", put(update))
        .add("/:team_pid", delete(delete))
        .add("/:team_pid/members", get(members))
        .add("/:team_pid/members/:member_pid/role", put(update_member_role))
        .add("/:team_pid/members/:member_pid", delete(remove_member))
        .add("/:team_pid/invite", post(invite))
        .add("/:team_pid/invitations", get(invitations))
        .add("/invitations", get(user_invitations))
        .add("/invitations/:invitation_pid/accept", post(accept_invitation))
        .add("/invitations/:invitation_pid/reject", post(reject_invitation))
}
