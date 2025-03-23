use async_trait::async_trait;
use sea_orm::{entity::*, query::*, DatabaseConnection, Set};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::{_entities::team_members, teams, users, ModelError, ModelResult};

pub use team_members::{ActiveModel, Column, Entity, Model, PrimaryKey, Relation};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateParams {
    pub team_id: i32,
    pub user_id: i32,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRoleParams {
    pub role: String,
}

#[async_trait]
pub trait TeamMemberActions {
    async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Model>;
    async fn find_by_id(db: &DatabaseConnection, id: i32) -> ModelResult<Model>;
    async fn find_by_team_and_user(db: &DatabaseConnection, team_id: i32, user_id: i32) -> ModelResult<Model>;
    async fn find_by_team(db: &DatabaseConnection, team_id: i32) -> ModelResult<Vec<Model>>;
    async fn find_by_user(db: &DatabaseConnection, user_id: i32) -> ModelResult<Vec<Model>>;
    async fn create(db: &DatabaseConnection, params: &CreateParams) -> ModelResult<Model>;
    async fn update_role(&self, db: &DatabaseConnection, params: &UpdateRoleParams) -> ModelResult<Model>;
    async fn delete(&self, db: &DatabaseConnection) -> ModelResult<()>;
    async fn is_team_owner(&self, db: &DatabaseConnection) -> ModelResult<bool>;
    async fn is_team_admin(&self, db: &DatabaseConnection) -> ModelResult<bool>;
    async fn can_manage_members(&self, db: &DatabaseConnection) -> ModelResult<bool>;
}

#[async_trait]
impl TeamMemberActions for Model {
    async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Model> {
        let uuid = Uuid::parse_str(pid).map_err(|_| ModelError::EntityNotFound)?;
        let member = Entity::find()
            .filter(Column::Pid.eq(uuid))
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)?;
        Ok(member)
    }

    async fn find_by_id(db: &DatabaseConnection, id: i32) -> ModelResult<Model> {
        let member = Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)?;
        Ok(member)
    }

    async fn find_by_team_and_user(db: &DatabaseConnection, team_id: i32, user_id: i32) -> ModelResult<Model> {
        let member = Entity::find()
            .filter(Column::TeamId.eq(team_id))
            .filter(Column::UserId.eq(user_id))
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)?;
        Ok(member)
    }

    async fn find_by_team(db: &DatabaseConnection, team_id: i32) -> ModelResult<Vec<Model>> {
        let members = Entity::find()
            .filter(Column::TeamId.eq(team_id))
            .all(db)
            .await?;
        Ok(members)
    }

    async fn find_by_user(db: &DatabaseConnection, user_id: i32) -> ModelResult<Vec<Model>> {
        let members = Entity::find()
            .filter(Column::UserId.eq(user_id))
            .all(db)
            .await?;
        Ok(members)
    }

    async fn create(db: &DatabaseConnection, params: &CreateParams) -> ModelResult<Model> {
        // Verify team exists
        let _team = teams::Model::find_by_id(db, params.team_id).await?;
        
        // Verify user exists
        let _user = users::Model::find_by_id(db, params.user_id).await?;
        
        // Check if member already exists
        let existing = Entity::find()
            .filter(Column::TeamId.eq(params.team_id))
            .filter(Column::UserId.eq(params.user_id))
            .one(db)
            .await?;
            
        if existing.is_some() {
            return Err(ModelError::EntityAlreadyExists);
        }
        
        // Create new member
        let member = ActiveModel {
            pid: Set(Uuid::new_v4()),
            team_id: Set(params.team_id),
            user_id: Set(params.user_id),
            role: Set(params.role.clone()),
            ..Default::default()
        };

        let member = member.insert(db).await?;
        Ok(member)
    }

    async fn update_role(&self, db: &DatabaseConnection, params: &UpdateRoleParams) -> ModelResult<Model> {
        let mut member: ActiveModel = self.clone().into();
        
        member.role = Set(params.role.clone());
        
        let member = member.update(db).await?;
        Ok(member)
    }

    async fn delete(&self, db: &DatabaseConnection) -> ModelResult<()> {
        let member: ActiveModel = self.clone().into();
        member.delete(db).await?;
        Ok(())
    }

    async fn is_team_owner(&self, db: &DatabaseConnection) -> ModelResult<bool> {
        let team = teams::Model::find_by_id(db, self.team_id).await?;
        Ok(team.owner_id == self.user_id)
    }

    async fn is_team_admin(&self, db: &DatabaseConnection) -> ModelResult<bool> {
        if self.is_team_owner(db).await? {
            return Ok(true);
        }
        
        Ok(self.role == "admin")
    }

    async fn can_manage_members(&self, db: &DatabaseConnection) -> ModelResult<bool> {
        self.is_team_admin(db).await
    }
}
