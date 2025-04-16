#![allow(non_upper_case_globals)]

use crate::models::users::{self, Model as UserModel};
use chrono::Local;
use loco_rs::{mailer::Mailer, prelude::*};
use sea_orm::{ActiveModelTrait, Set};
use sequoia_openpgp::{
    armor::{self, Kind},
    packet::Key,
    parse::Parse,
    policy::StandardPolicy,
    serialize::stream::{Encryptor2, LiteralWriter, Message},
    types::RevocationStatus,
    Cert,
};
use std::{io::Write, time::SystemTime};
use uuid::Uuid;

// Add necessary imports for lettre
use lettre::{message::header::ContentType, Message as LettreMessage, Transport};

static verification: Dir<'_> = include_dir!("src/mailers/gpg_verification/verification");

#[allow(clippy::module_name_repetitions)]
pub struct GpgVerificationMailer {}
impl Mailer for GpgVerificationMailer {}

impl GpgVerificationMailer {
    /// Send a GPG verification email
    pub async fn send_verification_email(ctx: &AppContext, user: &UserModel) -> Result<()> {
        let user_email = user.email.to_string();
        let gpg_key_text = match &user.gpg_key {
            Some(key) if !key.is_empty() => key,
            _ => return Err(Error::string("User does not have a GPG key configured")),
        };

        // Generate verification token and update user
        let token = Uuid::new_v4().to_string();
        let mut user_active: users::ActiveModel = user.clone().into();
        user_active.gpg_key_verification_token = Set(Some(token.clone()));
        user_active.gpg_key_verification_sent_at = Set(Some(Local::now().into()));
        user_active.update(&ctx.db).await.map_err(|db_err| {
            tracing::error!(error = ?db_err, "Failed to update user with GPG verification token");
            Error::string(&format!("DB Error: {}", db_err))
        })?;

        let verification_url = format!(
            "{}/users/verify-gpg/{}",
            ctx.config.server.full_url(),
            token
        );

        let cert = Cert::from_bytes(gpg_key_text.as_bytes()).map_err(|e| {
            tracing::error!(error = ?e, "Failed to parse user GPG key");
            Error::string(&format!("Invalid GPG key format: {}", e))
        })?;

        // Find a valid, usable key amalgamation for encryption from the Cert
        let policy = StandardPolicy::new();
        let now_system_time: SystemTime = SystemTime::now();
        let key_amalgamation = cert
            .keys()
            .with_policy(&policy, Some(now_system_time))
            .supported()
            .alive()
            // .revoked() // Filter later
            .for_transport_encryption()
            .last()
            .ok_or_else(|| {
                Error::string(
                    "No valid, alive, supported encryption key found in the GPG certificate",
                )
            })?;

        // Check revocation status explicitly
        let revocation_status = key_amalgamation.revocation_status();
        if !matches!(
            revocation_status,
            RevocationStatus::NotRevoked | RevocationStatus::NotAsFarAsWeKnow
        ) {
            tracing::warn!(key_id = ?key_amalgamation.key().keyid(), status = ?revocation_status, "Selected GPG key might be revoked");
            // Decide whether to proceed or return an error based on policy
            // return Err(Error::string("Selected GPG key is revoked"));
        }

        // Use TPK trait object for recipients
        let key_for_encryption = key_amalgamation.key();
        let recipients = vec![key_for_encryption];

        // Manually create email body
        let subject = "Verify your GPG Key".to_string();
        let text_body = format!(
            "Hello {},
\nPlease click the following link to verify your GPG key:\n{}\n\nThanks,\nYour Application Team",
            user.name, verification_url
        );

        // Encrypt the email body
        let mut encrypted_body_bytes = Vec::new();
        let message = Message::new(&mut encrypted_body_bytes);

        // Create the encrypting writer
        let message_writer = Encryptor2::for_recipients(message, recipients)
            .build()
            .map_err(|e| Error::string(&format!("Sequoia PGP encryptor build error: {}", e)))?;

        // Write the plaintext body to the encryptor
        let mut w = LiteralWriter::new(message_writer)
            .build()
            .map_err(|e| Error::string(&format!("Sequoia PGP writer build error: {}", e)))?;
        w.write_all(&text_body.as_bytes())
            .map_err(|e| Error::string(&format!("Sequoia PGP writer write error: {}", e)))?;
        w.finalize()
            .map_err(|e| Error::string(&format!("Sequoia PGP finalize error: {}", e)))?;

        // Armor the encrypted bytes
        let mut armored_message_bytes = Vec::new();
        let mut armor_writer = armor::Writer::new(&mut armored_message_bytes, Kind::Message)
            .map_err(|e| {
                Error::string(&format!("Sequoia PGP armor writer creation error: {}", e))
            })?;
        armor_writer
            .write_all(&encrypted_body_bytes)
            .map_err(|e| Error::string(&format!("Sequoia PGP armor write error: {}", e)))?;
        armor_writer
            .finalize()
            .map_err(|e| Error::string(&format!("Sequoia PGP armor finalize error: {}", e)))?;

        let armored_text = String::from_utf8(armored_message_bytes)
            .map_err(|e| Error::string(&format!("UTF8 conversion error: {}", e)))?;

        // Send the encrypted email using lettre directly
        let sender_email = match &ctx.config.mailer {
            Some(config) => match &config.smtp {
                Some(smtp_config) => smtp_config.username.clone(), // Use username as sender
                None => "noreply@example.com".to_string(),
            },
            None => "noreply@example.com".to_string(),
        };

        let email = LettreMessage::builder()
            .from(
                sender_email
                    .parse()
                    .map_err(|e| Error::string(&format!("Invalid sender email: {}", e)))?,
            )
            .to(user_email
                .parse()
                .map_err(|e| Error::string(&format!("Invalid recipient email: {}", e)))?)
            .subject(&subject)
            .header(ContentType::parse("text/plain; charset=utf-8").unwrap())
            .body(armored_text)
            .map_err(|e| Error::string(&format!("Failed to build email: {}", e)))?;

        // Use the pre-configured mailer transport from AppContext, handling Option
        match &ctx.mailer {
            Some(mailer_transport) => {
                mailer_transport // Call send directly on EmailSender
                    .send(&email)
                    .await
                    .map_err(|e| Error::string(&format!("Failed to send email: {}", e)))?;
            }
            None => {
                return Err(Error::string("Mailer transport not configured"));
            }
        }

        Ok(())
    }
}
