//! Team membership model implementation
use chrono::{Duration, Local};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, TransactionTrait};
use uuid::Uuid;

use crate::models::_entities::{team_memberships, users};
use crate::models::_entities::team_memberships::Role;
use crate::core::error::ModelError;
use crate::core::result::ModelResult;

/// Parameters for inviting a user to a team
#[derive(Debug)]
pub struct InviteParams {
    pub team_id: i32,
    pub email: String,
    pub role: Role,
}

/// Parameters for updating a team membership
#[derive(Debug)]
pub struct UpdateRoleParams {
    pub role: Role,
}

impl team_memberships::Model {
    /// Find a team membership by its pid
    ///
    /// # Errors
    ///
    /// Returns an error if the team membership is not found or if there's a database error
    pub async fn find_by_pid(db: &DatabaseConnection, pid: &Uuid) -> ModelResult<Self> {
        let membership = team_memberships::Entity::find()
            .filter(team_memberships::Column::Pid.eq(*pid))
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)?;
        Ok(membership)
    }

    /// Find a team membership by team ID and user ID
    ///
    /// # Errors
    ///
    /// Returns an error if the team membership is not found or if there's a database error
    pub async fn find_by_team_and_user(
        db: &DatabaseConnection,
        team_id: i32,
        user_id: i32,
    ) -> ModelResult<Self> {
        let membership = team_memberships::Entity::find()
            .filter(team_memberships::Column::TeamId.eq(team_id))
            .filter(team_memberships::Column::UserId.eq(user_id))
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)?;
        Ok(membership)
    }

    /// Find a team membership by invitation token
    ///
    /// # Errors
    ///
    /// Returns an error if the team membership is not found or if there's a database error
    pub async fn find_by_invitation_token(
        db: &DatabaseConnection,
        token: &str,
    ) -> ModelResult<Self> {
        let membership = team_memberships::Entity::find()
            .filter(team_memberships::Column::InvitationToken.eq(token))
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)?;
        Ok(membership)
    }

    /// Get the role of a user in a team
    ///
    /// # Errors
    ///
    /// Returns an error if the team membership is not found or if there's a database error
    pub async fn get_role(
        db: &DatabaseConnection,
        team_id: i32,
        user_id: i32,
    ) -> ModelResult<Role> {
        let membership = Self::find_by_team_and_user(db, team_id, user_id).await?;
        Ok(membership.role)
    }

    /// Check if a user is a member of a team
    ///
    /// # Errors
    ///
    /// Returns an error if there's a database error
    pub async fn is_member(
        db: &DatabaseConnection,
        team_id: i32,
        user_id: i32,
    ) -> ModelResult<bool> {
        let membership = team_memberships::Entity::find()
            .filter(team_memberships::Column::TeamId.eq(team_id))
            .filter(team_memberships::Column::UserId.eq(user_id))
            .one(db)
            .await?;
        
        Ok(membership.is_some())
    }

    /// Check if a user has a specific role in a team
    ///
    /// # Errors
    ///
    /// Returns an error if there's a database error
    pub async fn has_role(
        db: &DatabaseConnection,
        team_id: i32,
        user_id: i32,
        role: Role,
    ) -> ModelResult<bool> {
        let membership_result = team_memberships::Entity::find()
            .filter(team_memberships::Column::TeamId.eq(team_id))
            .filter(team_memberships::Column::UserId.eq(user_id))
            .one(db)
            .await?;
        
        if let Some(membership) = membership_result {
            Ok(membership.role == role)
        } else {
            Ok(false)
        }
    }

    /// Get all members of a team
    ///
    /// # Errors
    ///
    /// Returns an error if there's a database error
    pub async fn get_team_members(
        db: &DatabaseConnection,
        team_id: i32,
    ) -> ModelResult<Vec<Self>> {
        let memberships = team_memberships::Entity::find()
            .filter(team_memberships::Column::TeamId.eq(team_id))
            .all(db)
            .await?;
        
        Ok(memberships)
    }

    /// Invite a user to a team
    ///
    /// # Errors
    ///
    /// Returns an error if there's a database error
    pub async fn invite(db: &DatabaseConnection, params: InviteParams) -> ModelResult<Self> {
        // Check if user already exists
        let user_result = users::Entity::find()
            .filter(users::Column::Email.eq(&params.email))
            .one(db)
            .await?;
        
        let invitation_token = Uuid::new_v4().to_string();
        let expires_at = Local::now() + Duration::days(7);
        
        let membership = if let Some(user) = user_result {
            // User exists, create membership with user_id
            team_memberships::ActiveModel {
                pid: Set(Uuid::new_v4()),
                team_id: Set(params.team_id),
                user_id: Set(user.id),
                role: Set(params.role),
                invitation_token: Set(Some(invitation_token)),
                invitation_email: Set(None),
                invitation_expires_at: Set(Some(expires_at.into())),
                accepted_at: Set(None),
                ..Default::default()
            }
            .insert(db)
            .await?
        } else {
            // User doesn't exist, create membership with invitation_email
            team_memberships::ActiveModel {
                pid: Set(Uuid::new_v4()),
                team_id: Set(params.team_id),
                user_id: Set(0), // Placeholder, will be updated when user accepts
                role: Set(params.role),
                invitation_token: Set(Some(invitation_token)),
                invitation_email: Set(Some(params.email)),
                invitation_expires_at: Set(Some(expires_at.into())),
                accepted_at: Set(None),
                ..Default::default()
            }
            .insert(db)
            .await?
        };
        
        Ok(membership)
    }

    /// Accept an invitation
    ///
    /// # Errors
    ///
    /// Returns an error if there's a database error or if the invitation has expired
    pub async fn accept_invitation(
        &self,
        db: &DatabaseConnection,
        user_id: i32,
    ) -> ModelResult<Self> {
        // Check if invitation has expired
        if let Some(expires_at) = self.invitation_expires_at {
            if expires_at < Local::now().into() {
                return Err(ModelError::InvalidOperation("Invitation has expired".to_string()));
            }
        }
        
        let mut membership: team_memberships::ActiveModel = self.clone().into();
        membership.user_id = Set(user_id);
        membership.accepted_at = Set(Some(Local::now().into()));
        membership.invitation_token = Set(None);
        membership.invitation_expires_at = Set(None);
        
        let updated = membership.update(db).await?;
        Ok(updated)
    }

    /// Update the role of a team member
    ///
    /// # Errors
    ///
    /// Returns an error if there's a database error
    pub async fn update_role(
        &self,
        db: &DatabaseConnection,
        params: UpdateRoleParams,
    ) -> ModelResult<Self> {
        let mut membership: team_memberships::ActiveModel = self.clone().into();
        membership.role = Set(params.role);
        
        let updated = membership.update(db).await?;
        Ok(updated)
    }

    /// Remove a member from a team
    ///
    /// # Errors
    ///
    /// Returns an error if there's a database error
    pub async fn remove(&self, db: &DatabaseConnection) -> ModelResult<()> {
        let membership: team_memberships::ActiveModel = self.clone().into();
        membership.delete(db).await?;
        Ok(())
    }
}
