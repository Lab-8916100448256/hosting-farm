use sea_orm::{
    entity::prelude::*, ActiveModelBehavior, ActiveValue::{self, Set, Unchanged}, ColumnTrait, // Add Unchanged
    ConnectionTrait, QueryFilter, QuerySelect,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::models::ModelError; // Import ModelError

pub use super::_entities::ssh_keys::{ActiveModel, Column, Entity, Model as SshKeyEntityModel};

#[derive(Debug, Deserialize, Validate, Serialize)]
pub struct CreateSshKeyParams {
    // Renamed name to label as 'name' conflicts with sea_orm column name
    #[validate(length(min = 1, message = "Label cannot be empty"))]
    pub label: String,
    #[validate(length(min = 1, message = "Key cannot be empty"))]
    pub key: String,
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        tracing::debug!("SSH Keys ActiveModelBehavior::before_save called (insert: {})", insert);
        if insert {
            // Assuming pid is handled by the database or default(), if not:
            // if self.pid.is_not_set() {
            //     self.pid = Set(Uuid::new_v4());
            // }
            let now = chrono::Utc::now();
            if self.created_at.is_not_set() {
                self.created_at = Set(now.into());
            }
            if self.updated_at.is_not_set() {
                self.updated_at = Set(now.into());
            }
        } else {
            // Only update `updated_at` if it's an update operation
            let now = chrono::Utc::now();
            self.updated_at = Set(now.into());

             // Ensure fields are not modified unless explicitly Set
             if self.user_id.is_set() && !self.user_id.is_unchanged() {
                 tracing::warn!("Attempted to modify user_id during ssh_key update. Resetting to Unchanged.");
                 self.user_id = Unchanged(self.user_id.as_ref().clone());
            }
             if self.created_at.is_set() && !self.created_at.is_unchanged() {
                  tracing::warn!("Attempted to modify created_at during ssh_key update. Resetting to Unchanged.");
                 self.created_at = Unchanged(self.created_at.as_ref().clone());
            }
             // Do not automatically set label/public_key to Unchanged
             // If they are NotSet, SeaORM won't update them. If they are Set, let the update proceed.
        }
        Ok(self)
    }
}

// Custom Model struct wrapping the entity Model
#[derive(Debug, Clone, Serialize, Deserialize)] // Add Serialize, Deserialize
pub struct Model {
    pub inner: SshKeyEntityModel,
}

// Implement Deref to easily access inner fields
impl std::ops::Deref for Model {
    type Target = SshKeyEntityModel;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// Implement From<SshKeyEntityModel> for Model
impl From<SshKeyEntityModel> for Model {
    fn from(inner: SshKeyEntityModel) -> Self {
        Self { inner }
    }
}

impl Model {
    /// Find SSH keys by user ID.
    pub async fn find_by_user(
        db: &impl ConnectionTrait,
        user_id: i32,
    ) -> Result<Vec<Self>, ModelError> {
        Ok(Entity::find()
            .filter(Column::UserId.eq(user_id))
            .all(db)
            .await?
            .into_iter()
            .map(Self::from) // Convert each SshKeyEntityModel to custom Model
            .collect())
    }

    /// Find an SSH key by its ID (assuming pid is not a column).
    /// If PID is a column, change Column::Id to Column::Pid.
    pub async fn find_by_id(db: &impl ConnectionTrait, id: i32) -> Result<Self, ModelError> {
        // let uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        Entity::find()
            .filter(Column::Id.eq(id)) // Filter by primary key ID
            .one(db)
            .await?
            .map(Self::from) // Convert SshKeyEntityModel to custom Model
            .ok_or(ModelError::EntityNotFound)
    }

     /// Find an SSH key by its PID (if PID column exists).
     // If PID is not a database column, this method should be removed or adapted.
     // pub async fn find_by_pid(db: &impl ConnectionTrait, pid: &str) -> Result<Self, ModelError> {
     //     let uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
     //     Entity::find()
     //         .filter(Column::Pid.eq(uuid)) // Use Pid column if it exists
     //         .one(db)
     //         .await?
     //         .map(Self::from)
     //         .ok_or(ModelError::EntityNotFound)
     // }

    /// Create a new SSH key for a user.
    pub async fn create_key(
        db: &impl ConnectionTrait,
        user_id: i32,
        params: &CreateSshKeyParams,
    ) -> Result<Self, ModelError> {
        params.validate().map_err(|e| ModelError::Validation(e.into()))?;

        // Check for key uniqueness for this user using the correct column name
        if Entity::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::PublicKey.eq(&params.key)) // Use PublicKey column
            .one(db)
            .await?
            .is_some()
        {
            return Err(ModelError::Message(
                "This key is already registered for your account.".to_string(),
            ));
        }

         // Check for label uniqueness for this user using the correct column name
         if Entity::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::Label.eq(&params.label)) // Use Label column
            .one(db)
            .await?
            .is_some()
        {
            return Err(ModelError::Message(
                "An SSH key with this label already exists for your account.".to_string(),
            ));
        }

        let mut ssh_key_am = ActiveModel {
            user_id: Set(user_id),
            label: Set(params.label.clone()), // Use label field
            public_key: Set(params.key.clone()), // Use public_key field
            ..Default::default()
        };

        ssh_key_am.insert(db).await.map(Self::from)
    }

    /// Delete the SSH key.
    pub async fn delete(&self, db: &impl ConnectionTrait) -> Result<(), ModelError> {
        Entity::delete_by_id(self.id).exec(db).await?;
        Ok(())
    }
}
