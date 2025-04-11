use crate::{
    middleware::auth_no_error::JWTWithUserOpt,
    models::{
        _entities::{team_memberships, teams, users},
        team_memberships::{InviteMemberParams, UpdateRoleParams},
        teams::{CreateTeamParams, UpdateTeamParams},
    },
    utils::template::render_template,
    views::{error_fragment, error_page, htmx_redirect},
};
use axum::{
    debug_handler,
    extract::{Form, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::{ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect};
use serde::Deserialize;
use serde_json::json;
use tracing;

/// Create team page
#[debug_handler]
async fn create_team_page(
    auth: JWTWithUserOpt<users::Model>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    if auth.user.is_none() {
        return Ok(Redirect::to("/auth/login").into_response());
    }
    let user = auth.user.unwrap();

    // Get pending invitations count
    let invitations = team_memberships::Entity::find()
        .find_with_related(teams::Entity)
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

/// List teams page
#[debug_handler]
async fn list_teams(
    auth: JWTWithUserOpt<users::Model>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    if auth.user.is_none() {
        return Ok(Redirect::to("/auth/login").into_response());
    }
    let user = auth.user.unwrap();

    // Get all teams where the user is a member
    let teams_result = teams::Entity::find()
        .find_with_related(team_memberships::Entity)
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
    let invitations = team_memberships::Entity::find()
        .find_with_related(teams::Entity)
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

/// Team details page
#[debug_handler]
async fn team_details(
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<users::Model>,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    if auth.user.is_none() {
        return Ok(Redirect::to("/auth/login").into_response());
    }
    let user = auth.user.unwrap();

    tracing::info!("Accessing team details for team pid: {}", team_pid);

    // Find team
    let team = match teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            return error_page(&v, "Team not found", None);
        }
    };

    // Check if user is a member of this team
    let membership = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await?;

    if membership.is_none() {
        tracing::error!(
            "Access to team {} by unauthorized user: {:?}",
            team.name,
            user
        );
        return error_page(
            &v,
            "You are not authorized to view this team because you are not one of its members",
            None,
        );
    }

    // Check if user is an admin or owner
    let is_admin = if let Some(membership) = &membership {
        membership.role == "Owner" || membership.role == "Administrator"
    } else {
        false
    };

    // Get team members (non-pending)
    let memberships = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .all(&ctx.db)
        .await?;

    let mut members = Vec::new();
    for membership in memberships {
        let member = users::Entity::find_by_id(membership.user_id)
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
        let pending_memberships = team_memberships::Entity::find()
            .filter(team_memberships::Column::TeamId.eq(team.id))
            .filter(team_memberships::Column::Pending.eq(true))
            .all(&ctx.db)
            .await?;

        for membership in pending_memberships {
            let member = users::Entity::find_by_id(membership.user_id)
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
    let invitations = team_memberships::Entity::find()
        .find_with_related(teams::Entity)
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
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<users::Model>,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    if auth.user.is_none() {
        return Ok(Redirect::to("/auth/login").into_response());
    }
    let user = auth.user.unwrap();

    // Find team
    let team = match teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            return error_page(&v, "Team not found", None);
        }
    };

    // Check if user is an admin or owner of this team
    let membership = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await?;

    if let Some(membership) = membership {
        if membership.role != "Owner" && membership.role != "Administrator" {
            tracing::error!(
                "Unauthorized user: {:?} tried to invite into team {}",
                user,
                team.name,
            );
            return error_page(&v, "Only team administrators can invite members", None);
        }
    } else {
        tracing::error!(
            "Access to team {} by unauthorized user: {:?}",
            team.name,
            user
        );
        return error_page(&v, "You are not a member of this team", None);
    }

    // Get pending invitations count for current user
    let invitations = team_memberships::Entity::find()
        .find_with_related(teams::Entity)
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

    render_template(&ctx, "teams/invite.html", context)
}

/// Edit team page
#[debug_handler]
async fn edit_team_page(
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<users::Model>,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    if auth.user.is_none() {
        return Ok(Redirect::to("/auth/login").into_response());
    }
    let user = auth.user.unwrap();

    // Find team
    let team = match teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            return error_page(&v, "Team not found", None);
        }
    };

    // Check if user is an admin or owner of this team
    let membership = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await?;

    if let Some(membership) = membership {
        if membership.role != "Owner" && membership.role != "Administrator" {
            tracing::error!(
                "Unauthorized user: {:?} tried to edit team {}",
                user,
                team.name,
            );
            return error_page(&v, "Only team administrators can team details", None);
        }
    } else {
        tracing::error!(
            "Access to team {} by unauthorized user: {:?}",
            team.name,
            user
        );
        return error_page(&v, "You are not a member of this team", None);
    }

    // Get pending invitations count for current user
    let invitations = team_memberships::Entity::find()
        .find_with_related(teams::Entity)
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

/// Create team handler
#[debug_handler]
async fn create_team_handler(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    Form(params): Form<CreateTeamParams>,
) -> Result<impl IntoResponse> {
    if auth.user.is_none() {
        return htmx_redirect("/auth/login");
    }
    let user = auth.user.unwrap();

    // Create the team
    let team = match teams::Model::create_team(&ctx.db, user.id, &params).await {
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
            return error_fragment(&v, &format!("Failed to create team: {}", e));
        }
    };

    // Verify team exists in database using find_by_pid for consistency
    match teams::Model::find_by_pid(&ctx.db, &team.pid.to_string()).await {
        Ok(_) => {
            tracing::info!("Team verification successful: {}", team.pid);
        }
        Err(e) => {
            tracing::error!("Team verification failed after creation: {:?}", e);
            return error_fragment(
                &v,
                &format!("Team verification failed after creation: {}", e),
            );
        }
    }

    // Redirect to the team details page with explicit .to_string() to ensure proper conversion
    let redirect_url = format!("/teams/{}", team.pid);
    htmx_redirect(&redirect_url)
}

