// auth mailer
#![allow(non_upper_case_globals)]

use loco_rs::prelude::*;
use serde_json::json;

use crate::models::users;

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
}
