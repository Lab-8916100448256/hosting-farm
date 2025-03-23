use async_trait::async_trait;
use chrono::{Duration, Local};
use sea_orm::{entity::*, query::*, DatabaseConnection, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{_entities::team_invitations, teams, users, ModelError, ModelResult};

pub use team_invitations::{ActiveModel, Column, Entity, Model, PrimaryKey, Relation};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateParams {
    pub team_id: i32,
    pub email: String,
    pub role: String,
    pub invited_by_id: i32,
}

#[async_trait]
pub trait TeamInvitationActions {
    async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Model>;
    async fn find_by_id(db: &DatabaseConnection, id: i32) -> ModelResult<Model>;
    async fn find_by_token(db: &DatabaseConnection, token: &str) -> ModelResult<Model>;
    async fn find_by_team(db: &DatabaseConnection, team_id: i32) -> ModelResult<Vec<Model>>;
    async fn find_by_email(db: &DatabaseConnection, email: &str) -> ModelResult<Vec<Model>>;
    async fn find_pending_by_email(db: &DatabaseConnection, email: &str) -> ModelResult<Vec<Model>>;
    async fn create(db: &DatabaseConnection, params: &CreateParams) -> ModelResult<Model>;
    async fn accept(&self, db: &DatabaseConnection, user_id: i32) -> ModelResult<()>;
    async fn reject(&self, db: &DatabaseConnection) -> ModelResult<Model>;
    async fn is_expired(&self) -> bool;
    async fn is_pending(&self) -> bool;
}

#[async_trait]
impl TeamInvitationActions for Model {
    async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Model> {
        let uuid = Uuid::parse_str(pid).map_err(|_| ModelError::EntityNotFound)?;
        let invitation = Entity::find()
            .filter(Column::Pid.eq(uuid))
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)?;
        Ok(invitation)
    }

    async fn find_by_id(db: &DatabaseConnection, id: i32) -> ModelResult<Model> {
        let invitation = Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)?;
        Ok(invitation)
    }

    async fn find_by_token(db: &DatabaseConnection, token: &str) -> ModelResult<Model> {
        let invitation = Entity::find()
            .filter(Column::Token.eq(token))
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)?;
        Ok(invitation)
    }

    async fn find_by_team(db: &DatabaseConnection, team_id: i32) -> ModelResult<Vec<Model>> {
        let invitations = Entity::find()
            .filter(Column::TeamId.eq(team_id))
            .all(db)
            .await?;
        Ok(invitations)
    }

    async fn find_by_email(db: &DatabaseConnection, email: &str) -> ModelResult<Vec<Model>> {
        let invitations = Entity::find()
            .filter(Column::Email.eq(email))
            .all(db)
            .await?;
        Ok(invitations)
    }

    async fn find_pending_by_email(db: &DatabaseConnection, email: &str) -> ModelResult<Vec<Model>> {
        let invitations = Entity::find()
            .filter(Column::Email.eq(email))
            .filter(Column::AcceptedAt.is_null())
            .filter(Column::RejectedAt.is_null())
            .all(db)
            .await?;
        
        // Filter out expired invitations
        let now = Local::now();
        let pending = invitations
            .into_iter()
            .filter(|inv| {
                if let Some(expires_at) = inv.expires_at {
                    expires_at > now.into()
                } else {
                    true
                }
            })
            .collect();
            
        Ok(pending)
    }

    async fn create(db: &DatabaseConnection, params: &CreateParams) -> ModelResult<Model> {
        // Verify team exists
        let _team = teams::Model::find_by_id(db, params.team_id).await?;
        
        // Verify inviter exists
        let _inviter = users::Model::find_by_id(db, params.invited_by_id).await?;
        
        // Generate token and set expiration (7 days from now)
        let token = Uuid::new_v4().to_string();
        let expires_at = Local::now() + Duration::days(7);
        
        // Create new invitation
        let invitation = ActiveModel {
            pid: Set(Uuid::new_v4()),
            team_id: Set(params.team_id),
            email: Set(params.email.clone()),
            role: Set(params.role.clone()),
            invited_by_id: Set(params.invited_by_id),
            token: Set(Some(token)),
            expires_at: Set(Some(expires_at.into())),
            ..Default::default()
        };

        let invitation = invitation.insert(db).await?;
        Ok(invitation)
    }

    async fn accept(&self, db: &DatabaseConnection, user_id: i32) -> ModelResult<()> {
        // Check if invitation is valid
        if !self.is_pending().await {
            return Err(ModelError::InvalidOperation("Invitation is not pending".into()));
        }
        
        if self.is_expired().await {
            return Err(ModelError::InvalidOperation("Invitation has expired".into()));
        }
        
        // Start transaction
        let txn = db.begin().await?;
        
        // Create team member
        let member_params = crate::models::team_members::CreateParams {
            team_id: self.team_id,
            user_id,
            role: self.role.clone(),
        };
        
        let _member = crate::models::team_members::Model::create(&txn, &member_params).await?;
        
        // Mark invitation as accepted
        let mut invitation: ActiveModel = self.clone().into();
        invitation.accepted_at = Set(Some(Local::now().into()));
        let _updated = invitation.update(&txn).await?;
        
        // Commit transaction
        txn.commit().await?;
        
        Ok(())
    }

    async fn reject(&self, db: &DatabaseConnection) -> ModelResult<Model> {
        // Check if invitation is valid
        if !self.is_pending().await {
            return Err(ModelError::InvalidOperation("Invitation is not pending".into()));
        }
        
        // Mark invitation as rejected
        let mut invitation: ActiveModel = self.clone().into();
        invitation.rejected_at = Set(Some(Local::now().into()));
        let updated = invitation.update(db).await?;
        
        Ok(updated)
    }

    async fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            expires_at < Local::now().into()
        } else {
            false
        }
    }

    async fn is_pending(&self) -> bool {
        self.accepted_at.is_none() && self.rejected_at.is_none()
    }
}
