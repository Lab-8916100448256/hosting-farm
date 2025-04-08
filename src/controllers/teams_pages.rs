use crate::{
    middleware::auth_no_error::JWTWithUserOpt,
    models::{
        _entities::{
            team_memberships::{
                self, ActiveModel as TeamMembershipActiveModel, Column as TeamMembershipColumn,
            },
            teams::{self, ActiveModel as TeamActiveModel, Column as TeamColumn},
            users,
        },
        team_memberships::{InviteMemberParams, UpdateRoleParams},
        teams::{CreateTeamParams, UpdateTeamParams},
    },
    utils::template::render_template,
    views::error_page,
};
use axum::{
    debug_handler,
    extract::{Form, Path, State},
    response::{IntoResponse, Redirect},
};
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
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

    render_template(&ctx, "teams/details.html", context)
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

    if membership.is_none() {
        tracing::error!(
            "Access to team {} by unauthorized user: {:?}",
            team.name,
            user
        );
        return error_page(
            &v,
            "You are not authorized to invite members to this team because you are not one of its administrators",
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
            "You are not authorized to invite members to this team because you are not one of its administrators",
            None,
        );
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

    if membership.is_none() {
        tracing::error!(
            "Access to team {} by unauthorized user: {:?}",
            team.name,
            user
        );
        return error_page(
            &v,
            "You are not authorized to edit this team because you are not one of its administrators",
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
            "You are not authorized to edit this team because you are not one of its administrators",
            None,
        );
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
    State(ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    Form(params): Form<CreateTeamParams>,
) -> Result<impl IntoResponse> {
    if auth.user.is_none() {
        return Ok(Redirect::to("/auth/login").into_response());
    }
    let user = auth.user.unwrap();

    let mut team = TeamActiveModel::new();
    team.name = sea_orm::Set(params.name);
    team.description = sea_orm::Set(params.description);
    let team = team.save(&ctx.db).await?;

    let mut membership = TeamMembershipActiveModel::new();
    membership.team_id = sea_orm::Set(team.id.unwrap());
    membership.user_id = sea_orm::Set(user.id);
    membership.role = sea_orm::Set("owner".to_string());
    membership.pending = sea_orm::Set(false);
    membership.save(&ctx.db).await?;

    Ok(Redirect::to(&format!("/teams/{}", team.pid.unwrap())).into_response())
}

/// Update team handler
#[debug_handler]
async fn update_team_handler(
    State(ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    Path(team_pid): Path<String>,
    Form(_params): Form<UpdateTeamParams>,
) -> Result<impl IntoResponse> {
    if auth.user.is_none() {
        return Ok(Redirect::to("/auth/login").into_response());
    }
    let user = auth.user.unwrap();

    let team = teams::Entity::find()
        .filter(TeamColumn::Pid.eq(&team_pid))
        .one(&ctx.db)
        .await?;

    if let Some(team) = team {
        let membership = team_memberships::Entity::find()
            .filter(TeamMembershipColumn::TeamId.eq(team.id))
            .filter(TeamMembershipColumn::UserId.eq(user.id))
            .one(&ctx.db)
            .await?;

        if let Some(membership) = membership {
            if membership.role != "owner" && membership.role != "admin" {
                return Ok(
                    Redirect::to(&format!("/teams/{}?error=Unauthorized", team_pid))
                        .into_response(),
                );
            }

            Ok(Redirect::to(&format!("/teams/{}", team_pid)).into_response())
        } else {
            Ok(Redirect::to(&format!("/teams/{}?error=Unauthorized", team_pid)).into_response())
        }
    } else {
        Ok(Redirect::to("/teams?error=Team not found").into_response())
    }
}

/// Accept invitation handler
#[debug_handler]
async fn accept_invitation(
    State(ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse> {
    if auth.user.is_none() {
        return Ok(Redirect::to("/auth/login").into_response());
    }
    let user = auth.user.unwrap();

    let membership = team_memberships::Entity::find()
        .filter(team_memberships::Column::InvitationToken.eq(&token))
        .one(&ctx.db)
        .await?;

    if let Some(membership) = membership {
        if membership.user_id != user.id {
            return Ok(Redirect::to("/teams?error=Invalid invitation").into_response());
        }

        let mut active_model: TeamMembershipActiveModel = membership.into();
        active_model.pending = sea_orm::Set(false);
        active_model.invitation_token = sea_orm::Set(None);
        let membership = active_model.update(&ctx.db).await?;

        let team = teams::Entity::find_by_id(membership.team_id)
            .one(&ctx.db)
            .await?;
        if let Some(team) = team {
            Ok(Redirect::to(&format!("/teams/{}", team.pid)).into_response())
        } else {
            Ok(Redirect::to("/teams?error=Team not found").into_response())
        }
    } else {
        tracing::error!("Invalid or expired invitation token: {}", &token);
        Ok(Redirect::to("/teams?error=Invalid or expired invitation").into_response())
    }
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
        return Ok(Redirect::to("/auth/login").into_response());
    }
    let user = auth.user.unwrap();

    // Find invitation
    let membership = team_memberships::Entity::find()
        .filter(team_memberships::Column::InvitationToken.eq(&token))
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(true))
        .one(&ctx.db)
        .await?;

    if membership.is_none() {
        tracing::error!("Invalid or expired invitation token: {}", token);
        return error_page(&v, "Invalid or expired invitation token", None);
    }

    let membership = membership.unwrap();

    // Delete membership
    membership.delete(&ctx.db).await?;

    // Redirect to teams list
    let response = Response::builder()
        .header("HX-Redirect", "/teams")
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

    // Find invitation
    let membership = team_memberships::Entity::find()
        .filter(team_memberships::Column::InvitationToken.eq(&token))
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::Pending.eq(true))
        .one(&ctx.db)
        .await?;

    if membership.is_none() {
        tracing::error!("Invalid or expired invitation token: {}", token);
        return error_page(&v, "Invalid or expired invitation token", None);
    }

    let membership = membership.unwrap();

    // Delete membership
    membership.delete(&ctx.db).await?;

    // Redirect to team details
    let response = Response::builder()
        .header("HX-Redirect", format!("/teams/{}", team.pid))
        .body(axum::body::Body::empty())?;
    Ok(response)
}

/// Update member role handler
#[debug_handler]
async fn update_member_role(
    State(ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    Path((team_pid, user_id)): Path<(String, i32)>,
    Form(params): Form<UpdateRoleParams>,
) -> Result<impl IntoResponse> {
    if auth.user.is_none() {
        return Ok(Redirect::to("/auth/login").into_response());
    }
    let _user = auth.user.unwrap();

    let team = teams::Entity::find()
        .filter(TeamColumn::Pid.eq(&team_pid))
        .one(&ctx.db)
        .await?;

    if let Some(team) = team {
        let membership = team_memberships::Entity::find()
            .filter(TeamMembershipColumn::TeamId.eq(team.id))
            .filter(TeamMembershipColumn::UserId.eq(user_id))
            .one(&ctx.db)
            .await?;

        if let Some(membership) = membership {
            let mut active_model: TeamMembershipActiveModel = membership.into();
            active_model.role = sea_orm::Set(params.role);
            active_model.update(&ctx.db).await?;
            Ok(Redirect::to(&format!("/teams/{}", team_pid)).into_response())
        } else {
            Ok(
                Redirect::to(&format!("/teams/{}?error=Member not found", team_pid))
                    .into_response(),
            )
        }
    } else {
        Ok(Redirect::to("/teams?error=Team not found").into_response())
    }
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

    if membership.is_none() {
        tracing::error!(
            "Access to team {} by unauthorized user: {:?}",
            team.name,
            user
        );
        return error_page(
            &v,
            "You are not authorized to remove members from this team because you are not one of its administrators",
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
            "You are not authorized to remove members from this team because you are not one of its administrators",
            None,
        );
    }

    // Find member
    let member = match users::Model::find_by_pid(&ctx.db, &user_pid).await {
        Ok(member) => member,
        Err(e) => {
            tracing::error!("Failed to find user with pid {}: {:?}", user_pid, e);
            return error_page(&v, "User not found", None);
        }
    };

    // Find member's membership
    let membership = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(member.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .one(&ctx.db)
        .await?;

    if membership.is_none() {
        tracing::error!("User {} is not a member of team {}", member.name, team.name);
        return error_page(&v, "User is not a member of this team", None);
    }

    let membership = membership.unwrap();

    // Delete membership
    membership.delete(&ctx.db).await?;

    // Redirect to team details
    let response = Response::builder()
        .header("HX-Redirect", format!("/teams/{}", team.pid))
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

    // Check if user is an owner of this team
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
            "You are not authorized to delete this team because you are not its owner",
            None,
        );
    }

    let is_owner = if let Some(membership) = &membership {
        membership.role == "Owner"
    } else {
        false
    };

    if !is_owner {
        tracing::error!("Access to team {} by non-owner user: {:?}", team.name, user);
        return error_page(
            &v,
            "You are not authorized to delete this team because you are not its owner",
            None,
        );
    }

    // Delete team
    team.delete(&ctx.db).await?;

    // Redirect to teams list
    let response = Response::builder()
        .header("HX-Redirect", "/teams")
        .body(axum::body::Body::empty())?;
    Ok(response)
}

/// Invite member handler
#[debug_handler]
async fn invite_member_handler(
    State(ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    Path(team_pid): Path<String>,
    Form(params): Form<InviteMemberParams>,
) -> Result<impl IntoResponse> {
    if auth.user.is_none() {
        return Ok(Redirect::to("/auth/login").into_response());
    }
    let user = auth.user.unwrap();

    let team = teams::Entity::find()
        .filter(TeamColumn::Pid.eq(&team_pid))
        .one(&ctx.db)
        .await?;

    if let Some(team) = team {
        let membership = team_memberships::Entity::find()
            .filter(TeamMembershipColumn::TeamId.eq(team.id))
            .filter(TeamMembershipColumn::UserId.eq(user.id))
            .one(&ctx.db)
            .await?;

        if let Some(membership) = membership {
            if membership.role != "owner" && membership.role != "admin" {
                return Ok(
                    Redirect::to(&format!("/teams/{}?error=Unauthorized", team_pid))
                        .into_response(),
                );
            }

            let invited_user = users::Entity::find()
                .filter(users::Column::Email.eq(&params.email))
                .one(&ctx.db)
                .await?;

            if let Some(invited_user) = invited_user {
                let membership = team_memberships::Entity::find()
                    .filter(TeamMembershipColumn::TeamId.eq(team.id))
                    .filter(TeamMembershipColumn::UserId.eq(invited_user.id))
                    .one(&ctx.db)
                    .await?;

                if membership.is_some() {
                    return Ok(Redirect::to(&format!(
                        "/teams/{}?error=User is already a member",
                        team_pid
                    ))
                    .into_response());
                }

                let mut new_membership = TeamMembershipActiveModel::new();
                new_membership.team_id = sea_orm::Set(team.id);
                new_membership.user_id = sea_orm::Set(invited_user.id);
                new_membership.role = sea_orm::Set("member".to_string());
                new_membership.pending = sea_orm::Set(false);
                new_membership.save(&ctx.db).await?;

                Ok(Redirect::to(&format!("/teams/{}", team_pid)).into_response())
            } else {
                Ok(
                    Redirect::to(&format!("/teams/{}?error=User not found", team_pid))
                        .into_response(),
                )
            }
        } else {
            Ok(Redirect::to(&format!("/teams/{}?error=Unauthorized", team_pid)).into_response())
        }
    } else {
        Ok(Redirect::to("/teams?error=Team not found").into_response())
    }
}

/// Team routes
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/teams")
        .add("/new", get(create_team_page))
        .add("", get(list_teams))
        .add("/{team_pid}", get(team_details))
        .add("/{team_pid}/invite", get(invite_member_page))
        .add("/{team_pid}/edit", get(edit_team_page))
        .add("", post(create_team_handler))
        .add("/{team_pid}", put(update_team_handler))
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
        .add("/{team_pid}", delete(delete_team))
        .add("/{team_pid}/invite", post(invite_member_handler))
}

pub async fn revoke_invitation(
    State(ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    Path((team_pid, token)): Path<(String, String)>,
) -> Result<impl IntoResponse> {
    if auth.user.is_none() {
        return Ok(Redirect::to("/auth/login").into_response());
    }
    let user = auth.user.unwrap();

    let team = teams::Entity::find()
        .filter(TeamColumn::Pid.eq(&team_pid))
        .one(&ctx.db)
        .await?;

    if let Some(team) = team {
        let membership = team_memberships::Entity::find()
            .filter(TeamMembershipColumn::TeamId.eq(team.id))
            .filter(TeamMembershipColumn::UserId.eq(user.id))
            .one(&ctx.db)
            .await?;

        if let Some(membership) = membership {
            if membership.role != "owner" && membership.role != "admin" {
                return Ok(
                    Redirect::to(&format!("/teams/{}?error=Unauthorized", team_pid))
                        .into_response(),
                );
            }

            let invitation = team_memberships::Entity::find()
                .filter(TeamMembershipColumn::InvitationToken.eq(&token))
                .one(&ctx.db)
                .await?;

            if let Some(invitation) = invitation {
                let mut active_model: TeamMembershipActiveModel = invitation.into();
                active_model.pending = sea_orm::Set(false);
                active_model.invitation_token = sea_orm::Set(None);
                active_model.update(&ctx.db).await?;
                Ok(Redirect::to(&format!("/teams/{}", team_pid)).into_response())
            } else {
                tracing::error!("Invalid or expired invitation token: {}", &token);
                Ok(
                    Redirect::to(&format!("/teams/{}?error=Invalid invitation", team_pid))
                        .into_response(),
                )
            }
        } else {
            Ok(Redirect::to(&format!("/teams/{}?error=Unauthorized", team_pid)).into_response())
        }
    } else {
        Ok(Redirect::to("/teams?error=Team not found").into_response())
    }
}

pub async fn reject_invitation(
    State(ctx): State<AppContext>,
    auth: JWTWithUserOpt<users::Model>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse> {
    if auth.user.is_none() {
        return Ok(Redirect::to("/auth/login").into_response());
    }
    let user = auth.user.unwrap();

    let membership = team_memberships::Entity::find()
        .filter(TeamMembershipColumn::InvitationToken.eq(&token))
        .one(&ctx.db)
        .await?;

    if let Some(membership) = membership {
        if membership.user_id != user.id {
            return Ok(Redirect::to("/teams?error=Invalid invitation").into_response());
        }

        let active_model: TeamMembershipActiveModel = membership.into();
        active_model.delete(&ctx.db).await?;
        Ok(Redirect::to("/teams").into_response())
    } else {
        tracing::error!("Invalid or expired invitation token: {}", &token);
        Ok(Redirect::to("/teams?error=Invalid or expired invitation").into_response())
    }
}
