use async_trait::async_trait;
use loco_rs::prelude::*;
use sea_orm::{ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::_entities::teams::{self, ActiveModel, Entity, Model};
use super::_entities::team_memberships;
use super::_entities::users;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTeamParams {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTeamParams {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Validate, Deserialize)]
pub struct Validator {
    #[validate(length(min = 2, message = "Name must be at least 2 characters long."))]
    pub name: String,
}

impl Validatable for ActiveModel {
    fn validator(&self) -> Box<dyn Validate> {
        Box::new(Validator {
            name: self.name.as_ref().to_owned(),
        })
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for super::_entities::teams::ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        self.validate()?;
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
    /// Finds a team by the provided pid
    ///
    /// # Errors
    ///
    /// When could not find team or DB query error
    pub async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Self> {
        let parse_uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        let team = teams::Entity::find()
            .filter(
                model::query::condition()
                    .eq(teams::Column::Pid, parse_uuid)
                    .build(),
            )
            .one(db)
            .await?;
        team.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// Creates a new team and adds the creator as the Owner
    ///
    /// # Errors
    ///
    /// When could not save the team or team membership into the DB
    pub async fn create_team(
        db: &DatabaseConnection,
        user_id: i32,
        params: &CreateTeamParams,
    ) -> ModelResult<Self> {
        let txn = db.begin().await?;

        // Create team
        let team = teams::ActiveModel {
            name: ActiveValue::set(params.name.to_string()),
            description: ActiveValue::set(params.description.clone()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        // Add creator as owner
        let _membership = team_memberships::ActiveModel {
            team_id: ActiveValue::set(team.id),
            user_id: ActiveValue::set(user_id),
            role: ActiveValue::set("Owner".to_string()),
            pending: ActiveValue::set(false),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;
        Ok(team)
    }

    /// Updates the team details
    ///
    /// # Errors
    ///
    /// When could not save the team into the DB
    pub async fn update(&self, db: &DatabaseConnection, params: &UpdateTeamParams) -> ModelResult<Self> {
        let mut team: teams::ActiveModel = self.clone().into();
        team.name = ActiveValue::set(params.name.to_string());
        team.description = ActiveValue::set(params.description.clone());
        
        team.update(db).await.map_err(|e| ModelError::Any(e.into()))
    }

    /// Gets all team members with their roles
    ///
    /// # Errors
    ///
    /// When DB query error
    pub async fn get_members(&self, db: &DatabaseConnection) -> ModelResult<Vec<(users::Model, String)>> {
        let memberships = team_memberships::Entity::find()
            .filter(
                model::query::condition()
                    .eq(team_memberships::Column::TeamId, self.id)
                    .eq(team_memberships::Column::Pending, false)
                    .build(),
            )
            .find_with_related(users::Entity)
            .all(db)
            .await?;

        let result = memberships
            .into_iter()
            .map(|(membership, user)| {
                if let Some(user) = user.into_iter().next() {
                    (user, membership.role)
                } else {
                    unreachable!("User must exist for membership")
                }
            })
            .collect();

        Ok(result)
    }

    /// Checks if a user has a specific role or higher in the team
    ///
    /// # Errors
    ///
    /// When DB query error
    pub async fn has_role(&self, db: &DatabaseConnection, user_id: i32, role: &str) -> ModelResult<bool> {
        let role_level = match role {
            "Owner" => 4,
            "Administrator" => 3,
            "Developer" => 2,
            "Observer" => 1,
            _ => return Ok(false),
        };

        let membership = team_memberships::Entity::find()
            .filter(
                model::query::condition()
                    .eq(team_memberships::Column::TeamId, self.id)
                    .eq(team_memberships::Column::UserId, user_id)
                    .eq(team_memberships::Column::Pending, false)
                    .build(),
            )
            .one(db)
            .await?;

        if let Some(membership) = membership {
            let member_role_level = match membership.role.as_str() {
                "Owner" => 4,
                "Administrator" => 3,
                "Developer" => 2,
                "Observer" => 1,
                _ => 0,
            };
            Ok(member_role_level >= role_level)
        } else {
            Ok(false)
        }
    }

    /// Deletes the team and all associated memberships
    ///
    /// # Errors
    ///
    /// When could not delete the team from the DB
    pub async fn delete(&self, db: &DatabaseConnection) -> ModelResult<()> {
        teams::Entity::delete_by_id(self.id)
            .exec(db)
            .await
            .map_err(|e| ModelError::Any(e.into()))?;
        Ok(())
    }
}
