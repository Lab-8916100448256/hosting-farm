use async_trait::async_trait;
use bcrypt::{hash, verify, BcryptError, DEFAULT_COST};
use loco_rs::prelude::*; // Rely fully on prelude for loco types/traits
use loco_rs::app::AppContext; // Keep AppContext explicit
use loco_rs::config::ConfigExt; // Keep ConfigExt trait explicit
use loco_rs::authentication::users::DisplayUser; // Keep DisplayUser explicit for From impl
// Explicit sea_orm imports
use sea_orm::{
    entity::prelude::*, // Still useful for Uuid, etc.
    ActiveModelBehavior, ActiveModelTrait,
    ActiveValue::{self, Set, Unchanged},
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    IntoActiveModel,
    IntoCondition,
    ModelTrait,
    QueryFilter, QueryOrder, QuerySelect, RelationTrait,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};
use validator::Validate;
use uuid::Uuid; // Ensure Uuid is imported

// Alias the generated entity model and related items
pub use super::_entities::users::{ActiveModel, Column, Entity, Model as UserEntityModel};
use crate::models::{
    _entities::{team_memberships, teams},
    teams::{CreateTeamParams, Model as TeamModel, Role},
};

// Define constants for user status
pub const USER_STATUS_NEW: &str = "new";
pub const USER_STATUS_APPROVED: &str = "approved";
pub const USER_STATUS_REJECTED: &str = "rejected";

// --- Structs for Params ---
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterParams {
    #[validate(length(min = 3, message = "Name must be at least 3 characters long."))]
    pub name: String,
    #[validate(email(message = "Invalid email format."))]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters long."))]
    pub password: String,
    #[validate(must_match(other = "password", message = "Passwords must match."))]
    pub password_confirmation: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginParams {
    #[validate(email(message = "Invalid email format."))]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters long."))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ForgotPasswordParams {
    #[validate(email(message = "Invalid email format."))]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordParams {
    #[validate(length(min = 8, message = "Password must be at least 8 characters long."))]
    pub password: String,
    #[validate(must_match(other = "password", message = "Passwords must match."))]
    pub password_confirmation: String,
    pub reset_token: String,
}

// --- ADDING ActiveModelBehavior implementation BACK ---
#[async_trait]
impl ActiveModelBehavior for ActiveModel {
    // Called before saving an ActiveModel
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> std::result::Result<Self, DbErr> // Use std::result::Result
    where
        C: ConnectionTrait,
    {
        // Hash password BEFORE save if it's set and we are inserting or updating
        if let ActiveValue::Set(password) = &self.password {
             if !password.starts_with("$2b$") { // Avoid re-hashing existing hashes
                let hashed_password = hash_password(password).map_err(|e| {
                    // Convert ModelError to DbErr for ActiveModelBehavior
                    DbErr::Custom(format!("Password hashing failed: {}", e))
                })?;
                self.password = Set(hashed_password);
             }
        }

        // Set PID and timestamps on insert
        if insert {
            if self.pid.is_not_set() {
                self.pid = Set(Uuid::new_v4());
            }
            let now = chrono::Utc::now().into(); // DateTimeWithTimeZone
            if self.created_at.is_not_set() {
                self.created_at = Set(now);
            }
            if self.updated_at.is_not_set() {
                self.updated_at = Set(now);
            }
            // Set default status on insert if not provided
            if self.status.is_not_set() {
                 self.status = Set(USER_STATUS_NEW.to_string());
            }
            // Generate API key on insert if not provided
            if self.api_key.is_not_set() {
                 self.api_key = Set(Some(Uuid::new_v4().to_string()));
            }

        } else {
            // Only update `updated_at` timestamp on update
            self.updated_at = Set(chrono::Utc::now().into());

            // Ensure certain fields are not accidentally modified during update
            // Set them to Unchanged if they were Set
            if self.created_at.is_set() { self.created_at = Unchanged(self.created_at.take().unwrap()); }
            if self.pid.is_set() { self.pid = Unchanged(self.pid.take().unwrap()); }
            if self.email.is_set() { self.email = Unchanged(self.email.take().unwrap()); } // Email shouldn't change via generic update
        }

        Ok(self)
    }
}


// --- Custom Model struct wrapping the entity Model ---
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    pub inner: UserEntityModel,
}

