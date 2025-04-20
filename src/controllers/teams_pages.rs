use crate::{
    middleware::auth_no_error::JWTWithUserOpt,
    models::{
        _entities::{team_memberships, teams, users},
        team_memberships::{InviteMemberParams, UpdateRoleParams},
        teams::{CreateTeamParams, UpdateTeamParams},
    },
    views::render_template,
    views::{error_fragment, error_page, redirect},
};
use axum::{
    debug_handler,
    extract::{Form, Path, Query, State},
    http::header::HeaderMap,
    http::StatusCode,
    response::{Html, IntoResponse},
};
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QuerySelect,
}; // Added ActiveModelTrait
use serde::Deserialize;
use serde_json::json;
use tracing;

/// Helper function to get the admin team name from config
fn get_admin_team_name(ctx: &AppContext) -> String {
    ctx.config
        .settings
        .as_ref() // Get Option<&Value>
        .and_then(|settings| settings.get("app")) // Get Option<&Value> for "app" key
        .and_then(|app_settings| app_settings.get("admin_team_name")) // Get Option<&Value> for "admin_team_name"
        .and_then(|value| value.as_str()) // Get Option<&str>
        .map(|s| s.to_string()) // Convert to Option<String>
        .unwrap_or_else(|| {
            tracing::warn!(
                "'app.admin_team_name' not found or not a string in config, using default 'Administrators'"
            );
            "Administrators".to_string()
        })
}


/// Create team page
#[debug_handler]
async fn create_team_page(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    // Get pending invitations count
    let invitations_result = team_memberships::Entity::find()
        .find_with_related(teams::Entity)
        .all(&ctx.db)
        .await;

    let invitations = match invitations_result {
        Ok(data) => data
            .into_iter()
            .filter(|(membership, _)| membership.user_id == user.id && membership.pending)
            .count(),
        Err(e) => {
            tracing::error!(
                "Failed to load invitation count for user {}: {}",
                user.id,
                e
            );
            return error_page(
                &v,
                "Could not load your invitation count. Please try again later.",
                Some(e.into()),
            );
        }
    };

    render_template(
        &v,
        "teams/new.html",
        data!({
            "user": &user,
            "active_page": "teams",
            "invitation_count": &invitations,
        }),
    )
}

/// List teams page
#[debug_handler]
async fn list_teams(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    // Get all teams where the user is a member
    let teams_data_result = teams::Entity::find()
        .find_with_related(team_memberships::Entity)
        .all(&ctx.db)
        .await;

    let teams_data = match teams_data_result {
        Ok(data) => data,
        Err(e) => {
            tracing::error!("Failed to load teams for user {}: {}", user.id, e);
            return error_page(
                &v,
                "Could not load your team information. Please try again later.",
                Some(e.into()),
            );
        }
    };

    let teams_result = teams_data
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
    let invitations_result = team_memberships::Entity::find()
        .find_with_related(teams::Entity)
        .all(&ctx.db)
        .await;

    let invitations = match invitations_result {
        Ok(data) => data
            .into_iter()
            .filter(|(membership, _)| membership.user_id == user.id && membership.pending)
            .count(),
        Err(e) => {
            tracing::error!(
                "Failed to load invitation count for user {}: {}",
                user.id,
                e
            );
            return error_page(
                &v,
                "Could not load your invitation count. Please try again later.",
                Some(e.into()),
            );
        }
    };

    render_template(
        &v,
        "teams/list.html",
        data!({
            "user": &user,
            "teams": &teams_result,
            "active_page": "teams",
            "invitation_count": &invitations,
        }),
    )
}

