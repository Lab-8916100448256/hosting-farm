use async_trait::async_trait;
use sea_orm::{entity::*, query::*, DatabaseConnection, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{_entities::teams, users, ModelError, ModelResult};

pub use teams::{ActiveModel, Column, Entity, Model, PrimaryKey, Relation};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateParams {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateParams {
    pub name: String,
    pub description: Option<String>,
}

#[async_trait]
pub trait TeamActions {
    async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Model>;
    async fn find_by_id(db: &DatabaseConnection, id: i32) -> ModelResult<Model>;
    async fn find_all(db: &DatabaseConnection) -> ModelResult<Vec<Model>>;
    async fn find_by_owner(db: &DatabaseConnection, owner_id: i32) -> ModelResult<Vec<Model>>;
    async fn find_by_user(db: &DatabaseConnection, user_id: i32) -> ModelResult<Vec<Model>>;
    async fn create(db: &DatabaseConnection, owner_id: i32, params: &CreateParams) -> ModelResult<Model>;
    async fn update(&self, db: &DatabaseConnection, params: &UpdateParams) -> ModelResult<Model>;
    async fn delete(&self, db: &DatabaseConnection) -> ModelResult<()>;
}

#[async_trait]
impl TeamActions for Model {
    async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Model> {
        let uuid = Uuid::parse_str(pid).map_err(|_| ModelError::EntityNotFound)?;
        let team = Entity::find()
            .filter(Column::Pid.eq(uuid))
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)?;
        Ok(team)
    }

    async fn find_by_id(db: &DatabaseConnection, id: i32) -> ModelResult<Model> {
        let team = Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)?;
        Ok(team)
    }

    async fn find_all(db: &DatabaseConnection) -> ModelResult<Vec<Model>> {
        let teams = Entity::find().all(db).await?;
        Ok(teams)
    }

    async fn find_by_owner(db: &DatabaseConnection, owner_id: i32) -> ModelResult<Vec<Model>> {
        let teams = Entity::find()
            .filter(Column::OwnerId.eq(owner_id))
            .all(db)
            .await?;
        Ok(teams)
    }

    async fn find_by_user(db: &DatabaseConnection, user_id: i32) -> ModelResult<Vec<Model>> {
        // This will be implemented later when we have the team_members model
        // For now, just return teams where the user is the owner
        Self::find_by_owner(db, user_id).await
    }

    async fn create(db: &DatabaseConnection, owner_id: i32, params: &CreateParams) -> ModelResult<Model> {
        let user = users::Model::find_by_id(db, owner_id).await?;
        
        let team = ActiveModel {
            pid: Set(Uuid::new_v4()),
            name: Set(params.name.clone()),
            description: Set(params.description.clone()),
            owner_id: Set(user.id),
            ..Default::default()
        };

        let team = team.insert(db).await?;
        Ok(team)
    }

    async fn update(&self, db: &DatabaseConnection, params: &UpdateParams) -> ModelResult<Model> {
        let mut team: ActiveModel = self.clone().into();
        
        team.name = Set(params.name.clone());
        team.description = Set(params.description.clone());
        
        let team = team.update(db).await?;
        Ok(team)
    }

    async fn delete(&self, db: &DatabaseConnection) -> ModelResult<()> {
        let team: ActiveModel = self.clone().into();
        team.delete(db).await?;
        Ok(())
    }
}