// Implement Deref to easily access inner fields
impl std::ops::Deref for Model {
    type Target = UserEntityModel;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

// Implement From<UserEntityModel> for Model
impl From<UserEntityModel> for Model {
    fn from(inner: UserEntityModel) -> Self {
        Self { inner }
    }
}

// Implement From<&UserEntityModel> for Model to handle references
impl From<&UserEntityModel> for Model {
    fn from(inner: &UserEntityModel) -> Self {
        Self { inner: inner.clone() }
    }
}

// --- Authenticable Implementation for Custom Model ---
// Use the Authenticable trait from the prelude
#[async_trait]
impl Authenticable for Model {
    async fn find_by_claims_key(db: &DatabaseConnection, key: &str) -> ModelResult<Self> {
        Self::find_by_pid(db, key).await
    }

    fn get_claims_key(&self) -> String {
        self.pid.to_string()
    }

    async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self> {
        Self::find_user(db, Column::ApiKey.eq(Some(api_key.to_string()))).await
    }
}

// --- Custom Model Methods ---
impl Model {
    /// Find a user by pid
    pub async fn find_by_pid(db: &impl ConnectionTrait, pid: &str) -> ModelResult<Self> {
        let uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        Self::find_user(db, Column::Pid.eq(uuid)).await
    }

    /// Find a user by email
    pub async fn find_by_email(db: &impl ConnectionTrait, email: &str) -> ModelResult<Self> {
        Self::find_user(db, Column::Email.eq(email.to_lowercase())).await
    }

    /// Find a user by email and password, checking approval status
    pub async fn find_by_email_and_password(
        db: &impl ConnectionTrait,
        params: &LoginParams,
    ) -> ModelResult<Self> {
        let user = Self::find_by_email(db, &params.email).await?;

        // User must be approved to log in
        if user.status != USER_STATUS_APPROVED {
            tracing::warn!("Attempted login by non-approved user: {}", params.email);
            return Err(ModelError::Message(
                "Your account is not approved. Please wait for an administrator to approve it."
                    .to_string(),
            ));
        }

        let password_hash = &user.password;

        verify_password(password_hash, &params.password)?;
        Ok(user)
    }

