use loco_rs::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr,
    EntityTrait, QueryFilter, TransactionTrait,
}; // Removed QuerySelect
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

pub use super::_entities::team_memberships;
pub use super::_entities::teams::{self, ActiveModel, Model};
pub use super::_entities::users::{self, Model as UserModel};

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

impl Model {
    /// Finds a team by the provided pid
    ///
    /// # Errors
    ///
    /// When could not find team or DB query error
    pub async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Self> {
        tracing::debug!("Finding team by pid: {}", pid);

        // Parse UUID
        let parse_uuid = match Uuid::parse_str(pid) {
            Ok(uuid) => uuid,
            Err(e) => {
                tracing::error!("Failed to parse UUID '{}': {}", pid, e);
                return Err(ModelError::Any(e.into()));
            }
        };

        // Find team
        let team = teams::Entity::find()
            .filter(teams::Column::Pid.eq(parse_uuid))
            .one(db)
            .await?;

        match team {
            Some(team) => {
                tracing::debug!("Found team with pid: {}, id: {}", team.pid, team.id);
                Ok(team)
            }
            None => {
                tracing::error!("Team not found with pid: {}", pid);
                Err(ModelError::EntityNotFound)
            }
        }
    }

    /// Creates a new team and adds the creator as the Owner
    ///
    /// # Errors
    ///
    /// When could not save the team or team membership into the DB, or if name is not unique
    pub async fn create_team(
        db: &DatabaseConnection,
        user_id: i32,
        params: &CreateTeamParams,
    ) -> ModelResult<Self> {
        let txn = db.begin().await?;

        // Check for name uniqueness before creating
        if teams::Entity::find()
            .filter(teams::Column::Name.eq(&params.name))
            .one(&txn)
            .await?
            .is_some()
        {
            // Return ModelError::Message for uniqueness constraints
            return Err(ModelError::Message(format!(
                "Team name '{}' already exists.",
                params.name
            )));
        }

        tracing::info!("Creating team with name: {}", params.name);

        // Create team with explicit pid
        let team_pid = Uuid::new_v4();
        let team_model = teams::ActiveModel {
            name: ActiveValue::set(params.name.to_string()),
            description: ActiveValue::set(params.description.clone()),
            pid: ActiveValue::set(team_pid), // Set PID explicitly here
            ..Default::default()
        };

        // Validate before inserting
        team_model.validate()?; // Manually call validate before insert
        let team = team_model.insert(&txn).await?; // Use standard insert

        tracing::info!("Team created with id: {}, pid: {}", team.id, team.pid);

        // Add creator as owner
        let membership = team_memberships::ActiveModel {
            team_id: ActiveValue::set(team.id),
            user_id: ActiveValue::set(user_id),
            role: ActiveValue::set("Owner".to_string()),
            pending: ActiveValue::set(false),
            ..Default::default()
        }
        .insert(&txn) // Membership doesn't need complex validation here
        .await?;

        tracing::info!("Team membership created with id: {}", membership.id);

        txn.commit().await?;
        tracing::info!("Transaction committed successfully");

        Ok(team)
    }

    /// Updates the team details
    ///
    /// # Errors
    ///
    /// When could not save the team into the DB, or if the new name is not unique
    pub async fn update(
        &self,
        db: &DatabaseConnection,
        params: &UpdateTeamParams,
    ) -> ModelResult<Self> {
        let mut team_model: teams::ActiveModel = self.clone().into();

        // Check if name is being changed and if the new name already exists
        // Note: params.name is the *new* proposed name from the form
        if params.name != self.name {
            // Only check if the name is actually different
            if teams::Entity::find()
                .filter(teams::Column::Name.eq(&params.name)) // Check against the new name
                .filter(teams::Column::Id.ne(self.id)) // Exclude the current team
                .one(db)
                .await?
                .is_some()
            {
                // Return ModelError::Message for uniqueness constraints
                return Err(ModelError::Message(format!(
                    "Team name '{}' already exists.",
                    params.name
                )));
            }
            team_model.name = ActiveValue::set(params.name.clone());
        }

        team_model.description = ActiveValue::set(params.description.clone());

        // Validate before updating
        team_model.validate()?; // Manually call validate before update
        team_model
            .update(db)
            .await
            .map_err(|e| ModelError::Any(e.into())) // Use standard update
    }

    /// Gets all team members with their roles
    ///
    /// # Errors
    ///
    /// When DB query error
    pub async fn get_members(
        &self,
        db: &DatabaseConnection,
    ) -> ModelResult<Vec<(UserModel, String)>> {
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
            .filter_map(|(membership, user_opt)| {
                // Use filter_map to handle potential None user
                user_opt
                    .into_iter()
                    .next()
                    .map(|user| (user, membership.role))
            })
            .collect();

        Ok(result)
    }

    /// Checks if a user has a specific role or higher in the team
    ///
    /// # Errors
    ///
    /// When DB query error
    pub async fn has_role(
        &self,
        db: &DatabaseConnection,
        user_id: i32,
        role: &str,
    ) -> ModelResult<bool> {
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
        let txn = db.begin().await?;
        // Delete memberships first due to foreign key constraint
        team_memberships::Entity::delete_many()
            .filter(team_memberships::Column::TeamId.eq(self.id))
            .exec(&txn)
            .await?;

        // Then delete the team
        teams::Entity::delete_by_id(self.id).exec(&txn).await?;

        txn.commit().await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for super::_entities::teams::ActiveModel {
    async fn before_save<C>(self, _db: &C, _insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        // Validation is now handled explicitly before insert/update in the Model methods
        // self.validate()?;
        // No need to set PID here as it's handled in create_team
        Ok(self)
    }
}
