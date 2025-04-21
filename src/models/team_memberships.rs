use chrono::Utc;
use loco_rs::prelude::*; // Keep the prelude for other things
// Add explicit imports for types that seem missing or causing issues
use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DbErr,
    EntityTrait, ModelTrait, QueryFilter, RelationTrait,
};
// Import DatabaseConnection explicitly
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use super::_entities::team_memberships::{self, ActiveModel, Entity, Model};
use super::_entities::teams;
use super::_entities::users;

#[derive(Debug, Serialize, Deserialize)]
pub struct InviteMemberParams {
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRoleParams {
    pub role: String,
}

pub const VALID_ROLES: [&str; 4] = ["Owner", "Administrator", "Developer", "Observer"];

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    // Added DbErr explicitly
    async fn before_save<C>(self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait, // Added ConnectionTrait explicitly
    {
        if insert {
            let mut this = self;
            this.pid = ActiveValue::Set(Uuid::new_v4());
            Ok(this)
        } else {
            Ok(self)
        }
    }
}

impl Model {
    /// Finds a membership by the invitation token
    ///
    /// # Errors
    ///
    /// When could not find membership or DB query error
    pub async fn find_by_invitation_token(
        db: &DatabaseConnection,
        token: &str,
    ) -> ModelResult<Self> {
        let membership = Entity::find()
            .filter(team_memberships::Column::InvitationToken.eq(token)) // Simplified filter
            .one(db)
            .await?;
        membership.ok_or_else(|| ModelError::EntityNotFound) // Added ModelError explicitly
    }

    /// Finds a membership by team and user IDs
    ///
    /// # Errors
    ///
    /// When could not find membership or DB query error
    pub async fn find_by_team_and_user(
        db: &DatabaseConnection,
        team_id: i32,
        user_id: i32,
    ) -> ModelResult<Self> {
        let membership = Entity::find()
            .filter(team_memberships::Column::TeamId.eq(team_id))
            .filter(team_memberships::Column::UserId.eq(user_id))
            .filter(team_memberships::Column::Pending.eq(false))
            .one(db)
            .await?;
        membership.ok_or_else(|| ModelError::EntityNotFound) // Added ModelError explicitly
    }

    /// Creates an invitation to join a team
    ///
    /// # Errors
    ///
    /// When could not save the membership into the DB or user not found
    pub async fn create_invitation(
        db: &DatabaseConnection,
        team_id: i32,
        email: &str,
    ) -> ModelResult<Self> {
        // Find user by email
        let user = users::Entity::find()
            .filter(users::Column::Email.eq(email)) // Simplified filter
            .one(db)
            .await?
            .ok_or_else(|| ModelError::msg("User not found"))?; // Added ModelError explicitly

        // Check if already a member
        let existing = Entity::find()
            .filter(team_memberships::Column::TeamId.eq(team_id))
            .filter(team_memberships::Column::UserId.eq(user.id))
            .one(db)
            .await?;

        if existing.is_some() {
            return Err(ModelError::msg("User is already a member of this team")); // Added ModelError explicitly
        }

        // Create invitation
        let membership = ActiveModel {
            team_id: ActiveValue::Set(team_id),
            user_id: ActiveValue::Set(user.id),
            role: ActiveValue::Set("Observer".to_string()), // Default role
            pending: ActiveValue::Set(true),
            invitation_token: ActiveValue::Set(Some(Uuid::new_v4().to_string())),
            invitation_sent_at: ActiveValue::Set(Some(Utc::now().into())),
            ..Default::default()
        }
        .insert(db)
        .await?;

        Ok(membership)
    }

    /// Accepts an invitation to join a team
    ///
    /// # Errors
    ///
    /// When could not update the membership
    pub async fn accept_invitation(&self, db: &DatabaseConnection) -> ModelResult<Self> {
        let mut membership: ActiveModel = self.clone().into();
        membership.pending = ActiveValue::Set(false);
        membership.invitation_token = ActiveValue::Set(None);

        membership
            .update(db)
            .await
            .map_err(|e| ModelError::Any(e.into())) // Added ModelError explicitly
    }

    /// Declines an invitation to join a team by deleting the membership
    ///
    /// # Errors
    ///
    /// When could not delete the membership
    pub async fn decline_invitation(&self, db: &DatabaseConnection) -> ModelResult<()> {
        Entity::delete_by_id(self.id)
            .exec(db)
            .await
            .map_err(|e| ModelError::Any(e.into()))?; // Added ModelError explicitly
        Ok(())
    }

    /// Updates the role of a team member
    ///
    /// # Errors
    ///
    /// When could not update the membership or invalid role
    pub async fn update_role(&self, db: &DatabaseConnection, new_role: &str) -> ModelResult<Self> {
        // Validate role
        if !VALID_ROLES.contains(&new_role) {
            return Err(ModelError::msg("Invalid role")); // Added ModelError explicitly
        }

        let mut membership: ActiveModel = self.clone().into();
        membership.role = ActiveValue::Set(new_role.to_string());

        membership
            .update(db)
            .await
            .map_err(|e| ModelError::Any(e.into())) // Added ModelError explicitly
    }

    /// Gets all pending invitations for a user
    ///
    /// # Errors
    ///
    /// When DB query error
    pub async fn get_user_invitations(
        db: &DatabaseConnection,
        user_id: i32,
    ) -> ModelResult<Vec<(Self, teams::Model)>> {
        let invitations = Entity::find()
            .filter(team_memberships::Column::UserId.eq(user_id))
            .filter(team_memberships::Column::Pending.eq(true))
            .find_with_related(teams::Entity)
            .all(db)
            .await?;

        let result = invitations
            .into_iter()
            .map(|(membership, team)| {
                if let Some(team) = team.into_iter().next() {
                    (membership, team)
                } else {
                    // This case should ideally not happen if DB constraints are set up correctly
                    // Consider logging an error or handling it differently if it's possible
                    panic!("Database inconsistency: Team membership found without a corresponding team.");
                }
            })
            .collect();

        Ok(result)
    }

    /// Removes a user from a team by deleting the membership
    ///
    /// # Errors
    ///
    /// When could not delete the membership
    pub async fn remove_from_team(&self, db: &DatabaseConnection) -> ModelResult<()> {
        Entity::delete_by_id(self.id)
            .exec(db)
            .await
            .map_err(|e| ModelError::Any(e.into()))?; // Added ModelError explicitly
        Ok(())
    }

    /// Removes a user from all teams by deleting their memberships.
    ///
    /// # Errors
    ///
    /// Returns a `DbErr` if the database operation fails.
    // Replaced DbConn with DatabaseConnection
    pub async fn remove_user_from_all_teams(db: &DatabaseConnection, user_id: i32) -> Result<(), DbErr> {
        Entity::delete_many()
            .filter(team_memberships::Column::UserId.eq(user_id))
            .exec(db)
            .await?;
        Ok(())
    }
}
