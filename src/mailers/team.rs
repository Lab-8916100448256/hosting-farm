use loco_rs::{environment::Environment, prelude::*};
use serde_json::json;

use crate::models::_entities::{teams::Model as TeamModel, users::Model as UserModel};

// Define the static template directory
static INVITATION: Dir<'_> = include_dir!("src/mailers/team/invitation");

pub struct TeamMailer {}
impl Mailer for TeamMailer {}

impl TeamMailer {
    /// Send a team invitation email
    pub async fn send_invitation(
        ctx: &AppContext,
        inviting_user: &UserModel,
        invited_user: &UserModel,
        team: &TeamModel,
    ) -> Result<()> {
        let invited_email = invited_user.email.clone();

        // Get frontend URL from config
        let frontend_url = &ctx.config.server.host;

        // Create the locals JSON for template rendering
        let locals = json!({
            "name": invited_user.name,
            "other_user": inviting_user.name,
            "team_name": team.name,
            "invitation_url": format!("{}/users/invitations", frontend_url)
        });

        // Check if mailer is configured
        if ctx.mailer.is_none() {
            tracing::warn!(
                "Mailer not configured, skipping email delivery to {}",
                invited_email
            );
            return Ok(());
        }

        let mut args = mailer::Args {
            to: invited_email.clone(),
            locals,
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

        match Self::mail_template(ctx, &INVITATION, args).await {
            Ok(_) => {
                tracing::info!("Sent team invitation email to {}", invited_email);
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to send team invitation email: {}", e);

                // In test environment, we don't want to fail the test due to email issues
                if matches!(ctx.environment, Environment::Test) {
                    tracing::warn!("Test environment detected, ignoring email error");
                    Ok(())
                } else {
                    Err(e)
                }
            }
        }
    }
}
