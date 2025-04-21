use async_trait::async_trait; // Import async_trait
use sea_orm::{
    ActiveModelTrait, ActiveValue::{self, Set, Unchanged}, ColumnTrait, ConnectionTrait, // REMOVED ActiveModelBehavior import
    DbErr, EntityTrait, QueryFilter, TransactionTrait, QuerySelect, Condition, JoinType, RelationTrait
};
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator}; // Import EnumIter and IntoEnumIterator
use tracing;
use validator::Validate; // Import Validate

use crate::models::{
    _entities::{team_memberships, teams, users},
    team_memberships::Model as TeamMembershipModel, // Import TeamMembershipModel for return type
    users::Model as UserModel, // Import UserModel
};

// Import ActiveModelBehavior for implementation (If needed, but trying without first)
pub use super::_entities::teams::{ActiveModel, Entity, Model as TeamEntityModel}; // Alias generated Model
use crate::models::ModelError; // Import ModelError

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, Serialize, Deserialize, strum::IntoEnumIterator)] // Add EnumIter and IntoEnumIterator
pub enum Role {
    Owner,
    Admin,
    Developer,
    Observer,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Owner => "Owner",
            Role::Admin => "Administrator",
            Role::Developer => "Developer",
            Role::Observer => "Observer",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Owner" => Some(Role::Owner),
            "Administrator" => Some(Role::Admin),
            "Developer" => Some(Role::Developer),
            "Observer" => Some(Role::Observer),
            _ => None,
        }
    }

    // Helper for permission checks (e.g., Admin implies Developer permissions)
    pub fn includes(&self, required_role: &Role) -> bool {
        match self {
            Role::Owner => true, // Owner includes all roles
            Role::Admin => required_role == &Role::Admin
                || required_role == &Role::Developer
                || required_role == &Role::Observer,
            Role::Developer => required_role == &Role::Developer || required_role == &Role::Observer,
            Role::Observer => required_role == &Role::Observer,
        }
    }
    // Helper to get all roles except Observer, used for team membership check
    pub fn member_roles() -> Vec<Role> {
        vec![Role::Owner, Role::Admin, Role::Developer]
    }

    // Helper to get all possible roles
    pub fn all_roles() -> Vec<Role> {
        // Use IntoEnumIterator trait provided by strum
        Role::iter().collect()
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// Implement FromStr for easy parsing in handlers
impl std::str::FromStr for Role {
    type Err = ModelError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Role::from_str(s).ok_or_else(|| {
            ModelError::Message(format!("Invalid role string: {}", s))
        })
    }
}

// Add derive(Validate)
#[derive(Debug, Deserialize, Validate, Serialize)]
pub struct CreateTeamParams {
    #[validate(length(min = 3, message = "Name must be at least 3 characters long."))]
    pub name: String,
    pub description: Option<String>,
}

// Add derive(Validate)
#[derive(Debug, Deserialize, Validate, Serialize)]
pub struct UpdateTeamParams {
    #[validate(length(min = 3, message = "Name must be at least 3 characters long."))]
    pub name: String,
    pub description: Option<String>,
}

// NOTE: DeriveEntityModel in _entities/teams.rs handles the basic ActiveModelBehavior

// Custom Model struct wrapping the entity Model
#[derive(Debug, Clone, Serialize, Deserialize)] // Added Serialize, Deserialize for passing to views
pub struct Model {
    #[serde(flatten)] // Flatten the inner struct fields for direct access in JSON/templates
    pub inner: TeamEntityModel,
}

// Implement Deref to easily access inner fields
impl std::ops::Deref for Model {
    type Target = TeamEntityModel;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// Implement From<TeamEntityModel> for Model
impl From<TeamEntityModel> for Model {
    fn from(inner: TeamEntityModel) -> Self {
        Self { inner }
    }
}

// Implement From<&TeamEntityModel> for Model to handle references
impl From<&TeamEntityModel> for Model {
    fn from(inner: &TeamEntityModel) -> Self {
        Self { inner: inner.clone() }
    }
}

impl Model {
    /// Finds a team by its PID.
    pub async fn find_by_pid(db: &impl ConnectionTrait, pid: &str) -> Result<Self, ModelError> {
        let parse_uuid = sea_orm::prelude::Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        let team = Entity::find()
            .filter(teams::Column::Pid.eq(parse_uuid))
            .one(db)
            .await?;
        team.map(Self::from)
            .ok_or_else(|| ModelError::EntityNotFound)
    }