/// Update team handler
#[debug_handler]
async fn update_team_handler(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    Path(team_pid): Path<String>,
    Form(params): Form<UpdateTeamParams>,
) -> Result<impl IntoResponse> {
    if auth.user.is_none() {
        return htmx_redirect("/auth/login");
    }
    let user = auth.user.unwrap();

    // Find team
    let team = match teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            return error_fragment(&v, "Team not found");
        }
    };

    // Check if user is an owner of this team
    let membership = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await?;

    if let Some(membership) = membership {
        if membership.role != "Owner" {
            return error_fragment(&v, "Only team owners can edit team details");
        }
    } else {
        return error_fragment(&v, "You are not a member of this team");
    }

    // Update the team
    let updated_team = team.update(&ctx.db, &params).await?;

    // Redirect to the team details page
    let redirect_url = format!("/teams/{}", updated_team.pid);
    htmx_redirect(&redirect_url)
}

/// Accept invitation handler
#[debug_handler]
async fn accept_invitation(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse> {
    if auth.user.is_none() {
        return htmx_redirect("/auth/login");
    }
    let user = auth.user.unwrap();

    // Find invitation by token
    let invitation = match team_memberships::Entity::find()
        .filter(team_memberships::Column::InvitationToken.eq(token.clone()))
        .one(&ctx.db)
        .await
    {
        Ok(value) => match value {
            Some(invitation) => invitation,
            None => return error_page(&v, "Invitation not found", None),
        },
        Err(err) => {
            tracing::error!("Failed to find invitation: {:?}", err);
            return error_page(&v, "Database error while searching for invitation", None);
        }
    };

    if invitation.user_id != user.id {
        return error_page(&v, "This invitation is not for you", None);
    }

    // Accept invitation
    let mut invitation_model: team_memberships::ActiveModel = invitation.into();
    invitation_model.pending = sea_orm::ActiveValue::set(false);
    invitation_model.update(&ctx.db).await?;

    // Remove the invitation row from the UI
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("HX-Trigger", "updateInvitationCount")
        .body(axum::body::Body::empty())?;

    Ok(response)
}

/// Decline invitation handler
#[debug_handler]
async fn decline_invitation(
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<users::Model>,
    State(ctx): State<AppContext>,
    Path(token): Path<String>,
) -> Result<Response> {
    if auth.user.is_none() {
        return htmx_redirect("/auth/login");
    }
    let user = auth.user.unwrap();

    // Find invitation by token
    let invitation = match team_memberships::Entity::find()
        .filter(team_memberships::Column::InvitationToken.eq(token.clone()))
        .one(&ctx.db)
        .await
    {
        Ok(value) => match value {
            Some(invitation) => invitation,
            None => return error_page(&v, "Invitation not found", None),
        },
        Err(err) => {
            tracing::error!("Failed to find invitation: {:?}", err);
            return error_page(&v, "Database error while searching for invitation", None);
        }
    };

    if invitation.user_id != user.id {
        return error_page(&v, "This invitation is not for you", None);
    }

    // Decline invitation - delete the invitation
    let invitation_model: team_memberships::ActiveModel = invitation.into();
    invitation_model.delete(&ctx.db).await?;

    // Remove the invitation row from the UI
    let response = Response::builder()
        .status(StatusCode::OK)
        .header("HX-Trigger", "updateInvitationCount")
        .body(axum::body::Body::empty())?;

    Ok(response)
}

/// Cancel invitation handler
#[debug_handler]
async fn cancel_invitation(
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<users::Model>,
    State(ctx): State<AppContext>,
    Path((team_pid, token)): Path<(String, String)>,
) -> Result<Response> {
    if auth.user.is_none() {
        return htmx_redirect("/auth/login");
    }
    let user = auth.user.unwrap();

    // Find team
    let team = match teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            return error_page(&v, "Team not found", None);
        }
    };

    // Check if user is an admin or owner of this team
    let membership = match team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await
    {
        Ok(value) => value,
        Err(err) => {
            tracing::error!("Failed to find membership: {:?}", err);
            return error_page(&v, "Database error while searching for membership", None);
        }
    };

    if membership.is_none() {
        tracing::error!(
            "Access to team {} by unauthorized user: {:?}",
            team.name,
            user
        );
        return error_page(
            &v,
            "You are not authorized to cancel invitations for this team because you are not one of its administrators",
            None,
        );
    }

    let is_admin = if let Some(membership) = &membership {
        membership.role == "Owner" || membership.role == "Administrator"
    } else {
        false
    };

    if !is_admin {
        tracing::error!("Access to team {} by non-admin user: {:?}", team.name, user);
        return error_page(
            &v,
            "You are not authorized to cancel invitations for this team because you are not one of its administrators",
            None,
        );
    }

    // Find invitation by token and team ID
    let invitation = match team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::InvitationToken.eq(token.clone()))
        .filter(team_memberships::Column::Pending.eq(true))
        .one(&ctx.db)
        .await
    {
        Ok(value) => match value {
            Some(invitation) => invitation,
            None => return error_fragment(&v, "Invitation not found"),
        },
        Err(err) => {
            tracing::error!("Failed to find invitation: {:?}", err);
            return error_fragment(&v, "Database error while searching for invitation");
        }
    };

    tracing::info!(
        "Found invitation for user_id: {}, preparing to delete",
        invitation.user_id
    );

    // Cancel invitation - delete the membership
    let invitation_model: team_memberships::ActiveModel = invitation.into();
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

