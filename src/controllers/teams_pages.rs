use std::collections::HashMap;

use crate::{
    middleware::auth_no_error::JWTWithUserOpt,
    models::{
        _entities::{team_memberships, teams, users}, // Import entities
        ssh_keys,                                   // Import ssh_keys model if needed for users
        team_memberships::{ActiveModel as TeamMembershipActiveModel, Entity as TeamMembership}, // Import specific models
        teams::{
            self, CreateTeamParams, Entity as Team, Model as TeamModel, Role, UpdateTeamParams,
        }, // Import specific models
        users::{Model as UserModel, USER_STATUS_APPROVED}, // Import specific models and status
    },
    views::{
        error_fragment, error_page, redirect, render_template, // Import necessary view functions
        teams::{TeamMembershipsListResponse, UserSearchResultsResponse}, // Import specific response types
    },
};
use axum::{
    debug_handler,
    extract::{Form, Path, Query, State},
    http::{header::HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
};
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, Condition, DatabaseTransaction, EntityTrait,
    ModelTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait, JoinType, RelationTrait,
};
use serde::Deserialize;
use serde_json::json;
use loco_rs::config::ConfigExt; // For config .get_string()
use strum::IntoEnumIterator; // Import the trait needed for Role::iter()
use strum::IntoEnumIterator; // Import the trait needed for Role::iter()
use tracing::{error, info, warn};
use uuid::Uuid;

// Route parameter extraction structs
#[derive(Deserialize)]
pub struct TeamPathParams {
    pub team_pid: String,
}

#[derive(Deserialize)]
pub struct TeamMemberPathParams {
    pub team_pid: String,
    pub user_pid: String,
}

// Form payload structs
#[derive(Deserialize, Validate)]
pub struct InviteMemberPayload {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    // Role is handled separately or implicitly set
}

#[derive(Deserialize, Validate)]
pub struct UpdateRolePayload {
    pub role: String, // Expects a string representation of the Role enum
}

#[derive(Deserialize)]
pub struct UserSearchQuery {
    pub query: Option<String>,
}

#[derive(Deserialize)]
pub struct InvitationQuery {
    pub token: String,
}

// Helper functions
async fn get_admin_team_name(ctx: &AppContext) -> String {
    // Explicitly use the trait method
    ConfigExt::get_string(&ctx.config, "app.admin_team_name")
        .unwrap_or_else(|e| {
            if let config::Error::NotFound(_) = e {
                 tracing::debug!("'app.admin_team_name' not found in config. Using default 'Administrators'");
            } else {
                 tracing::warn!("Failed to read 'app.admin_team_name' from config ({}). Using default 'Administrators'", e);
            }
            "Administrators".to_string()
        })
}

// Ensure user has required role for the team
async fn ensure_team_role(
    ctx: &AppContext,
    user: &UserModel,
    team_pid: &str,
    required_roles: Vec<Role>,
) -> Result<TeamModel> {
    let team = TeamModel::find_by_pid(&ctx.db, team_pid).await?;
    if !team
        .has_role(&ctx.db, user.id, required_roles.clone())
        .await?
    {
        warn!(
            user_pid = %user.pid,
            team_pid = %team_pid,
            required_roles = ?required_roles,
            "User does not have required role for team action"
        );
        // Return a specific error type if needed, otherwise ModelError::Unauthorized
        return Err(Error::Unauthorized("User is not authorized to perform this action".to_string()));
    }
    Ok(team)
}

// Render the team list page
#[debug_handler]
async fn teams_list_page(
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<UserModel>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(user_pid = %user.pid, "Loading teams list page");

    // Fetch teams the user is a member of (excluding pending)
    let memberships = match team_memberships::Entity::find()
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .find_with_related(Team) // Use Team entity
        .order_by_asc(teams::Column::Name) // Order teams by name
        .all(&ctx.db)
        .await
    {
        Ok(m) => m,
        Err(e) => {
            error!(error = ?e, "Failed to load teams for user {}", user.pid);
            return error_page(&v, "Could not load your teams.", Some(Error::from(e)));
        }
    };

    let teams_json: Vec<_> = memberships
        .into_iter()
        .filter_map(|(_membership, team_vec)| {
            team_vec.into_iter().next().map(|team| {
                json!({
                    "pid": team.pid.to_string(),
                    "name": team.name,
                    "description": team.description.unwrap_or_default()
                })
            })
        })
        .collect();

    // Get pending invitations count
    let invitations_count = match team_memberships::Entity::find()
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(true))
        .count(&ctx.db)
        .await
    {
        Ok(count) => count,
        Err(e) => {
            error!(error = ?e, "Failed to load invitation count for user {}", user.pid);
            // Don't fail the whole page load, default to 0
            0
        }
    };

    render_template(
        &v,
        "teams/list.html",
        json!({
            "user": &user.inner,
            "teams": teams_json,
            "active_page": "teams",
            "invitation_count": invitations_count,
        }),
    )
}

