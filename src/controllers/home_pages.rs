use axum::debug_handler;
use loco_rs::prelude::*;

use crate::{
    middleware::auth_no_error::JWTWithUserOpt,
    models::users,
};

#[debug_handler]
async fn home(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
) -> Result<Response> {
    match auth.user {
        Some(user) => {
            tracing::info!(
                message = "generating home page for authenticated user,",
                user_email = &user.email,
            );             
        // Get pending invitations count
            let invitations = crate::models::_entities::team_memberships::Entity::find()
                .find_with_related(crate::models::_entities::teams::Entity)
                .all(&ctx.db)
                .await?
                .into_iter()
                .filter(|(membership, _)| membership.user_id == user.id && membership.pending)
                .count();
            // Render the index view, authenticated user parameters
            format::render().view(&v, "home/index.html", data!({
                "user": user, 
                "active_page": "home", 
                "invitation_count": invitations}))
        }
        None => {
            // Render the index view, non-authenticated user parameters
            tracing::info!(
                message = "generating home page for non-authenticated user,",
            );             
           format::render().view(&v, "home/index.html", data!({
                "active_page": "home"}))
        }
    }
}

/// Home page routes
pub fn routes() -> Routes {
    Routes::new()
      .add("/", get(home))
      .add("/home", get(home))
}