/// Update member role handler
#[debug_handler]
async fn update_member_role(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    Path((team_pid, user_pid)): Path<(String, String)>,
    Form(params): Form<UpdateRoleParams>,
) -> Result<impl IntoResponse> {
    if auth.user.is_none() {
        return htmx_redirect("/auth/login");
    }
    let current_user = auth.user.unwrap();
    let team = teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    let target_user = users::Model::find_by_pid(&ctx.db, &user_pid).await?;

    // Validate role
    if !crate::models::team_memberships::VALID_ROLES.contains(&params.role.as_str()) {
        return error_fragment(
            &v,
            &format!(
                "Invalid role. Valid roles are: {:?}",
                crate::models::team_memberships::VALID_ROLES
            ),
        );
    }

    // Check if current user is an owner of this team
    let is_owner = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(current_user.id))
        .filter(team_memberships::Column::Role.eq("Owner"))
        .filter(team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await?
        .is_some();

    if !is_owner {
        return error_fragment(&v, "Only team owners can update member roles");
    }

    // Get membership
    let membership = match team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(target_user.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await
    {
        Ok(value) => match value {
            Some(membership) => membership,
            None => return error_fragment(&v, "User is not a member of this team"),
        },
        Err(err) => {
            tracing::error!("Failed to find membership: {:?}", err);
            return error_fragment(&v, "Database error while searching for membership");
        }
    };

    // Cannot change owner's role if there's only one owner
    if membership.role == "Owner" && params.role != "Owner" {
        let owners_count = team_memberships::Entity::find()
            .filter(team_memberships::Column::TeamId.eq(team.id))
            .filter(team_memberships::Column::Role.eq("Owner"))
            .filter(team_memberships::Column::Pending.eq(false))
            .count(&ctx.db)
            .await?;

        if owners_count <= 1 {
            return error_fragment(&v, "Cannot change the role of the last owner");
        }
    }

    // Update role
    let mut membership_model: team_memberships::ActiveModel = membership.into();
    membership_model.role = sea_orm::ActiveValue::set(params.role);
    membership_model.update(&ctx.db).await?;

    // Return a response that refreshes the page
    let response = Response::builder()
        .header("HX-Refresh", "true")
        .body(axum::body::Body::empty())?;

    Ok(response)
}

/// Remove member handler
#[debug_handler]
async fn remove_member(
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<users::Model>,
    State(ctx): State<AppContext>,
    Path((team_pid, user_pid)): Path<(String, String)>,
) -> Result<Response> {
    if auth.user.is_none() {
        return htmx_redirect("/auth/login");
    }
    let current_user = auth.user.unwrap();
    let team = teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    let target_user = users::Model::find_by_pid(&ctx.db, &user_pid).await?;

    // Cannot remove yourself - use leave_team for that
    if current_user.id == target_user.id {
        return error_page(
            &v,
            "Cannot remove yourself from a team. Use leave_team instead.",
            None,
        );
    }

    // Get target user's membership
    let target_membership = match team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(target_user.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await
    {
        Ok(value) => match value {
            Some(membership) => membership,
            None => return error_page(&v, "User is not a member of this team", None),
        },
        Err(err) => {
            tracing::error!("Failed to find membership: {:?}", err);
            return error_page(&v, "Database error while searching for membership", None);
        }
    };

    // Check permissions
    let current_user_membership = match team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(current_user.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await
    {
        Ok(value) => match value {
            Some(membership) => membership,
            None => return error_page(&v, "You are not a member of this team", None),
        },
        Err(err) => {
            tracing::error!("Failed to find membership: {:?}", err);
            return error_page(&v, "Database error while searching for membership", None);
        }
    };

    let current_user_is_owner = current_user_membership.role == "Owner";
    let current_user_is_admin =
        current_user_membership.role == "Administrator" || current_user_is_owner;

    // Cannot remove an owner unless you're an owner
    if target_membership.role == "Owner" && !current_user_is_owner {
        return error_page(&v, "Only team owners can remove another owner", None);
    }

    // Cannot remove an admin unless you're an owner
    if target_membership.role == "Administrator" && !current_user_is_owner {
        return error_page(&v, "Only team owners can remove an administrator", None);
    }

    // Cannot remove a developer/observer unless you're an admin or owner
    if !current_user_is_admin {
        return error_page(
            &v,
            "Only team administrators and owners can remove members",
            None,
        );
    }

    // Special case: Prevent removing the last owner
    if target_membership.role == "Owner" {
        // Count owners
        let owner_count = team_memberships::Entity::find()
            .filter(team_memberships::Column::TeamId.eq(team.id))
            .filter(team_memberships::Column::Role.eq("Owner"))
            .filter(team_memberships::Column::Pending.eq(false))
            .count(&ctx.db)
            .await?;

        if owner_count <= 1 {
            return error_page(&v, "Cannot remove the last owner", None);
        }
    }

    // Remove the member
    let target_membership_model: team_memberships::ActiveModel = target_membership.into();
    target_membership_model.delete(&ctx.db).await?;

    // Return a response that refreshes the page
    let response = Response::builder()
        .header("HX-Refresh", "true")
        .body(axum::body::Body::empty())?;

    Ok(response)
}

/// Delete team handler
#[debug_handler]
async fn delete_team(
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<users::Model>,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    if auth.user.is_none() {
        return htmx_redirect("/auth/login");
    }
    let user = auth.user.unwrap();

    // Find team
    let team = match teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            return error_page(&v, "Team not found", None);
        }
    };

    // Check if user is an owner of this team
    let is_owner = team.has_role(&ctx.db, user.id, "Owner").await?;
    if !is_owner {
        return error_page(&v, "Only team owners can delete a team", None);
    }

    // Delete the team
    team.delete(&ctx.db).await?;

    // Instead of returning empty JSON, send a redirect to the teams list page
    let redirect_url = "/teams";
    htmx_redirect(redirect_url)
}

/// Invite member handler
#[debug_handler]
async fn invite_member_handler(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    Path(team_pid): Path<String>,
    Form(params): Form<InviteMemberParams>,
) -> Result<impl IntoResponse> {
    if auth.user.is_none() {
        return htmx_redirect("/auth/login");
    }
    let user = auth.user.unwrap();
    let team = match teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            return error_fragment(&v, "Team not found");
        }
    };

    // Check if user is an admin of this team
    let is_admin = team.has_role(&ctx.db, user.id, "Administrator").await?;

    if !is_admin {
        // Return error message with HTMX
        return error_fragment(&v, "Only team administrators can invite members");
    }

    // Find the target user by email
    let target_user = match users::Model::find_by_email(&ctx.db, &params.email).await {
        Ok(user) => user,
        Err(_) => {
            // If user doesn't exist, abort with an error
            return error_fragment(&v, &format!("No user found with e-mail {}", &params.email));
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
                params.email
            )
        } else {
            format!("User {} is already a member of this team", params.email)
        };
        return error_fragment(&v, &error_message);
    }

    // If we get here, the user exists but isn't a member,
    // so, proceed with inviting to the team

    // Create invitation entity
    /*let invitation =*/
    match team_memberships::Model::create_invitation(&ctx.db, team.id, &params.email).await {
        Ok(invit) => invit,
        Err(e) => {
            // Something terribly wrong happened, abort with an error message

            // First log the error
            let message = "Failed to create a team invitation entity";
            tracing::error!(
                error = e.to_string(),
                team_id = team.id,
                target_user = &params.email,
                message
            );

            // then return an error message with HTMX to the front-end
            let error_message = format!(
                "An internal error occured while creating the invitation {}",
                &e
            );
            return error_fragment(&v, &error_message);
        }
    };

    // Send notification e_mail to target user
    crate::mailers::team::TeamMailer::send_invitation(&ctx, &user, &target_user, &team).await?;

    // Redirect back to the team page
    let redirect_url = format!("/teams/{}", team_pid);
    htmx_redirect(&redirect_url)
}