// Render the new team page
#[debug_handler]
async fn new_team_page(
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<UserModel>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    // Get pending invitations count
    let invitations_count = match team_memberships::Entity::find()
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(true))
        .count(&ctx.db)
        .await
    {
        Ok(count) => count,
        Err(e) => {
            error!(error = ?e, "Failed to load invitation count for user {}", user.pid);
            0 // Default to 0 on error
        }
    };

    render_template(
        &v,
        "teams/new.html",
        json!({
            "user": &user.inner,
            "active_page": "teams",
            "invitation_count": invitations_count,
        }),
    )
}

// Handle new team creation form submission
#[debug_handler]
async fn create_team_handler(
    auth: JWTWithUserOpt<UserModel>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(params): Form<CreateTeamParams>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(user_pid = %user.pid, team_name = %params.name, "Attempting to create team");

    // Validate parameters
    if let Err(errors) = params.validate() {
        warn!(user_pid = %user.pid, validation_errors = ?errors, "Create team validation failed");
        // Fix validation error processing
        let error_message = errors.field_errors().iter().flat_map(|(_, errors)| errors.iter()).filter_map(|e| e.message.as_ref().map(|m| m.to_string())).collect::<Vec<_>>().join(" ");
        return error_fragment(&v, &error_message, "#new-team-error");
    }

    // Trim name
    let trimmed_params = CreateTeamParams {
        name: params.name.trim().to_string(),
        description: params.description.map(|d| d.trim().to_string()),
    };

    match TeamModel::create_team(&ctx.db, user.id, &trimmed_params).await {
        Ok(team) => {
            info!(user_pid = %user.pid, team_pid = %team.pid, "Team created successfully");
            // Redirect to the new team's detail page
            redirect(&format!("/teams/{}", team.pid), headers)
        }
        Err(e) => {
            error!(error = ?e, user_pid = %user.pid, "Failed to create team");
            let error_message = match e {
                 ModelError::Message(msg) => msg, // For uniqueness errors
                 _ => "Failed to create team due to an unexpected error.".to_string(),
            };
            error_fragment(&v, &error_message, "#new-team-error")
        }
    }
}

// Render the team details page
#[debug_handler]
async fn team_details(
    Path(TeamPathParams { team_pid }): Path<TeamPathParams>,
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<UserModel>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(user_pid = %user.pid, team_pid = %team_pid, "Loading team details page");

    // Fetch team and ensure user is a member (any role)
    let team = match ensure_team_role(
        &ctx,
        &user,
        &team_pid,
        Role::all_roles(), // Use helper for all roles
    )
    .await
    {
        Ok(team) => team,
        Err(Error::Model(ModelError::EntityNotFound)) => {
            return error_page(
                &v,
                "Team not found.",
                Some(Error::Model(ModelError::EntityNotFound)),
            );
        }
        Err(Error::Unauthorized(_)) => {
            return error_page(
                &v,
                "You are not authorized to view this team.",
                Some(Error::Model(ModelError::Unauthorized)),
            );
        }
        Err(e) => {
            error!(error = ?e, "Failed to load team {}", team_pid);
            return error_page(
                &v,
                "Could not load team details.",
                Some(Error::from(e)),
            );
        }
    };

    // Check if the current user is an Admin or Owner for this team
    let is_admin = team
        .has_role(&ctx.db, user.id, vec![Role::Admin, Role::Owner])
        .await?;

    // Fetch team members (excluding pending)
    let members = match team.get_members(&ctx.db).await {
        Ok(m) => m,
        Err(e) => {
            error!(error = ?e, "Failed to load members for team {}", team_pid);
            return error_page(
                &v,
                "Could not load team members.",
                Some(Error::from(e)),
            );
        }
    };

    // Fetch pending members (invitations)
    let pending_members = match team.get_pending_members(&ctx.db).await {
        Ok(p) => p,
        Err(e) => {
            error!(error = ?e, "Failed to load pending members for team {}", team_pid);
            return error_page(
                &v,
                "Could not load pending invitations.",
                Some(Error::from(e)),
            );
        }
    };

    // Get pending invitations count for the logged-in user (badge)
    let invitations_count = match team_memberships::Entity::find()
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(true))
        .count(&ctx.db)
        .await
    {
        Ok(count) => count,
        Err(e) => {
            error!(error = ?e, "Failed to load invitation count for user {}", user.pid);
            0 // Default to 0 on error
        }
    };

    // Get all possible roles for the dropdown
    let roles: Vec<String> = <Role as IntoEnumIterator>::iter().map(|r| r.to_string()).collect(); // Use iter() from IntoEnumIterator trait

    // Check if this team is the system admin team
    let admin_team_name = get_admin_team_name(&ctx).await;
    let is_system_admin_team = team.name == admin_team_name;

    render_template(
        &v,
        "teams/show.html",
        json!({
            "user": &user.inner,
            "team": &team.inner,
            "is_admin": is_admin,
            "is_system_admin_team": is_system_admin_team, // Pass flag to template
            "members": members,
            "pending_members": pending_members,
            "roles": roles,
            "active_page": "teams",
            "invitation_count": invitations_count,
        }),
    )
}

