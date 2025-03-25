//! Controllers for team management
use axum::{
    extract::{Path, State},
    routing::{get, post, put, delete},
    Json, Router,
};
use loco_rs::{
    auth,
    controller::prelude::*,
    model::ModelError,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{teams, team_memberships, users};
use crate::models::_entities::team_memberships::Role;
use crate::views::auth::CurrentResponse;

/// Parameters for creating a new team
#[derive(Debug, Deserialize)]
pub struct CreateTeamParams {
    pub name: String,
    pub description: Option<String>,
}

/// Parameters for updating a team
#[derive(Debug, Deserialize)]
pub struct UpdateTeamParams {
    pub name: String,
    pub description: Option<String>,
}

/// Parameters for inviting a user to a team
#[derive(Debug, Deserialize)]
pub struct InviteParams {
    pub email: String,
    pub role: Role,
}

/// Parameters for updating a team member's role
#[derive(Debug, Deserialize)]
pub struct UpdateRoleParams {
    pub role: Role,
}

/// Response for team operations
#[derive(Debug, Serialize)]
pub struct TeamResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub slug: String,
}

/// Response for team listing
#[derive(Debug, Serialize)]
pub struct TeamListResponse {
    pub teams: Vec<TeamResponse>,
}

/// Response for team membership operations
#[derive(Debug, Serialize)]
pub struct TeamMembershipResponse {
    pub id: Uuid,
    pub team_id: Uuid,
    pub user: Option<CurrentResponse>,
    pub role: Role,
    pub invitation_email: Option<String>,
    pub accepted: bool,
}

/// Response for team membership listing
#[derive(Debug, Serialize)]
pub struct TeamMembershipListResponse {
    pub members: Vec<TeamMembershipResponse>,
}

impl From<&teams::Model> for TeamResponse {
    fn from(team: &teams::Model) -> Self {
        Self {
            id: team.pid,
            name: team.name.clone(),
            description: team.description.clone(),
            slug: team.slug.clone(),
        }
    }
}

/// Create a new team
#[debug_handler]
async fn create_team(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateTeamParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    let team = teams::Model::create(
        &ctx.db,
        teams::CreateParams {
            name: params.name,
            description: params.description,
            creator_id: user.id,
        },
    ).await?;
    
    format::json(TeamResponse::from(&team))
}

/// Get a list of teams the current user is a member of
#[debug_handler]
async fn list_teams(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // This would need to be implemented in the users model
    let teams = user.get_teams(&ctx.db).await?;
    
    let team_responses: Vec<TeamResponse> = teams.iter().map(TeamResponse::from).collect();
    
    format::json(TeamListResponse {
        teams: team_responses,
    })
}

/// Get details of a specific team
#[debug_handler]
async fn get_team(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(team_id): Path<Uuid>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    
    // Check if user is a member of the team
    // This would need to be implemented in the team_memberships model
    if !team_memberships::Model::is_member(&ctx.db, team.id, user.id).await? {
        return unauthorized("You are not a member of this team");
    }
    
    format::json(TeamResponse::from(&team))
}

/// Update a team
#[debug_handler]
async fn update_team(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(team_id): Path<Uuid>,
    Json(params): Json<UpdateTeamParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    
    // Check if user is an owner of the team
    // This would need to be implemented in the team_memberships model
    if !team_memberships::Model::has_role(&ctx.db, team.id, user.id, Role::Owner).await? {
        return unauthorized("Only team owners can update team details");
    }
    
    let updated_team = team.update(
        &ctx.db,
        teams::UpdateParams {
            name: params.name,
            description: params.description,
        },
    ).await?;
    
    format::json(TeamResponse::from(&updated_team))
}

/// Delete a team
#[debug_handler]
async fn delete_team(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(team_id): Path<Uuid>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    
    // Check if user is an owner of the team
    // This would need to be implemented in the team_memberships model
    if !team_memberships::Model::has_role(&ctx.db, team.id, user.id, Role::Owner).await? {
        return unauthorized("Only team owners can delete teams");
    }
    
    team.delete(&ctx.db).await?;
    
    format::empty_json()
}

/// Invite a user to a team
#[debug_handler]
async fn invite_member(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(team_id): Path<Uuid>,
    Json(params): Json<InviteParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    
    // Check if user is an owner or administrator of the team
    // This would need to be implemented in the team_memberships model
    let user_role = team_memberships::Model::get_role(&ctx.db, team.id, user.id).await?;
    if user_role != Role::Owner && user_role != Role::Administrator {
        return unauthorized("Only team owners and administrators can invite members");
    }
    
    // Create the invitation
    let membership = team_memberships::Model::invite(
        &ctx.db,
        team_memberships::InviteParams {
            team_id: team.id,
            email: params.email,
            role: params.role,
        },
    ).await?;
    
    // Send invitation email (would be implemented in a mailer)
    // TeamMailer::send_invitation(&ctx, &membership).await?;
    
    format::empty_json()
}