/// Parameters for user search query
#[derive(Deserialize, Debug)]
pub struct SearchQuery {
    q: Option<String>,
}

/// Search users handler for auto-complete
#[debug_handler]
async fn search_users(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    Path(team_pid): Path<String>,
    Query(params): Query<SearchQuery>,
) -> Result<Response> {
    // Ensure user is authenticated
    if auth.user.is_none() {
        // Although the page itself requires login, this endpoint could be called directly
        // Returning an empty response is safer than redirecting
        return Ok(format::render().fragment(v, "", tera::Context::new()));
    }
    let current_user = auth.user.unwrap();

    let search_term = match params.q {
        Some(term) if !term.trim().is_empty() => term.trim().to_lowercase(),
        _ => return Ok(format::render().fragment(v, "", tera::Context::new())), // Return empty if query is empty or missing
    };

    // Find team
    let team = match teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            // Don't expose internal errors in fragment
            return Ok(format::render().fragment(v, "", tera::Context::new()));
        }
    };

    // Security check: Ensure current user is an admin/owner of the team
    let is_admin = match team.has_role(&ctx.db, current_user.id, "Administrator").await {
        Ok(admin) => admin,
        Err(e) => {
            tracing::error!("Failed to check role for user {} in team {}: {:?}", current_user.id, team.id, e);
            return Ok(format::render().fragment(v, "", tera::Context::new()));
        }
    };
    if !is_admin {
        tracing::warn!("Unauthorized user {} attempted to search users for team {}", current_user.pid, team.pid);
        return Ok(format::render().fragment(v, "", tera::Context::new()));
    }

    // Get IDs of users already associated with the team (members or pending)
    let existing_user_ids: Vec<i32> = team_memberships::Entity::find()
        .select_only()
        .column(team_memberships::Column::UserId)
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .into_tuple()
        .all(&ctx.db)
        .await?;

    // Search for users matching the query (name or email), excluding existing members/invitees
    let matching_users = users::Entity::find()
        .filter(
            Condition::any()
                .add(users::Column::Name.contains(&search_term))
                .add(users::Column::Email.contains(&search_term)),
        )
        .filter(users::Column::Id.is_not_in(existing_user_ids))
        .limit(10) // Limit results
        .all(&ctx.db)
        .await?;

    let mut context = tera::Context::new();
    context.insert("users", &matching_users);

    // Render the fragment template
    format::render().fragment(v, "teams/_user_search_results.html", context)
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
        .add("/{team_pid}/search-users", get(search_users))
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