    /// Creates a new team and assigns the creator as the owner.
    // Constrain db with TransactionTrait for begin/commit
    pub async fn create_team(
        db: &impl TransactionTrait,
        user_id: i32,
        params: &CreateTeamParams,
    ) -> Result<Self, ModelError> {
        let txn = db.begin().await?; // Use TransactionTrait's begin

        // Validate parameters using the derived Validate trait
        params
            .validate()
            .map_err(|e| ModelError::Validation(e.into()))?;

        // Check for name uniqueness
        let trimmed_name = params.name.trim();
        if Entity::find()
            .filter(teams::Column::Name.eq(trimmed_name))
            .one(&txn)
            .await?
            .is_some()
        {
            return Err(ModelError::Message(
                "A team with this name already exists.".to_string(),
            ));
        }

        // DeriveEntityModel's default behavior handles pid, created_at, updated_at
        let team_am = teams::ActiveModel {
            name: Set(trimmed_name.to_string()),
            description: Set(params.description.clone()),
            ..Default::default()
        };

        // Insert the team using the ActiveModel (before_save is called implicitly by derive)
        let team = team_am.insert(&txn).await?;

        // Add creator as Owner
        // DeriveEntityModel's default behavior handles pid, created_at, updated_at for memberships
        let membership_am = team_memberships::ActiveModel {
            user_id: Set(user_id),
            team_id: Set(team.id),
            role: Set(Role::Owner.as_str().to_string()),
            pending: Set(false),
            ..Default::default()
        };
        // Insert membership
        membership_am.insert(&txn).await?;

        txn.commit().await?;

        Ok(Self::from(team))
    }

    /// Updates team details (name, description).
    pub async fn update(
        &self,
        db: &impl ConnectionTrait,
        params: &UpdateTeamParams,
    ) -> Result<Self, ModelError> {
        let mut team_am: ActiveModel = self.inner.clone().into();

        // Validate parameters using derived Validate
        params
            .validate()
            .map_err(|e| ModelError::Validation(e.into()))?;

        let trimmed_name = params.name.trim();

        // Only update if values changed
        let mut changed = false;
        if team_am.name.as_ref() != trimmed_name {
             // Check for name uniqueness if name is changing
             if Entity::find()
                 .filter(teams::Column::Name.eq(trimmed_name))
                 .filter(teams::Column::Id.ne(self.id)) // Exclude self
                 .one(db)
                 .await?
                 .is_some()
             {
                 return Err(ModelError::Message(
                     "A team with this name already exists.".to_string(),
                 ));
             }
            team_am.name = Set(trimmed_name.to_string());
            changed = true;
        }

        // Correctly compare Option<String> using ActiveValue::as_ref() and Option::as_deref()
        if team_am.description.as_ref().as_deref() != params.description.as_deref() {
            team_am.description = Set(params.description.clone());
            changed = true;
        }

        if changed {
            // Update should work, before_save handles updated_at
            team_am.update(db).await.map(Self::from)
        } else {
            Ok(Self::from(self.inner.clone())) // Return self if no changes
        }
    }

    /// Deletes the team and all associated memberships.
    // Constrain db with TransactionTrait for begin/commit
    pub async fn delete(&self, db: &impl TransactionTrait) -> Result<(), ModelError> {
        let txn = db.begin().await?;
        // Delete memberships first
        team_memberships::Entity::delete_many()
            .filter(team_memberships::Column::TeamId.eq(self.id))
            .exec(&txn)
            .await?;
        // Delete the team
        Entity::delete_by_id(self.id).exec(&txn).await?;
        txn.commit().await?;
        Ok(())
    }

    /// Checks if a user has *at least* one of the specified roles within this team.
    pub async fn has_role(
        &self,
        db: &impl ConnectionTrait,
        user_id: i32,
        required_roles: Vec<Role>,
    ) -> Result<bool, ModelError> {
        let membership = team_memberships::Entity::find()
            .filter(team_memberships::Column::TeamId.eq(self.id))
            .filter(team_memberships::Column::UserId.eq(user_id))
            .filter(team_memberships::Column::Pending.eq(false)) // Ensure member is active
            .one(db)
            .await?;

        match membership {
            Some(m) => {
                if let Some(user_role) = Role::from_str(&m.role) {
                    // Check if the user's role includes any of the required roles
                    Ok(required_roles
                        .iter()
                        .any(|required| user_role.includes(required)))
                } else {
                    // Invalid role string in DB, treat as no permission
                    tracing::warn!("Invalid role '{}' found for user {} in team {}", m.role, user_id, self.id);
                    Ok(false)
                }
            }
            None => Ok(false), // User is not a member
        }
    }

     /// Gets all active members of the team along with their roles.
     pub async fn get_members(
         &self,
         db: &impl ConnectionTrait,
     ) -> Result<Vec<(UserModel, Role)>, ModelError> {
         self.get_members_internal(db, false).await
     }

     /// Gets all pending members (invitations) of the team.
     pub async fn get_pending_members(
         &self,
         db: &impl ConnectionTrait,
     ) -> Result<Vec<(UserModel, TeamMembershipModel)>, ModelError> { // Use TeamMembershipModel here
         self.get_members_internal_pending(db).await
     }

