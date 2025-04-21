use axum::debug_handler;
use loco_rs::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QuerySelect, TryIntoModel}; // Import TryIntoModel
use strum::IntoEnumIterator; // Import strum trait for iter()

use crate::{
    mailers::team::TeamMailer,
    models::{
        _entities::{
            team_memberships::{
                self, Column as TeamMembershipColumn, Entity as TeamMembershipEntity,
                Model as TeamMembershipModel,
            },
            teams::{Entity as TeamEntity, Model as TeamEntityModel}, // Import Entity and Model alias
            users::Entity as UserEntity,
        },
        team_memberships::{InviteMemberParams, UpdateRoleParams, Model as TeamMembershipCustomModel}, // Added custom model for team_memberships
        teams::{CreateTeamParams, UpdateTeamParams, Role, Model as TeamModel}, // Import Role and custom TeamModel
        users::{self, Model as UserModel},
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

    // Create a new team using the custom model method
    // Call create_team on the custom TeamModel, not the entity
    let team = TeamModel::create_team(&ctx.db, user.id, &params).await?;

    // Pass the custom TeamModel to TeamResponse::from
    format::json(TeamResponse::from(&team))
}

#[debug_handler]
async fn list_teams(auth: JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;

    // Get all teams where the user is a member
    let memberships = TeamMembershipEntity::find()
        .filter(TeamMembershipColumn::UserId.eq(user.id))
        .filter(TeamMembershipColumn::Pending.eq(false))
        .all(&ctx.db)
        .await?;

    let team_ids = memberships.iter().map(|m| m.team_id).collect::<Vec<_>>();

    if team_ids.is_empty() {
        return format::json(Vec::<TeamResponse>::new());
    }

    let teams = TeamEntity::find()
        .filter(crate::models::_entities::teams::Column::Id.is_in(team_ids))
        .all(&ctx.db)
        .await?;

    let team_responses = teams.into_iter()
        // Pass the entity model to TeamResponse::from
        .map(|team_entity| TeamResponse::from(&team_entity))
        .collect::<Vec<_>>();

    format::json(team_responses)
}

#[debug_handler]
async fn get_team(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team_model = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;

    // Check if user is a member of this team (any role)
    let has_access = team_model.has_role(&ctx.db, user.id, vec![Role::Observer, Role::Developer, Role::Admin, Role::Owner]).await?;
    if !has_access {
        return unauthorized("You are not a member of this team");
    }

    // Pass the custom TeamModel to TeamResponse::from
    format::json(TeamResponse::from(&team_model))
}

#[debug_handler]
async fn update_team(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
    Json(params): Json<UpdateTeamParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team_model = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;

    // Check if user is an owner of this team
    let is_owner = team_model.has_role(&ctx.db, user.id, vec![Role::Owner]).await?;
    if !is_owner {
        return unauthorized("Only team owners can update team details");
    }

    let updated_team_model = team_model.update(&ctx.db, &params).await?;

    // Pass the updated custom TeamModel to TeamResponse::from
    format::json(TeamResponse::from(&updated_team_model))
}

#[debug_handler]
async fn delete_team(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team_model = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;

    // Check if user is an owner of this team
    let is_owner = team_model.has_role(&ctx.db, user.id, vec![Role::Owner]).await?;
    if !is_owner {
        return unauthorized("Only team owners can delete a team");
    }

    // Delete the team using the custom model
    team_model.delete(&ctx.db).await?;

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
    let team_model = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;

    // Check if user is a member of this team
    let has_access = team_model.has_role(&ctx.db, user.id, vec![Role::Observer, Role::Developer, Role::Admin, Role::Owner]).await?;
    if !has_access {
        return unauthorized("You are not a member of this team");
    }

    // Get all memberships for this team using the custom model method
    let members = team_model.get_members(&ctx.db).await?;

    let responses = members
        .into_iter()
        .map(|(user_model, role)| MemberResponse {
            user_pid: user_model.pid.to_string(),
            name: user_model.name,
            email: user_model.email,
            role: role.to_string(), // Convert Role enum to String
        })
        .collect::<Vec<_>>();

    format::json(responses)
}

#[debug_handler]
async fn invite_member(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
    Json(params): Json<InviteMemberParams>,
) -> Result<Response> {
    let current_user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team_model = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;

    // Check if user is an admin or owner of this team
    let is_admin = team_model.has_role(&ctx.db, current_user.id, vec![Role::Admin, Role::Owner]).await?;
    if !is_admin {
        return unauthorized("Only team administrators or owners can invite members");
    }

    // Find the target user by email
    let target_user = match users::Model::find_by_email(&ctx.db, &params.email).await {
        Ok(user) => user,
        Err(_) => {
            return bad_request(format!("No user found with e-mail {}", &params.email));
        }
    };

    // Check if user is already a member or has a pending invitation
    let existing_membership = TeamMembershipEntity::find()
        .filter(TeamMembershipColumn::TeamId.eq(team_model.id))
        .filter(TeamMembershipColumn::UserId.eq(target_user.id))
        .one(&ctx.db)
        .await?;

    if let Some(membership) = existing_membership {
        // User already has a relationship with this team
        let error_message = if membership.pending {
            format!(
                "User {} already has a pending invitation to this team",
                params.email
            )
        } else {
            format!("User {} is already a member of this team", params.email)
        };

        return bad_request(error_message);
    }
    // If we get here, the user exists but isn't a member,
    // so, proceed with inviting to the team

    // Create invitation entity using the model function from team_memberships
    match TeamMembershipCustomModel::create_invitation(&ctx.db, team_model.id, &params.email).await {
        Ok(_invitation) => {
             // Refetch the Team *Entity* Model needed by the mailer
             let team_entity = TeamEntity::find_by_id(team_model.id).one(&ctx.db).await?;
             if let Some(team_entity) = team_entity {
                 // Send notification e_mail to target user
                 TeamMailer::send_invitation(&ctx, &current_user, &target_user, &team_entity).await?;
                 format::empty_json()
             } else {
                  tracing::error!("Could not refetch team entity (id: {}) after creating invitation for mailer.", team_model.id);
                  // Still return success to API, but log the mail failure
                  format::empty_json()
             }
        }
        Err(e) => {
            // Something terribly wrong happened, abort with an error message
            let message = "Failed to create a team invitation entity";
            tracing::error!(
                error = e.to_string(),
                team_id = team_model.id,
                target_user = &params.email,
                %message
            );
            Err(Error::InternalServerError)
        }
    }
}

#[debug_handler]
async fn update_member_role(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path((team_pid, user_pid)): Path<(String, String)>,
    Json(params): Json<UpdateRoleParams>,
) -> Result<Response> {
    let current_user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team_model = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;
    let target_user = users::Model::find_by_pid(&ctx.db, &user_pid).await?;

    // Validate role string and convert to Role enum
    let new_role = match Role::from_str(&params.role) {
        Some(role) => role,
        None => {
            // Use Role::iter() here
            let valid_roles_str = Role::iter() // Use iter() here
                .map(|r| r.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return bad_request(&format!("Invalid role. Valid roles are: {}", valid_roles_str));
        }
    };

    // Check if current user is an owner of this team
    let is_owner = team_model.has_role(&ctx.db, current_user.id, vec![Role::Owner]).await?;
    if !is_owner {
        return unauthorized("Only team owners can update member roles");
    }

    // Get membership entity
    let membership_entity =
        TeamMembershipEntity::find()
            .filter(TeamMembershipColumn::TeamId.eq(team_model.id))
            .filter(TeamMembershipColumn::UserId.eq(target_user.id))
            .one(&ctx.db)
            .await?
            .ok_or_else(|| ModelError::msg("User is not a member of this team"))?;

    // Convert entity to custom Model to call update_role
    let membership_model: TeamMembershipCustomModel = membership_entity.try_into_model()?;

    // Cannot change owner's role if there's only one owner
    if membership_model.role == Role::Owner.as_str() && new_role != Role::Owner {
        let owners_count = TeamMembershipEntity::find()
            .filter(TeamMembershipColumn::TeamId.eq(team_model.id))
            .filter(TeamMembershipColumn::Role.eq(Role::Owner.as_str()))
            .count(&ctx.db)
            .await?;

        if owners_count <= 1 {
            return bad_request("Cannot change the role of the last owner");
        }
    }

    // Update role using the model method
    membership_model.update_role(&ctx.db, new_role.as_str()).await?;

    format::empty_json()
}

#[debug_handler]
async fn remove_member(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path((team_pid, user_pid)): Path<(String, String)>,
) -> Result<Response> {
    let current_user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team_model = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;
    let target_user = users::Model::find_by_pid(&ctx.db, &user_pid).await?;

    // Cannot remove yourself - use leave_team for that
    if current_user.id == target_user.id {
        return bad_request("Cannot remove yourself from a team. Use leave_team instead.");
    }

    // Get target user's membership entity
    let target_membership_entity = TeamMembershipEntity::find()
        .filter(TeamMembershipColumn::TeamId.eq(team_model.id))
        .filter(TeamMembershipColumn::UserId.eq(target_user.id))
        .filter(TeamMembershipColumn::Pending.eq(false))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| ModelError::msg("User is not a member of this team"))?;

    // Convert entity to custom model to call methods
    let target_membership: TeamMembershipCustomModel = target_membership_entity.try_into_model()?;
    let target_role = Role::from_str(&target_membership.role).unwrap_or(Role::Observer); // Default to Observer if invalid

    // Check permissions
    let current_user_is_owner = team_model.has_role(&ctx.db, current_user.id, vec![Role::Owner]).await?;
    let current_user_is_admin = team_model.has_role(&ctx.db, current_user.id, vec![Role::Admin, Role::Owner]).await?;

    // Cannot remove an owner unless you're an owner
    if target_role == Role::Owner && !current_user_is_owner {
        return unauthorized("Only team owners can remove another owner");
    }

    // Cannot remove an admin unless you're an owner
    if target_role == Role::Admin && !current_user_is_owner {
        return unauthorized("Only team owners can remove an administrator");
    }

    // Cannot remove a developer/observer unless you're an admin or owner
    if (target_role == Role::Developer || target_role == Role::Observer) && !current_user_is_admin {
        return unauthorized("Only team administrators and owners can remove members");
    }

    // Special case: Prevent removing the last owner
    if target_role == Role::Owner {
        let owner_count = TeamMembershipEntity::find()
            .filter(TeamMembershipColumn::TeamId.eq(team_model.id))
            .filter(TeamMembershipColumn::Role.eq(Role::Owner.as_str()))
            .filter(TeamMembershipColumn::Pending.eq(false))
            .count(&ctx.db)
            .await?;

        if owner_count <= 1 {
            return bad_request("Cannot remove the last owner");
        }
    }

    // Remove the member using the model method
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
    let team_model = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;

    // Get the user's membership entity
    let membership_entity = TeamMembershipEntity::find()
        .filter(TeamMembershipColumn::TeamId.eq(team_model.id))
        .filter(TeamMembershipColumn::UserId.eq(user.id))
        .filter(TeamMembershipColumn::Pending.eq(false))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| ModelError::msg("You are not a member of this team"))?;

    // Convert to custom model to call methods
    let membership: TeamMembershipCustomModel = membership_entity.try_into_model()?;
    let member_role = Role::from_str(&membership.role).unwrap_or(Role::Observer);

    // If user is an owner, check if they're the last owner
    if member_role == Role::Owner {
        let owner_count = TeamMembershipEntity::find()
            .filter(TeamMembershipColumn::TeamId.eq(team_model.id))
            .filter(TeamMembershipColumn::Role.eq(Role::Owner.as_str()))
            .filter(TeamMembershipColumn::Pending.eq(false))
            .count(&ctx.db)
            .await?;

        if owner_count <= 1 {
            return bad_request("As the last owner, you cannot leave the team. Either delete the team or transfer ownership first.");
        }
    }

    // Remove the membership using the model method
    membership.remove_from_team(&ctx.db).await?;

    format::empty_json()
}

#[debug_handler]
async fn list_invitations(auth: JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;

    let invitations = TeamMembershipCustomModel::get_user_invitations(&ctx.db, user.id).await?;

    let responses = invitations
        .into_iter()
        .map(|(membership, team)| {
            // Use the team *entity* model here (as returned by get_user_invitations)
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
