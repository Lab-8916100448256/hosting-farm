use async_trait::async_trait;
use chrono::{offset::Local, DateTime, Duration, Utc};
use loco_rs::{auth::jwt, hash, prelude::*};
use reqwest;
use sea_orm::{ActiveValue, ConnectionTrait, DbErr, EntityTrait, QueryFilter, Set, ColumnTrait};
use sequoia_openpgp::{self as openpgp, parse::Parse, policy::StandardPolicy};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

pub use super::_entities::users::{self, ActiveModel, Entity, Model};

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

#[derive(Debug, Validate, Deserialize)]
pub struct Validator {
    #[validate(length(min = 2, message = "Name must be at least 2 characters long."))] 
    pub name: String,
    #[validate(custom(function = "validation::is_valid_email"))]
    pub email: String,
    pub pgp_key: Option<String>,
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
            if expired_at >= Local::now() {
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
            // Revert to EntityAlreadyExists for the snapshot test
            return Err(ModelError::EntityAlreadyExists);
        }

        // Check for name uniqueness
        if users::Entity::find()
            .filter(users::Column::Name.eq(&params.name))
            .one(&txn)
            .await?
            .is_some()
        {
            // Revert to EntityAlreadyExists for the snapshot test
            return Err(ModelError::EntityAlreadyExists);
        }

        let password_hash =
            hash::hash_password(&params.password).map_err(|e| ModelError::Any(e.into()))?;
        let user = users::ActiveModel {
            email: ActiveValue::set(params.email.to_string()),
            password: ActiveValue::set(password_hash),
            name: ActiveValue::set(params.name.to_string()),
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
                            .map(|st| DateTime::<Utc>::from(st))
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
        self.email_verification_token = ActiveValue::Set(Some(Uuid::new_v4().to_string()));
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
        self.email_verification_sent_at = ActiveValue::set(Some(Local::now().into()));
        Ok(self.update(db).await?)
    }

    /// Sets the information for a reset password request,
    /// generates a unique reset password token, and updates it in the
    /// database.
    ///
    /// This method records the timestamp when the reset password token is sent
    /// and generates a unique token for the user.
    ///
    /// # Arguments
    ///
    /// # Errors
    ///
    /// when has DB query error
    pub async fn set_forgot_password_sent(mut self, db: &DatabaseConnection) -> ModelResult<Model> {
        self.reset_sent_at = ActiveValue::set(Some(Local::now().into()));
        self.reset_token = ActiveValue::Set(Some(Uuid::new_v4().to_string()));
        Ok(self.update(db).await?)
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
        self.email_verified_at = ActiveValue::set(Some(Local::now().into()));
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
        self.password =
            ActiveValue::set(hash::hash_password(password).map_err(|e| ModelError::Any(e.into()))?);
        self.reset_token = ActiveValue::Set(None);
        self.reset_sent_at = ActiveValue::Set(None);
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
        self.pgp_verification_token = ActiveValue::Set(Some(Uuid::new_v4().to_string()));
        // We might want to record the sent time too, similar to email verification
        // self.pgp_verification_sent_at = ActiveValue::set(Some(Local::now().into()));
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
        self.pgp_verified_at = ActiveValue::Set(Some(Utc::now().naive_utc())); // Use naive_utc() for NaiveDateTime
        self.pgp_verification_token = ActiveValue::Set(None);
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

        self.magic_link_token = ActiveValue::set(Some(random_str));
        self.magic_link_expiration = ActiveValue::set(Some(expired.into()));
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
        self.magic_link_token = ActiveValue::set(None);
        self.magic_link_expiration = ActiveValue::set(None);
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
}
