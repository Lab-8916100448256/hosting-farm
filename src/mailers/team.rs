use loco_rs::prelude::*;
use serde_json::json;

use crate::models::{
    _entities::{
        team_memberships::Model as TeamMembershipModel,
        teams::Model as TeamModel,
        users::Model as UserModel,
    },
};

pub struct TeamMailer;

impl TeamMailer {
    /// Send a team invitation email
    pub async fn send_invitation(
        ctx: &AppContext,
        invited_user: &UserModel,
        team: &TeamModel,
        invitation: &TeamMembershipModel,
    ) -> Result<()> {
        let token = invitation.invitation_token.as_ref()
            .ok_or_else(|| Error::string("Invitation token is missing"))?;
            
        let invited_email = invited_user.email.clone();
        
        // Get frontend URL from config
        let frontend_url = &ctx.config.server.host;

        ctx.mailer.send(
            &invited_email,
            &format!("Invitation to join team {}", team.name),
            &format!(
                "You have been invited to join the team {}. Visit {}/invitations/{}/accept to accept.",
                team.name,
                frontend_url,
                token
            ),
        ).await
    }
} 