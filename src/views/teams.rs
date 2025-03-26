//! Team views for HTML rendering
use axum::{
    extract::{Path, State, Extension},
    response::IntoResponse,
    routing::{get, post},
    Form, Router,
};
use loco_rs::prelude::*;
use loco_rs::view::render_template;
use loco_rs::controller::views::ViewEngine;
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
    Extension(view_engine): Extension<ViewEngine<engines::TeraView>>,
) -> Result<impl IntoResponse> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // Get all teams the user is a member of
    let user_teams = user.get_teams(&ctx.db).await?;
    
    let mut teams_data = Vec::new();
    for team in user_teams {
        // Get the user's role in this team
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
        &view_engine,
        "teams/list",
        &TeamListData { teams: teams_data },
    )
}

/// Render the new team form
#[axum::debug_handler]
async fn new_team_form(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Extension(view_engine): Extension<ViewEngine<engines::TeraView>>,
) -> Result<impl IntoResponse> {
    render_template(&view_engine, "teams/new", &())
}

/// Handle the new team form submission
#[axum::debug_handler]
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
#[axum::debug_handler]
async fn team_details(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Extension(view_engine): Extension<ViewEngine<engines::TeraView>>,
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
    
    render_template(&view_engine, "teams/details", &team_data)
}

/// Render the edit team form
#[axum::debug_handler]
async fn edit_team_form(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Extension(view_engine): Extension<ViewEngine<engines::TeraView>>,
    Path(team_id): Path<Uuid>,
) -> Result<impl IntoResponse> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    
    // Check if user is an owner of the team
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
    
    render_template(&view_engine, "teams/edit", &team_data)
}

/// Handle the edit team form submission
#[axum::debug_handler]
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
#[axum::debug_handler]
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
#[axum::debug_handler]
async fn team_members(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Extension(view_engine): Extension<ViewEngine<engines::TeraView>>,
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
    let memberships = team_memberships::Model::get_team_members(&ctx.db, team.id).await?;
    
    // Convert to template data format
    // This would need additional implementation to fetch user details
    let member_data = Vec::new(); // Placeholder
    
    render_template(
        &view_engine,
        "teams/members",
        &TeamMembersData {
            team: team_data,
            members: member_data,
        },
    )
}

/// Render the invite member form
#[axum::debug_handler]
async fn invite_member_form(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Extension(view_engine): Extension<ViewEngine<engines::TeraView>>,
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
    
    render_template(&view_engine, "teams/invite", &team_data)
}

/// Handle the invite member form submission
#[axum::debug_handler]
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
    
    // Invite the user
    team_memberships::Model::invite(
        &ctx.db,
        team_memberships::InviteParams {
            team_id: team.id,
            email: form.email,
            role: form.role,
        },
    ).await?;
    
    // Redirect to the team members page
    Ok(Redirect::to(&format!("/teams/{}/members", team.pid)))
}

/// Handle updating a team member's role
#[axum::debug_handler]
async fn update_member_role(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path((team_id, membership_id)): Path<(Uuid, Uuid)>,
    Form(form): Form<UpdateRoleForm>,
) -> Result<impl IntoResponse> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    
    // Check if user is an owner of the team
    if !team_memberships::Model::has_role(&ctx.db, team.id, user.id, Role::Owner).await? {
        return unauthorized("Only team owners can update member roles");
    }
    
    // Get the membership to update
    let membership = team_memberships::Model::find_by_pid(&ctx.db, &membership_id).await?;
    
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

/// Handle removing a team member
#[axum::debug_handler]
async fn remove_member(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path((team_id, membership_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    
    // Check if user is an owner of the team
    if !team_memberships::Model::has_role(&ctx.db, team.id, user.id, Role::Owner).await? {
        return unauthorized("Only team owners can remove members");
    }
    
    // Get the membership to remove
    let membership = team_memberships::Model::find_by_pid(&ctx.db, &membership_id).await?;
    
    // Remove the member
    membership.remove(&ctx.db).await?;
    
    // Redirect to the team members page
    Ok(Redirect::to(&format!("/teams/{}/members", team.pid)))
}

/// Register team routes
pub fn routes() -> Router {
    Router::new()
        .route("/teams", get(teams_list))
        .route("/teams/new", get(new_team_form))
        .route("/teams", post(create_team))
        .route("/teams/:team_id", get(team_details))
        .route("/teams/:team_id/edit", get(edit_team_form))
        .route("/teams/:team_id", post(update_team))
        .route("/teams/:team_id/delete", post(delete_team))
        .route("/teams/:team_id/members", get(team_members))
        .route("/teams/:team_id/invite", get(invite_member_form))
        .route("/teams/:team_id/invite", post(invite_member))
        .route("/teams/:team_id/members/:membership_id", post(update_member_role))
        .route("/teams/:team_id/members/:membership_id/remove", post(remove_member))
}
