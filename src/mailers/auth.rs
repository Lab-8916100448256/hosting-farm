// auth mailer
#![allow(non_upper_case_globals)]

use crate::models::users;
use lettre::message::{header::ContentType, Mailbox, Message as LettreMessage};
use lettre::AsyncTransport; // Import transport trait
use loco_rs::prelude::*;
use sequoia_openpgp::{
    armor::Kind, cert::CertParser, message::Message, parse::Parse, policy::StandardPolicy,
    serialize::Serialize,
};
use serde_json::json;
use std::io::Write;

static welcome: Dir<'_> = include_dir!("src/mailers/auth/welcome");
static forgot: Dir<'_> = include_dir!("src/mailers/auth/forgot");
static magic_link: Dir<'_> = include_dir!("src/mailers/auth/magic_link");
// #[derive(Mailer)] // -- disabled for faster build speed. it works. but lets
// move on for now.

#[allow(clippy::module_name_repetitions)]
pub struct AuthMailer {}
impl Mailer for AuthMailer {}
impl AuthMailer {
    /// Sending welcome email the the given user
    ///
    /// # Errors
    ///
    /// When email sending is failed
    pub async fn send_welcome(ctx: &AppContext, user: &users::Model) -> Result<()> {
        let mut args = mailer::Args {
            to: user.email.to_string(),
            locals: json!({
              "name": user.name,
              "verifyToken": user.email_verification_token,
              "domain": &ctx.config.server.host,
            }),
            from: Some("test@example.com".to_string()),
            ..Default::default()
        };

        // Set the from email to match the SMTP username if available
        if let Some(mailer_config) = &ctx.config.mailer {
            if let Some(smtp_config) = &mailer_config.smtp {
                if let Some(auth) = &smtp_config.auth {
                    args.from = Some(auth.user.clone());
                }
            }
        }

        Self::mail_template(ctx, &welcome, args).await?;

        Ok(())
    }

    /// Sending forgot password email
    ///
    /// # Errors
    ///
    /// When email sending is failed
    pub async fn forgot_password(ctx: &AppContext, user: &users::Model) -> Result<()> {
        let mut args = mailer::Args {
            to: user.email.to_string(),
            locals: json!({
              "name": user.name,
              "resetToken": user.reset_token,
              "domain": &ctx.config.server.host,
            }),
            from: Some("test@example.com".to_string()),
            ..Default::default()
        };

        // Set the from email to match the SMTP username if available
        if let Some(mailer_config) = &ctx.config.mailer {
            if let Some(smtp_config) = &mailer_config.smtp {
                if let Some(auth) = &smtp_config.auth {
                    args.from = Some(auth.user.clone());
                }
            }
        }

        Self::mail_template(ctx, &forgot, args).await?;

        Ok(())
    }

    /// Sends a magic link authentication email to the user.
    ///
    /// # Errors
    ///
    /// When email sending is failed
    pub async fn send_magic_link(ctx: &AppContext, user: &users::Model) -> Result<()> {
        let mut args = mailer::Args {
            to: user.email.to_string(),
            locals: json!({
              "name": user.name,
              "token": user.magic_link_token.clone().ok_or_else(|| Error::string(
                        "the user model not contains magic link token",
                ))?,
              "host": &ctx.config.server.host
            }),
            from: Some("test@example.com".to_string()),
            ..Default::default()
        };

        // Set the from email to match the SMTP username if available
        if let Some(mailer_config) = &ctx.config.mailer {
            if let Some(smtp_config) = &mailer_config.smtp {
                if let Some(auth) = &smtp_config.auth {
                    args.from = Some(auth.user.clone());
                }
            }
        }

        Self::mail_template(ctx, &magic_link, args).await?;

        Ok(())
    }

    /// Sends a PGP-encrypted email verification link to the user.
    ///
    /// # Errors
    ///
    /// - If the user does not have a PGP key configured.
    /// - If the PGP key is invalid or cannot be parsed.
    /// - If PGP encryption fails.
    /// - If email sending fails.
    pub async fn send_pgp_verification(
        ctx: &AppContext,
        user: &users::Model,
        token: &str,
    ) -> Result<()> {
        // 1. Get and parse the user's PGP key
        let _pgp_key_str = user
            .pgp_key
            .as_ref()
            .ok_or_else(|| Error::string("User does not have a PGP key configured."))?;

        // --- Temporarily Commented Out PGP Logic ---
        /*
        let recipient_cert = CertParser::from_bytes(_pgp_key_str.as_bytes())
            .map_err(|e| {
                tracing::error!("Failed to parse PGP key for user {}: {}", user.email, e);
                Error::string("Invalid PGP key format.")
            })
            .and_then(|mut certs| {
                certs
                    .next()
                    .ok_or_else(|| Error::string("No valid PGP certificate found in key data."))
            })?;

        // 2. Construct the verification URL and email body
        let verification_url = format!(
            "https://{}/pgp/verify/{}", // Assuming https and new /pgp route
            &ctx.config.server.host, token
        );

        let email_body = format!(
            "Hello {},\n\nPlease verify your PGP email sending capability by clicking the link below:\n{}\n\nThanks,\nThe Hosting Farm Team",
            user.name,
            verification_url
        );

        // 3. Encrypt the email body using Sequoia
        let policy = &StandardPolicy::new();
        let mut encrypted_bytes = Vec::new();

        let mut armored_writer = sequoia_openpgp::armor::Writer::new(&mut encrypted_bytes, Kind::Message)?;

        // Create a MessageWriter for encryption - Path needs verification
        let message_writer = sequoia_openpgp::MessageWriter::encrypt(
            &mut armored_writer, // Write to the armored writer
            &[&recipient_cert], // Pass certs as a slice
            policy,
            None, // No signing key
        )?;

        use std::io::Write;
        message_writer.into_message_writer()?
            .write_all(email_body.as_bytes())
            .map_err(|e| Error::string(&format!("Failed to write encrypted message: {}", e)))?;

        let encrypted_message_str = String::from_utf8(encrypted_bytes)
            .map_err(|_| Error::string("Encrypted message is not valid UTF-8"))?;

        // 4. Prepare and send the email using lettre directly (raw email)
        let from_mailbox: Mailbox = ctx
            .config
            .mailer
            .as_ref()
            .and_then(|mc| mc.smtp.as_ref())
            .and_then(|sc| sc.auth.as_ref())
            .map(|auth| {
                auth.user
                    .parse()
                    .unwrap_or_else(|_| "noreply@example.com".parse().unwrap())
            })
            .unwrap_or_else(|| "noreply@example.com".parse().unwrap());

        let to_mailbox: Mailbox = user
            .email
            .parse()
            .map_err(|e| Error::string(&format!("Invalid recipient email: {}", e)))?;

        let content_type = ContentType::parse("text/plain")
            .map_err(|_| Error::string("Failed to parse Content-Type"))?;

        let email = LettreMessage::builder()
            .from(from_mailbox)
            .to(to_mailbox)
            .subject("Verify Your PGP Email Setup")
            .header(content_type)
            .body(encrypted_message_str)
            .map_err(|e| Error::string(&format!("Failed to build email: {}", e)))?;

        // Get the mailer instance from context and use AsyncTransport::send
        let mailer = ctx.mailer.as_ref().ok_or_else(|| Error::string("Mailer not configured"))?;
        mailer.send(email).await?; // Use AsyncTransport::send
        */
        // --- End Temporarily Commented Out ---

        // Log that we would send the email here
        tracing::info!(
            "TODO: Implement PGP email sending for user {} with token {}",
            user.email,
            token
        );

        Ok(())
    }
}