    /// Create a new user with the given parameters.
    /// Handles uniqueness checks, default status, and admin team creation for the first user.
    /// Password hashing is handled by ActiveModelBehavior::before_save.
    #[tracing::instrument(name = "create_user_with_password", skip(ctx, db, params))]
    pub async fn create_with_password(
        ctx: &AppContext,
        db: &impl TransactionTrait, // Changed to TransactionTrait
        params: &RegisterParams,
    ) -> ModelResult<Self> {
        tracing::info!(email = params.email, name = params.name, "creating user");

        // Trim inputs
        let trimmed_email = params.email.trim().to_lowercase();
        let trimmed_name = params.name.trim();

        // --- Uniqueness Checks (Inside Transaction) ---
        if Entity::find()
            .filter(Column::Email.eq(&trimmed_email))
            .one(db) // Use the transaction object
            .await?
            .is_some()
        {
            return Err(ModelError::Message(
                "A user with this email already exists.".to_string(),
            ));
        }
        if Entity::find()
            .filter(Column::Name.eq(trimmed_name))
            .one(db) // Use the transaction object
            .await?
            .is_some()
        {
            return Err(ModelError::Message(
                "A user with this name already exists.".to_string(),
            ));
        }

        // --- Determine Status ---
        let is_first_user = Entity::find().count(db).await? == 0; // Use transaction object
        let initial_status = if is_first_user {
            USER_STATUS_APPROVED
        } else {
            USER_STATUS_NEW
        };

        let now = chrono::Utc::now();
        let email_verified_at = if is_first_user { Some(now.into()) } else { None };

        // --- Create ActiveModel ---
        let user_am = ActiveModel {
            name: Set(trimmed_name.to_string()),
            email: Set(trimmed_email),
            password: Set(params.password.to_string()), // Set raw password, before_save will hash
            status: Set(initial_status.to_string()),    // Set status explicitly
            email_verified_at: Set(email_verified_at),
            ..Default::default()
        };

        // --- Insert User ---
        // Use transaction object for insert; before_save is called automatically
        let user_entity = user_am.insert(db).await?;
        let user_model = Model::from(user_entity);

        // --- First User Admin Team Setup ---
        if is_first_user {
            tracing::info!("First user registered. Creating Administrators team and assigning ownership.");
            let admin_team_name = get_admin_team_name(ctx)?; // Get config value

            match TeamModel::create_team(
                db, // Use the transaction object
                user_model.id,
                &CreateTeamParams {
                    name: admin_team_name.clone(),
                    description: Some("System Administrators".to_string()),
                },
            )
            .await
            {
                Ok(team) => {
                    tracing::info!(
                        "Administrators team created (id: {}) for first user (id: {})",
                        team.id,
                        user_model.id
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to create Administrators team for first user {}: {}",
                        user_model.id,
                        e
                    );
                    // Rollback is handled by the caller (e.g., controller) if db is a transaction
                    return Err(ModelError::Message(
                        "Failed to set up initial administrator team.".to_string(),
                    ));
                }
            }
        }
        // Removed explicit transaction commit/rollback - should be handled by caller

        tracing::info!(user_id = user_model.id, "user created successfully");
        Ok(user_model)
    }

    /// Generate an API key for the user.
    pub async fn generate_api_key(&self, db: &impl ConnectionTrait) -> ModelResult<String> {
        let mut user_am: ActiveModel = self.inner.clone().into();
        let new_key = Uuid::new_v4().to_string();
        user_am.api_key = Set(Some(new_key.clone()));
        let updated_entity = user_am.update(db).await.map_err(ModelError::from)?;
        Ok(updated_entity.api_key.unwrap_or_default())
    }

    /// Generate a reset token and set the reset_sent_at time.
    pub async fn generate_reset_token(&self, db: &impl ConnectionTrait) -> ModelResult<String> {
        let mut user_am: ActiveModel = self.inner.clone().into();
        let token = uuid::Uuid::new_v4().to_string();
        user_am.reset_token = Set(Some(token.clone()));
        user_am.reset_sent_at = Set(Some(chrono::Utc::now().into()));
        let _updated_entity = user_am.update(db).await.map_err(ModelError::from)?;
        Ok(token)
    }

    /// Generate an email verification token and set the sent_at time.
    pub async fn generate_email_verification_token(
        &self,
        db: &impl ConnectionTrait,
    ) -> ModelResult<String> {
        let mut user_am: ActiveModel = self.inner.clone().into();
        let token = uuid::Uuid::new_v4().to_string();
        user_am.email_verification_token = Set(Some(token.clone()));
        user_am.email_verification_sent_at = Set(Some(chrono::Utc::now().into()));
        let _updated_entity = user_am.update(db).await.map_err(ModelError::from)?;
        Ok(token)
    }

    /// Verify the email verification token and mark email as verified.
    pub async fn verify_email(&self, db: &impl ConnectionTrait, token: &str) -> ModelResult<Self> {
        if self.email_verification_token.as_ref() != Some(&token.to_string()) {
            return Err(ModelError::Message("invalid token".to_string()));
        }
        let mut user_am: ActiveModel = self.inner.clone().into();
        user_am.email_verified_at = Set(Some(chrono::Utc::now().into()));
        user_am.email_verification_token = Set(None); // Clear token
        let updated_entity = user_am.update(db).await.map_err(ModelError::from)?;
        Ok(Self::from(updated_entity))
    }

