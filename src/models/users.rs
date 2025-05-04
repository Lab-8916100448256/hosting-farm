use async_trait::async_trait;
use chrono::{offset::Local, DateTime, Duration, Utc};
use loco_rs::{auth::jwt, hash, prelude::*};
use reqwest;
use sea_orm::{
    ActiveValue, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, PaginatorTrait, QueryFilter, Set,
};
use sequoia_openpgp::{self as openpgp, parse::Parse, policy::StandardPolicy};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

pub use super::_entities::users::{self, ActiveModel, Entity, Model};
use super::_entities::{team_memberships, teams};

pub const MAGIC_LINK_LENGTH: i8 = 32;
pub const MAGIC_LINK_EXPIRATION_MIN: i8 = 5;

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginParams {
    pub email: String,
    pub password: String,
    #[serde(rename = "remember-me")]
    pub remember_me: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ForgotPasswordParams {
    pub email: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResetPasswordParams {
    pub token: String,
    pub password: String,
    pub password_confirmation: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterParams {
    pub email: String,
    pub password: String,
    pub name: String,
    pub password_confirmation: String,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct UpdateDetailsParams {
    #[validate(length(min = 2, message = "Name must be at least 2 characters long."))]
    pub name: String,
    #[validate(email(message = "Invalid email format."))]
    pub email: String,
}

#[derive(Debug, Validate, Deserialize)]
pub struct Validator {
    #[validate(length(min = 2, message = "Name must be at least 2 characters long."))]
    pub name: String,
    #[validate(email(message = "Invalid email format."))]
    pub email: String,
    pub pgp_key: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct LayoutContext {
    pub is_app_admin: bool,
    pub invitation_count: i64,
    pub pending_user_count: u64,
}

impl Validatable for ActiveModel {
    fn validator(&self) -> Box<dyn Validate> {
        let name = match &self.name {
            ActiveValue::Set(n) | ActiveValue::Unchanged(n) => n.clone(),
            _ => "".to_string(),
        };
        let email = match &self.email {
            ActiveValue::Set(e) | ActiveValue::Unchanged(e) => e.clone(),
            _ => "".to_string(),
        };
        let pgp_key = match &self.pgp_key {
            ActiveValue::Set(pk) | ActiveValue::Unchanged(pk) => pk.clone(),
            _ => None,
        };

        Box::new(Validator {
            name,
            email,
            pgp_key,
        })
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for super::_entities::users::ActiveModel {
    async fn before_save<C>(self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        self.validate()?;
        if insert {
            let mut this = self;
            if matches!(this.pid, ActiveValue::NotSet) {
                this.pid = ActiveValue::Set(Uuid::new_v4());
            }
            if matches!(this.api_key, ActiveValue::NotSet) {
                this.api_key = ActiveValue::Set(format!("lo-{}", Uuid::new_v4()));
            }
            Ok(this)
        } else {
            Ok(self)
        }
    }
}

#[async_trait]
impl Authenticable for Model {
    async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(
                model::query::condition()
                    .eq(users::Column::ApiKey, api_key)
                    .build(),
            )
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    async fn find_by_claims_key(db: &DatabaseConnection, claims_key: &str) -> ModelResult<Self> {
        Self::find_by_pid(db, claims_key).await
    }
}

impl Model {
    /// finds a user by the provided email
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_email(db: &DatabaseConnection, email: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(users::Column::Email.eq(email))
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// finds a user by the provided name
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_name(db: &DatabaseConnection, user_name: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(users::Column::Name.eq(user_name))
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// finds a user by the provided verification token
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_verification_token(
        db: &DatabaseConnection,
        token: &str,
    ) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(users::Column::EmailVerificationToken.eq(token))
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// finds a user by the magic token and verify and token expiration
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error ot token expired
    pub async fn find_by_magic_token(db: &DatabaseConnection, token: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(users::Column::MagicLinkToken.eq(token))
            .one(db)
            .await?;

        let user = user.ok_or_else(|| ModelError::EntityNotFound)?;
        if let Some(expired_at) = user.magic_link_expiration {
            if expired_at.with_timezone(&Utc) >= Utc::now() {
                Ok(user)
            } else {
                tracing::debug!(
                    user_pid = user.pid.to_string(),
                    token_expiration = expired_at.to_string(),
                    "magic token expired for the user."
                );
                Err(ModelError::msg("magic token expired"))
            }
        } else {
            tracing::error!(
                user_pid = user.pid.to_string(),
                "magic link expiration time not exists"
            );
            Err(ModelError::msg("expiration token not exists"))
        }
    }

    /// finds a user by the provided reset token
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_reset_token(db: &DatabaseConnection, token: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(users::Column::ResetToken.eq(token))
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// finds a user by the provided pid
    ///
    /// # Errors
    ///
    /// When could not find user  or DB query error
    pub async fn find_by_pid(db: &DatabaseConnection, pid: &str) -> ModelResult<Self> {
        let parse_uuid = Uuid::parse_str(pid).map_err(|e| ModelError::Any(e.into()))?;
        let user = users::Entity::find()
            .filter(users::Column::Pid.eq(parse_uuid))
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// finds a user by the provided api key
    ///
    /// # Errors
    ///
    /// When could not find user by the given token or DB query error
    pub async fn find_by_api_key(db: &DatabaseConnection, api_key: &str) -> ModelResult<Self> {
        let user = users::Entity::find()
            .filter(users::Column::ApiKey.eq(api_key))
            .one(db)
            .await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// Verifies whether the provided plain password matches the hashed password
    ///
    /// # Errors
    ///
    /// when could not verify password
    #[must_use]
    pub fn verify_password(&self, password: &str) -> bool {
        hash::verify_password(password, &self.password)
    }

    /// Asynchronously creates a user with a password and saves it to the
    /// database.
    ///
    /// # Errors
    ///
    /// When could not save the user into the DB or if email/name is not unique
    pub async fn create_with_password(
        db: &DatabaseConnection,
        params: &RegisterParams,
    ) -> ModelResult<Self> {
        let txn = db.begin().await?;

        // Check for email uniqueness
        if users::Entity::find()
            .filter(users::Column::Email.eq(&params.email))
            .one(&txn)
            .await?
            .is_some()
        {
            return Err(ModelError::Message("Email already registered.".to_string()));
        }

        // Check for name uniqueness
        if users::Entity::find()
            .filter(users::Column::Name.eq(params.name.trim())) // Trim username for the check
            .one(&txn)
            .await?
            .is_some()
        {
            return Err(ModelError::Message(
                "Username already taken. Please choose another.".to_string(),
            ));
        }

        let password_hash =
            hash::hash_password(&params.password).map_err(|e| ModelError::Any(e.into()))?;
        let user = users::ActiveModel {
            email: ActiveValue::set(params.email.to_string()),
            password: ActiveValue::set(password_hash),
            name: ActiveValue::set(params.name.trim().to_string()),
            ..Default::default()
        }
        .insert(&txn)
        .await?;

        txn.commit().await?;

        Ok(user)
    }

    /// Creates a JWT
    ///
    /// # Errors
    ///
    /// when could not convert user claims to jwt token
    pub fn generate_jwt(&self, secret: &str, expiration: &u64) -> ModelResult<String> {
        Ok(jwt::JWT::new(secret).generate_token(
            *expiration,
            self.pid.to_string(),
            serde_json::Map::new(),
        )?)
    }

    /// finds a user by the provided id
    ///
    /// # Errors
    ///
    /// When could not find user or DB query error
    pub async fn find_by_id(db: &DatabaseConnection, id: i32) -> ModelResult<Self> {
        let user = users::Entity::find_by_id(id).one(db).await?;
        user.ok_or_else(|| ModelError::EntityNotFound)
    }

    /// Parses the user's PGP key string.
    fn parse_pgp_key(key_str: &str) -> openpgp::Result<openpgp::Cert> {
        openpgp::Cert::from_bytes(key_str.as_bytes())
    }

    /// Parses the user's PGP key and returns its fingerprint.
    pub fn pgp_fingerprint(&self) -> Option<String> {
        self.pgp_key
            .as_ref()
            .and_then(|key_str| match Self::parse_pgp_key(key_str) {
                Ok(cert) => Some(cert.fingerprint().to_hex()),
                Err(e) => {
                    tracing::warn!("Failed to parse PGP key for fingerprint: {}", e);
                    None
                }
            })
    }

    /// Parses the user's PGP key and returns its expiration date string.
    pub fn pgp_validity(&self) -> Option<String> {
        self.pgp_key
            .as_ref()
            .and_then(|key_str| match Self::parse_pgp_key(key_str) {
                Ok(cert) => {
                    let policy = &StandardPolicy::new();
                    match cert.with_policy(policy, None) {
                        Ok(cert_verifier) => cert_verifier
                            .primary_key()
                            .key_expiration_time()
                            .map(DateTime::<Utc>::from)
                            .map(|dt| dt.format("%Y-%m-%d").to_string()),
                        Err(e) => {
                            tracing::warn!("Failed policy check for PGP key validity: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse PGP key for validity: {}", e);
                    None
                }
            })
    }

    /// Helper function to get the admin team name from config
    /// Moved from teams_pages.rs
    pub fn get_admin_team_name(ctx: &AppContext) -> String {
        ctx.config
            .settings
            .as_ref() // Get Option<&Value>
            .and_then(|settings| settings.get("app")) // Get Option<&Value> for "app" key
            .and_then(|app_settings| app_settings.get("admin_team_name")) // Get Option<&Value> for "admin_team_name"
            .and_then(|value| value.as_str()) // Get Option<&str>
            .map(|s| s.to_string()) // Convert to Option<String>
            .unwrap_or_else(|| {
                tracing::warn!(
                    "'app.admin_team_name' not found or not a string in config, using default 'Administrators'"
                );
                "Administrators".to_string()
            })
    }

    /// Checks if a given user is a member of the configured administrator team.
    ///
    /// # Arguments
    ///
    /// * `db` - The database connection.
    /// * `ctx` - The application context to read configuration.
    ///
    /// # Errors
    ///
    /// Returns a `ModelError` if database queries fail.
    pub async fn is_admin(&self, db: &DatabaseConnection, ctx: &AppContext) -> ModelResult<bool> {
        let admin_team_name = Self::get_admin_team_name(ctx);

        // Find the admin team by name
        let admin_team = teams::Entity::find()
            .filter(teams::Column::Name.eq(admin_team_name))
            .one(db)
            .await?;

        if let Some(team) = admin_team {
            // Check if the user is a member of this team (non-pending)
            let is_member = team_memberships::Entity::find()
                .filter(team_memberships::Column::TeamId.eq(team.id))
                .filter(team_memberships::Column::UserId.eq(self.id))
                .filter(team_memberships::Column::Pending.eq(false))
                .count(db) // Use count for efficiency
                .await?
                > 0;
            Ok(is_member)
        } else {
            // Admin team not found, so user cannot be an admin
            Ok(false)
        }
    }

    /// Updates the profile details (name, email) for a user.
    /// Handles validation, email uniqueness check, and resetting verification status if email changes.
    ///
    /// # Arguments
    ///
    /// * `db` - The database connection.
    /// * `params` - The new details to update.
    ///
    /// # Errors
    ///
    /// Returns a `ModelError` if validation fails, email is already taken, or database update fails.
    pub async fn update_profile_details(
        &self,
        db: &DatabaseConnection,
        params: &UpdateDetailsParams,
    ) -> ModelResult<Self> {
        // Handle validation error mapping correctly
        params
            .validate()
            .map_err(|e| ModelError::Message(e.to_string()))?;

        let mut active_user: ActiveModel = self.clone().into();
        let mut email_changed = false;

        let new_name = params.name.trim().to_string();
        let new_email = params.email.trim().to_lowercase();

        if self.name != new_name {
            active_user.name = Set(new_name);
        }

        if self.email != new_email {
            let existing_user = Entity::find()
                .filter(users::Column::Email.eq(&new_email))
                .filter(users::Column::Id.ne(self.id))
                .one(db)
                .await?;

            if existing_user.is_some() {
                return Err(ModelError::Message(
                    "Email address is already in use.".to_string(),
                ));
            }

            active_user.email = Set(new_email);
            active_user.email_verified_at = Set(None);
            active_user.email_verification_token = Set(None);
            active_user.email_verification_sent_at = Set(None);
            active_user.pgp_key = Set(None);
            active_user.pgp_verified_at = Set(None);
            active_user.pgp_verification_token = Set(None);

            email_changed = true;
        }

        let updated_user = active_user.update(db).await?;

        if email_changed {
            tracing::info!(
                user_pid = updated_user.pid.to_string(),
                "User email changed, verification status reset."
            );
        }

        Ok(updated_user)
    }

    /// Generates and saves a password reset token and its expiration time for the user.
    ///
    /// # Arguments
    ///
    /// * `db` - The database connection.
    ///
    /// # Errors
    ///
    /// Returns a `ModelError` if the database update fails.
    pub async fn initiate_password_reset(&self, db: &DatabaseConnection) -> ModelResult<Self> {
        let mut user_model: ActiveModel = self.clone().into();
        let token = Uuid::new_v4().to_string();
        user_model.reset_token = ActiveValue::Set(Some(token));
        user_model.reset_sent_at = ActiveValue::Set(Some(Utc::now().into()));
        user_model.update(db).await.map_err(Into::into)
    }

    pub async fn get_base_layout_context(
        &self,
        db: &DatabaseConnection,
        ctx: &AppContext,
    ) -> LayoutContext {
        let is_admin = self.is_admin(db, ctx).await.unwrap_or(false);

        // Get pending invitations count
        let invitation_count = team_memberships::Entity::find()
            .filter(team_memberships::Column::UserId.eq(self.id))
            .filter(team_memberships::Column::Pending.eq(true))
            .count(db)
            .await
            .unwrap_or(0);

        LayoutContext {
            is_app_admin: is_admin,
            invitation_count: invitation_count as i64,
            pending_user_count: 0,
        }
    }
}

impl ActiveModel {
    /// Sets the email verification token for the user and
    /// updates it in the database.
    ///
    /// This method is used to generate a unique verification token for the user.
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn generate_email_verification_token(
        mut self,
        db: &DatabaseConnection,
    ) -> ModelResult<Model> {
        let token = Uuid::new_v4().to_string();
        self.email_verification_token = Set(Some(token));
        self.email_verification_sent_at = Set(None);
        Ok(self.update(db).await?)
    }

    /// Sets the email verification send timestamp for the user and
    /// updates it in the database.
    ///
    /// This method is used to record the timestamp when the email verification
    /// was sent.
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn set_email_verification_sent(
        mut self,
        db: &DatabaseConnection,
    ) -> ModelResult<Model> {
        self.email_verification_sent_at = Set(Some(Utc::now().into()));
        Ok(self.update(db).await?)
    }

    /// Sets the forgot password sent timestamp.
    pub async fn set_forgot_password_sent(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.reset_sent_at = Set(Some(Utc::now().into()));
        self.update(db).await.map_err(Into::into)
    }

    /// Records the verification time when a user verifies their
    /// email and updates it in the database.
    ///
    /// This method sets the timestamp when the user successfully verifies their
    /// email.
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn verified(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.email_verified_at = Set(Some(Utc::now().into()));
        self.email_verification_token = Set(None);
        Ok(self.update(db).await?)
    }

    /// Resets the current user password with a new password and
    /// updates it in the database.
    ///
    /// This method hashes the provided password and sets it as the new password
    /// for the user.
    ///
    /// # Errors
    ///
    /// when has DB query error or could not hashed the given password
    pub async fn reset_password(
        mut self,
        db: &DatabaseConnection,
        password: &str,
    ) -> ModelResult<Model> {
        self.password = Set(hash::hash_password(password).map_err(|e| ModelError::Any(e.into()))?);
        self.reset_token = Set(None);
        self.reset_sent_at = Set(None);
        Ok(self.update(db).await?)
    }

    /// Sets the PGP verification token for the user and
    /// updates it in the database.
    ///
    /// This method is used to generate a unique PGP verification token for the user.
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn generate_pgp_verification_token(
        mut self,
        db: &DatabaseConnection,
    ) -> ModelResult<Model> {
        self.pgp_verification_token = Set(Some(Uuid::new_v4().to_string()));
        Ok(self.update(db).await?)
    }

    /// Records the PGP verification time when a user verifies their
    /// PGP communication capability and updates it in the database.
    ///
    /// This method sets the timestamp when the user successfully verifies their
    /// PGP email setup and clears the verification token.
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn set_pgp_verified(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.pgp_verified_at = Set(Some(Utc::now().naive_utc()));
        self.pgp_verification_token = Set(None);
        Ok(self.update(db).await?)
    }

    /// Creates a magic link token for passwordless authentication.
    ///
    /// Generates a random token with a specified length and sets an expiration time
    /// for the magic link. This method is used to initiate the magic link authentication flow.
    ///
    /// # Errors
    /// - Returns an error if database update fails
    pub async fn create_magic_link(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        let random_str = hash::random_string(MAGIC_LINK_LENGTH as usize);
        let expired = Local::now() + Duration::minutes(MAGIC_LINK_EXPIRATION_MIN.into());

        self.magic_link_token = Set(Some(random_str));
        self.magic_link_expiration = Set(Some(expired.into()));
        Ok(self.update(db).await?)
    }

    /// Verifies and invalidates the magic link after successful authentication.
    ///
    /// Clears the magic link token and expiration time after the user has
    /// successfully authenticated using the magic link.
    ///
    /// # Errors
    /// - Returns an error if database update fails
    pub async fn clear_magic_link(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.magic_link_token = Set(None);
        self.magic_link_expiration = Set(None);
        Ok(self.update(db).await?)
    }

    /// Fetches PGP key from keys.openpgp.org based on the user's email,
    /// validates it, and updates the user record.
    /// Returns the updated Model if successful.
    pub async fn fetch_and_update_pgp_key(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        let email = match &self.email {
            ActiveValue::Set(e) | ActiveValue::Unchanged(e) => e.clone(),
            _ => return Err(ModelError::msg("User email not set in ActiveModel")),
        };
        let user_pid_str = match &self.pid {
            ActiveValue::Set(p) | ActiveValue::Unchanged(p) => p.to_string(),
            _ => "unknown_pid".to_string(),
        };

        let pgp_key_url = format!("https://keys.openpgp.org/vks/v1/by-email/{}", email);
        let mut key_found_and_valid = false;
        let mut fetched_key_text: Option<String> = None;

        tracing::debug!(user_email = %email, url = %pgp_key_url, "Attempting PGP key lookup");

        match reqwest::get(&pgp_key_url).await {
            Ok(response) => {
                if response.status().is_success() {
                    match response.text().await {
                        Ok(key_text) => {
                            if !key_text.is_empty() {
                                // Basic validation: Check if it looks like an armored key
                                // More robust validation happens on parsing
                                if key_text.contains("-----BEGIN PGP PUBLIC KEY BLOCK-----") {
                                    // Further validate by trying to parse it
                                    match openpgp::Cert::from_bytes(key_text.as_bytes()) {
                                        Ok(_) => {
                                            key_found_and_valid = true;
                                            fetched_key_text = Some(key_text);
                                            tracing::info!(user_email = %email, "Successfully found and validated PGP key.");
                                        }
                                        Err(parse_err) => {
                                            tracing::warn!(user_email = %email, error = ?parse_err, "Found key text, but failed to parse as valid PGP key.");
                                        }
                                    }
                                } else {
                                    tracing::info!(user_email = %email, "No valid PGP key found on keyserver (invalid response format).");
                                }
                            } else {
                                tracing::info!(user_email = %email, "No PGP key found on keyserver (empty response).");
                            }
                        }
                        Err(text_err) => {
                            tracing::warn!(user_email = %email, error = ?text_err, "Failed to read PGP key response body.");
                        }
                    }
                } else if response.status() == reqwest::StatusCode::NOT_FOUND {
                    tracing::info!(user_email = %email, "No PGP key found on keyserver (404).");
                } else {
                    tracing::warn!(user_email = %email, status = ?response.status(), "Unexpected status code when fetching PGP key.");
                }
            }
            Err(req_err) => {
                // Log error but don't fail the whole operation, just proceed without updating the key
                tracing::error!(user_email = %email, error = ?req_err, "Failed to query PGP keyserver.");
            }
        }

        // Update the model only if a valid key was found
        if key_found_and_valid {
            self.pgp_key = Set(fetched_key_text);
            tracing::debug!(user_pid=%user_pid_str, "Updating user PGP key in database.");
            self.update(db).await.map_err(ModelError::DbErr)
        } else {
            tracing::debug!(user_pid=%user_pid_str, "No valid PGP key found/fetched, skipping database update.");
            // Let's update self.pgp_key to None if not found/invalid.
            // This makes the state consistent with the fetch result.
            self.pgp_key = Set(None);
            tracing::debug!(user_pid=%user_pid_str, "Setting user PGP key to None in database.");
            self.update(db).await.map_err(ModelError::DbErr)
        }
    }

    /// Sets the password for the user, hashing it before saving.
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn set_password(
        mut self,
        db: &DatabaseConnection,
        password: &str,
    ) -> ModelResult<Model> {
        self.password = Set(hash::hash_password(password).map_err(|e| ModelError::Any(e.into()))?);
        Ok(self.update(db).await?)
    }
}
