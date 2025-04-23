use async_trait::async_trait;
use bcrypt::{hash, verify, BcryptError, DEFAULT_COST};
use loco_rs::prelude::*; // Rely fully on prelude for loco types/traits
use loco_rs::app::AppContext; // Keep AppContext explicit
use loco_rs::config::ConfigExt; // Keep ConfigExt trait explicit
use loco_rs::authentication::users::DisplayUser; // Keep DisplayUser explicit for From impl
// Explicit sea_orm imports
use sea_orm::{{
    entity::prelude::*, // Still useful for Uuid, etc.
    ActiveModelBehavior, // Needed for custom hooks
    ActiveModelTrait,
    ActiveValue::{self, Set, Unchanged},
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    IntoActiveModel, // Keep this trait import as it's used by the derived `into()`
    IntoCondition,
    ModelTrait,
    QueryFilter, QueryOrder, QuerySelect, RelationTrait,
    TransactionTrait,
}};
use serde::{Deserialize, Serialize}; // Added Serialize
use validator::Validate;
use uuid::Uuid; // Ensure Uuid is imported

// Alias the generated entity model and related items
// Explicitly alias the generated ActiveModel as UserActiveModel
pub use super::_entities::users::{ActiveModel as UserActiveModel, Column, Entity, Model as UserEntityModel};
use crate::models::{{
    _entities::{team_memberships, teams}, // Added team_memberships here
    teams::{CreateTeamParams, Model as TeamModel, Role},
}};

// Define constants for user status
pub const USER_STATUS_NEW: &str = "new";
pub const USER_STATUS_APPROVED: &str = "approved";
pub const USER_STATUS_REJECTED: &str = "rejected";

// --- Structs for Params ---
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterParams {{
    #[validate(length(min = 3, message = "Name must be at least 3 characters long."))]
    pub name: String,
    #[validate(email(message = "Invalid email format."))]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters long."))]
    pub password: String,
    #[validate(must_match(other = "password", message = "Passwords must match."))]
    pub password_confirmation: String,
}}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginParams {{
    #[validate(email(message = "Invalid email format."))]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters long."))]
    pub password: String,
}}

#[derive(Debug, Deserialize, Validate)]
pub struct ForgotPasswordParams {{
    #[validate(email(message = "Invalid email format."))]
    pub email: String,
}}

#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordParams {{
    #[validate(length(min = 8, message = "Password must be at least 8 characters long."))]
    pub password: String,
    #[validate(must_match(other = "password", message = "Passwords must match."))]
    pub password_confirmation: String,
    pub reset_token: String,
}}

// --- Custom Model struct wrapping the entity Model ---
// Do NOT implement Deref or IntoActiveModel here. Access fields via .inner.
// Rely on UserEntityModel::into() for conversions to ActiveModel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {{
    pub inner: UserEntityModel,
}}


// --- Trait Implementations for Custom Model ---

// Implement From<UserEntityModel> for Model
impl From<UserEntityModel> for Model {{
    fn from(inner: UserEntityModel) -> Self {{
        Self {{ inner }}
    }}
}}

// Implement From<&UserEntityModel> for Model to handle references
impl From<&UserEntityModel> for Model {{
    fn from(inner: &UserEntityModel) -> Self {{
        Self {{ inner: inner.clone() }}
    }}
}}

// Implement Authenticable trait
#[async_trait]
impl Authenticable for Model {{
    // Access inner fields directly
    fn get_id(&self) -> i32 {{
        self.inner.id
    }}

    fn get_claims_key(&self) -> String {{
        self.inner.pid.to_string()
    }}

    fn claims(&self, secret: &JWTSecret, expiration: Option<chrono::Duration>) -> loco_rs::authentication::JWTClaims {{
        let default_expiration = chrono::Duration::days(secret.jwt_config.expiration_days);
        let exp = (chrono::Utc::now() + expiration.unwrap_or(default_expiration)).timestamp() as usize;
        loco_rs::authentication::JWTClaims {{
            id: self.get_id(), // Calls self.get_id()
            pid: self.get_claims_key(), // Calls self.get_claims_key()
            exp,
        }}
    }}

    async fn generate_token(
        &self,
        secret: &JWTSecret,
        expiration: Option<chrono::Duration>,
    ) -> std::result::Result<String, jsonwebtoken::errors::Error> {{
        let claims = self.claims(secret, expiration);
        secret.encode(&claims)
    }}

    async fn find_by_claims_key(db: &DatabaseConnection, key: &str) -> ModelResult<Self> {{
        Self::find_by_pid(db, key).await
    }}

