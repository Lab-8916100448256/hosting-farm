use loco_rs::prelude::*; 
use loco_rs::app::AppContext;
use crate::utils::template::render_template;
use crate::models::_entities::users::{self, Entity as User};
use axum::{
    debug_handler,
    extract::{Path, Query, State},
    routing::get,
    response::IntoResponse,
};
use serde::Deserialize;
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, Condition, QueryOrder, QuerySelect};
use tera::Context;

/// Search users handler
#[debug_handler]
async fn search_users_handler(
    State(ctx): State<AppContext>,
    Path(team_pid): Path<String>,
    Query(params): Query<SearchQuery>,
) -> Result<impl IntoResponse> {
    let search_term = params.user_email.trim();

    let users = if search_term.is_empty() {
        Vec::new() // Return empty results if search term is empty
    } else {
        let search_pattern = format!("%{}%", search_term);
        // Use sea_orm query
        User::find()
            .filter(
                Condition::any()
                    .add(users::Column::Email.like(&search_pattern))
                    .add(users::Column::Name.like(&search_pattern)), // Use Name column
            )
            .order_by_asc(users::Column::Name) // Use Name column
            .limit(10)
            .all(&ctx.db)
            .await?
    };

    // Create Tera context and add users
    let mut context = Context::new();
    context.insert("users", &users);

    // Render the partial template with the results
    render_template(&ctx, "teams/_user_search_results.html", context)
}

/// Team routes
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/teams")
        // Only include the search route for now
        .add("/{team_pid}/invite/search-users", get(search_users_handler))
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    user_email: String, // Matches the input field name in invite.html
}
