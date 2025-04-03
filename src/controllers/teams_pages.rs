use crate::models::_entities::teams::Model as TeamModel;
use crate::models::{_entities, users};
use crate::utils::template::render_template;
use axum::debug_handler;
use axum::extract::Form;
use loco_rs::prelude::*;
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde_json::json;

type JWT = loco_rs::controller::middleware::auth::JWT;

/// Create team page
#[debug_handler]
async fn create_team_page(auth: JWT, State(ctx): State<AppContext>) -> Result<Response> {
    match users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await {
        Ok(user) => {
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

            render_template(&ctx, "teams/new.html", context)
        }
        Err(err) => {
            tracing::error!(
                error = err.to_string(),
                user_pid = auth.claims.pid,
                "user not found during authentication verification in /teams/new handler"
            );
            let response = Response::builder()
                .header(
                    "Set-Cookie",
                    "auth_token=; Path=/; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
                )
                // TODO: Use HX-Location instead of HX-Redirect
                .header("HX-Redirect", "/auth/login")
                .body(axum::body::Body::empty())?;
            Ok(response)
        }
    }
}

/// List teams page
#[debug_handler]
async fn list_teams(auth: JWT, State(ctx): State<AppContext>) -> Result<Response> {
    match users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await {
        Ok(user) => {
            // Get all teams where the user is a member
            let teams_result = _entities::teams::Entity::find()
                .find_with_related(_entities::team_memberships::Entity)
                .all(&ctx.db)
                .await?
                .into_iter()
                .filter_map(|(team, memberships)| {
                    let is_member = memberships
                        .iter()
                        .any(|m| m.user_id == user.id && !m.pending);
                    if is_member {
                        // Find user's role in this team
                        let role = memberships
                            .iter()
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

            render_template(&ctx, "teams/list.html", context)
        }
        Err(err) => {
            tracing::error!(
                error = err.to_string(),
                user_pid = auth.claims.pid,
                "user not found during authentication verification in /teams handler"
            );
            let response = Response::builder()
                .header(
                    "Set-Cookie",
                    "auth_token=; Path=/; Expires=Thu, 01 Jan 1970 00:00:00 GMT",
                )
                // TODO: Use HX-Location instead of HX-Redirect
                .header("HX-Redirect", "/auth/login")
                .body(axum::body::Body::empty())?;
            Ok(response)
        }
    }
}

/// Team details page
#[debug_handler]
async fn team_details(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;

    tracing::info!("Accessing team details for team pid: {}", team_pid);

    // Find team
    let team = match _entities::teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            return Err(Error::string("Team not found"));
        }
    };

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

    // Get team members (non-pending)
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
                "role": membership.role,
                "pending": false
            }));
        }
    }

    // Get pending invitations for this team
    if is_admin {
        let pending_memberships = _entities::team_memberships::Entity::find()
            .filter(_entities::team_memberships::Column::TeamId.eq(team.id))
            .filter(_entities::team_memberships::Column::Pending.eq(true))
            .all(&ctx.db)
            .await?;

        for membership in pending_memberships {
            let member = _entities::users::Entity::find_by_id(membership.user_id)
                .one(&ctx.db)
                .await?;

            if let Some(member) = member {
                members.push(json!({
                    "id": member.id,
                    "user_pid": member.pid.to_string(),
                    "name": member.name,
                    "email": member.email,
                    "role": "Invited",
                    "pending": true,
                    "invitation_token": membership.invitation_token
                }));
            }
        }
    }

    // Get pending invitations count for current user
    let invitations = _entities::team_memberships::Entity::find()
        .find_with_related(_entities::teams::Entity)
        .all(&ctx.db)
        .await?
        .into_iter()
        .filter(|(membership, _)| membership.user_id == user.id && membership.pending)
        .count();

    let mut context = tera::Context::new();
    context.insert("user", &user);
    context.insert(
        "team",
        &json!({
            "pid": team.pid.to_string(),
            "name": team.name,
            "description": team.description
        }),
    );
    context.insert("members", &members);
    context.insert("active_page", "teams");
    context.insert("is_admin", &is_admin);
    context.insert("invitation_count", &invitations);

    render_template(&ctx, "teams/show.html", context)
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
    context.insert(
        "team",
        &json!({
            "pid": team.pid.to_string(),
            "name": team.name
        }),
    );
    context.insert("active_page", "teams");
    context.insert("invitation_count", &invitations);

    render_template(&ctx, "teams/invite.html", context)
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
    context.insert(
        "team",
        &json!({
            "pid": team.pid.to_string(),
            "name": team.name,
            "description": team.description
        }),
    );
    context.insert("active_page", "teams");
    context.insert("invitation_count", &invitations);

    render_template(&ctx, "teams/edit.html", context)
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
            tracing::info!(
                "Team created successfully with id: {}, pid: {}",
                team.id,
                team.pid
            );
            team
        }
        Err(e) => {
            tracing::error!("Failed to create team: {:?}", e);
            return Err(Error::string(&format!("Failed to create team: {}", e)));
        }
    };

    // Verify team exists in database using find_by_pid for consistency
    match _entities::teams::Model::find_by_pid(&ctx.db, &team.pid.to_string()).await {
        Ok(_) => {
            tracing::info!("Team verification successful: {}", team.pid);
        }
        Err(e) => {
            tracing::error!("Team verification failed after creation: {:?}", e);
        }
    }

    // Redirect to the team details page with explicit .to_string() to ensure proper conversion
    let redirect_url = format!("/teams/{}", team.pid.to_string());
    tracing::info!("Redirecting to: {}", redirect_url);
    let response = Response::builder()
        // TODO: Use HX-Location instead of HX-Redirect
        .header("HX-Redirect", redirect_url)
        .body(axum::body::Body::empty())?;

    Ok(response)
}