/// Team details page
#[debug_handler]
async fn team_details(
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<users::Model>,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
    headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

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
    let membership_result = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await;

    let membership = match membership_result {
        Ok(membership) => membership,
        Err(e) => {
            tracing::error!(
                "Failed to load membership for user {} in team {}: {}",
                user.id,
                team.id,
                e
            );
            return error_page(
                &v,
                "Could not check your team membership. Please try again later.",
                Some(e.into()),
            );
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

    // Retrieve the configured administrator team name
    let admin_team_name = get_admin_team_name(&ctx);
    // Check if the current team is the administrators team
    let is_system_admin_team = team.name == admin_team_name;

    // Get team members (non-pending)
    let memberships_result = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .all(&ctx.db)
        .await;

    let memberships = match memberships_result {
        Ok(memberships) => memberships,
        Err(e) => {
            tracing::error!("Failed to load members for team {}: {}", team.id, e);
            return error_page(
                &v,
                "Could not load team members. Please try again later.",
                Some(e.into()),
            );
        }
    };

    let mut members = Vec::new();
    for membership in memberships {
        let member_result = users::Entity::find_by_id(membership.user_id)
            .one(&ctx.db)
            .await;

        let member = match member_result {
            Ok(member) => member,
            Err(e) => {
                tracing::error!("Failed to find user with id {}: {}", membership.user_id, e);
                return error_page(
                    &v,
                    "Could not load team member details. Please try again later.",
                    Some(e.into()),
                );
            }
        };

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
        let pending_memberships_result = team_memberships::Entity::find()
            .filter(team_memberships::Column::TeamId.eq(team.id))
            .filter(team_memberships::Column::Pending.eq(true))
            .all(&ctx.db)
            .await;

        let pending_memberships = match pending_memberships_result {
            Ok(memberships) => memberships,
            Err(e) => {
                tracing::error!("Failed to load pending members for team {}: {}", team.id, e);
                return error_page(
                    &v,
                    "Could not load pending team invitations. Please try again later.",
                    Some(e.into()),
                );
            }
        };

        for membership in pending_memberships {
            let member_result = users::Entity::find_by_id(membership.user_id)
                .one(&ctx.db)
                .await;

            let member = match member_result {
                Ok(member) => member,
                Err(e) => {
                    tracing::error!("Failed to find user with id {}: {}", membership.user_id, e);
                    return error_page(
                        &v,
                        "Could not load invited user details. Please try again later.",
                        Some(e.into()),
                    );
                }
            };

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
    let invitations_result = team_memberships::Entity::find()
        .find_with_related(teams::Entity)
        .all(&ctx.db)
        .await;

    let invitations = match invitations_result {
        Ok(data) => data
            .into_iter()
            .filter(|(membership, _)| membership.user_id == user.id && membership.pending)
            .count(),
        Err(e) => {
            tracing::error!(
                "Failed to load invitation count for user {}: {}",
                user.id,
                e
            );
            return error_page(
                &v,
                "Could not load your invitation count. Please try again later.",
                Some(e.into()),
            );
        }
    };

    render_template(
        &v,
        "teams/show.html",
        data!({
            "user": &user,
            "team": {
                "pid": team.pid.to_string(),
                "name": team.name,
                "description": team.description
            },
            "members": &members,
            "is_admin": &is_admin,
            "active_page": "teams",
            "invitation_count": &invitations,
            "is_system_admin_team": &is_system_admin_team // Add this line
        }),
    )
}

/// Invite member page
#[debug_handler]
async fn invite_member_page(
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<users::Model>,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
    headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    // Find team
    let team = match teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            return error_page(&v, "Team not found", None);
        }
    };

    // Check if user is an admin or owner of this team
    let membership_result = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await;

    let membership = match membership_result {
        Ok(membership) => membership,
        Err(e) => {
            tracing::error!(
                "Failed to load membership for user {} in team {}: {}",
                user.id,
                team.id,
                e
            );
            return error_page(
                &v,
                "Could not verify your permissions. Please try again later.",
                Some(e.into()),
            );
        }
    };

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
    let invitations_result = team_memberships::Entity::find()
        .find_with_related(teams::Entity)
        .all(&ctx.db)
        .await;

    let invitations = match invitations_result {
        Ok(data) => data
            .into_iter()
            .filter(|(membership, _)| membership.user_id == user.id && membership.pending)
            .count(),
        Err(e) => {
            tracing::error!(
                "Failed to load invitation count for user {}: {}",
                user.id,
                e
            );
            return error_page(
                &v,
                "Could not load your invitation count. Please try again later.",
                Some(e.into()),
            );
        }
    };

    render_template(
        &v,
        "teams/invite.html",
        data!({
            "user": &user,
            "team": {
                "pid": team.pid.to_string(),
                "name": team.name,
                "description": team.description
            },
            "active_page": "teams",
            "invitation_count": &invitations,
        }),
    )
}

/// Edit team page
#[debug_handler]
async fn edit_team_page(
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<users::Model>,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
    headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    // Find team
    let team = match teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            return error_page(&v, "Team not found", None);
        }
    };

    // Check if user is an admin or owner of this team
    let membership_result = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await;

    let membership = match membership_result {
        Ok(membership) => membership,
        Err(e) => {
            tracing::error!(
                "Failed to load membership for user {} in team {}: {}",
                user.id,
                team.id,
                e
            );
            return error_page(
                &v,
                "Could not verify your permissions. Please try again later.",
                Some(e.into()),
            );
        }
    };

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
    let invitations_result = team_memberships::Entity::find()
        .find_with_related(teams::Entity)
        .all(&ctx.db)
        .await;

    let invitations = match invitations_result {
        Ok(data) => data
            .into_iter()
            .filter(|(membership, _)| membership.user_id == user.id && membership.pending)
            .count(),
        Err(e) => {
            tracing::error!(
                "Failed to load invitation count for user {}: {}",
                user.id,
                e
            );
            return error_page(
                &v,
                "Could not load your invitation count. Please try again later.",
                Some(e.into()),
            );
        }
    };

    render_template(
        &v,
        "teams/edit.html",
        data!({
            "user": &user,
            "team": {
                "pid": team.pid.to_string(),
                "name": team.name,
                "description": team.description
            },
            "active_page": "teams",
            "invitation_count": &invitations,
        }),
    )
}

