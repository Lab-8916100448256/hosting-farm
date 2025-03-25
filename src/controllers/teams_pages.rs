use axum::debug_handler;
use loco_rs::prelude::*;
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};
use crate::models::{users, teams, team_memberships::{self, Entity as TeamMembershipEntity, Column as TeamMembershipColumn}};

type JWT = loco_rs::controller::middleware::auth::JWT;

/// Renders the teams list page
#[debug_handler]
async fn index(
    auth: JWT,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // Get all teams where the user is a member
    let teams = teams::Entity::find()
        .find_with_related(team_memberships::Entity)
        .all(&ctx.db)
        .await?
        .into_iter()
        .filter_map(|(team, memberships)| {
            let is_member = memberships.iter().any(|m| m.user_id == user.id && !m.pending);
            if is_member {
                Some(tera::Context::from_serialize(json!({
                    "pid": team.pid.to_string(),
                    "name": team.name,
                    "description": team.description
                })).unwrap())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    
    let mut context = tera::Context::new();
    context.insert("user", &user);
    context.insert("teams", &teams);
    context.insert("active_page", "teams");
    
    format::render_template(&ctx, "teams/index.html.tera", context)
}

/// Renders the new team page
#[debug_handler]
async fn new(
    auth: JWT,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    let mut context = tera::Context::new();
    context.insert("user", &user);
    
    format::render_template(&ctx, "teams/new.html.tera", context)
}

/// Renders the team details page
#[debug_handler]
async fn show(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    
    // Check if user is a member of this team
    let has_access = team.has_role(&ctx.db, user.id, "Observer").await?;
    if !has_access {
        return unauthorized("You are not a member of this team");
    }
    
    // Check if user is an admin or owner
    let is_admin = team.has_role(&ctx.db, user.id, "Administrator").await?;
    
    // Get team members
    let memberships = TeamMembershipEntity::find()
        .filter(TeamMembershipColumn::TeamId.eq(team.id))
        .filter(TeamMembershipColumn::Pending.eq(false))
        .find_with_related(users::Entity)
        .all(&ctx.db)
        .await?;
    
    let members = memberships
        .into_iter()
        .filter_map(|(membership, users)| {
            if let Some(user) = users.first() {
                Some(tera::Context::from_serialize(json!({
                    "user_pid": user.pid.to_string(),
                    "name": user.name,
                    "email": user.email,
                    "role": membership.role
                })).unwrap())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    
    let mut context = tera::Context::new();
    context.insert("user", &user);
    context.insert("team", &tera::Context::from_serialize(json!({
        "pid": team.pid.to_string(),
        "name": team.name,
        "description": team.description
    })).unwrap());
    context.insert("members", &members);
    context.insert("is_admin", &is_admin);
    context.insert("active_page", "teams");
    
    format::render_template(&ctx, "teams/show.html.tera", context)
}

/// Renders the edit team page
#[debug_handler]
async fn edit(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    
    // Check if user is an owner of this team
    let is_owner = team.has_role(&ctx.db, user.id, "Owner").await?;
    if !is_owner {
        return unauthorized("Only team owners can edit team details");
    }
    
    let mut context = tera::Context::new();
    context.insert("user", &user);
    context.insert("team", &tera::Context::from_serialize(json!({
        "pid": team.pid.to_string(),
        "name": team.name,
        "description": team.description
    })).unwrap());
    
    format::render_template(&ctx, "teams/edit.html.tera", context)
}

/// Renders the invite team member page
#[debug_handler]
async fn invite(
    auth: JWT,
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_pid).await?;
    
    // Check if user is an admin of this team
    let is_admin = team.has_role(&ctx.db, user.id, "Administrator").await?;
    if !is_admin {
        return unauthorized("Only team administrators can invite members");
    }
    
    let mut context = tera::Context::new();
    context.insert("user", &user);
    context.insert("team", &tera::Context::from_serialize(json!({
        "pid": team.pid.to_string(),
        "name": team.name
    })).unwrap());
    
    format::render_template(&ctx, "teams/invite.html.tera", context)
}

/// Team page routes
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/teams")
        .add("/", get(index).middleware(auth::middleware::auth()))
        .add("/new", get(new).middleware(auth::middleware::auth()))
        .add("/:team_pid", get(show).middleware(auth::middleware::auth()))
        .add("/:team_pid/edit", get(edit).middleware(auth::middleware::auth()))
        .add("/:team_pid/invite", get(invite).middleware(auth::middleware::auth()))
} 