use serde::{Deserialize, Serialize};

use crate::models::_entities::teams::Model as TeamModel;

#[derive(Debug, Serialize, Deserialize)]
pub struct TeamResponse {
    pub pid: String,
    pub name: String,
    pub description: Option<String>,
}

impl From<&TeamModel> for TeamResponse {
    fn from(team: &TeamModel) -> Self {
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