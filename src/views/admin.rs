use axum::response::{IntoResponse, Response};
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::ItemsAndPagesNumber; // Import for pagination info
use serde::Serialize; // Import for Tera context serialization
use tera::Context;

// Import the user entity model and custom model
use crate::models::{_entities::users, users::Model as UserModel};


// Helper struct for passing pagination info to Tera templates
#[derive(Serialize)]
struct PaginatorInfo {
    num_items: u64,
    num_pages: u64,
    current_page: u64,
    has_prev: bool,
    has_next: bool,
}

impl PaginatorInfo {
    fn new(pagination: ItemsAndPagesNumber, current_page: u64) -> Self {
        Self {
            num_items: pagination.number_of_items,
            num_pages: pagination.number_of_pages,
            current_page,
            has_prev: current_page > 1,
            has_next: current_page < pagination.number_of_pages,
        }
    }
}


/// Renders only the user list partial (for HTMX requests - initial load, filtering, pagination).
///
/// # Errors
///
/// Returns a `Error::TeraError` if the template cannot be rendered.
pub fn render_user_list_partial(
    ctx: &AppContext,
    users: Vec<users::Model>,         // List of user entities for the current page
    pagination: ItemsAndPagesNumber, // Pagination metadata for the current view
    current_page: u64,               // The current page number (1-based)
    status_filter: Option<String>,   // The current status filter applied (for pagination links)
) -> Result<impl IntoResponse> { // Changed return type
    let mut context = Context::new();

    // Create PaginatorInfo based on the fetched page data
    let paginator = PaginatorInfo::new(pagination, current_page);

    // Pass data needed by the partial template
    context.insert("users", &users);
    context.insert("paginator", &paginator);
    context.insert("status_filter", &status_filter); // Needed for pagination links

    // Render the partial user list template
    super::render(ctx, "admin/_user_list.html", context)
}


/// Renders a single user row partial (for HTMX updates after actions).
///
/// # Errors
///
/// Returns a `Error::TeraError` if the template cannot be rendered.
pub fn render_user_row_partial(
    ctx: &AppContext,
    user: users::Model, // The updated user entity
) -> Result<impl IntoResponse> { // Changed return type
    let mut context = Context::new();
    // Pass the user object directly, localization happens in template
    context.insert("user", &user);

    // Render the single user row partial template
    super::render(ctx, "admin/_user_row.html", context)
}