     // Internal helper to get members (active)
     async fn get_members_internal(
         &self,
         db: &impl ConnectionTrait,
         pending: bool,
     ) -> Result<Vec<(UserModel, Role)>, ModelError> {
         let memberships_with_users = team_memberships::Entity::find()
             .filter(team_memberships::Column::TeamId.eq(self.id))
             .filter(team_memberships::Column::Pending.eq(pending))
             .find_with_related(users::Entity)
             .all(db)
             .await?;

         let mut members_with_roles = Vec::new();
         for (membership, user_vec) in memberships_with_users {
             if let Some(user_entity) = user_vec.into_iter().next() {
                 if let Some(role) = Role::from_str(&membership.role) {
                     members_with_roles.push((UserModel::from(user_entity), role));
                 } else {
                     tracing::warn!(
                         "Invalid role string '{}' for user {} in team {}",
                         membership.role,
                         membership.user_id,
                         self.id
                     );
                 }
             } else {
                 tracing::warn!(
                     "Membership found (id: {}) but related user vector was empty or None.",
                     membership.id
                 );
             }
         }
         Ok(members_with_roles)
     }

     // Internal helper to get pending members with full membership info
     async fn get_members_internal_pending(
        &self,
        db: &impl ConnectionTrait,
    ) -> Result<Vec<(UserModel, TeamMembershipModel)>, ModelError> { // Use TeamMembershipModel
        let memberships_with_users = team_memberships::Entity::find()
            .filter(team_memberships::Column::TeamId.eq(self.id))
            .filter(team_memberships::Column::Pending.eq(true))
            .find_with_related(users::Entity)
            .all(db)
            .await?;

        let mut pending_members = Vec::new();
        for (membership, user_vec) in memberships_with_users {
            if let Some(user_entity) = user_vec.into_iter().next() {
                // Convert the entity membership to the custom model
                pending_members.push((UserModel::from(user_entity), TeamMembershipModel::from(membership)));
            } else {
                tracing::warn!(
                    "Pending Membership found (id: {}) but related user vector was empty or None.",
                    membership.id
                );
            }
        }
        Ok(pending_members)
    }

    /// Adds a member to the team or creates a pending invitation.
    pub async fn add_member(
        &self,
        db: &impl TransactionTrait, // Requires transaction
        user_id: i32,
        role: Role,
        pending: bool,
    ) -> Result<team_memberships::Model, ModelError> { // Return the created entity membership
        // Check if membership already exists (either pending or active)
        if team_memberships::Entity::find()
            .filter(team_memberships::Column::TeamId.eq(self.id))
            .filter(team_memberships::Column::UserId.eq(user_id))
            .one(db)
            .await?
            .is_some()
        {
            return Err(ModelError::EntityAlreadyExists);
        }

        let token = if pending {
            Some(sea_orm::prelude::Uuid::new_v4().to_string())
        } else {
            None
        };
        let sent_at = if pending { Some(chrono::Utc::now().into()) } else { None };

        // Assume team_memberships also uses DeriveEntityModel for defaults
        let membership_am = team_memberships::ActiveModel {
            user_id: Set(user_id),
            team_id: Set(self.id),
            role: Set(role.to_string()),
            pending: Set(pending),
            invitation_token: Set(token),
            invitation_sent_at: Set(sent_at),
            ..Default::default()
        };

        membership_am.insert(db).await
    }

    /// Checks if a given user is the last owner of the team.
    pub async fn is_last_owner(&self, db: &impl ConnectionTrait, user_id: i32) -> Result<bool, ModelError> {
        // Find the membership for the user in question
        let user_membership = team_memberships::Entity::find()
            .filter(team_memberships::Column::TeamId.eq(self.id))
            .filter(team_memberships::Column::UserId.eq(user_id))
            .filter(team_memberships::Column::Pending.eq(false))
            .one(db)
            .await?;

        // If the user is not an owner or not a member, they can't be the last owner
        if !user_membership.map_or(false, |m| m.role == Role::Owner.as_str()) {
            return Ok(false);
        }

        // Count how many *other* owners exist in the team
        let other_owners_count = team_memberships::Entity::find()
            .filter(team_memberships::Column::TeamId.eq(self.id))
            .filter(team_memberships::Column::UserId.ne(user_id)) // Exclude the user in question
            .filter(team_memberships::Column::Role.eq(Role::Owner.as_str()))
            .filter(team_memberships::Column::Pending.eq(false))
            .count(db)
            .await?;

        // If there are no other owners, this user is the last one
        Ok(other_owners_count == 0)
    }

}

// Implement IntoActiveModel for &Model to simplify updates
impl<'a> IntoActiveModel<ActiveModel> for &'a Model {
    fn into_active_model(self) -> ActiveModel {
        self.inner.clone().into()
    }
}
