#![allow(clippy::missing_errors_doc)]
#![allow(clippy::unnecessary_struct_initialization)]
#![allow(clippy::unused_async)]
use loco_rs::prelude::*;
use axum::{
    debug_handler,
    extract::State,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::models::{users, teams, team_members, team_invitations, auth};

#[derive(Debug, Serialize)]
struct UserProfileResponse {
    pid: String,
    email: String,
    name: String,
    email_verified: bool,
}

#[derive(Debug, Serialize)]
struct UserTeamResponse {
    pid: String,
    name: String,
    description: Option<String>,
    role: String,
    is_owner: bool,
}

#[derive(Debug, Deserialize)]
struct UpdateProfileParams {
    name: String,
}

#[debug_handler]
async fn profile(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    format::json(UserProfileResponse {
        pid: user.pid.to_string(),
        email: user.email.clone(),
        name: user.name.clone(),
        email_verified: user.email_verified_at.is_some(),
    })
}

#[debug_handler]
async fn update_profile(
    auth: auth::JWT,
    State(ctx): State<AppContext>,
    Json(params): Json<UpdateProfileParams>,
) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    let user = user
        .into_active_model()
        .update_profile(&ctx.db, &params.name)
        .await?;
    
    format::json(UserProfileResponse {
        pid: user.pid.to_string(),
        email: user.email.clone(),
        name: user.name.clone(),
        email_verified: user.email_verified_at.is_some(),
    })
}

#[debug_handler]
async fn user_teams(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    // Get all teams where user is a member
    let memberships = team_members::Model::find_by_user(&ctx.db, user.id).await?;
    
    let mut response = Vec::new();
    for membership in memberships {
        let team = teams::Model::find_by_id(&ctx.db, membership.team_id).await?;
        
        response.push(UserTeamResponse {
            pid: team.pid.to_string(),
            name: team.name.clone(),
            description: team.description.clone(),
            role: membership.role.clone(),
            is_owner: team.owner_id == user.id,
        });
    }
    
    format::json(response)
}

#[debug_handler]
async fn user_invitations(auth: auth::JWT, State(ctx): State<AppContext>) -> Result<Response> {
    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;
    
    let invitations = team_invitations::Model::find_pending_by_email(&ctx.db, &user.email).await?;
    
    let mut response = Vec::new();
    for invitation in invitations {
        let team = teams::Model::find_by_id(&ctx.db, invitation.team_id).await?;
        let inviter = users::Model::find_by_id(&ctx.db, invitation.invited_by_id).await?;
        
        response.push(serde_json::json!({
            "pid": invitation.pid.to_string(),
            "team": {
                "pid": team.pid.to_string(),
                "name": team.name,
            },
            "role": invitation.role,
            "invited_by": {
                "name": inviter.name,
                "email": inviter.email,
            },
            "created_at": invitation.created_at.to_string(),
        }));
    }
    
    format::json(response)
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("api/users")
        .add("/profile", get(profile))
        .add("/profile", put(update_profile))
        .add("/teams", get(user_teams))
        .add("/invitations", get(user_invitations))
}