/// Create team handler
#[debug_handler]
async fn create_team_handler(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    headers: HeaderMap,
    Form(mut params): Form<CreateTeamParams>,
) -> Result<impl IntoResponse> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    // Trim whitespace from team name
    params.name = params.name.trim().to_string();

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
            tracing::error!("Failed to create team: {}", e);
            let error_message = match e {
                ModelError::Message(msg) => msg, // Use the message directly for uniqueness errors
                _ => "Failed to create team due to an unexpected error.".to_string(),
            };
            return error_fragment(&v, &error_message, "#error-container");
        }
    };

    // Verify team exists in database using find_by_pid for consistency
    match teams::Model::find_by_pid(&ctx.db, &team.pid.to_string()).await {
        Ok(_) => {
            tracing::info!("Team verification successful: {}", team.pid);
        }
        Err(e) => {
            tracing::error!("Team verification failed after creation: {}", e);
            return error_fragment(
                &v,
                &format!("Team verification failed after creation: {}", e),
                "#error-container",
            );
        }
    }

    // Redirect to the team details page with explicit .to_string() to ensure proper conversion
    let redirect_url = format!("/teams/{}", team.pid);
    redirect(&redirect_url, headers)
} // Added missing closing brace here


/// Update team handler
#[debug_handler]
async fn update_team_handler(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    Path(team_pid): Path<String>,
    headers: HeaderMap,
    Form(mut params): Form<UpdateTeamParams>,
) -> Result<impl IntoResponse> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    // Find team
    let team = match teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            return error_fragment(&v, "Team not found", "#error-container");
        }
    };

    // Check if user is an owner of this team
    let membership_result = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await;

    let membership = match membership_result {
        Ok(membership) => membership,
        Err(e) => {
            tracing::error!(
                "Failed to load membership for user {} in team {}: {}",
                user.id,
                team.id,
                e
            );
            return error_fragment(
                &v,
                "Could not verify your permissions. Please try again later.",
                "#error-container",
            );
        }
    };

    if let Some(membership) = membership {
        if membership.role != "Owner" {
            return error_fragment(
                &v,
                "Only team owners can edit team details",
                "#error-container",
            );
        }
    } else {
        return error_fragment(&v, "You are not a member of this team", "#error-container");
    }

    // Prevent renaming the admin team
    let admin_team_name = get_admin_team_name(&ctx);

    let incoming_name = params.name.trim(); // Trim incoming name for comparison
    if team.name == admin_team_name && incoming_name != team.name {
        // If it's the admin team AND the name is being changed, return an error fragment
        return error_fragment(&v, "The administrators team cannot be renamed.", "#error-container");
    }

    // Trim whitespace from team name before update
    params.name = params.name.trim().to_string();

    // Update the team
    let update_result = team.update(&ctx.db, &params).await;
    let updated_team = match update_result {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to update team {}: {}", team.id, e);
            let error_message = match e {
                ModelError::Message(msg) => msg, // Use message for uniqueness errors
                _ => "Could not update team details. Please try again later.".to_string(),
            };
            return error_fragment(&v, &error_message, "#error-container");
        }
    };

    // Redirect to the team details page
    let redirect_url = format!("/teams/{}", updated_team.pid);
    redirect(&redirect_url, headers)
}

