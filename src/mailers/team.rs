
use std::fmt::{Debug, Display}; // Import Display
use std::error::Error as StdError; // Import StdError

use loco_rs::prelude::*;
use loco_rs::mailer::MailerTrait; // Ensure MailerTrait is in scope

// Import Team Entity Model specifically
use crate::models::_entities::teams::Model as TeamEntityModel;
use crate::models::users::Model as UserModel;

// Use the Mailer derive macro provided by loco_rs
#[derive(Mailer, Debug)]
#[mailer(template_dir = "src/mailers/team/invitation")] // Specify full template dir
pub struct TeamMailer {
    pub team: TeamEntityModel, // Use the entity model
    pub user: UserModel,
    pub inviter: UserModel,
    pub host: String,
    pub token: String,
}

impl TeamMailer {
    /// sends a team invitation email to the user
    ///
    /// # Errors
    ///
    /// When email sending failed, or the invitation token is missing
    #[allow(clippy::explicit_deref_methods)]
    pub async fn send_invitation(
        ctx: &AppContext,
        inviter: &UserModel,
        user: &UserModel,
        team: &TeamEntityModel, // Use the entity model
    ) -> Result<()> {
        // Find the pending invitation to get the token
        // This lookup might be redundant if the token was generated just before calling this,
        // consider passing the token directly if possible.
        let invitation = crate::models::_entities::team_memberships::Entity::find()
            .filter(crate::models::_entities::team_memberships::Column::TeamId.eq(team.id))
            .filter(crate::models::_entities::team_memberships::Column::UserId.eq(user.id))
            .filter(crate::models::_entities::team_memberships::Column::Pending.eq(true))
            .one(&ctx.db)
            .await?;

        let token = if let Some(invitation) = invitation {
            match invitation.invitation_token {
                Some(token) => token,
                None => {
                    tracing::error!(
                        "Invitation found for user {} to team {} but token is missing",
                        user.id,
                        team.id
                    );
                    // Use Error::Any for custom error strings that don't impl StdError
                    return Err(Error::Any(Box::new(MailerError::Custom(
                        "Invitation token missing for pending membership.".to_string(),
                    ))));
                }
            }
        } else {
            tracing::error!(
                "No pending invitation found for user {} to team {} when sending email",
                user.id,
                team.id
            );
             // Use Error::Any for custom error strings that don't impl StdError
            return Err(Error::Any(Box::new(MailerError::Custom(
                "Pending invitation not found.".to_string(),
            ))));
        };

        let mailer = Self {
            user: user.clone(),
            inviter: inviter.clone(),
            team: team.clone(), // Clone the entity model
            host: ctx.config.server.full_url(),
            token,
        };

        // -- START NEW BLOCK --
        let sender = ctx.mailer
           .as_ref()
           .ok_or_else(|| Error::Message("Mailer is not configured".to_string()))?;

        // Explicitly call MailerTrait::send
        MailerTrait::send::<Self>(sender, &mailer).await?;
        // -- END NEW BLOCK --

        Ok(())
    }
}

// Define a custom error type for Mailer issues if needed, or use Box<dyn StdError>
// Make sure it implements StdError for Error::Any
#[derive(thiserror::Error, Debug)]
enum MailerError {
    #[error("{0}")]
    Custom(String),
}

impl Display for MailerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MailerError::Custom(msg) => write!(f, "{}", msg),
        }
    }
}

impl StdError for MailerError {} // Implement StdError trait
