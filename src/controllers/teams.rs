use axum::debug_handler;
use loco_rs::prelude::*;
use sea_orm::{PaginatorTrait, ColumnTrait, EntityTrait};
use serde::{Deserialize, Serialize};

use crate::{
    mailers::team::TeamMailer,
    models::{
        _entities::{
            team_memberships::{self, Column as TeamMembershipColumn, Entity as TeamMembershipEntity},
            teams::{self, Entity as TeamEntity},
            users::{self, Entity as UserEntity},
        },
        team_memberships::{self as team_memberships_model, UpdateRoleParams, VALID_ROLES, InviteMemberParams},
        teams::{self as teams_model, CreateTeamParams, UpdateTeamParams},
    },
    views::teams::{TeamResponse, MemberResponse},
};

type JWT = loco_rs::controller::middleware::auth::JWT;

#[debug_handler]
async fn create_team(
    auth: JWT,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateTeamParams>,
) -> Result<Response> {
    let user = crate::models::users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    let team = crate::models::teams::Model::create_team(&ctx.db, user.id, &params).await?;
    
    format::json(TeamResponse::from(&team))
}

#[debug_handler]
async fn list_teams(auth: JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = crate::models::users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    let teams = TeamEntity::find()
        .find_with_related(TeamMembershipEntity)
        .all(&ctx.db)
        .await?
        .into_iter()
        .filter_map(|(team, memberships)| {
            let is_member = memberships.iter().any(|m| m.user_id == user.id && !m.pending);
            if is_member {
                Some(TeamResponse::from(&team))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    
    format::json(teams)
}

#[debug_handler]
async fn get_team(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    let user = crate::models::users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = crate::models::teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    
    // Check if user is a member of this team
    let has_access = team.has_role(&ctx.db, user.id, "Observer").await?;
    if !has_access {
        return unauthorized("You are not a member of this team");
    }
    
    format::json(TeamResponse::from(&team))
}

#[debug_handler]
async fn update_team(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
    Json(params): Json<UpdateTeamParams>,
) -> Result<Response> {
    let user = crate::models::users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = crate::models::teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    
    // Check if user is an owner of this team
    let is_owner = team.has_role(&ctx.db, user.id, "Owner").await?;
    if !is_owner {
        return unauthorized("Only team owners can update team details");
    }
    
    let updated_team = team.update(&ctx.db, &params).await?;
    
    format::json(TeamResponse::from(&updated_team))
}

#[debug_handler]
async fn delete_team(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    let user = crate::models::users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = crate::models::teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    
    // Check if user is an owner of this team
    let is_owner = team.has_role(&ctx.db, user.id, "Owner").await?;
    if !is_owner {
        return unauthorized("Only team owners can delete a team");
    }
    
    // Delete the team
    team.remove(&ctx.db).await?;
    
    format::empty_json()
}

#[debug_handler]
async fn list_members(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    let user = crate::models::users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = crate::models::teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    
    // Check if user is a member of this team
    let has_access = team.has_role(&ctx.db, user.id, "Observer").await?;
    if !has_access {
        return unauthorized("You are not a member of this team");
    }
    
    // Get all memberships for this team
    let memberships = TeamMembershipEntity::find()
        .filter(TeamMembershipColumn::TeamId.eq(team.id))
        .filter(TeamMembershipColumn::Pending.eq(false))
        .all(&ctx.db)
        .await?;
    
    // Get user details for each membership
    let mut responses = Vec::new();
    for membership in memberships {
        let user = UserEntity::find_by_id(membership.user_id).one(&ctx.db).await?;
        if let Some(user) = user {
            responses.push(MemberResponse {
                user_pid: user.pid.to_string(),
                name: user.name,
                email: user.email,
                role: membership.role,
            });
        }
    }
    
    format::json(responses)
}

#[debug_handler]
async fn invite_member(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
    Json(params): Json<InviteMemberParams>,
) -> Result<Response> {
    let user = crate::models::users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = crate::models::teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    
    // Check if user is an admin of this team
    let is_admin = team.has_role(&ctx.db, user.id, "Admin").await?;
    if !is_admin {
        return unauthorized("Only team admins can invite members");
    }
    
    // Create invitation
    let invitation = crate::models::team_memberships::Model::create_invitation(&ctx.db, team.id, &params.email).await?;
    
    // Send invitation email
    TeamMailer::send_invitation(&ctx, &user, &team, &invitation).await?;
    
    format::empty_json()
}

#[debug_handler]
async fn accept_invitation(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(token): Path<String>,
) -> Result<Response> {
    let user = crate::models::users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    let invitation = team_memberships::Model::find_by_invitation_token(&ctx.db, &token).await?;
    if invitation.user_id != user.id {
        return unauthorized("This invitation is not for you");
    }
    
    // Accept invitation
    invitation.accept(&ctx.db).await?;
    
    format::empty_json()
}

#[debug_handler]
async fn decline_invitation(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(token): Path<String>,
) -> Result<Response> {
    let user = crate::models::users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    let invitation = team_memberships::Model::find_by_invitation_token(&ctx.db, &token).await?;
    if invitation.user_id != user.id {
        return unauthorized("This invitation is not for you");
    }
    
    // Decline invitation
    invitation.decline(&ctx.db).await?;
    
    format::empty_json()
}

#[debug_handler]
async fn update_member_role(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path((team_pid, user_pid)): Path<(String, String)>,
    Json(params): Json<UpdateRoleParams>,
) -> Result<Response> {
    let current_user = crate::models::users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = crate::models::teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    let target_user = crate::models::users::Model::find_by_pid(&ctx.db, &user_pid).await?;
    
    // Validate role
    if !VALID_ROLES.contains(&params.role.as_str()) {
        return bad_request(&format!("Invalid role. Valid roles are: {:?}", VALID_ROLES));
    }
    
    // Check if current user is an owner of this team
    let is_owner = team.has_role(&ctx.db, current_user.id, "Owner").await?;
    if !is_owner {
        return unauthorized("Only team owners can update member roles");
    }
    
    // Get membership
    let membership = team_memberships::Model::find_by_team_and_user(&ctx.db, team.id, target_user.id).await?;
    
    // Cannot change owner's role if there's only one owner
    if membership.role == "Owner" && params.role != "Owner" {
        let owners_count = team_memberships::Entity::find()
            .filter(TeamMembershipColumn::TeamId.eq(team.id))
            .filter(TeamMembershipColumn::Role.eq("Owner"))
            .count(&ctx.db)
            .await?;
        
        if owners_count <= 1 {
            return bad_request("Cannot change the role of the last owner");
        }
    }
    
    // Update role
    membership.update_role(&ctx.db, &params).await?;
    
    format::empty_json()
}

#[debug_handler]
async fn remove_member(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path((team_pid, user_pid)): Path<(String, String)>,
) -> Result<Response> {
    let current_user = crate::models::users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = crate::models::teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    let target_user = crate::models::users::Model::find_by_pid(&ctx.db, &user_pid).await?;
    
    // Cannot remove yourself - use leave_team for that
    if current_user.id == target_user.id {
        return bad_request("Cannot remove yourself from a team. Use leave_team instead.");
    }
    
    // Get target user's membership
    let target_membership = team_memberships::Entity::find()
        .filter(
            model::query::condition()
                .eq(team_memberships::Column::TeamId, team.id)
                .eq(team_memberships::Column::UserId, target_user.id)
                .eq(team_memberships::Column::Pending, false)
                .build(),
        )
        .one(&ctx.db)
        .await?
        .ok_or_else(|| ModelError::msg("User is not a member of this team"))?;
    
    // Check permissions
    let current_user_is_owner = team.has_role(&ctx.db, current_user.id, "Owner").await?;
    let current_user_is_admin = team.has_role(&ctx.db, current_user.id, "Administrator").await?;
    
    // Cannot remove an owner unless you're an owner
    if target_membership.role == "Owner" && !current_user_is_owner {
        return unauthorized("Only team owners can remove another owner");
    }
    
    // Cannot remove an admin unless you're an owner
    if target_membership.role == "Administrator" && !current_user_is_owner {
        return unauthorized("Only team owners can remove an administrator");
    }
    
    // Cannot remove a developer/observer unless you're an admin or owner
    if !current_user_is_admin {
        return unauthorized("Only team administrators and owners can remove members");
    }
    
    // Special case: Prevent removing the last owner
    if target_membership.role == "Owner" {
        // Count owners
        let owner_count = team_memberships::Entity::find()
            .filter(
                model::query::condition()
                    .eq(team_memberships::Column::TeamId, team.id)
                    .eq(team_memberships::Column::Role, "Owner")
                    .eq(team_memberships::Column::Pending, false)
                    .build(),
            )
            .count(&ctx.db)
            .await?;
        
        if owner_count <= 1 {
            return bad_request("Cannot remove the last owner");
        }
    }
    
    // Remove the member
    target_membership.remove_from_team(&ctx.db).await?;
    
    format::empty_json()
}

#[debug_handler]
async fn leave_team(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    let user = crate::models::users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = crate::models::teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    
    // Get the user's membership
    let membership = team_memberships::Entity::find()
        .filter(
            model::query::condition()
                .eq(team_memberships::Column::TeamId, team.id)
                .eq(team_memberships::Column::UserId, user.id)
                .eq(team_memberships::Column::Pending, false)
                .build(),
        )
        .one(&ctx.db)
        .await?
        .ok_or_else(|| ModelError::msg("You are not a member of this team"))?;
    
    // If user is an owner, check if they're the last owner
    if membership.role == "Owner" {
        // Count owners
        let owner_count = team_memberships::Entity::find()
            .filter(
                model::query::condition()
                    .eq(team_memberships::Column::TeamId, team.id)
                    .eq(team_memberships::Column::Role, "Owner")
                    .eq(team_memberships::Column::Pending, false)
                    .build(),
            )
            .count(&ctx.db)
            .await?;
        
        if owner_count <= 1 {
            return bad_request("As the last owner, you cannot leave the team. Either delete the team or transfer ownership first.");
        }
    }
    
    // Remove the membership
    membership.remove_from_team(&ctx.db).await?;
    
    format::empty_json()
}

#[debug_handler]
async fn list_invitations(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = crate::models::users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    let invitations = team_memberships::Model::get_user_invitations(&ctx.db, user.id).await?;
    
    let responses = invitations
        .into_iter()
        .map(|(membership, team)| {
            serde_json::json!({
                "token": membership.invitation_token,
                "team": {
                    "pid": team.pid.to_string(),
                    "name": team.name,
                    "description": team.description
                },
                "sent_at": membership.invitation_sent_at
            })
        })
        .collect::<Vec<_>>();
    
    format::json(responses)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api")
        .add("/teams", post(create_team))
        .add("/teams", get(list_teams))
        .add("/teams/:team_pid", get(get_team))
        .add("/teams/:team_pid", put(update_team))
        .add("/teams/:team_pid", delete(delete_team))
        .add("/teams/:team_pid/members", get(list_members))
        .add("/teams/:team_pid/invitations", post(invite_member))
        .add("/teams/invitations/:token/accept", post(accept_invitation))
        .add("/teams/invitations/:token/decline", post(decline_invitation))
        .add("/teams/:team_pid/members/:user_pid/role", put(update_member_role))
        .add("/teams/:team_pid/members/:user_pid", delete(remove_member))
        .add("/teams/:team_pid/leave", post(leave_team))
        .add("/teams/invitations", get(list_invitations))
} 