/// Handle team edit form submission
#[debug_handler]
async fn update_team_handler(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
    Form(params): Form<crate::models::teams::UpdateTeamParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;

    tracing::info!("Updating team with pid: {}", team_pid);

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

    // Update the team
    let updated_team = team.update(&ctx.db, &params).await?;

    // Redirect to the team details page
    let redirect_url = format!("/teams/{}", updated_team.pid.to_string());
    tracing::info!("Redirecting to: {}", redirect_url);
    let response = Response::builder()
        // TODO: Use HX-Location instead of HX-Redirect
        .header("HX-Redirect", redirect_url)
        .body(axum::body::Body::empty())?;

    Ok(response)
}

/// Accept a team invitation
#[debug_handler]
async fn accept_invitation(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(token): Path<String>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;

    // Find invitation by token
    let invitation = _entities::team_memberships::Entity::find()
        .filter(_entities::team_memberships::Column::InvitationToken.eq(token.clone()))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::string("Invitation not found"))?;

    if invitation.user_id != user.id {
        return unauthorized("This invitation is not for you");
    }

    // Accept invitation
    let mut invitation_model: _entities::team_memberships::ActiveModel = invitation.into();
    invitation_model.pending = sea_orm::ActiveValue::set(false);
    invitation_model.update(&ctx.db).await?;

    // For HTMX, return a response that will replace the invitation item
    let response = Response::builder()
        .header("Content-Type", "text/html")
        .body(axum::body::Body::from(
            "<div class='px-4 py-4 sm:px-6 text-center text-sm text-green-600'>Invitation accepted successfully.</div>"
        ))?;

    Ok(response)
}

/// Decline a team invitation
#[debug_handler]
async fn decline_invitation(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(token): Path<String>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;

    // Find invitation by token
    let invitation = _entities::team_memberships::Entity::find()
        .filter(_entities::team_memberships::Column::InvitationToken.eq(token.clone()))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::string("Invitation not found"))?;

    if invitation.user_id != user.id {
        return unauthorized("This invitation is not for you");
    }

    // Decline invitation - delete the invitation
    let invitation_model: _entities::team_memberships::ActiveModel = invitation.into();
    invitation_model.delete(&ctx.db).await?;

    // For HTMX, return a response that will replace the invitation item
    let response = Response::builder()
        .header("Content-Type", "text/html")
        .body(axum::body::Body::from(
            "<div class='px-4 py-4 sm:px-6 text-center text-sm text-gray-600'>Invitation declined.</div>"
        ))?;

    Ok(response)
}

/// Cancel a team invitation (by admin or owner)
#[debug_handler]
async fn cancel_invitation(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path((team_pid, token)): Path<(String, String)>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;

    tracing::info!(
        "Cancelling invitation with token: {} for team: {}",
        token,
        team_pid
    );

    // Find team
    let team = match _entities::teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            return Err(Error::string("Team not found"));
        }
    };

    // Check if user is an admin or owner of this team
    let membership = _entities::team_memberships::Entity::find()
        .filter(_entities::team_memberships::Column::TeamId.eq(team.id))
        .filter(_entities::team_memberships::Column::UserId.eq(user.id))
        .filter(_entities::team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await?;

    let is_admin = if let Some(membership) = &membership {
        membership.role == "Owner" || membership.role == "Administrator"
    } else {
        false
    };

    if !is_admin {
        return unauthorized("Only team administrators can cancel invitations");
    }

    // Find invitation by token and team ID
    let invitation = _entities::team_memberships::Entity::find()
        .filter(_entities::team_memberships::Column::TeamId.eq(team.id))
        .filter(_entities::team_memberships::Column::InvitationToken.eq(token.clone()))
        .filter(_entities::team_memberships::Column::Pending.eq(true))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::string("Invitation not found"))?;

    tracing::info!(
        "Found invitation for user_id: {}, preparing to delete",
        invitation.user_id
    );

    // Cancel invitation - delete the membership
    let invitation_model: _entities::team_memberships::ActiveModel = invitation.into();
    invitation_model.delete(&ctx.db).await?;

    tracing::info!("Invitation cancelled successfully");

    // For HTMX, return an empty response that will remove the list item
    // Using an empty body with the correct content-type is what HTMX expects
    // when using hx-swap="outerHTML" and targeting the closest li
    let response = Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .body(axum::body::Body::empty())?;

    Ok(response)
}

