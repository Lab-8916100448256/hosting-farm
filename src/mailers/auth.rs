// auth mailer
#![allow(non_upper_case_globals)]

use crate::models::users;
use lettre::AsyncTransport; // Import transport trait
use lettre::Tokio1Executor;
use lettre::message::{Mailbox, Message as LettreMessage, header::ContentType};
use lettre::transport::smtp::AsyncSmtpTransport;
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use loco_rs::prelude::*;
use sequoia_openpgp::{
    cert::{Cert, CertParser},
    parse::Parse,
    policy::StandardPolicy,
    // Import stream components and Recipient
    serialize::stream::{Armorer, Encryptor, LiteralWriter, Message as StreamMessage, Recipient},
    // Remove unused Serialize import if Message is not used directly
    // serialize::Serialize,
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
        if let Some(mailer_config) = &ctx.config.mailer
            && let Some(smtp_config) = &mailer_config.smtp
            && let Some(auth) = &smtp_config.auth
        {
            args.from = Some(auth.user.clone());
        }

        Self::mail_template(ctx, &welcome, args).await?;

        Ok(())
    }

    /// Sending forgot password email, PGP-encrypted if possible.
    ///
    /// If the user has a verified PGP key, this function attempts to send
    /// the password reset email encrypted. If encryption or sending fails,
    /// or if the user has no verified key, it falls back to sending a
    /// standard unencrypted email.
    ///
    /// # Errors
    ///
    /// Returns an error only if the fallback unencrypted email sending fails.
    /// PGP-related errors are logged but do not prevent the fallback.
    pub async fn forgot_password(ctx: &AppContext, user: &users::Model) -> Result<()> {
        // Check if user has a PGP key and it's verified
        if user.pgp_key.is_some() && user.pgp_verified_at.is_some() {
            match Self::_try_send_forgot_password_pgp(ctx, user).await {
                Ok(_) => {
                    tracing::info!(
                        "Successfully sent PGP-encrypted password reset email to {}",
                        user.email
                    );
                    return Ok(()); // PGP email sent successfully
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to send PGP-encrypted password reset to {}: {}. Falling back to unencrypted.",
                        user.email,
                        e
                    );
                    // Fall through to send unencrypted email
                }
            }
        } else {
            tracing::info!(
                "User {} does not have a verified PGP key. Sending unencrypted password reset.",
                user.email
            );
            // Fall through to send unencrypted email
        }

        // Fallback: Send standard unencrypted email
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

        if let Some(mailer_config) = &ctx.config.mailer
            && let Some(smtp_config) = &mailer_config.smtp
            && let Some(auth) = &smtp_config.auth
        {
            args.from = Some(auth.user.clone());
        }

        Self::mail_template(ctx, &forgot, args).await?;
        Ok(())
    }

    /// Attempts to send the forgot password email PGP-encrypted.
    async fn _try_send_forgot_password_pgp(ctx: &AppContext, user: &users::Model) -> Result<()> {
        // 1. Get and parse PGP key
        let pgp_key_str = user
            .pgp_key
            .as_ref()
            .ok_or_else(|| Error::string("User PGP key is None, shouldn't happen here"))?;

        let recipient_cert = CertParser::from_bytes(pgp_key_str.as_bytes())
            .map_err(|e| Error::string(&format!("Failed to parse PGP key: {}", e)))
            .and_then(|mut certs| {
                certs
                    .next()
                    .ok_or_else(|| Error::string("No valid PGP certificate found in key data."))
            })?
            .map_err(|e| Error::string(&format!("Failed to parse PGP certificate: {}", e)))?;

        // 2. Construct plain text body
        let reset_url = format!(
            "{}/auth/reset-password/{}",
            &ctx.config.server.host,
            user.reset_token.as_ref().unwrap_or(&String::new()) // Should always exist if called correctly
        );
        let email_body = format!(
            "Hello {},

You requested a password reset. Click the link below:
{}

If you didn't request this, please ignore this email.

Thanks,
Your Hosting Farm Server",
            user.name, reset_url
        );

        // 3. Encrypt body
        let encrypted_body = Self::_encrypt_message_pgp(&recipient_cert, &email_body)?;

        // 4. Prepare email details
        let from_mailbox: Mailbox = Self::_get_from_mailbox(ctx)?;
        let to_mailbox: Mailbox = user
            .email
            .parse()
            .map_err(|e| Error::string(&format!("Invalid recipient email: {}", e)))?;

        // 5. Send raw email
        Self::_send_raw_email(
            ctx,
            to_mailbox,
            from_mailbox,
            "Password Reset Request",
            encrypted_body,
        )
        .await
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
        if let Some(mailer_config) = &ctx.config.mailer
            && let Some(smtp_config) = &mailer_config.smtp
            && let Some(auth) = &smtp_config.auth
        {
            args.from = Some(auth.user.clone());
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
        let pgp_key_str = user
            .pgp_key
            .as_ref()
            .ok_or_else(|| Error::string("User does not have a PGP key configured."))?;

        let recipient_cert = CertParser::from_bytes(pgp_key_str.as_bytes())
            .map_err(|e| {
                tracing::error!("Failed to parse PGP key for user {}: {}", user.email, e);
                Error::string("Invalid PGP key format.")
            })
            .and_then(|mut certs| {
                certs
                    .next()
                    .ok_or_else(|| Error::string("No valid PGP certificate found in key data."))
            })?
            .map_err(|e| {
                tracing::error!("Failed to parse PGP key for user {}: {}", user.email, e);
                Error::string("Invalid PGP key format.")
            })?;

        // 2. Construct the verification URL and email body
        let verification_url = format!(
            "{}/pgp/verify/{}", // Assuming https and new /pgp route
            &ctx.config.server.host, token
        );

        let email_body = format!(
            "Hello {},

Please verify your PGP email sending capability by clicking the link below:
{}

Thanks,
Your Hosting Farm Server",
            user.name, verification_url
        );

        // 3. Encrypt the email body using the helper function
        let encrypted_message_str = Self::_encrypt_message_pgp(&recipient_cert, &email_body)?;

        // 4. Prepare email details
        let from_mailbox: Mailbox = Self::_get_from_mailbox(ctx)?;
        let to_mailbox: Mailbox = user
            .email
            .parse()
            .map_err(|e| Error::string(&format!("Invalid recipient email: {}", e)))?;

        // 5. Send the email using the helper function
        Self::_send_raw_email(
            ctx,
            to_mailbox,
            from_mailbox,
            "Verify Your PGP Email Setup",
            encrypted_message_str,
        )
        .await?;

        Ok(())
    }

    // --- Private Helper Functions ---

    /// Encrypts a plaintext message using PGP for a given recipient certificate.
    /// Returns the armored PGP message as a String.
    #[allow(clippy::result_large_err)]
    fn _encrypt_message_pgp(recipient_cert: &Cert, plaintext_body: &str) -> Result<String> {
        let policy = &StandardPolicy::new();
        let mut sink = Vec::new();

        let message_sink = StreamMessage::new(&mut sink);
        let message_armorer = Armorer::new(message_sink)
            .build()
            .map_err(|e| Error::string(&format!("Failed to build Armorer: {}", e)))?;

        let recipients: Vec<Recipient> = recipient_cert
            .keys()
            .with_policy(policy, None)
            .supported()
            .alive()
            .revoked(false)
            .for_transport_encryption()
            .map(Into::into)
            .collect();

        if recipients.is_empty() {
            return Err(Error::string(
                "No valid PGP key found for encryption in the provided certificate.",
            ));
        }

        let message_encryptor = Encryptor::for_recipients(message_armorer, recipients)
            .build()
            .map_err(|e| Error::string(&format!("Failed to build encryptor: {}", e)))?;

        let mut literal_writer = LiteralWriter::new(message_encryptor)
            .build()
            .map_err(|e| Error::string(&format!("Failed to build literal writer: {}", e)))?;

        literal_writer
            .write_all(plaintext_body.as_bytes())
            .map_err(|e| Error::string(&format!("Failed to write encrypted message: {}", e)))?;

        literal_writer
            .finalize()
            .map_err(|e| Error::string(&format!("Failed to finalize literal writer: {}", e)))?;

        String::from_utf8(sink).map_err(|_| Error::string("Encrypted message is not valid UTF-8"))
    }

    /// Sends a raw email (e.g., PGP encrypted) using the SMTP configuration from the context.
    async fn _send_raw_email(
        ctx: &AppContext,
        to: Mailbox,
        from: Mailbox,
        subject: &str,
        body: String, // Takes ownership of the body string
    ) -> Result<()> {
        let content_type = ContentType::parse("text/plain")
            .map_err(|_| Error::string("Failed to parse Content-Type"))?;

        let email = LettreMessage::builder()
            .from(from.clone()) // Clone `from` as builder consumes it
            .to(to)
            .subject(subject)
            .header(content_type)
            .body(body) // Body is moved here
            .map_err(|e| Error::string(&format!("Failed to build email: {}", e)))?;

        // Build and use lettre transport
        let mailer_config = ctx
            .config
            .mailer
            .as_ref()
            .ok_or_else(|| Error::string("Mailer configuration missing"))?;
        let smtp_config = mailer_config
            .smtp
            .as_ref()
            .ok_or_else(|| Error::string("SMTP configuration missing"))?;
        let smtp_auth = smtp_config
            .auth
            .as_ref()
            .ok_or_else(|| Error::string("SMTP auth configuration missing"))?;

        let smtp_host = &smtp_config.host;
        let smtp_port = smtp_config.port;
        let smtp_user = &smtp_auth.user;
        let smtp_pass = &smtp_auth.password;

        let creds = Credentials::new(smtp_user.to_string(), smtp_pass.to_string());

        let tls_params = TlsParameters::builder(smtp_host.to_string())
            .build()
            .map_err(|e| Error::string(&format!("Failed to build TLS parameters: {}", e)))?;

        // Use starttls_relay for broader compatibility, adjust if needed
        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(smtp_host)
            .map_err(|e| Error::string(&format!("Failed to build SMTP transport: {}", e)))?
            .port(smtp_port)
            .credentials(creds)
            .tls(Tls::Required(tls_params))
            .build();

        transport
            .send(email)
            .await
            .map_err(|e| Error::string(&format!("Failed to send email via SMTP: {}", e)))?;

        Ok(())
    }

    /// Gets the 'From' mailbox address from the AppContext configuration.
    #[allow(clippy::result_large_err)]
    fn _get_from_mailbox(ctx: &AppContext) -> Result<Mailbox> {
        ctx.config
            .mailer
            .as_ref()
            .and_then(|mc| mc.smtp.as_ref())
            .and_then(|sc| sc.auth.as_ref())
            .map(|auth| {
                auth.user
                    .parse::<Mailbox>()
                    .map_err(|e| Error::string(&format!("Invalid sender email format: {}", e)))
            })
            .unwrap_or_else(|| {
                // Fallback if config is missing
                "noreply@example.com"
                    .parse::<Mailbox>()
                    .map_err(|e| Error::string(&format!("Invalid default sender email: {}", e)))
            })
    }
}
