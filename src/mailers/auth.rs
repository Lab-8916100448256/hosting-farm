// auth mailer
#![allow(non_upper_case_globals)]

use loco_rs::prelude::*;
use serde_json::json;

use crate::models::users;

static welcome: Dir<'_> = include_dir!("src/mailers/auth/welcome");
static forgot: Dir<'_> = include_dir!("src/mailers/auth/forgot");
static magic_link: Dir<'_> = include_dir!("src/mailers/auth/magic_link");
static pgp_verification: Dir<'_> = include_dir!("src/mailers/auth/pgp_verification");
// #[derive(Mailer)] // -- disabled for faster build speed. it works. but lets
// move on for now.

#[allow(clippy::module_name_repetitions)]
pub struct AuthMailer {}
impl Mailer for AuthMailer {}
impl AuthMailer {
    /// Sends a PGP-encrypted email verification message to the user.
    ///
    /// # Errors
    ///
    /// Returns an error if the user does not have a valid PGP key, or if encryption or email sending fails.
    pub async fn send_pgp_verification(ctx: &AppContext, user: &users::Model) -> Result<()> {
        tracing::info!("Sending PGP verification email to {}", user.email);
        // Build the plaintext verification link for encryption
        let token = user
            .pgp_email_verification_token
            .as_ref()
            .ok_or_else(|| Error::string("Missing verification token"))?;
        let verification_url = format!("{}/auth/verify-pgp/{}", &ctx.config.server.host, token);
        let text_body = format!(
            "To verify your email via PGP, please click the following link:\n\n{}",
            verification_url
        );

        // Encrypt the text body
        use openpgp::parse::Parse;
        use openpgp::policy::StandardPolicy;
        use openpgp::serialize::stream::{Armorer, Encryptor, LiteralWriter, Message};
        use sequoia_openpgp as openpgp;
        use std::io::Write;

        let pgp_key = user
            .pgp_key
            .as_ref()
            .ok_or_else(|| Error::string("No PGP key found for user"))?;
        let cert = openpgp::Cert::from_bytes(pgp_key.as_bytes())
            .map_err(|_| Error::string("PGP parse error"))?;
        let policy = &StandardPolicy::new();
        let recipients: Vec<_> = cert
            .keys()
            .with_policy(policy, None)
            .supported()
            .alive()
            .revoked(false)
            .for_transport_encryption()
            .collect();
        if recipients.is_empty() {
            return Err(Error::string("No suitable encryption subkey found"));
        }
        let mut encrypted = Vec::new();
        {
            let message = Message::new(&mut encrypted);
            // ASCII armor the encrypted output
            let armorer = Armorer::new(message)
                .build()
                .map_err(|_| Error::string("Armorer error"))?;
            let encryptor = Encryptor::for_recipients(armorer, recipients)
                .build()
                .map_err(|_| Error::string("Encryptor error"))?;
            let mut literal_writer = LiteralWriter::new(encryptor)
                .build()
                .map_err(|_| Error::string("LiteralWriter error"))?;
            literal_writer
                .write_all(text_body.as_bytes())
                .map_err(|_| Error::string("Write error"))?;
            literal_writer
                .finalize()
                .map_err(|_| Error::string("Finalize error"))?;
        }
        let encrypted_text =
            String::from_utf8(encrypted).map_err(|_| Error::string("utf8 error"))?;

        // Build mail args for encryption template
        let locals = json!({
            "name": user.name,
            "verifyToken": user.pgp_email_verification_token,
            "domain": &ctx.config.server.host,
            "ciphertext": encrypted_text,
        });
        let mut args = mailer::Args {
            to: user.email.clone(),
            locals,
            from: None,
            ..Default::default()
        };
        // Set SMTP sender if configured
        if let Some(mailer_config) = &ctx.config.mailer {
            if let Some(smtp_config) = &mailer_config.smtp {
                if let Some(auth) = &smtp_config.auth {
                    args.from = Some(auth.user.clone());
                }
            }
        }
        tracing::debug!("PGP email args: {:?}", args);
        // Use template to send encrypted payload
        Self::mail_template(ctx, &pgp_verification, args).await?;
        tracing::info!("PGP verification email dispatched to {}", user.email);
        Ok(())
    }

    /// Sending welcome email the the given user
    ///
    /// # Errors
    ///
    /// When email sending is failed
    pub async fn send_welcome(ctx: &AppContext, user: &users::Model) -> Result<()> {
        tracing::info!("Sending welcome email to {}", user.email);
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
        tracing::debug!("Welcome email args: {:?}", args);
        Self::mail_template(ctx, &welcome, args).await?;
        tracing::info!("Welcome email dispatched to {}", user.email);
        Ok(())
    }

    /// Sending forgot password email
    ///
    /// # Errors
    ///
    /// When email sending is failed
    pub async fn forgot_password(ctx: &AppContext, user: &users::Model) -> Result<()> {
        tracing::info!("Sending forgot password email to {}", user.email);
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
        tracing::debug!("Forgot password email args: {:?}", args);
        Self::mail_template(ctx, &forgot, args).await?;
        tracing::info!("Forgot password email dispatched to {}", user.email);
        Ok(())
    }

    /// Sends a magic link authentication email to the user.
    ///
    /// # Errors
    ///
    /// When email sending is failed
    pub async fn send_magic_link(ctx: &AppContext, user: &users::Model) -> Result<()> {
        tracing::info!("Sending magic link email to {}", user.email);
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
        tracing::debug!("Magic link email args: {:?}", args);
        Self::mail_template(ctx, &magic_link, args).await?;
        tracing::info!("Magic link email dispatched to {}", user.email);
        Ok(())
    }
}