/// Update a team member's role
#[debug_handler]
async fn update_member_role(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path((team_pid, user_pid)): Path<(String, String)>,
    Form(params): Form<crate::models::team_memberships::UpdateRoleParams>,
) -> Result<Response> {
    let current_user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = _entities::teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    let target_user = users::Model::find_by_pid(&ctx.db, &user_pid).await?;

    // Validate role
    if !crate::models::team_memberships::VALID_ROLES.contains(&params.role.as_str()) {
        return bad_request(&format!(
            "Invalid role. Valid roles are: {:?}",
            crate::models::team_memberships::VALID_ROLES
        ));
    }

    // Check if current user is an owner of this team
    let is_owner = _entities::team_memberships::Entity::find()
        .filter(_entities::team_memberships::Column::TeamId.eq(team.id))
        .filter(_entities::team_memberships::Column::UserId.eq(current_user.id))
        .filter(_entities::team_memberships::Column::Role.eq("Owner"))
        .filter(_entities::team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await?
        .is_some();

    if !is_owner {
        return unauthorized("Only team owners can update member roles");
    }

    // Get membership
    let membership = _entities::team_memberships::Entity::find()
        .filter(_entities::team_memberships::Column::TeamId.eq(team.id))
        .filter(_entities::team_memberships::Column::UserId.eq(target_user.id))
        .filter(_entities::team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::string("User is not a member of this team"))?;

    // Cannot change owner's role if there's only one owner
    if membership.role == "Owner" && params.role != "Owner" {
        let owners_count = _entities::team_memberships::Entity::find()
            .filter(_entities::team_memberships::Column::TeamId.eq(team.id))
            .filter(_entities::team_memberships::Column::Role.eq("Owner"))
            .filter(_entities::team_memberships::Column::Pending.eq(false))
            .count(&ctx.db)
            .await?;

        if owners_count <= 1 {
            return bad_request("Cannot change the role of the last owner");
        }
    }

    // Update role
    let mut membership_model: _entities::team_memberships::ActiveModel = membership.into();
    membership_model.role = sea_orm::ActiveValue::set(params.role);
    membership_model.update(&ctx.db).await?;

    // Return a response that refreshes the page
    let response = Response::builder()
        .header("HX-Refresh", "true")
        .body(axum::body::Body::empty())?;

    Ok(response)
}

/// Remove a team member
#[debug_handler]
async fn remove_member(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path((team_pid, user_pid)): Path<(String, String)>,
) -> Result<Response> {
    let current_user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = _entities::teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    let target_user = users::Model::find_by_pid(&ctx.db, &user_pid).await?;

    // Cannot remove yourself - use leave_team for that
    if current_user.id == target_user.id {
        return bad_request("Cannot remove yourself from a team. Use leave_team instead.");
    }

    // Get target user's membership
    let target_membership = _entities::team_memberships::Entity::find()
        .filter(_entities::team_memberships::Column::TeamId.eq(team.id))
        .filter(_entities::team_memberships::Column::UserId.eq(target_user.id))
        .filter(_entities::team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::string("User is not a member of this team"))?;

    // Check permissions
    let current_user_membership = _entities::team_memberships::Entity::find()
        .filter(_entities::team_memberships::Column::TeamId.eq(team.id))
        .filter(_entities::team_memberships::Column::UserId.eq(current_user.id))
        .filter(_entities::team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| Error::string("You are not a member of this team"))?;

    let current_user_is_owner = current_user_membership.role == "Owner";
    let current_user_is_admin =
        current_user_membership.role == "Administrator" || current_user_is_owner;

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
        let owner_count = _entities::team_memberships::Entity::find()
            .filter(_entities::team_memberships::Column::TeamId.eq(team.id))
            .filter(_entities::team_memberships::Column::Role.eq("Owner"))
            .filter(_entities::team_memberships::Column::Pending.eq(false))
            .count(&ctx.db)
            .await?;

        if owner_count <= 1 {
            return bad_request("Cannot remove the last owner");
        }
    }

    // Remove the member
    let target_membership_model: _entities::team_memberships::ActiveModel =
        target_membership.into();
    target_membership_model.delete(&ctx.db).await?;

    // Return a response that refreshes the page
    let response = Response::builder()
        .header("HX-Refresh", "true")
        .body(axum::body::Body::empty())?;

    Ok(response)
}

