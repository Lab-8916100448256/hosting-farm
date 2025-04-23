use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use super::_entities::team_memberships::{self, ActiveModel, Entity, Model};
use super::_entities::teams;
use super::_entities::users;

#[derive(Debug, Serialize, Deserialize)]
pub struct InviteMemberParams {
    pub user_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRoleParams {
    pub role: String,
}

pub const VALID_ROLES: [&str; 4] = ["Owner", "Administrator", "Developer", "Observer"];

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
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
            .filter(
                model::query::condition()
                    .eq(team_memberships::Column::InvitationToken, token)
                    .build(),
            )
            .one(db)
            .await?;
        membership.ok_or_else(|| ModelError::EntityNotFound)
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
            .filter(
                model::query::condition()
                    .eq(team_memberships::Column::TeamId, team_id)
                    .eq(team_memberships::Column::UserId, user_id)
                    .eq(team_memberships::Column::Pending, false)
                    .build(),
            )
            .one(db)
            .await?;
        membership.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// Creates an invitation to join a team
    ///
    /// # Errors
    ///
    /// When could not save the membership into the DB or user not found
    pub async fn create_invitation(
        db: &DatabaseConnection,
        team_id: i32,
        user_name: &str,
    ) -> ModelResult<Self> {
        // Find user by name
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::Name, user_name)
                    .build(),
            )
            .one(db)
            .await?
            .ok_or_else(|| ModelError::msg("User not found"))?;

        // Check if already a member
        let existing = Entity::find()
            .filter(
                model::query::condition()
                    .eq(team_memberships::Column::TeamId, team_id)
                    .eq(team_memberships::Column::UserId, user.id)
                    .build(),
            )
            .one(db)
            .await?;

        if existing.is_some() {
            return Err(ModelError::msg("User is already a member of this team"));
        }

        // Create invitation
        let membership = ActiveModel {
            team_id: ActiveValue::set(team_id),
            user_id: ActiveValue::set(user.id),
            role: ActiveValue::set("Observer".to_string()), // Default role
            pending: ActiveValue::set(true),
            invitation_token: ActiveValue::set(Some(Uuid::new_v4().to_string())),
            invitation_sent_at: ActiveValue::set(Some(Utc::now().into())),
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
        membership.pending = ActiveValue::set(false);
        membership.invitation_token = ActiveValue::set(None);

        membership
            .update(db)
            .await
            .map_err(|e| ModelError::Any(e.into()))
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
            .map_err(|e| ModelError::Any(e.into()))?;
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
            return Err(ModelError::msg("Invalid role"));
        }

        let mut membership: ActiveModel = self.clone().into();
        membership.role = ActiveValue::set(new_role.to_string());

        membership
            .update(db)
            .await
            .map_err(|e| ModelError::Any(e.into()))
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
            .filter(
                model::query::condition()
                    .eq(team_memberships::Column::UserId, user_id)
                    .eq(team_memberships::Column::Pending, true)
                    .build(),
            )
            .find_with_related(teams::Entity)
            .all(db)
            .await?;

        let result = invitations
            .into_iter()
            .map(|(membership, team)| {
                if let Some(team) = team.into_iter().next() {
                    (membership, team)
                } else {
                    unreachable!("Team must exist for membership")
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
            .map_err(|e| ModelError::Any(e.into()))?;
        Ok(())
    }
}
