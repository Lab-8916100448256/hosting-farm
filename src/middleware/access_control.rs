//! Access control middleware for team-based permissions
use axum::{
    extract::{Path, State},
    http::Request,
    middleware::Next,
    response::Response,
};
use loco_rs::{
    auth,
    controller::prelude::*,
};
use uuid::Uuid;

use crate::models::{teams, team_memberships, users};
use crate::models::_entities::team_memberships::Role;

/// Middleware to check if a user is a member of a team
pub async fn require_team_member<B>(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(team_id): Path<Uuid>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    
    // Check if user is a member of the team
    if !team_memberships::Model::is_member(&ctx.db, team.id, user.id).await? {
        return unauthorized("You are not a member of this team");
    }
    
    Ok(next.run(request).await)
}

/// Middleware to check if a user has a specific role in a team
pub async fn require_team_role<B>(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(team_id): Path<Uuid>,
    request: Request<B>,
    next: Next<B>,
    required_role: Role,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    
    // Check if user has the required role in the team
    if !team_memberships::Model::has_role(&ctx.db, team.id, user.id, required_role).await? {
        return unauthorized("You don't have the required permissions for this action");
    }
    
    Ok(next.run(request).await)
}

/// Middleware to check if a user is an owner or administrator of a team
pub async fn require_team_admin<B>(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(team_id): Path<Uuid>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    
    // Get the user's role in the team
    let role = team_memberships::Model::get_role(&ctx.db, team.id, user.id).await?;
    
    // Check if user is an owner or administrator
    if role != Role::Owner && role != Role::Administrator {
        return unauthorized("Only team owners and administrators can perform this action");
    }
    
    Ok(next.run(request).await)
}

/// Middleware to check if a user is an owner of a team
pub async fn require_team_owner<B>(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Path(team_id): Path<Uuid>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    let team = teams::Model::find_by_pid(&ctx.db, &team_id).await?;
    
    // Check if user is an owner of the team
    if !team_memberships::Model::has_role(&ctx.db, team.id, user.id, Role::Owner).await? {
        return unauthorized("Only team owners can perform this action");
    }
    
    Ok(next.run(request).await)
}

/// Helper function to check if a user can change another user's role
pub fn can_change_role(user_role: Role, target_role: Role, new_role: Role) -> bool {
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

/// Helper function to check if a user can remove another user
pub fn can_remove_member(user_role: Role, target_role: Role) -> bool {
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
