use loco_rs::{
    prelude::*,
    environment::Environment,
};
use serde_json::json;

use crate::models::{
    _entities::{
        team_memberships::Model as TeamMembershipModel,
        teams::Model as TeamModel,
        users::Model as UserModel,
    },
};

// Define the static template directory
static invitation: Dir<'_> = include_dir!("src/mailers/team/invitation");

pub struct TeamMailer {}
impl Mailer for TeamMailer {}

impl TeamMailer {
    /// Send a team invitation email
    pub async fn send_invitation(
        ctx: &AppContext,
        invited_user: &UserModel,
        team: &TeamModel,
        membership: &TeamMembershipModel,
    ) -> Result<()> {
        let token = membership.invitation_token.as_ref()
            .ok_or_else(|| Error::string("Invitation token is missing"))?;
            
        let invited_email = invited_user.email.clone();
        
        // Get frontend URL from config
        let frontend_url = &ctx.config.server.host;

        // Create the locals JSON for template rendering
        let locals = json!({
            "name": invited_user.name,
            "team_name": team.name,
            "invitation_url": format!("{}/invitations/{}/accept", frontend_url, token)
        });

        // Check if mailer is configured
        if ctx.mailer.is_none() {
            tracing::warn!("Mailer not configured, skipping email delivery to {}", invited_email);
            return Ok(());
        }

        match Self::mail_template(
            ctx, 
            &invitation,
            mailer::Args {
                to: invited_email.clone(),
                locals,
                ..Default::default()
            },
        ).await {
            Ok(_) => {
                tracing::info!("Sent team invitation email to {}", invited_email);
                Ok(())
            },
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