// Render the team edit page
#[debug_handler]
async fn edit_team_page(
    Path(TeamPathParams { team_pid }): Path<TeamPathParams>,
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<UserModel>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(user_pid = %user.pid, team_pid = %team_pid, "Loading team edit page");

    // Ensure user is Admin or Owner to edit
    let team = match ensure_team_role(&ctx, &user, &team_pid, vec![Role::Admin, Role::Owner]).await {
        Ok(team) => team,
        Err(Error::Model(ModelError::EntityNotFound)) => {
            return error_page(&v, "Team not found.", None);
        }
        Err(Error::Unauthorized(_)) => {
            return error_page(
                &v,
                "You are not authorized to edit this team.",
                None,
            );
        }
        Err(e) => {
            error!(error = ?e, "Failed to load team {} for editing", team_pid);
            return error_page(&v, "Could not load team for editing.", Some(e));
        }
    };

    // Check if this is the system admin team (cannot be renamed)
    let admin_team_name = get_admin_team_name(&ctx).await;
    let is_system_admin_team = team.name == admin_team_name;

    // Get pending invitations count
    let invitations_count = match team_memberships::Entity::find()
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(true))
        .count(&ctx.db)
        .await
    {
        Ok(count) => count,
        Err(e) => {
            error!(error = ?e, "Failed to load invitation count for user {}", user.pid);
            0 // Default to 0 on error
        }
    };

    render_template(
        &v,
        "teams/edit.html",
        json!({
            "user": &user.inner,
            "team": &team.inner,
            "is_system_admin_team": is_system_admin_team, // Pass flag to template
            "active_page": "teams",
            "invitation_count": invitations_count,
        }),
    )
}

// Handle team update form submission
#[debug_handler]
async fn update_team_handler(
    Path(TeamPathParams { team_pid }): Path<TeamPathParams>,
    auth: JWTWithUserOpt<UserModel>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(params): Form<UpdateTeamParams>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(user_pid = %user.pid, team_pid = %team_pid, new_name = %params.name, "Attempting to update team");

    // Ensure user is Admin or Owner to update
    let team = match ensure_team_role(&ctx, &user, &team_pid, vec![Role::Admin, Role::Owner]).await {
        Ok(team) => team,
        Err(Error::Model(ModelError::EntityNotFound)) => {
             // Render error within the form context if possible
            return error_fragment(&v, "Team not found.", "#edit-team-error");
        }
        Err(Error::Unauthorized(_)) => {
            return error_fragment(&v, "You are not authorized to edit this team.", "#edit-team-error");
        }
        Err(e) => {
            error!(error = ?e, "Failed to load team {} for update", team_pid);
            return error_fragment(&v, "Could not load team for editing.", "#edit-team-error");
        }
    };

    // Validate parameters
    if let Err(errors) = params.validate() {
        warn!(user_pid = %user.pid, validation_errors = ?errors, "Update team validation failed");
         // Fix validation error processing
        let error_message = errors.field_errors().iter().flat_map(|(_, errors)| errors.iter()).filter_map(|e| e.message.as_ref().map(|m| m.to_string())).collect::<Vec<_>>().join(" ");
        return error_fragment(&v, &error_message, "#edit-team-error");
    }

    // Trim inputs
    let mut trimmed_params = UpdateTeamParams {
        name: params.name.trim().to_string(),
        description: params.description.map(|d| d.trim().to_string()),
    };

    // Prevent renaming the administrators team
    let admin_team_name = get_admin_team_name(&ctx).await;
    if team.name == admin_team_name && team.name != trimmed_params.name {
        warn!(
            user_pid = %user.pid,
            team_pid = %team_pid,
            "Attempted to rename the administrators team"
        );
        return error_fragment(
            &v,
            "The administrators team cannot be renamed.",
            "#edit-team-error",
        );
    }

    match team.update(&ctx.db, &trimmed_params).await {
        Ok(updated_team) => {
            info!(user_pid = %user.pid, team_pid = %updated_team.pid, "Team updated successfully");
            // Redirect back to the team details page
            redirect(&format!("/teams/{}", updated_team.pid), headers)
        }
        Err(e) => {
            error!(error = ?e, user_pid = %user.pid, team_pid = %team_pid, "Failed to update team");
            let error_message = match e {
                 ModelError::Message(msg) => msg, // For uniqueness errors
                 _ => "Failed to update team due to an unexpected error.".to_string(),
            };
            error_fragment(&v, &error_message, "#edit-team-error")
        }
    }
}