/// Accept a team invitation
#[axum::debug_handler]
async fn accept_invitation(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(token): Path<String>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // Find the invitation by token
    let membership = team_memberships::Model::find_by_invitation_token(&ctx.db, &token).await?;
    
    // Accept the invitation
    membership.accept_invitation(&ctx.db, user.id).await?;
    
    format::empty_json()
}

/// Update a team member's role
#[debug_handler]
async fn update_member_role(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path((team_id, member_id)): Path<(Uuid, Uuid)>,
    Json(params): Json<UpdateRoleParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    let membership = team_memberships::Model::find_by_pid(&ctx.db, &member_id).await?;
    
    // Check if the membership belongs to the specified team
    if membership.team_id != team.id {
        return bad_request("Membership does not belong to the specified team");
    }
    
    // Check if user has permission to change roles
    let user_role = team_memberships::Model::get_role(&ctx.db, team.id, user.id).await?;
    let target_user_role = membership.role.clone();
    
    // Implement role change permission logic based on the requirements
    // For example, owners can change roles of administrators, developers, and observers
    // Administrators can change roles of developers and observers
    if !can_change_role(user_role, target_user_role, params.role.clone()) {
        return unauthorized("You don't have permission to change this member's role");
    }
    
    // Update the role
    membership.update_role(
        &ctx.db,
        team_memberships::UpdateRoleParams {
            role: params.role,
        },
    ).await?;
    
    format::empty_json()
}

/// Remove a member from a team
#[debug_handler]
async fn remove_member(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path((team_id, member_id)): Path<(Uuid, Uuid)>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    let membership = team_memberships::Model::find_by_pid(&ctx.db, &member_id).await?;
    
    // Check if the membership belongs to the specified team
    if membership.team_id != team.id {
        return bad_request("Membership does not belong to the specified team");
    }
    
    // Check if user has permission to remove members
    let user_role = team_memberships::Model::get_role(&ctx.db, team.id, user.id).await?;
    let target_user_role = membership.role.clone();
    
    // Implement member removal permission logic based on the requirements
    // For example, owners can remove administrators, developers, and observers
    // Administrators can remove developers and observers
    if !can_remove_member(user_role, target_user_role) {
        return unauthorized("You don't have permission to remove this member");
    }
    
    // Remove the member
    membership.remove(&ctx.db).await?;
    
    format::empty_json()
}

/// Get a list of team members
#[debug_handler]
async fn list_members(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(team_id): Path<Uuid>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    
    // Check if user is a member of the team
    if !team_memberships::Model::is_member(&ctx.db, team.id, user.id).await? {
        return unauthorized("You are not a member of this team");
    }
    
    // Get all team members
    // This would need to be implemented in the team_memberships model
    let members = team_memberships::Model::get_team_members(&ctx.db, team.id).await?;
    
    // Convert to response format
    // This would need additional implementation to fetch user details
    let member_responses = Vec::new(); // Placeholder
    
    format::json(TeamMembershipListResponse {
        members: member_responses,
    })
}

// Helper function to determine if a user can change another user's role
fn can_change_role(user_role: Role, target_role: Role, new_role: Role) -> bool {
    match user_role {
        Role::Owner => {
            // Owners can change roles of administrators, developers, and observers
            target_role != Role::Owner
        },
        Role::Administrator => {
            // Administrators can change roles of developers and observers
            (target_role == Role::Developer || target_role == Role::Observer) &&
            (new_role == Role::Developer || new_role == Role::Observer)
        },
        _ => false, // Developers and observers cannot change roles
    }
}

// Helper function to determine if a user can remove another user
fn can_remove_member(user_role: Role, target_role: Role) -> bool {
    match user_role {
        Role::Owner => {
            // Owners can remove administrators, developers, and observers
            target_role != Role::Owner
        },
        Role::Administrator => {
            // Administrators can remove developers and observers
            target_role == Role::Developer || target_role == Role::Observer
        },
        _ => false, // Developers and observers cannot remove members
    }
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/teams")
        .add("/", post(create_team))
        .add("/", get(list_teams))
        .add("/:id", get(get_team))
        .add("/:id", put(update_team))
        .add("/:id", delete(delete_team))
        .add("/:id/invitations", post(invite_member))
        .add("/:id/members", get(list_members))
        .add("/:id/members/:member_id", put(update_member_role))
        .add("/:id/members/:member_id", delete(remove_member))
        .add("/invitations/:token/accept", post(accept_invitation))
}