    async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self> {{
        Self::find_user(db, Column::ApiKey.eq(Some(api_key.to_string()))).await
    }}
}}

// --- New Struct for Layout Context ---
#[derive(Serialize)]
pub struct LayoutContext {{
    pub user: UserEntityModel,
    pub invitation_count: i64,
    pub is_system_admin: bool,
    pub active_page: String,
    pub pending_user_count: u64,
}}

// --- Custom Model Methods (associated functions and methods on Model) ---
impl Model {{
    /// Find a user by pid (associated function)
    pub async fn find_by_pid(db: &impl ConnectionTrait, pid: &str) -> ModelResult<Self> {{
        let uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        Self::find_user(db, Column::Pid.eq(uuid)).await
    }}

    /// Find a user by email (associated function)
    pub async fn find_by_email(db: &impl ConnectionTrait, email: &str) -> ModelResult<Self> {{
        Self::find_user(db, Column::Email.eq(email.to_lowercase())).await
    }}

    /// Find a user by email and password, checking approval status (associated function)
    pub async fn find_by_email_and_password(
        db: &impl ConnectionTrait,
        params: &LoginParams,
    ) -> ModelResult<Self> {{
        let user = Self::find_by_email(db, &params.email).await?;

        // Access status via inner
        if user.inner.status != USER_STATUS_APPROVED {{
            tracing::warn!("Attempted login by non-approved user: {{}}", params.email);
            return Err(ModelError::Message(
                "Your account is not approved. Please wait for an administrator to approve it."
                    .to_string(),
            ));
        }}

        // Access password via inner
        let password_hash_opt = &user.inner.password;
        let Some(password_hash) = password_hash_opt else {{
             tracing::error!(user_pid = %user.inner.pid, "User attempting login has no password hash set.");
             return Err(ModelError::Message("Internal server error: invalid user state.".to_string()));
        }};

        verify_password(password_hash, &params.password)?;
        Ok(user)
    }}

    /// Create a new user with the given parameters (associated function).
    #[tracing::instrument(name = "create_user_with_password", skip(ctx, db, params))]
    pub async fn create_with_password(
        ctx: &AppContext,
        db: &impl TransactionTrait,
        params: &RegisterParams,
    ) -> ModelResult<Self> {{
        tracing::info!(email = params.email, name = params.name, "creating user");

        let trimmed_email = params.email.trim().to_lowercase();
        let trimmed_name = params.name.trim();

        if Entity::find().filter(Column::Email.eq(&trimmed_email)).one(db).await?.is_some() {{
            return Err(ModelError::Message("A user with this email already exists.".to_string()));
        }}
        if Entity::find().filter(Column::Name.eq(trimmed_name)).one(db).await?.is_some() {{
            return Err(ModelError::Message("A user with this name already exists.".to_string()));
        }}

        let is_first_user = Entity::find().count(db).await? == 0;
        let initial_status = if is_first_user {{ USER_STATUS_APPROVED }} else {{ USER_STATUS_NEW }};
        let email_verified_at = if is_first_user {{ Some(chrono::Utc::now().into()) }} else {{ None }};

        // Password will be hashed by ActiveModelBehavior
        let now = chrono::Utc::now();

        let user_am = UserActiveModel {{
            name: Set(trimmed_name.to_string()),
            email: Set(trimmed_email),
            password: Set(Some(params.password.clone())), // Set plain password, hashed in before_save
            status: Set(initial_status.to_string()),
            email_verified_at: Set(email_verified_at),
            pid: Set(Uuid::new_v4()),
            created_at: Set(now.into()),
            updated_at: Set(now.into()), // Set initially, updated again in before_save
            api_key: Set(Some(Uuid::new_v4().to_string())),
            ..Default::default()
        }};

        // ActiveModelBehavior::before_save will be called automatically by insert
        let user_entity = user_am.insert(db).await?;
        let user_model = Model::from(user_entity);

        if is_first_user {{
            tracing::info!("First user registered. Creating Administrators team and assigning ownership.");
            let admin_team_name = get_admin_team_name(ctx)?;
            // Access user id via inner
            match TeamModel::create_team(db, user_model.inner.id, &CreateTeamParams {{ name: admin_team_name.clone(), description: Some("System Administrators".to_string()) }}).await {{
                Ok(team) => tracing::info!("Administrators team created (id: {{}}) for first user (id: {{}})", team.id, user_model.inner.id),
                Err(e) => {{
                    tracing::error!("Failed to create Administrators team for first user {{}}: {{}}", user_model.inner.id, e);
                    return Err(ModelError::Message("Failed to set up initial administrator team.".to_string()));
                }}
            }}
        }}

        tracing::info!(user_id = user_model.inner.id, "user created successfully");
        Ok(user_model)
    }}

