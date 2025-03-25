//! Team views for HTML rendering
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
    Form, Router,
};
use loco_rs::{
    auth,
    controller::prelude::*,
    view::render_template,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{teams, team_memberships, users};
use crate::models::_entities::team_memberships::Role;

/// Form data for creating a new team
#[derive(Debug, Deserialize)]
pub struct CreateTeamForm {
    pub name: String,
    pub description: Option<String>,
}

/// Form data for updating a team
#[derive(Debug, Deserialize)]
pub struct UpdateTeamForm {
    pub name: String,
    pub description: Option<String>,
}

/// Form data for inviting a user to a team
#[derive(Debug, Deserialize)]
pub struct InviteForm {
    pub email: String,
    pub role: Role,
}

/// Form data for updating a team member's role
#[derive(Debug, Deserialize)]
pub struct UpdateRoleForm {
    pub role: Role,
}

/// Data for team list template
#[derive(Debug, Serialize)]
pub struct TeamListData {
    pub teams: Vec<TeamData>,
}

/// Data for team detail template
#[derive(Debug, Serialize)]
pub struct TeamData {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub slug: String,
    pub user_role: Role,
}

/// Data for team members template
#[derive(Debug, Serialize)]
pub struct TeamMembersData {
    pub team: TeamData,
    pub members: Vec<MemberData>,
}

/// Data for team member template
#[derive(Debug, Serialize)]
pub struct MemberData {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub role: Role,
    pub accepted: bool,
}

/// Render the teams list page
#[axum::debug_handler]
async fn teams_list(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
) -> Result<impl IntoResponse> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // Get all teams the user is a member of
    // This would need to be implemented in the users model
    let user_teams = user.get_teams(&ctx.db).await?;
    
    let mut teams_data = Vec::new();
    for team in user_teams {
        // Get the user's role in this team
        // This would need to be implemented in the team_memberships model
        let role = team_memberships::Model::get_role(&ctx.db, team.id, user.id).await?;
        
        teams_data.push(TeamData {
            id: team.pid,
            name: team.name,
            description: team.description,
            slug: team.slug,
            user_role: role,
        });
    }
    
    render_template(
        &ctx.view_engine,
        "teams/list",
        &TeamListData { teams: teams_data },
    )
}

/// Render the new team form
#[axum::debug_handler]
async fn new_team_form(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
) -> Result<impl IntoResponse> {
    render_template(&ctx.view_engine, "teams/new", &())
}

/// Handle the new team form submission
#[debug_handler]
async fn create_team(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Form(form): Form<CreateTeamForm>,
) -> Result<impl IntoResponse> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    let team = teams::Model::create(
        &ctx.db,
        teams::CreateParams {
            name: form.name,
            description: form.description,
            creator_id: user.id,
        },
    ).await?;
    
    // Redirect to the team details page
    Ok(Redirect::to(&format!("/teams/{}", team.pid)))
}