// Handle team deletion
#[debug_handler]
async fn delete_team(
    Path(TeamPathParams { team_pid }): Path<TeamPathParams>,
    auth: JWTWithUserOpt<UserModel>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap, // Need headers for potential redirect
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(user_pid = %user.pid, team_pid = %team_pid, "Attempting to delete team");

    // Ensure user is Owner to delete
    let team = match ensure_team_role(&ctx, &user, &team_pid, vec![Role::Owner]).await {
        Ok(team) => team,
        Err(Error::Model(ModelError::EntityNotFound)) => {
            return Ok(StatusCode::NOT_FOUND.into_response()); // Return 404 Not Found
        }
        Err(Error::Unauthorized(_)) => {
            return Ok(StatusCode::FORBIDDEN.into_response()); // Return 403 Forbidden
        }
        Err(e) => {
            error!(error = ?e, "Failed to load team {} for deletion", team_pid);
            // Return a generic error response if loading fails
            return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
        }
    };

    // Prevent deletion of the administrators team
    let admin_team_name = get_admin_team_name(&ctx).await;
    if team.name == admin_team_name {
        warn!(
            user_pid = %user.pid,
            team_pid = %team_pid,
            "Attempted to delete the administrators team"
        );
        // Optionally return a message indicating failure, but likely just redirect back
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    match team.delete(&ctx.db).await {
        Ok(_) => {
            info!(user_pid = %user.pid, team_pid = %team_pid, "Team deleted successfully");
            // Send HX-Redirect header to navigate user to teams list
            let mut headers = HeaderMap::new();
            headers.insert("HX-Redirect", "/teams".parse().unwrap());
            Ok((StatusCode::OK, headers, Html("")).into_response()) // Empty body, redirect handled by HTMX
        }
        Err(e) => {
            error!(error = ?e, user_pid = %user.pid, team_pid = %team_pid, "Failed to delete team");
            // Return error message to be displayed (e.g., in a banner on the team page)
            // Ideally, this would be rendered into an error container on the page.
            // For now, return 500 status with a simple message.
            Ok((StatusCode::INTERNAL_SERVER_ERROR, Html("Failed to delete team.")).into_response())
        }
    }
}

// Render the invite member section (could be part of team details or a separate page/modal)
#[debug_handler]
async fn invite_member_form(
    Path(TeamPathParams { team_pid }): Path<TeamPathParams>,
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<UserModel>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    // Ensure user is Admin or Owner to invite
    let team = match ensure_team_role(&ctx, &user, &team_pid, vec![Role::Admin, Role::Owner]).await {
        Ok(team) => team,
        Err(_) => return Ok(StatusCode::FORBIDDEN.into_response()), // Simplified error handling for fragment
    };

    // Render just the invite form fragment
    render_template(
        &v,
        "teams/_invite_form.html", // Assuming template exists
        json!({ "team": &team.inner }),
    )
}

// Handle invite member form submission
#[debug_handler]
async fn invite_member_handler(
    Path(TeamPathParams { team_pid }): Path<TeamPathParams>,
    auth: JWTWithUserOpt<UserModel>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(params): Form<InviteMemberPayload>,
) -> Result<Response> {
    let inviter = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(inviter_pid = %inviter.pid, team_pid = %team_pid, invitee_email = %params.email, "Attempting to invite member");

    // Ensure inviter is Admin or Owner
    let team = match ensure_team_role(&ctx, &inviter, &team_pid, vec![Role::Admin, Role::Owner]).await {
        Ok(team) => team,
        Err(Error::Model(ModelError::EntityNotFound)) => {
            return error_fragment(&v, "Team not found.", "#invite-error");
        }
        Err(Error::Unauthorized(_)) => {
            return error_fragment(
                &v,
                "You are not authorized to invite members to this team.",
                "#invite-error",
            );
        }
        Err(e) => {
            error!(error = ?e, "Failed to load team {} for invite", team_pid);
            return error_fragment(&v, "Could not load team information.", "#invite-error");
        }
    };

    // Validate payload
    if let Err(errors) = params.validate() {
        warn!(inviter_pid = %inviter.pid, validation_errors = ?errors, "Invite validation failed");
         // Fix validation error processing
        let error_message = errors.field_errors().iter().flat_map(|(_, errors)| errors.iter()).filter_map(|e| e.message.as_ref().map(|m| m.to_string())).collect::<Vec<_>>().join(" ");
        return error_fragment(&v, &error_message, "#invite-error");
    }

    let target_email = params.email.trim().to_lowercase();

    // Find the user to invite by email
    let target_user_result = users::Entity::find()
        .filter(users::Column::Email.eq(&target_email))
        .one(&ctx.db)
        .await;

    let target_user_entity = match target_user_result {
        Ok(Some(user)) => user,
        Ok(None) => {
            warn!(inviter_pid = %inviter.pid, invitee_email = %target_email, "Invite failed: User not found");
            return error_fragment(&v, "User with this email not found.", "#invite-error");
        }
        Err(e) => {
            error!(error = ?e, "Database error looking up user {}", target_email);
            return error_fragment(
                &v,
                "Could not look up user due to a database error.",
                "#invite-error",
            );
        }
    };

    // Cannot invite yourself
    if target_user_entity.id == inviter.id {
        return error_fragment(&v, "You cannot invite yourself.", "#invite-error");
    }

    // Check if user is already a member or has a pending invite
    let existing_membership = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(target_user_entity.id))
        .one(&ctx.db)
        .await?;

    if let Some(membership) = existing_membership {
        if membership.pending {
            return error_fragment(
                &v,
                "An invitation is already pending for this user.",
                "#invite-error",
            );
        } else {
            return error_fragment(
                &v,
                "This user is already a member of the team.",
                "#invite-error",
            );
        }
    }

    // Proceed with invitation (create pending membership)
    // Use a transaction
    let txn = ctx.db.begin().await?;
    match team
        .add_member(&txn, target_user_entity.id, Role::Developer, true)
        .await // Start with Developer role, pending=true
    {
        Ok(membership_entity) => { // add_member returns the membership model
            // Send invitation email - Use the entity model for team
            // Fix type mismatch: convert target_user_entity (_entities::users::Model) to UserModel (models::users::Model)
            let target_user_model = UserModel::from(target_user_entity.clone());
            // Pass inviter: &UserModel, user: &UserModel, team: &TeamEntityModel
            match crate::mailers::team::TeamMailer::send_invitation(&ctx, &inviter, &target_user_model, &team.inner).await {
                 Ok(_) => {
                     info!(inviter_pid = %inviter.pid, team_pid = %team_pid, invitee_id = %target_user_entity.id, "Invitation sent successfully");
                     txn.commit().await?;
                     // Re-render the pending members list
                     let pending_members = team.get_pending_members(&ctx.db).await?;
                     let context = json!({
                         "pending_members": pending_members,
                         "team": &team.inner, // Needed for cancellation URLs
                         "is_admin": true // Assume inviter is admin to show cancel button
                     });
                     format::render().view(v, "teams/_pending_members_list.html", context)
                 }
                 Err(e) => {
                     error!(error = ?e, "Failed to send invitation email for user {}", target_user_entity.id);
                     txn.rollback().await?;
                     error_fragment(&v, "Invitation created, but failed to send email.", "#invite-error")
                 }
             }
        }
        Err(e) => {
            txn.rollback().await?;
            error!(error = ?e, user_pid = %inviter.pid, "Failed to add pending member");
            error_fragment(
                &v,
                "Failed to create invitation due to an unexpected error.",
                "#invite-error",
            )
        }
    }
}