    /// Generate an API key for the user (method on Model).
    pub async fn generate_api_key(&self, db: &impl ConnectionTrait) -> ModelResult<String> {{
        let mut user_am: UserActiveModel = self.inner.clone().into(); // Use derived into()
        let new_key = Uuid::new_v4().to_string();
        user_am.api_key = Set(Some(new_key.clone()));
        // updated_at will be handled by ActiveModelBehavior::before_save
        let updated_entity = user_am.update(db).await.map_err(ModelError::from)?;
        Ok(updated_entity.api_key.unwrap_or_default())
    }}

    /// Generate a reset token and set the reset_sent_at time (method on Model).
    pub async fn generate_reset_token(&self, db: &impl ConnectionTrait) -> ModelResult<String> {{
        let mut user_am: UserActiveModel = self.inner.clone().into(); // Use derived into()
        let token = uuid::Uuid::new_v4().to_string();
        user_am.reset_token = Set(Some(token.clone()));
        user_am.reset_sent_at = Set(Some(chrono::Utc::now().into()));
        // updated_at will be handled by ActiveModelBehavior::before_save
        let _updated_entity = user_am.update(db).await.map_err(ModelError::from)?;
        Ok(token)
    }}

    /// Generate an email verification token and set the sent_at time (method on Model).
    pub async fn generate_email_verification_token(&self, db: &impl ConnectionTrait) -> ModelResult<String> {{
        let mut user_am: UserActiveModel = self.inner.clone().into(); // Use derived into()
        let token = uuid::Uuid::new_v4().to_string();
        user_am.email_verification_token = Set(Some(token.clone()));
        user_am.email_verification_sent_at = Set(Some(chrono::Utc::now().into()));
        // updated_at will be handled by ActiveModelBehavior::before_save
        let _updated_entity = user_am.update(db).await.map_err(ModelError::from)?;
        Ok(token)
    }}

    /// Verify the email verification token and mark email as verified (method on Model).
    pub async fn verify_email(&self, db: &impl ConnectionTrait, token: &str) -> ModelResult<Self> {{
        // Access via inner
        if self.inner.email_verification_token.as_ref() != Some(&token.to_string()) {{
            return Err(ModelError::Message("invalid token".to_string()));
        }}
        let mut user_am: UserActiveModel = self.inner.clone().into(); // Use derived into()
        user_am.email_verified_at = Set(Some(chrono::Utc::now().into()));
        user_am.email_verification_token = Set(None);
        // updated_at will be handled by ActiveModelBehavior::before_save
        let updated_entity = user_am.update(db).await.map_err(ModelError::from)?;
        Ok(Self::from(updated_entity))
    }}

    /// Generate a PGP verification token (method on Model).
    pub async fn generate_pgp_verification_token(&self, db: &impl ConnectionTrait) -> ModelResult<String> {{
        let mut user_am: UserActiveModel = self.inner.clone().into(); // Use derived into()
        let token = uuid::Uuid::new_v4().to_string();
        user_am.pgp_verification_token = Set(Some(token.clone()));
        // updated_at will be handled by ActiveModelBehavior::before_save
        let _updated_entity = user_am.update(db).await.map_err(ModelError::from)?;
        Ok(token)
    }}

    /// Verify the PGP verification token and mark PGP as verified (method on Model).
    pub async fn verify_pgp(&self, db: &impl ConnectionTrait, token: &str) -> ModelResult<Self> {{
        // Access via inner
        if self.inner.pgp_verification_token.as_ref() != Some(&token.to_string()) {{
            return Err(ModelError::Message("invalid token".to_string()));
        }}
        let mut user_am: UserActiveModel = self.inner.clone().into(); // Use derived into()
        user_am.pgp_verified_at = Set(Some(chrono::Utc::now().into()));
        user_am.pgp_verification_token = Set(None);
        // updated_at will be handled by ActiveModelBehavior::before_save
        let updated_entity = user_am.update(db).await.map_err(ModelError::from)?;
        Ok(Self::from(updated_entity))
    }}

