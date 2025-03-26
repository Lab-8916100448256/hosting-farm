//! Team model implementation
use chrono::Local;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, TransactionTrait};
use uuid::Uuid;
use loco_rs::prelude::*;

use crate::models::_entities::{teams, team_memberships};
use crate::models::_entities::team_memberships::Role;

/// Parameters for creating a new team
#[derive(Debug)]
pub struct CreateParams {
    pub name: String,
    pub description: Option<String>,
    pub creator_id: i32,
}

/// Parameters for updating a team
#[derive(Debug)]
pub struct UpdateParams {
    pub name: String,
    pub description: Option<String>,
}

impl teams::Model {
    /// Find a team by its pid
    ///
    /// # Errors
    ///
    /// Returns an error if the team is not found or if there's a database error
    pub async fn find_by_pid(db: &DatabaseConnection, pid: &Uuid) -> ModelResult<Self> {
        let team = teams::Entity::find()
            .filter(teams::Column::Pid.eq(*pid))
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)?;
        Ok(team)
    }

    /// Find a team by its ID
    ///
    /// # Errors
    ///
    /// Returns an error if the team is not found or if there's a database error
    pub async fn find_by_id(db: &DatabaseConnection, id: i32) -> ModelResult<Self> {
        let team = teams::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)?;
        Ok(team)
    }

    /// Create a new team with the given parameters
    ///
    /// # Errors
    ///
    /// Returns an error if there's a database error
    pub async fn create(db: &DatabaseConnection, params: CreateParams) -> ModelResult<Self> {
        let txn = db.begin().await?;
        
        // Generate a slug from the team name
        let slug = params.name.to_lowercase().replace(' ', "-");
        
        // Create the team
        let team = teams::ActiveModel {
            pid: Set(Uuid::new_v4()),
            name: Set(params.name),
            description: Set(params.description),
            slug: Set(slug),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        
        // Add the creator as an owner
        let _membership = team_memberships::ActiveModel {
            pid: Set(Uuid::new_v4()),
            team_id: Set(team.id),
            user_id: Set(params.creator_id),
            role: Set(Role::Owner),
            invitation_token: Set(None),
            invitation_email: Set(None),
            invitation_expires_at: Set(None),
            accepted_at: Set(Some(Local::now().into())),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        
        txn.commit().await?;
        Ok(team)
    }

    /// Update a team with the given parameters
    ///
    /// # Errors
    ///
    /// Returns an error if there's a database error
    pub async fn update(&self, db: &DatabaseConnection, params: UpdateParams) -> ModelResult<Self> {
        let mut team: teams::ActiveModel = self.clone().into();
        team.name = Set(params.name);
        team.description = Set(params.description);
        let updated_team = team.update(db).await?;
        Ok(updated_team)
    }

    /// Delete a team
    ///
    /// # Errors
    ///
    /// Returns an error if there's a database error
    pub async fn delete(&self, db: &DatabaseConnection) -> ModelResult<()> {
        let txn = db.begin().await?;
        
        // Delete all team memberships first
        team_memberships::Entity::delete_many()
            .filter(team_memberships::Column::TeamId.eq(self.id))
            .exec(&txn)
            .await?;
        
        // Delete the team
        let team: teams::ActiveModel = self.clone().into();
        team.delete(&txn).await?;
        
        txn.commit().await?;
        Ok(())
    }
}