// Handle user search for invitation autocomplete
#[debug_handler]
async fn search_users(
    Path(TeamPathParams { team_pid }): Path<TeamPathParams>,
    auth: JWTWithUserOpt<UserModel>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Query(UserSearchQuery { query }): Query<UserSearchQuery>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    // Ensure user is Admin or Owner of the team to search
    let _team = match ensure_team_role(&ctx, &user, &team_pid, vec![Role::Admin, Role::Owner]).await {
        Ok(team) => team,
        Err(_) => return Ok(StatusCode::FORBIDDEN.into_response()), // Simplified error handling
    };

    let search_term = query.unwrap_or_default();
    if search_term.len() < 2 {
        // Don't search if query is too short
        return Ok(Html("".to_string()).into_response());
    }

    // Find users matching the query by name or email, excluding self, only approved users
    let users_found = users::Entity::find()
        .filter(
            Condition::any()
                .add(users::Column::Name.contains(&search_term))
                .add(users::Column::Email.contains(&search_term)),
        )
        .filter(users::Column::Id.ne(user.id))
        .filter(users::Column::Status.eq(USER_STATUS_APPROVED)) // Only find approved users
        .limit(10) // Limit results
        .all(&ctx.db)
        .await?;

    let response = UserSearchResultsResponse { users: users_found };
    Ok(response.into_response())
}

