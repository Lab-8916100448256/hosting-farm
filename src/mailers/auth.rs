// auth mailer
#![allow(non_upper_case_globals)]

use crate::models::users;
use lettre::message::{header::ContentType, Mailbox, Message as LettreMessage};
use lettre::transport::smtp::authentication::Credentials;
use lettre::transport::smtp::client::{Tls, TlsParameters};
use lettre::transport::smtp::AsyncSmtpTransport;
use lettre::AsyncTransport; // Import transport trait
use lettre::Tokio1Executor;
use loco_rs::prelude::*;
use sequoia_openpgp::{
    cert::CertParser,
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
            "Hello {},\n\nPlease verify your PGP email sending capability by clicking the link below:\n{}\n\nThanks,\nThe Hosting Farm Team",
            user.name,
            verification_url
        );

        // 3. Encrypt the email body using Sequoia stream API (following example)
        let policy = &StandardPolicy::new();
        let mut sink = Vec::new();

        // Create the initial message sink targeting the output vector
        let message_sink = StreamMessage::new(&mut sink);

        // Wrap the sink in an Armorer
        let message_armorer = Armorer::new(message_sink)
            .build()
            .map_err(|e| Error::string(&format!("Failed to build Armorer: {}", e)))?;

        // Extract valid recipient keys from the Cert
        let recipients: Vec<Recipient> = recipient_cert
            .keys()
            .with_policy(policy, None) // Apply policy to filter keys
            .supported()
            .alive()
            .revoked(false)
            .for_transport_encryption() // Filter for keys usable for encryption
            .map(Into::into) // Convert valid keys/subkeys into Recipient
            .collect();

        if recipients.is_empty() {
            return Err(Error::string(
                "No valid PGP key found for encryption in the provided certificate.",
            ));
        }

        // Create an encryptor targeting the Armorer, for the collected recipients
        let message_encryptor = Encryptor::for_recipients(message_armorer, recipients)
            .build()
            .map_err(|e| Error::string(&format!("Failed to build encryptor: {}", e)))?;

        // Create a literal writer targeting the encryptor
        let mut literal_writer = LiteralWriter::new(message_encryptor)
            .build()
            .map_err(|e| Error::string(&format!("Failed to build literal writer: {}", e)))?;

        // Write the plaintext body to the literal writer
        literal_writer
            .write_all(email_body.as_bytes())
            .map_err(|e| Error::string(&format!("Failed to write encrypted message: {}", e)))?;

        // Finalize the literal writer to finish encryption/armoring
        literal_writer
            .finalize()
            .map_err(|e| Error::string(&format!("Failed to finalize literal writer: {}", e)))?;

        let encrypted_message_str =
            String::from_utf8(sink) // Use sink directly
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

        // Manually build and use lettre transport
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
}