    /// Update the user's password using the reset token (associated function).
    pub async fn reset_password(db: &impl ConnectionTrait, params: &ResetPasswordParams) -> ModelResult<Self> {{
        params.validate().map_err(|e| ModelError::Validation(ModelValidationErrors::from(e)))?;

        let user_entity = Entity::find().filter(Column::ResetToken.eq(params.reset_token.clone())).one(db).await?;
        let Some(user_entity) = user_entity else {{ return Err(ModelError::Message("invalid reset token".to_string())); }};

        if let Some(sent_at) = user_entity.reset_sent_at {{
            if sent_at + chrono::Duration::hours(1) < chrono::Utc::now() {{ return Err(ModelError::Message("token expired".to_string())); }}
        }} else {{ return Err(ModelError::Message("invalid reset token state".to_string())); }}

        // Password will be hashed in ActiveModelBehavior::before_save
        let mut user_am: UserActiveModel = user_entity.into(); // Use derived into() for entity
        user_am.password = Set(Some(params.password.clone()));
        user_am.reset_token = Set(None);
        user_am.reset_sent_at = Set(None);
        // updated_at will be handled by ActiveModelBehavior::before_save
        let updated_entity = user_am.update(db).await.map_err(ModelError::from)?;
        Ok(Self::from(updated_entity))
    }}

    /// Update the user's status (e.g., approve/reject) (method on Model).
    pub async fn update_status(&self, db: &impl ConnectionTrait, new_status: &str) -> ModelResult<Self> {{
        if ![USER_STATUS_NEW, USER_STATUS_APPROVED, USER_STATUS_REJECTED].contains(&new_status) {{
            return Err(ModelError::Message(format!("Invalid status: {{}}", new_status)));
        }}
        let mut user_am = self.inner.clone().into(); // Use derived into()
        user_am.status = Set(new_status.to_string());
        // updated_at will be handled by ActiveModelBehavior::before_save
        let updated_entity = user_am.update(db).await.map_err(ModelError::from)?;
        Ok(Self::from(updated_entity))
    }}

    /// Internal helper to find a user by a condition (associated function).
    async fn find_user<T>(db: &impl ConnectionTrait, condition: T) -> ModelResult<Self>
    where T: IntoCondition + Send, {{
        Entity::find().filter(condition).one(db).await?.map(Self::from).ok_or_else(|| ModelError::EntityNotFound)
    }}

    /// Check if the user is a system administrator (associated function).
    pub async fn is_system_admin(ctx: &AppContext, user: &Model) -> Result<bool> {{
        let admin_team_name = get_admin_team_name(ctx)?;
        let Some(admin_team) = teams::Entity::find().filter(teams::Column::Name.eq(&admin_team_name)).one(&ctx.db).await.map_err(|e| Error::Model(ModelError::Any(e.into())))? else {{
            tracing::warn!("Configured admin team '{{}}' not found in the database.", admin_team_name);
            return Ok(false);
        }};
        let admin_team_model = TeamModel::from(admin_team);
        // Use user.inner.id
        admin_team_model.has_role(&ctx.db, user.inner.id, vec![Role::Owner, Role::Admin]).await.map_err(Error::Model)
    }}

    /// Gather common parameters needed for the base layout template (method on Model).
    pub async fn get_layout_context(&self, ctx: &AppContext, active_page: &str) -> ModelResult<LayoutContext> {{
        let db = &ctx.db;
        // Access user id via inner
        let invitation_count = team_memberships::Entity::find()
            .filter(team_memberships::Column::UserId.eq(self.inner.id))
            .filter(team_memberships::Column::Pending.eq(true))
            .count(db).await? as i64;

        let is_system_admin = Model::is_system_admin(ctx, self).await.map_err(|e| {{
            tracing::error!("Failed to check system admin status during layout context build: {{}}", e);
            match e {{ Error::Model(model_err) => model_err, _ => ModelError::Message(e.to_string()) }}
        }})?;

        let pending_user_count = if is_system_admin {{
            Entity::find().filter(Column::Status.eq(USER_STATUS_NEW)).count(db).await?
        }} else {{ 0 }};

        let user_entity = self.inner.clone();
        Ok(LayoutContext {{
            user: user_entity,
            invitation_count,
            is_system_admin,
            active_page: active_page.to_string(),
            pending_user_count,
        }})
    }}

    /// Helper function to get all users, optionally filtered by status (associated function).
    pub async fn get_all(db: &impl ConnectionTrait, status_filter: Option<String>) -> ModelResult<Vec<Self>> {{
        let mut query = Entity::find().order_by_asc(Column::Id);
        if let Some(status) = status_filter {{
            query = query.filter(Column::Status.eq(status));
        }}
        query.all(db).await.map_err(ModelError::from).map(|entities| entities.into_iter().map(Self::from).collect())
    }}