/// Accept invitation handler
#[debug_handler]
async fn accept_invitation(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    Path(token): Path<String>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

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
    let update_result = invitation_model.update(&ctx.db).await; // Requires ActiveModelTrait
    if let Err(e) = update_result {
        tracing::error!("Failed to accept invitation : {}", e);
        return error_page(
            &v,
            "Could not accept the invitation. Please try again later.",
            Some(e.into()),
        );
    }

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
    headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

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
    let delete_result = invitation_model.delete(&ctx.db).await; // Requires ActiveModelTrait
    if let Err(e) = delete_result {
        tracing::error!("Failed to decline invitation: {}", e);
        return error_page(
            &v,
            "Could not decline the invitation. Please try again later.",
            Some(e.into()),
        );
    }

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
    headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    // Find team
    let team = match teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            return error_page(&v, "Team not found", None);
        }
    };

    // Check if user is an admin or owner of this team
    let membership_result = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await;

    let membership = match membership_result {
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
            None => return error_fragment(&v, "Invitation not found", "#error-container"),
        },
        Err(err) => {
            tracing::error!("Failed to find invitation: {:?}", err);
            return error_fragment(
                &v,
                "Database error while searching for invitation",
                "#error-container",
            );
        }
    };

    tracing::info!(
        "Found invitation for user_id: {}, preparing to delete",
        invitation.user_id
    );

    // Cancel invitation - delete the membership
    let invitation_model: team_memberships::ActiveModel = invitation.into();
    let delete_result = invitation_model.delete(&ctx.db).await; // Requires ActiveModelTrait
    if let Err(e) = delete_result {
        tracing::error!("Failed to cancel invitation: {}", e);
        return error_fragment(
            &v,
            "Could not cancel the invitation. Please try again later.",
            "#error-container",
        );
    }

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
    headers: HeaderMap,
    Form(params): Form<UpdateRoleParams>,
) -> Result<impl IntoResponse> {
    let current_user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    let team_result = teams::Model::find_by_pid(&ctx.db, &team_pid).await;
    let team = match team_result {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team {}: {}", team_pid, e);
            return error_fragment(&v, "Team not found.", "#error-container");
        }
    };
    let target_user_result = users::Model::find_by_pid(&ctx.db, &user_pid).await;
    let target_user = match target_user_result {
        Ok(user) => user,
        Err(e) => {
            tracing::error!("Failed to find user {}: {}", user_pid, e);
            return error_fragment(&v, "Target user not found.", "#error-container");
        }
    };

    // Validate role
    if !crate::models::team_memberships::VALID_ROLES.contains(&params.role.as_str()) {
        return error_fragment(
            &v,
            &format!(
                "Invalid role. Valid roles are: {:?}",
                crate::models::team_memberships::VALID_ROLES
            ),
            "#error-container",
        );
    }

    // Check if current user is an owner of this team
    let is_owner_result = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(current_user.id))
        .filter(team_memberships::Column::Role.eq("Owner"))
        .filter(team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await;

    let is_owner = match is_owner_result {
        Ok(membership_opt) => membership_opt.is_some(),
        Err(e) => {
            tracing::error!(
                "Failed to check ownership for user {} in team {}: {}",
                current_user.id,
                team.id,
                e
            );
            return error_fragment(
                &v,
                "Could not verify your permissions. Please try again later.",
                "#error-container",
            );
        }
    };

    if !is_owner {
        return error_fragment(
            &v,
            "Only team owners can update member roles",
            "#error-container",
        );
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
            None => {
                return error_fragment(&v, "User is not a member of this team", "#error-container")
            }
        },
        Err(err) => {
            tracing::error!("Failed to find membership: {:?}", err);
            return error_fragment(
                &v,
                "Database error while searching for membership",
                "#error-container",
            );
        }
    };

    // Cannot change owner's role if there's only one owner
    if membership.role == "Owner" && params.role != "Owner" {
        let owners_count_result = team_memberships::Entity::find()
            .filter(team_memberships::Column::TeamId.eq(team.id))
            .filter(team_memberships::Column::Role.eq("Owner"))
            .filter(team_memberships::Column::Pending.eq(false))
            .count(&ctx.db)
            .await;

        let owners_count = match owners_count_result {
            Ok(count) => count,
            Err(e) => {
                tracing::error!("Failed to count owners for team {}: {}", team.id, e);
                return error_fragment(
                    &v,
                    "Could not verify team ownership status. Please try again later.",
                    "#error-container",
                );
            }
        };

        if owners_count <= 1 {
            return error_fragment(
                &v,
                "Cannot change the role of the last owner",
                "#error-container",
            );
        }
    }

    // Update role
    let mut membership_model: team_memberships::ActiveModel = membership.into();
    membership_model.role = sea_orm::ActiveValue::set(params.role);
    let update_result = membership_model.update(&ctx.db).await; // Requires ActiveModelTrait
    if let Err(e) = update_result {
        tracing::error!(
            "Failed to update role for user {} in team {}: {}",
            target_user.id,
            team.id,
            e
        );
        return error_fragment(
            &v,
            "Could not update member role. Please try again later.",
            "#error-container",
        );
    };

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
    headers: HeaderMap,
) -> Result<Response> {
    let current_user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    let team_result = teams::Model::find_by_pid(&ctx.db, &team_pid).await;
    let team = match team_result {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team {}: {}", team_pid, e);
            return error_page(&v, "Team not found", None);
        }
    };
    let target_user_result = users::Model::find_by_pid(&ctx.db, &user_pid).await;
    let target_user = match target_user_result {
        Ok(user) => user,
        Err(e) => {
            tracing::error!("Failed to find user {}: {}", user_pid, e);
            return error_page(&v, "Target user not found", None);
        }
    };

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
        let owner_count_result = team_memberships::Entity::find()
            .filter(team_memberships::Column::TeamId.eq(team.id))
            .filter(team_memberships::Column::Role.eq("Owner"))
            .filter(team_memberships::Column::Pending.eq(false))
            .count(&ctx.db)
            .await;

        let owner_count = match owner_count_result {
            Ok(count) => count,
            Err(e) => {
                tracing::error!("Failed to count owners for team {}: {}", team.id, e);
                return error_page(
                    &v,
                    "Could not verify team ownership status. Please try again later.",
                    Some(e.into()),
                );
            }
        };

        if owner_count <= 1 {
            return error_page(&v, "Cannot remove the last owner", None);
        }
    }

    // Remove the member
    let target_membership_model: team_memberships::ActiveModel = target_membership.into();
    let delete_result = target_membership_model.delete(&ctx.db).await; // Requires ActiveModelTrait
    if let Err(e) = delete_result {
        tracing::error!(
            "Failed to remove user {} from team {}: {}",
            target_user.id,
            team.id,
            e
        );
        return error_page(
            &v,
            "Could not remove member. Please try again later.",
            Some(e.into()),
        );
    };

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
    headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    // Find team
    let team = match teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            return error_page(&v, "Team not found", None);
        }
    };

    // Check if user is an owner of this team
    let is_owner_result = team.has_role(&ctx.db, user.id, "Owner").await;
    let is_owner = match is_owner_result {
        Ok(is_owner) => is_owner,
        Err(e) => {
            tracing::error!(
                "Failed to check ownership for user {} in team {}: {}",
                user.id,
                team.id,
                e
            );
            return error_page(
                &v,
                "Could not verify your permissions. Please try again later.",
                Some(e.into()),
            );
        }
    };
    if !is_owner {
        return error_page(&v, "Only team owners can delete a team", None);
    }

    // Read admin team name from configuration using helper
    let admin_team_name = get_admin_team_name(&ctx);

    // Check if this is the admin team
    if team.name == admin_team_name {
        // If they match, return an error page
        return error_page(&v, "The administrators team cannot be deleted.", None);
    }

    // Delete the team
    let delete_result = team.delete(&ctx.db).await;
    if let Err(e) = delete_result {
        tracing::error!("Failed to delete team: {}", e);
        return error_page(
            &v,
            "Could not delete the team. Please try again later.",
            Some(e.into()),
        );
    };

    // Instead of returning empty JSON, send a redirect to the teams list page
    let redirect_url = "/teams";
    redirect(redirect_url, headers)
}

