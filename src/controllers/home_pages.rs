use axum::debug_handler;
use loco_rs::prelude::*;
use crate::models::users;
use tera;
use crate::utils::template::render_template;
use axum::response::Redirect;

/// Renders the home page for non-authenticated users
#[debug_handler]
async fn index(State(ctx): State<AppContext>) -> Result<Response> {
    let mut context = tera::Context::new();
    context.insert("active_page", "home");
    context.insert("invitation_count", &0);
    render_template(&ctx, "home/index.html.tera", context)
}

/// Renders the home page for authenticated users
#[debug_handler]
async fn authenticated_index(
    State(ctx): State<AppContext>,
    auth: auth::JWT,
) -> Result<Response> {
    let mut context = tera::Context::new();
    
    match users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await {
        Ok(user) => {
            // Get pending invitations count
            let invitations = crate::models::_entities::team_memberships::Entity::find()
                .find_with_related(crate::models::_entities::teams::Entity)
                .all(&ctx.db)
                .await?
                .into_iter()
                .filter(|(membership, _)| membership.user_id == user.id && membership.pending)
                .count();
                
            context.insert("user", &user);
            context.insert("active_page", "home");
            context.insert("invitation_count", &invitations);
            render_template(&ctx, "home/index.html.tera", context)
        }
        Err(err) => {
            tracing::error!(
                error = err.to_string(),
                user_pid = auth.claims.pid,
                "user not found during authentication"
            );
            Ok(Redirect::to("/auth/login").into_response())
        }
    }
}

/// Home page routes
pub fn routes() -> Routes {
    Routes::new()
        .add("/", get(index))
        .add("/home", get(authenticated_index))
} 