/// Render the team details page
#[debug_handler]
async fn team_details(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(team_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    
    // Check if user is a member of the team
    // This would need to be implemented in the team_memberships model
    if !team_memberships::Model::is_member(&ctx.db, team.id, user.id).await? {
        return unauthorized("You are not a member of this team");
    }
    
    // Get the user's role in this team
    let role = team_memberships::Model::get_role(&ctx.db, team.id, user.id).await?;
    
    let team_data = TeamData {
        id: team.pid,
        name: team.name,
        description: team.description,
        slug: team.slug,
        user_role: role,
    };
    
    render_template(&ctx.view_engine, "teams/details", &team_data)
}

/// Render the edit team form
#[debug_handler]
async fn edit_team_form(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(team_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    
    // Check if user is an owner of the team
    // This would need to be implemented in the team_memberships model
    if !team_memberships::Model::has_role(&ctx.db, team.id, user.id, Role::Owner).await? {
        return unauthorized("Only team owners can edit team details");
    }
    
    let team_data = TeamData {
        id: team.pid,
        name: team.name,
        description: team.description,
        slug: team.slug,
        user_role: Role::Owner,
    };
    
    render_template(&ctx.view_engine, "teams/edit", &team_data)
}

/// Handle the edit team form submission
#[debug_handler]
async fn update_team(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(team_id): Path<Uuid>,
    Form(form): Form<UpdateTeamForm>,
) -> Result<impl IntoResponse> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    
    // Check if user is an owner of the team
    if !team_memberships::Model::has_role(&ctx.db, team.id, user.id, Role::Owner).await? {
        return unauthorized("Only team owners can update team details");
    }
    
    team.update(
        &ctx.db,
        teams::UpdateParams {
            name: form.name,
            description: form.description,
        },
    ).await?;
    
    // Redirect to the team details page
    Ok(Redirect::to(&format!("/teams/{}", team.pid)))
}

/// Handle the delete team form submission
#[debug_handler]
async fn delete_team(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(team_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    
    // Check if user is an owner of the team
    if !team_memberships::Model::has_role(&ctx.db, team.id, user.id, Role::Owner).await? {
        return unauthorized("Only team owners can delete teams");
    }
    
    team.delete(&ctx.db).await?;
    
    // Redirect to the teams list page
    Ok(Redirect::to("/teams"))
}

/// Render the team members page
#[debug_handler]
async fn team_members(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(team_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    
    // Check if user is a member of the team
    if !team_memberships::Model::is_member(&ctx.db, team.id, user.id).await? {
        return unauthorized("You are not a member of this team");
    }
    
    // Get the user's role in this team
    let role = team_memberships::Model::get_role(&ctx.db, team.id, user.id).await?;
    
    let team_data = TeamData {
        id: team.pid,
        name: team.name,
        description: team.description,
        slug: team.slug,
        user_role: role,
    };
    
    // Get all team members
    // This would need to be implemented in the team_memberships model
    let memberships = team_memberships::Model::get_team_members(&ctx.db, team.id).await?;
    
    // Convert to template data format
    // This would need additional implementation to fetch user details
    let member_data = Vec::new(); // Placeholder
    
    render_template(
        &ctx.view_engine,
        "teams/members",
        &TeamMembersData {
            team: team_data,
            members: member_data,
        },
    )
}

/// Render the invite member form
#[debug_handler]
async fn invite_member_form(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(team_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    
    // Check if user is an owner or administrator of the team
    let role = team_memberships::Model::get_role(&ctx.db, team.id, user.id).await?;
    if role != Role::Owner && role != Role::Administrator {
        return unauthorized("Only team owners and administrators can invite members");
    }
    
    let team_data = TeamData {
        id: team.pid,
        name: team.name,
        description: team.description,
        slug: team.slug,
        user_role: role,
    };
    
    render_template(&ctx.view_engine, "teams/invite", &team_data)
}

/// Handle the invite member form submission
#[debug_handler]
async fn invite_member(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(team_id): Path<Uuid>,
    Form(form): Form<InviteForm>,
) -> Result<impl IntoResponse> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    
    // Check if user is an owner or administrator of the team
    let role = team_memberships::Model::get_role(&ctx.db, team.id, user.id).await?;
    if role != Role::Owner && role != Role::Administrator {
        return unauthorized("Only team owners and administrators can invite members");
    }
    
    // Create the invitation
    let membership = team_memberships::Model::invite(
        &ctx.db,
        team_memberships::InviteParams {
            team_id: team.id,
            email: form.email,
            role: form.role,
        },
    ).await?;
    
    // Send invitation email (would be implemented in a mailer)
    // TeamMailer::send_invitation(&ctx, &membership).await?;
    
    // Redirect to the team members page
    Ok(Redirect::to(&format!("/teams/{}/members", team.pid)))
}

/// Handle the accept invitation form submission
#[axum::debug_handler]
async fn accept_invitation(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(token): Path<String>,
) -> Result<impl IntoResponse> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // Find the invitation by token
    let membership = team_memberships::Model::find_by_invitation_token(&ctx.db, &token).await?;
    
    // Accept the invitation
    membership.accept_invitation(&ctx.db, user.id).await?;
    
    // Redirect to the teams list page
    Ok(Redirect::to("/teams"))
}

/// Handle the update member role form submission
#[debug_handler]
async fn update_member_role(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path((team_id, member_id)): Path<(Uuid, Uuid)>,
    Form(form): Form<UpdateRoleForm>,
) -> Result<impl IntoResponse> {
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
    if !can_change_role(user_role, target_user_role, form.role.clone()) {
        return unauthorized("You don't have permission to change this member's role");
    }
    
    // Update the role
    membership.update_role(
        &ctx.db,
        team_memberships::UpdateRoleParams {
            role: form.role,
        },
    ).await?;
    
    // Redirect to the team members page
    Ok(Redirect::to(&format!("/teams/{}/members", team.pid)))
}

/// Handle the remove member form submission
#[debug_handler]
async fn remove_member(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path((team_id, member_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse> {
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
    if !can_remove_member(user_role, target_user_role) {
        return unauthorized("You don't have permission to remove this member");
    }
    
    // Remove the member
    membership.remove(&ctx.db).await?;
    
    // Redirect to the team members page
    Ok(Redirect::to(&format!("/teams/{}/members", team.pid)))
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
        .prefix("/teams")
        .add("/", get(teams_list))
        .add("/new", get(new_team_form))
        .add("/", post(create_team))
        .add("/:id", get(team_details))
        .add("/:id/edit", get(edit_team_form))
        .add("/:id", post(update_team))
        .add("/:id/delete", post(delete_team))
        .add("/:id/members", get(team_members))
        .add("/:id/invite", get(invite_member_form))
        .add("/:id/invite", post(invite_member))
        .add("/:id/members/:member_id/role", post(update_member_role))
        .add("/:id/members/:member_id/remove", post(remove_member))
        .add("/invitations/:token/accept", post(accept_invitation))
}
