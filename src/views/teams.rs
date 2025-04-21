use loco_rs::prelude::*; // Keep prelude for potential future use
use serde::{Deserialize, Serialize};

// Use the custom model for teams
use crate::models::teams::{Model as TeamModel};
use crate::models::_entities::{teams::Model as TeamEntityModel, users::Model as UserModel}; // Import UserModel

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamResponse {
    pub pid: String,
    pub name: String,
    pub description: Option<String>,
}

// Implement From<&TeamModel> (custom model)
impl From<&TeamModel> for TeamResponse {
    fn from(team: &TeamModel) -> Self {
        Self {
            pid: team.pid.to_string(),
            name: team.name.clone(),
            description: team.description.clone(),
        }
    }
}

// Implement From<&TeamEntityModel> (entity model)
impl From<&TeamEntityModel> for TeamResponse {
    fn from(team: &TeamEntityModel) -> Self {
        Self {
            pid: team.pid.to_string(),
            name: team.name.clone(),
            description: team.description.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemberResponse {
    pub user_pid: String,
    pub name: String,
    pub email: String,
    pub role: String,
}

// Removed incorrect `impl From<&UserModel> for MemberResponse`
// The role information is not available on the UserModel alone.
// MemberResponse should be constructed directly in the controller
// where both the UserModel and the Role are available.