    /// Find a user by their email verification token (associated function).
    pub async fn find_by_email_verification_token(db: &impl ConnectionTrait, token: &str) -> ModelResult<Self> {{
        Self::find_user(db, Column::EmailVerificationToken.eq(Some(token.to_string()))).await
    }}

    /// Generate a magic link token and set its expiration (method on Model).
    pub async fn generate_magic_link_token(&self, db: &impl ConnectionTrait) -> ModelResult<String> {{
        let mut user_am: UserActiveModel = self.inner.clone().into(); // Use derived into()
        let token = uuid::Uuid::new_v4().to_string();
        user_am.magic_link_token = Set(Some(token.clone()));
        user_am.magic_link_expiration = Set(Some((chrono::Utc::now() + chrono::Duration::minutes(15)).into()));
        // updated_at will be handled by ActiveModelBehavior::before_save
        let _updated_entity = user_am.update(db).await.map_err(ModelError::from)?;
        Ok(token)
    }}

    /// Find a user by their magic link token, checking expiration (associated function).
    pub async fn find_by_magic_link_token(db: &impl ConnectionTrait, token: &str) -> ModelResult<Self> {{
        let user_entity = Entity::find().filter(Column::MagicLinkToken.eq(Some(token.to_string()))).one(db).await?;
        let Some(user_entity) = user_entity else {{ return Err(ModelError::EntityNotFound); }};

        if let Some(expiration) = user_entity.magic_link_expiration {{
            if expiration < chrono::Utc::now() {{ return Err(ModelError::Message("Magic link token expired".to_string())); }}
        }} else {{ return Err(ModelError::Message("Invalid magic link token state".to_string())); }}

        Ok(Self::from(user_entity))
    }}
}}

// --- Helper Functions ---

/// Helper function to hash a password.
pub fn hash_password(password: &str) -> std::result::Result<String, ModelError> { // Made pub
    hash(password, DEFAULT_COST).map_err(|e| {{
        tracing::error!(error = ?e, "failed to hash password");
        ModelError::BcryptError(e)
    }})
}

/// Helper function to verify a password against a hash.
pub fn verify_password(hash: &str, password: &str) -> std::result::Result<(), ModelError> {{
    verify(password, hash).map_err(|e| match e {{
        BcryptError::InvalidHash(_) | BcryptError::InvalidPrefix(_) => {{
            tracing::error!(error = ?e, "password hash verification failed due to invalid hash format");
            ModelError::BcryptError(e)
        }}
        _ => {{
            tracing::debug!(error = ?e, "password hash verification failed (likely mismatch)");
            ModelError::Message("invalid email or password".to_string())
        }}
    }})
}}

/// Helper to get the configured admin team name.
fn get_admin_team_name(ctx: &AppContext) -> std::result::Result<String, loco_rs::config::Error> {{
    let admin_team_name_res: std::result::Result<String, loco_rs::config::Error> = ctx.config.get("app.admin_team_name");
    Ok(admin_team_name_res.unwrap_or_else(|e| {{
        if let loco_rs::config::Error::NotFound(_) = e {{
            tracing::debug!("'app.admin_team_name' not found in config. Using default 'Administrators'");
        }} else {{
            tracing::warn!("Failed to read 'app.admin_team_name' from config ({{}}). Using default 'Administrators'", e);
        }}
        "Administrators".to_string()
    }}))
}}


/// Convert a `Model` to a `DisplayUser` used by the auth middleware.
impl From<&Model> for DisplayUser {{
    fn from(user: &Model) -> Self {{
        Self {{
            // Access inner fields directly
            pid: user.inner.pid.to_string(),
            name: user.inner.name.clone(),
            email: user.inner.email.clone(),
            email_verified: user.inner.email_verified_at.is_some(),
        }}
    }}
}}

// Manual ActiveModelBehavior implementation for UserActiveModel
#[async_trait]
impl ActiveModelBehavior for UserActiveModel {
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        // Hash password before saving if it has been set/changed
        if let ActiveValue::Set(Some(password)) = &self.password {
            let hashed = hash_password(password).map_err(|e| DbErr::Custom(e.to_string()))?;
            self.password = Set(Some(hashed));
        } else if insert && self.password == ActiveValue::NotSet {
            // If password is NotSet during insert, error because it's required.
            return Err(DbErr::Custom("Password is required for new user".to_string()));
        }

        // Update `updated_at` timestamp before saving, unless it's explicitly set to Unchanged
        if self.updated_at != ActiveValue::Unchanged {{
            self.updated_at = Set(chrono::Utc::now().into());
        }}

        Ok(self)
    }

    // Add other ActiveModelBehavior methods if needed (before_delete, after_save, after_delete)
}