// Handle accepting an invitation
#[debug_handler]
async fn accept_invitation(
    auth: JWTWithUserOpt<UserModel>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Query(query): Query<InvitationQuery>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(user_pid = %user.pid, token = %query.token, "Attempting to accept invitation");

    let txn = ctx.db.begin().await?;

    // Find the pending membership by token and user ID
    let membership_opt = team_memberships::Entity::find()
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::InvitationToken.eq(Some(query.token.clone())))
        .filter(team_memberships::Column::Pending.eq(true))
        .one(&txn)
        .await?;

    if let Some(membership) = membership_opt {
        let mut active_membership: TeamMembershipActiveModel = membership.into();
        active_membership.pending = Set(false);
        active_membership.invitation_token = Set(None); // Clear token
        active_membership.invitation_sent_at = Set(None); // Clear sent_at

        match active_membership.update(&txn).await {
            Ok(updated_membership) => { // update returns the updated model
                txn.commit().await?;
                info!(user_pid = %user.pid, token = %query.token, "Invitation accepted successfully");
                // Redirect to the team page
                // Need team pid from the membership record
                let team = Team::find_by_id(updated_membership.team_id) // Access team_id directly
                    .one(&ctx.db)
                    .await?
                    .ok_or(ModelError::EntityNotFound)?;
                redirect(&format!("/teams/{}", team.pid), headers)
            }
            Err(e) => {
                txn.rollback().await?;
                error!(error = ?e, "Failed to update membership for token {}", query.token);
                // Show error on invitations page?
                redirect_with_error(
                    "/users/invitations",
                    "Failed to accept invitation.",
                    headers,
                )
            }
        }
    } else {
        txn.rollback().await?;
        warn!(user_pid = %user.pid, token = %query.token, "Invalid or expired invitation token used");
        redirect_with_error(
            "/users/invitations",
            "Invalid or expired invitation link.",
            headers,
        )
    }
}

// Handle declining an invitation
#[debug_handler]
async fn decline_invitation(
    auth: JWTWithUserOpt<UserModel>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Query(query): Query<InvitationQuery>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(user_pid = %user.pid, token = %query.token, "Attempting to decline invitation");

    // Find and delete the pending membership
    let delete_result = team_memberships::Entity::delete_many()
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::InvitationToken.eq(Some(query.token.clone())))
        .filter(team_memberships::Column::Pending.eq(true))
        .exec(&ctx.db)
        .await?;

    if delete_result.rows_affected > 0 {
        info!(user_pid = %user.pid, token = %query.token, "Invitation declined successfully");
        // Redirect back to invitations page, maybe with success message?
        // Trigger HTMX refresh of the list
        let mut headers = HeaderMap::new();
        headers.insert("HX-Trigger", "updateInvitationCount".parse().unwrap());
        redirect("/users/invitations", headers)
    } else {
        warn!(user_pid = %user.pid, token = %query.token, "Invalid or expired invitation token for decline");
        redirect_with_error(
            "/users/invitations",
            "Could not decline invitation (invalid or expired link).",
            headers,
        )
    }
}

// Handle cancelling an invitation (by Admin/Owner)
#[debug_handler]
async fn cancel_invitation(
    Path(TeamMemberPathParams { team_pid, user_pid }) : Path<TeamMemberPathParams>,
    auth: JWTWithUserOpt<UserModel>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
     let admin_user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(admin_pid = %admin_user.pid, team_pid = %team_pid, target_user_pid = %user_pid, "Attempting to cancel invitation");

     // Ensure admin user is Admin or Owner of the team
    let team = match ensure_team_role(&ctx, &admin_user, &team_pid, vec![Role::Admin, Role::Owner]).await {
        Ok(team) => team,
        Err(_) => return Ok(StatusCode::FORBIDDEN.into_response()), // Simplified error handling for HTMX
    };

    // Find the target user by PID
    let target_user = match crate::models::users::Model::find_by_pid(&ctx.db, &user_pid).await {
        Ok(u) => u,
        Err(_) => {
            warn!(admin_pid = %admin_user.pid, target_user_pid = %user_pid, "Target user not found for invitation cancellation");
            // Don't leak user existence, just rerender list
            return render_updated_pending_list(&v, &ctx, &team).await;
        }
    };

    // Find and delete the pending membership
    let delete_result = team_memberships::Entity::delete_many()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(target_user.id))
        .filter(team_memberships::Column::Pending.eq(true))
        .exec(&ctx.db)
        .await?;

    if delete_result.rows_affected > 0 {
        info!(admin_pid = %admin_user.pid, target_user_pid = %user_pid, "Invitation cancelled successfully");
    } else {
        warn!(admin_pid = %admin_user.pid, target_user_pid = %user_pid, "No pending invitation found to cancel");
    }

    // Re-render the pending members list regardless of deletion result
    render_updated_pending_list(&v, &ctx, &team).await

}