/// Invite member handler
#[debug_handler]
async fn invite_member_handler(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    Path(team_pid): Path<String>,
    headers: HeaderMap,
    Form(params): Form<InviteMemberParams>,
) -> Result<impl IntoResponse> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    let team = match teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            return error_fragment(&v, "Team not found", "#error-container");
        }
    };

    // Check if user is an admin of this team
    let is_admin_result = team.has_role(&ctx.db, user.id, "Administrator").await;
    let is_admin = match is_admin_result {
        Ok(is_admin) => is_admin,
        Err(e) => {
            tracing::error!(
                "Failed to check admin role for user {} in team {}: {}",
                user.id,
                team.id,
                e
            );
            return error_fragment(
                &v,
                "Could not verify your permissions. Please try again later.",
                "#error-container",
            );
        }
    };

    if !is_admin {
        // Return error message with HTMX
        return error_fragment(
            &v,
            "Only team administrators can invite members",
            "#error-container",
        );
    }

    // Find the target user by email
    let target_user = match users::Model::find_by_email(&ctx.db, &params.email).await {
        Ok(user) => user,
        Err(ModelError::EntityNotFound) => {
            return error_fragment(
                &v,
                &format!("No user found with e-mail {}", &params.email),
                "#error-container",
            );
        }
        Err(e) => {
            tracing::error!("Failed to find user by email: {}", e);
            return error_fragment(
                &v,
                &format!("Error searching for user with e-mail {}", &params.email),
                "#error-container",
            );
        }
    };

    // Check if user is already a member or has a pending invitation
    let existing_membership_result = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(target_user.id))
        .one(&ctx.db)
        .await;

    let existing_membership = match existing_membership_result {
        Ok(membership_opt) => membership_opt,
        Err(e) => {
            tracing::error!(
                "Failed to check existing membership for user {} in team {}: {}",
                target_user.id,
                team.id,
                e
            );
            return error_fragment(
                &v,
                "Could not check existing membership. Please try again later.",
                "#error-container",
            );
        }
    };

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
        return error_fragment(&v, &error_message, "#error-container");
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
            return error_fragment(&v, &error_message, "#error-container");
        }
    };

    // Send notification e_mail to target user
    let mailer_result =
        crate::mailers::team::TeamMailer::send_invitation(&ctx, &user, &target_user, &team).await;
    if let Err(e) = mailer_result {
        // Log the error but proceed with the redirect, as the invitation was created.
        // TODO: The error message in the UI might be confusing if the redirect happens anyway.
        // TODO: Consider if a different approach is needed, maybe showing a success message
        // with a warning about the email potentially not being sent.
        tracing::error!("Failed to send invitation email: {}", e);
        // Returning an error fragment here might prevent the redirect, which could be desired
        // depending on the required UX. For now, we'll just log and continue.
        // Optionally, return an error fragment:
        // TODO: return error_fragment(&v, "Invitation created, but failed to send email notification.", "#error-container");
    }

    // Redirect back to the team page
    let redirect_url = format!("/teams/{}", team_pid);
    redirect(&redirect_url, headers)
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
    tracing::info!("Searching for users with query {:?}", params.q);
    // Ensure user is authenticated
    let current_user = if let Some(user) = auth.user {
        user
    } else {
        // Return empty HTML for HTMX
        return Ok(Html("".to_string()).into_response());
    };

    let search_term = match params.q {
        Some(term) if !term.trim().is_empty() => term.trim().to_lowercase(),
        // Return empty HTML for HTMX if query is empty
        _ => {
            tracing::info!("Query is empty, returning empty HTML");
            return Ok(Html("".to_string()).into_response());
        }
    };

    // Find team
    let team = match teams::Model::find_by_pid(&ctx.db, &team_pid).await {
        Ok(team) => team,
        Err(e) => {
            tracing::error!("Failed to find team with pid {}: {:?}", team_pid, e);
            // Return empty HTML for HTMX on error
            return Ok(Html("".to_string()).into_response());
        }
    };

    // Security check: Ensure current user is an admin/owner of the team
    let is_admin = match team
        .has_role(&ctx.db, current_user.id, "Administrator")
        .await
    {
        Ok(admin) => admin,
        Err(e) => {
            tracing::error!(
                "Failed to check role for user {} in team {}: {:?}",
                current_user.id,
                team.id,
                e
            );
            // Return empty HTML for HTMX on error
            return Ok(Html("".to_string()).into_response());
        }
    };
    if !is_admin {
        tracing::warn!(
            "Unauthorized user {} attempted to search users for team {}",
            current_user.pid,
            team.pid
        );
        // Return empty HTML for HTMX if unauthorized
        return Ok(Html("".to_string()).into_response());
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

    // Create serializable context data
    let context_data = json!({ "users": &matching_users });

    // Render the fragment template using the view engine
    tracing::info!(
        "Rendering user search results with context data: {:?}",
        context_data
    );
    format::render().view(&v, "teams/_user_search_results.html", context_data)
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