/// Delete a team
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
    let redirect_url = "/teams";
    tracing::info!("Redirecting to: {}", redirect_url);
    let response = Response::builder()
        // TODO: Use HX-Location instead of HX-Redirect
        .header("HX-Redirect", redirect_url)
        .body(axum::body::Body::empty())?;

    Ok(response)
}

/// Handle team member invitation
#[debug_handler]
async fn invite_member_handler(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
    Form(params): Form<crate::models::team_memberships::InviteMemberParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = _entities::teams::Model::find_by_pid(&ctx.db, &team_pid).await?;

    // Check if user is an admin of this team
    let is_admin = _entities::team_memberships::Entity::find()
        .filter(_entities::team_memberships::Column::TeamId.eq(team.id))
        .filter(_entities::team_memberships::Column::UserId.eq(user.id))
        .filter(
            _entities::team_memberships::Column::Role
                .eq("Owner")
                .or(_entities::team_memberships::Column::Role.eq("Administrator")),
        )
        .filter(_entities::team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await?
        .is_some();

    if !is_admin {
        return unauthorized("Only team administrators can invite members");
    }

    // Find the user by email
    let target_user = match users::Model::find_by_email(&ctx.db, &params.email).await {
        Ok(user) => user,
        Err(_) => {
            // If user doesn't exist, proceed with invitation as normal
            // Create invitation
            let invitation = _entities::team_memberships::Model::create_invitation(
                &ctx.db,
                team.id,
                &params.email,
            )
            .await?;

            // Find the invited user
            let invited_user = users::Model::find_by_id(&ctx.db, invitation.user_id).await?;

            // Send invitation email using the TeamMailer
            crate::mailers::team::TeamMailer::send_invitation(
                &ctx,
                &invited_user,
                &team,
                &invitation,
            )
            .await?;

            // Redirect back to the team page
            let redirect_url = format!("/teams/{}", team_pid);
            tracing::info!("Redirecting to: {}", redirect_url);
            let response = Response::builder()
                // TODO: Use HX-Location instead of HX-Redirect
                .header("HX-Redirect", redirect_url)
                .body(axum::body::Body::empty())?;

            return Ok(response);
        }
    };

    // Check if user is already a member or has a pending invitation
    let existing_membership = _entities::team_memberships::Entity::find()
        .filter(_entities::team_memberships::Column::TeamId.eq(team.id))
        .filter(_entities::team_memberships::Column::UserId.eq(target_user.id))
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

        // Return error message with HTMX
        let html_response = format!(
            "<div id='invitation-error' class='mt-2 text-sm text-red-600'>{}</div>",
            error_message
        );

        let response = Response::builder()
            .header("Content-Type", "text/html")
            .header("HX-Retarget", "#invitation-error-container")
            .header("HX-Swap", "innerHTML")
            .body(axum::body::Body::from(html_response))?;

        return Ok(response);
    }

    // If we get here, the user exists but isn't a member, so create invitation
    let invitation =
        _entities::team_memberships::Model::create_invitation(&ctx.db, team.id, &params.email)
            .await?;

    // Send invitation email using the TeamMailer
    crate::mailers::team::TeamMailer::send_invitation(&ctx, &target_user, &team, &invitation)
        .await?;

    // Redirect back to the team page
    let redirect_url = format!("/teams/{}", team_pid);
    tracing::info!("Redirecting to: {}", redirect_url);
    let response = Response::builder()
        // TODO: Use HX-Location instead of HX-Redirect
        .header("HX-Redirect", redirect_url)
        .body(axum::body::Body::empty())?;

    Ok(response)
}

/// Team routes
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/teams")
        .add("/", get(list_teams))
        .add("/new", get(create_team_page))
        .add("/new", post(create_team_handler))
        .add("/{team_pid}", get(team_details))
        .add("/{team_pid}", delete(delete_team))
        .add("/{team_pid}/edit", get(edit_team_page))
        .add("/{team_pid}/update", post(update_team_handler))
        .add("/{team_pid}/invite", get(invite_member_page))
        .add("/{team_pid}/invite", post(invite_member_handler))
        .add("/invitations/{token}/accept", post(accept_invitation))
        .add("/invitations/{token}/decline", post(decline_invitation))
        .add(
            "/{team_pid}/invitations/{token}/cancel",
            post(cancel_invitation),
        )
        .add(
            "/{team_pid}/members/{user_pid}/role",
            put(update_member_role),
        )
        .add("/{team_pid}/members/{user_pid}", delete(remove_member))
}