// Handle removing a member from the team (by Admin/Owner)
#[debug_handler]
async fn remove_member(
    Path(TeamMemberPathParams { team_pid, user_pid }) : Path<TeamMemberPathParams>,
    auth: JWTWithUserOpt<UserModel>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let admin_user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(admin_pid = %admin_user.pid, team_pid = %team_pid, target_user_pid = %user_pid, "Attempting to remove member");

     // Ensure admin user is Admin or Owner of the team
    let team = match ensure_team_role(&ctx, &admin_user, &team_pid, vec![Role::Admin, Role::Owner]).await {
        Ok(team) => team,
        Err(_) => return Ok(StatusCode::FORBIDDEN.into_response()), // Simplified error handling for HTMX
    };

     // Find the target user by PID
    let target_user = match crate::models::users::Model::find_by_pid(&ctx.db, &user_pid).await {
        Ok(u) => u,
        Err(_) => {
            warn!(admin_pid = %admin_user.pid, target_user_pid = %user_pid, "Target user not found for removal");
             // Re-render member list to reflect potential error implicitly
             return render_updated_member_list(&v, &ctx, &team).await;
        }
    };

    // Prevent removing self
    if admin_user.id == target_user.id {
        warn!(admin_pid = %admin_user.pid, "Attempted to remove self from team {}", team_pid);
        // Cannot remove self, maybe return error fragment?
        // For now, just re-render list.
         return render_updated_member_list(&v, &ctx, &team).await;
    }

    // Prevent removing the last owner
    if team.is_last_owner(&ctx.db, target_user.id).await? {
        warn!(admin_pid = %admin_user.pid, target_user_pid = %user_pid, "Attempted to remove last owner from team {}", team_pid);
         return error_fragment(&v, "Cannot remove the last owner of the team.", "#member-list-error");
    }

    // Find and delete the (non-pending) membership
    let delete_result = team_memberships::Entity::delete_many()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(target_user.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .exec(&ctx.db)
        .await?;

    if delete_result.rows_affected > 0 {
        info!(admin_pid = %admin_user.pid, target_user_pid = %user_pid, "Member removed successfully from team {}", team_pid);
    } else {
        warn!(admin_pid = %admin_user.pid, target_user_pid = %user_pid, "Member not found or already removed from team {}", team_pid);
    }

     // Re-render the members list
    render_updated_member_list(&v, &ctx, &team).await
}

// Handle updating a member's role (by Admin/Owner)
#[debug_handler]
async fn update_member_role(
    Path(TeamMemberPathParams { team_pid, user_pid }) : Path<TeamMemberPathParams>,
    auth: JWTWithUserOpt<UserModel>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(params): Form<UpdateRolePayload>,
) -> Result<Response> {
    let admin_user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(admin_pid = %admin_user.pid, team_pid = %team_pid, target_user_pid = %user_pid, new_role=%params.role, "Attempting to update member role");

     // Ensure admin user is Owner of the team (only Owners can change roles)
    let team = match ensure_team_role(&ctx, &admin_user, &team_pid, vec![Role::Owner]).await {
        Ok(team) => team,
        Err(Error::Unauthorized(_)) => {
            // Render error message in the list container
            return error_fragment(&v, "Only Owners can change member roles.", "#member-list-error");
        }
        Err(_) => return Ok(StatusCode::FORBIDDEN.into_response()), // Other errors (e.g., team not found)
    };

    // Find the target user by PID
    let target_user = match crate::models::users::Model::find_by_pid(&ctx.db, &user_pid).await {
        Ok(u) => u,
        Err(_) => {
            warn!(admin_pid = %admin_user.pid, target_user_pid = %user_pid, "Target user not found for role update");
             // Re-render member list
             return render_updated_member_list(&v, &ctx, &team).await;
        }
    };

    // Parse the new role from the string payload
    let new_role = match Role::from_str(&params.role) { // Use Role::from_str
        Some(role) => role,
        None => {
            warn!(admin_pid = %admin_user.pid, invalid_role = %params.role, "Invalid role specified for update");
             return error_fragment(&v, "Invalid role selected.", "#member-list-error");
        }
    };

    // Cannot change own role
    if admin_user.id == target_user.id {
        warn!(admin_pid = %admin_user.pid, "Attempted to change own role in team {}", team_pid);
         return error_fragment(&v, "You cannot change your own role.", "#member-list-error");
    }

    // Prevent removing the last owner by changing their role
    if team.is_last_owner(&ctx.db, target_user.id).await? && new_role != Role::Owner {
         warn!(admin_pid = %admin_user.pid, target_user_pid = %user_pid, "Attempted to change role of last owner in team {}", team_pid);
         return error_fragment(&v, "Cannot change the role of the last owner.", "#member-list-error");
    }

    // Find the existing membership
    let membership_opt = team_memberships::Entity::find()
        .filter(team_memberships::Column::TeamId.eq(team.id))
        .filter(team_memberships::Column::UserId.eq(target_user.id))
        .filter(team_memberships::Column::Pending.eq(false)) // Only update active members
        .one(&ctx.db)
        .await?;

    if let Some(membership) = membership_opt {
        let mut active_membership: TeamMembershipActiveModel = membership.into();
        active_membership.role = Set(new_role.to_string()); // Set role as string

        match active_membership.update(&ctx.db).await {
            Ok(_) => {
                 info!(admin_pid = %admin_user.pid, target_user_pid = %user_pid, new_role=%new_role, "Member role updated successfully in team {}", team_pid);
            }
            Err(e) => {
                 error!(error = ?e, "Failed to update role for user {}", target_user.pid);
                  // Display error in the list container
                 return error_fragment(&v, "Failed to update role.", "#member-list-error");
            }
        }
    } else {
         warn!(admin_pid = %admin_user.pid, target_user_pid = %user_pid, "Membership not found for role update in team {}", team_pid);
         // Display error in the list container
         return error_fragment(&v, "Member not found.", "#member-list-error");
    }

     // Re-render the members list
    render_updated_member_list(&v, &ctx, &team).await
}

// Helper to re-render pending members list fragment
async fn render_updated_pending_list(
    v: &TeraView,
    ctx: &AppContext,
    team: &TeamModel,
) -> Result<Response> {
    let pending_members = team.get_pending_members(&ctx.db).await?;
    // Need to check if the current user is admin/owner for the cancel button
    // We don't have the current user here easily, assume true for now for simplicity
    // Ideally, pass the logged-in user or their admin status down.
    let is_admin = true; // Placeholder
    let context = json!({
        "pending_members": pending_members,
        "team": &team.inner,
        "is_admin": is_admin
    });
    format::render().view(v, "teams/_pending_members_list.html", context)
}

// Helper to re-render active members list fragment
async fn render_updated_member_list(
    v: &TeraView,
    ctx: &AppContext,
    team: &TeamModel,
) -> Result<Response> {
    let members = team.get_members(&ctx.db).await?;
    // Use Role::iter() from the IntoEnumIterator trait
    let roles: Vec<String> = <Role as IntoEnumIterator>::iter().map(|r| r.to_string()).collect();
    // Need to check if the current user is owner for role changes/removal
    // Assume true for simplicity placeholder.
    let is_admin = true; // Placeholder, should be owner check
    let context = json!({
        "members": members,
        "team": &team.inner,
        "roles": roles,
        "is_admin": is_admin
    });
    format::render().view(v, "teams/_members_list.html", context)
}

// Helper for redirection with error flash message (requires session middleware)
fn redirect_with_error(uri: &str, error_message: &str, headers: HeaderMap) -> Result<Response> {
    // This requires session middleware and flash message setup, which isn't fully implemented.
    // For now, just redirect. A better solution would involve query params or HTMX response.
    warn!("Redirecting with error (flash message not implemented): {}", error_message);
    redirect(uri, headers)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/teams")
        .add("/", get(teams_list_page)) // List user's teams
        .add("/new", get(new_team_page).post(create_team_handler)) // Create new team
        .add("/:team_pid", get(team_details)) // View team details
        .add("/:team_pid/edit", get(edit_team_page).post(update_team_handler)) // Edit team
        .add("/:team_pid", delete(delete_team)) // Delete team
        .add("/:team_pid/invite", post(invite_member_handler)) // Invite member (POST)
        .add("/:team_pid/search-users", get(search_users)) // Search users for invite
        .add("/invitations/accept", get(accept_invitation)) // Accept invitation (GET from link)
        .add("/invitations/decline", get(decline_invitation)) // Decline invitation (GET from link)
        .add("/:team_pid/members/:user_pid/cancel", post(cancel_invitation)) // Cancel invitation (Admin)
        .add("/:team_pid/members/:user_pid/remove", post(remove_member)) // Remove member (Admin)
        .add("/:team_pid/members/:user_pid/role", post(update_member_role)) // Update role (Owner)
}