    /// Generate a PGP verification token.
    pub async fn generate_pgp_verification_token(
        &self,
        db: &impl ConnectionTrait,
    ) -> ModelResult<String> {
        let mut user_am: ActiveModel = self.inner.clone().into();
        let token = uuid::Uuid::new_v4().to_string();
        user_am.pgp_verification_token = Set(Some(token.clone()));
        let _updated_entity = user_am.update(db).await.map_err(ModelError::from)?;
        Ok(token)
    }

    /// Verify the PGP verification token and mark PGP as verified.
    pub async fn verify_pgp(&self, db: &impl ConnectionTrait, token: &str) -> ModelResult<Self> {
        if self.pgp_verification_token.as_ref() != Some(&token.to_string()) {
            return Err(ModelError::Message("invalid token".to_string()));
        }
        let mut user_am: ActiveModel = self.inner.clone().into();
        user_am.pgp_verified_at = Set(Some(chrono::Utc::now().into()));
        user_am.pgp_verification_token = Set(None); // Clear token
        let updated_entity = user_am.update(db).await.map_err(ModelError::from)?;
        Ok(Self::from(updated_entity))
    }

    /// Update the user's password using the reset token.
    pub async fn reset_password(
        db: &impl ConnectionTrait, // Can be Connection or Transaction
        params: &ResetPasswordParams,
    ) -> ModelResult<Self> {
        params
            .validate()
            .map_err(|e| ModelError::Validation(ModelValidationErrors::from(e)))?; // Use ModelValidationErrors::from

        let user_entity = Entity::find()
            .filter(Column::ResetToken.eq(params.reset_token.clone()))
            .one(db)
            .await?;

        let Some(user_entity) = user_entity else {
            return Err(ModelError::Message("invalid reset token".to_string()));
        };

        // Token Expiry Check
        if let Some(sent_at) = user_entity.reset_sent_at {
            if sent_at + chrono::Duration::hours(1) < chrono::Utc::now() {
                return Err(ModelError::Message("token expired".to_string()));
            }
        } else {
            return Err(ModelError::Message(
                "invalid reset token state".to_string(),
            ));
        }

        // Set raw password, before_save will hash it
        let mut user_am: ActiveModel = user_entity.into();
        user_am.password = Set(params.password.clone());
        user_am.reset_token = Set(None);
        user_am.reset_sent_at = Set(None);
        let updated_entity = user_am.update(db).await.map_err(ModelError::from)?;
        Ok(Self::from(updated_entity))
    }

    /// Update the user's status (e.g., approve/reject).
    pub async fn update_status(&self, db: &impl ConnectionTrait, new_status: &str) -> ModelResult<Self> {
        if ![USER_STATUS_NEW, USER_STATUS_APPROVED, USER_STATUS_REJECTED].contains(&new_status) {
            return Err(ModelError::Message(format!("Invalid status: {}", new_status)));
        }
        let mut user_am = self.inner.clone().into_active_model(); // Use trait method
        user_am.status = Set(new_status.to_string());
        let updated_entity = user_am.update(db).await.map_err(ModelError::from)?;
        Ok(Self::from(updated_entity))
    }

    /// Internal helper to find a user by a condition.
    async fn find_user<T>(db: &impl ConnectionTrait, condition: T) -> ModelResult<Self>
    where
        T: IntoCondition + Send,
    {
        Entity::find()
            .filter(condition)
            .one(db)
            .await?
            .map(Self::from)
            .ok_or_else(|| ModelError::EntityNotFound)
    }

