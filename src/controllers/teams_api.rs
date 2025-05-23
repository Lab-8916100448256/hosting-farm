use axum::debug_handler;
use loco_rs::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait};

use crate::{
    mailers::team::TeamMailer,
    models::{
        _entities::{
            team_memberships::{
                self, Column as TeamMembershipColumn, Entity as TeamMembershipEntity,
                Model as TeamMembershipModel,
            },
            teams::{Entity as TeamEntity, Model as TeamModel},
            users::Entity as UserEntity,
        },
        team_memberships::{InviteMemberParams, UpdateRoleParams, VALID_ROLES},
        teams::{CreateTeamParams, UpdateTeamParams},
        users,
    },
    views::teams::{MemberResponse, TeamResponse},
};

use loco_rs::controller::middleware::auth::JWT;

#[debug_handler]
async fn create_team(
    auth: JWT,
    State(ctx): State<AppContext>,
    Json(params): Json<CreateTeamParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;

    // Create a new team
    let team = TeamModel::create_team(&ctx.db, user.id, &params).await?;

    format::json(TeamResponse::from(&team))
}

#[debug_handler]
async fn list_teams(auth: JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;

    // Get all teams where the user is a member
    let teams = TeamEntity::find()
        .find_with_related(TeamMembershipEntity)
        .all(&ctx.db)
        .await?
        .into_iter()
        .filter_map(|(team, memberships)| {
            let is_member = memberships
                .iter()
                .any(|m| m.user_id == user.id && !m.pending);
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
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;

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
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;

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
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;

    // Check if user is an owner of this team
    let is_owner = team.has_role(&ctx.db, user.id, "Owner").await?;
    if !is_owner {
        return unauthorized("Only team owners can delete a team");
    }

    // Delete the team
    team.delete(&ctx.db).await?;

    // Instead of returning empty JSON, send a redirect to the teams list page
    let response = Response::builder()
        // TODO: Use HX-Location instead of HX-Redirect
        .header("HX-Redirect", "/teams")
        .body(axum::body::Body::empty())?;

    Ok(response)
}

#[debug_handler]
async fn list_members(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;

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
        let user = UserEntity::find_by_id(membership.user_id)
            .one(&ctx.db)
            .await?;
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
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;

    // Check if user is an admin of this team
    let is_admin = team.has_role(&ctx.db, user.id, "Administrator").await?;
    if !is_admin {
        return unauthorized("Only team administrators can invite members");
    }

    // Find the target user by name
    let target_user = match users::Model::find_by_name(&ctx.db, &params.user_name).await {
        Ok(user) => user,
        Err(_) => {
            return bad_request(format!("No user found with name {}", &params.user_name));
        }
    };

    // Check if user is already a member or has a pending invitation
    let existing_membership = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(target_user.id))
        .one(&ctx.db)
        .await?;

    if let Some(membership) = existing_membership {
        // User already has a relationship with this team
        let error_message = if membership.pending {
            format!(
                "User {} already has a pending invitation to this team",
                params.user_name
            )
        } else {
            format!("User {} is already a member of this team", params.user_name)
        };

        return bad_request(error_message);
    }
    // If we get here, the user exists but isn't a member,
    // so, proceed with inviting to the team

    // Create invitation entity
    /*let invitation =*/
    match team_memberships::Model::create_invitation(&ctx.db, team.id, &params.user_name).await {
        Ok(invit) => invit,
        Err(e) => {
            // Something terribly wrong happened, abort with an error message

            // First log the error
            let message = "Failed to create a team invitation entity";
            tracing::error!(
                error = e.to_string(),
                team_id = team.id,
                target_user = &params.user_name,
                message
            );

            // then return an error to the caller
            return Err(Error::InternalServerError);
        }
    };

    // Send notification e_mail to target user
    TeamMailer::send_invitation(&ctx, &user, &target_user, &team).await?;

    format::empty_json()
}

#[debug_handler]
async fn update_member_role(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path((team_pid, user_pid)): Path<(String, String)>,
    Json(params): Json<UpdateRoleParams>,
) -> Result<Response> {
    let current_user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;
    let target_user = users::Model::find_by_pid(&ctx.db, &user_pid).await?;

    // Validate role
    if !VALID_ROLES.contains(&params.role.as_str()) {
        return bad_request(format!("Invalid role. Valid roles are: {:?}", VALID_ROLES));
    }

    // Check if current user is an owner of this team
    let is_owner = team.has_role(&ctx.db, current_user.id, "Owner").await?;
    if !is_owner {
        return unauthorized("Only team owners can update member roles");
    }

    // Get membership
    let membership =
        TeamMembershipModel::find_by_team_and_user(&ctx.db, team.id, target_user.id).await?;

    // Cannot change owner's role if there's only one owner
    if membership.role == "Owner" && params.role != "Owner" {
        let owners_count = TeamMembershipEntity::find()
            .filter(TeamMembershipColumn::TeamId.eq(team.id))
            .filter(TeamMembershipColumn::Role.eq("Owner"))
            .count(&ctx.db)
            .await?;

        if owners_count <= 1 {
            return bad_request("Cannot change the role of the last owner");
        }
    }

    // Update role
    membership.update_role(&ctx.db, &params.role).await?;

    format::empty_json()
}

#[debug_handler]
async fn remove_member(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path((team_pid, user_pid)): Path<(String, String)>,
) -> Result<Response> {
    let current_user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;
    let target_user = users::Model::find_by_pid(&ctx.db, &user_pid).await?;

    // Cannot remove yourself - use leave_team for that
    if current_user.id == target_user.id {
        return bad_request("Cannot remove yourself from a team. Use leave_team instead.");
    }

    // Get target user's membership
    let target_membership = TeamMembershipEntity::find()
        .filter(TeamMembershipColumn::TeamId.eq(team.id))
        .filter(TeamMembershipColumn::UserId.eq(target_user.id))
        .filter(TeamMembershipColumn::Pending.eq(false))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| ModelError::msg("User is not a member of this team"))?;

    // Check permissions
    let current_user_is_owner = team.has_role(&ctx.db, current_user.id, "Owner").await?;
    let current_user_is_admin = team
        .has_role(&ctx.db, current_user.id, "Administrator")
        .await?;

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
        let owner_count = TeamMembershipEntity::find()
            .filter(TeamMembershipColumn::TeamId.eq(team.id))
            .filter(TeamMembershipColumn::Role.eq("Owner"))
            .filter(TeamMembershipColumn::Pending.eq(false))
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
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;

    // Get the user's membership
    let membership = TeamMembershipEntity::find()
        .filter(TeamMembershipColumn::TeamId.eq(team.id))
        .filter(TeamMembershipColumn::UserId.eq(user.id))
        .filter(TeamMembershipColumn::Pending.eq(false))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| ModelError::msg("You are not a member of this team"))?;

    // If user is an owner, check if they're the last owner
    if membership.role == "Owner" {
        // Count owners
        let owner_count = TeamMembershipEntity::find()
            .filter(TeamMembershipColumn::TeamId.eq(team.id))
            .filter(TeamMembershipColumn::Role.eq("Owner"))
            .filter(TeamMembershipColumn::Pending.eq(false))
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
async fn list_invitations(auth: JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;

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
        .prefix("/api")
        .add("/teams", get(list_teams))
        .add("/teams", post(create_team))
        .add("/teams/{team_pid}", get(get_team))
        .add("/teams/{team_pid}", put(update_team))
        .add("/teams/{team_pid}", delete(delete_team))
        .add("/teams/{team_pid}/members", get(list_members))
        .add("/teams/{team_pid}/invitations", post(invite_member))
        .add(
            "/teams/{team_pid}/members/{user_pid}/role",
            put(update_member_role),
        )
        .add(
            "/teams/{team_pid}/members/{user_pid}",
            delete(remove_member),
        )
        .add("/teams/{team_pid}/leave", post(leave_team))
        .add("/teams/invitations", get(list_invitations))
}