    /// Check if the user is a system administrator.
    // Use loco_rs::Result here, not ModelResult
    pub async fn is_system_admin(ctx: &AppContext, user: &Model) -> Result<bool> {
        let admin_team_name = get_admin_team_name(ctx)?; // Use helper

        let admin_team_opt = teams::Entity::find()
            .filter(teams::Column::Name.eq(&admin_team_name))
            .one(&ctx.db)
            .await.map_err(|e| Error::Model(ModelError::Any(e.into())))?; // Convert DbErr to loco Error

        let Some(admin_team) = admin_team_opt else {
            tracing::warn!(
                "Configured admin team '{}' not found in the database.",
                admin_team_name
            );
            return Ok(false);
        };

        let admin_team_model = TeamModel::from(admin_team);
        admin_team_model
            .has_role(
                &ctx.db,
                user.id,
                vec![Role::Owner, Role::Admin],
            )
            .await.map_err(Error::Model) // Propagate ModelError as loco Error
    }

    /// Convert custom Model into ActiveModel for updates.
    pub fn into_active_model(self) -> ActiveModel {
        self.inner.into_active_model()
    }

    /// Helper function to get all users, optionally filtered by status.
    pub async fn get_all(
        db: &impl ConnectionTrait,
        status_filter: Option<String>,
    ) -> ModelResult<Vec<Self>> {
        let mut query = Entity::find().order_by_asc(Column::Id);
        if let Some(status) = status_filter {
            query = query.filter(Column::Status.eq(status));
        }
        query
            .all(db)
            .await
            .map_err(ModelError::from)
            .map(|entities| entities.into_iter().map(Self::from).collect())
    }
}

// --- Helper Functions ---

/// Helper function to hash a password.
fn hash_password(password: &str) -> std::result::Result<String, ModelError> { // Use std::result::Result
    hash(password, DEFAULT_COST).map_err(|e| {
        tracing::error!(error = ?e, "failed to hash password");
        ModelError::BcryptError(e)
    })
}

/// Helper function to verify a password against a hash.
pub fn verify_password(hash: &str, password: &str) -> std::result::Result<(), ModelError> { // Use std::result::Result
    verify(password, hash).map_err(|e| match e {
        BcryptError::InvalidHash(_) | BcryptError::InvalidPrefix(_) => {
            tracing::error!(
                error = ?e,
                "password hash verification failed due to invalid hash format"
            );
            ModelError::BcryptError(e)
        }
        _ => {
            tracing::debug!(error = ?e, "password hash verification failed (likely mismatch)");
            ModelError::Message("invalid email or password".to_string())
        }
    })
}

/// Helper to get the configured admin team name.
// Use std::result::Result<String, loco_rs::config::Error>
fn get_admin_team_name(ctx: &AppContext) -> std::result::Result<String, loco_rs::config::Error> {
    let admin_team_name_res: std::result::Result<String, loco_rs::config::Error> =
        ctx.config.get("app.admin_team_name"); // Use ConfigExt trait method
    Ok(admin_team_name_res.unwrap_or_else(|e| {
        // Use loco_rs::config::Error::NotFound
        if let loco_rs::config::Error::NotFound(_) = e {
            tracing::debug!(
                "'app.admin_team_name' not found in config. Using default 'Administrators'"
            );
        } else {
            tracing::warn!(
                "Failed to read 'app.admin_team_name' from config ({}). Using default 'Administrators'",
                e
            );
        }
        "Administrators".to_string()
    }))
}


/// Convert a `Model` to a `DisplayUser` used by the auth middleware.
impl From<&Model> for DisplayUser {
    fn from(user: &Model) -> Self {
        Self {
            pid: user.pid.to_string(),
            name: user.name.clone(),
            email: user.email.clone(),
            email_verified: user.email_verified_at.is_some(),
        }
    }
}

// Implement IntoActiveModel for &Model to simplify updates
impl<'a> IntoActiveModel<ActiveModel> for &'a Model {
    fn into_active_model(self) -> ActiveModel {
        self.inner.clone().into()
    }
}

// ActiveModelBehavior hooks are defined above within the `impl ActiveModelBehavior for ActiveModel` block.
