use axum::{
    debug_handler,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    routing::{get, post}, // Ensure post is imported
    Router,
};
use loco_rs::{app::AppContext, prelude::*};
use sea_orm::{
    ActiveModelTrait, EntityTrait, ItemsAndPagesNumber, PaginatorTrait, QueryOrder, Set,
    TransactionTrait,
}; // Ensure necessary imports
use serde::{Deserialize, Serialize}; // Import Serialize for response struct
use tera::Context; // Import Context for manual rendering
use tracing::{error, info, warn};

use crate::{
    middleware::auth_no_error::{self as auth, JWTWithUser, JWTWithUserOpt}, // Ensure JWTWithUser is imported
    models::{
        _entities::{team_memberships, users}, // Import team_memberships entity
        team_memberships as tm_model, // Import team_memberships model
        users::{
            self as users_model, ActiveModel as UserActiveModel, Model as UserModel,
            USER_STATUS_APPROVED, USER_STATUS_NEW, USER_STATUS_REJECTED,
        },
    },
    views,
};

// Define page size constant
const USERS_PER_PAGE: u64 = 25;

// Struct to capture query parameters for filtering and pagination
#[derive(Deserialize, Debug, Clone)]
pub struct UsersQuery {
    status: Option<String>,
    page: Option<u64>,
}

// Response struct for the full page template (not HTMX)
#[derive(Serialize)]
struct AdminUsersFullPageResponse {
    status_filter: Option<String>,
    // Pagination info isn't strictly needed here as the list is loaded via HTMX
}

// Helper struct for passing pagination info to Tera templates (for HTMX partial)
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


// Handler to list users with filtering and pagination
#[debug_handler]
async fn list_users(
    auth: auth::JWTWithUserOpt<UserModel>,
    State(ctx): State<AppContext>,
    ViewEngine(v): ViewEngine<TeraView>,
    headers: HeaderMap,
    Query(query): Query<UsersQuery>,
) -> Result<Response> {
    let current_user = match auth.user {
        Some(user) => user,
        None => {
            // Redirect to login if not authenticated (for both full page and HTMX)
            return Ok(views::redirect("/auth/login", HeaderMap::new())?);
        }
    };

    // Authorization Check: Ensure the user is a system admin
    // Use the Model::is_system_admin method
    if !UserModel::is_system_admin(&ctx, &current_user).await? {
        warn!(user_pid = %current_user.inner.pid, "Non-admin user attempted to access admin user list");
        // Render error page for full request, error fragment for HTMX
        if headers.contains_key("HX-Request") {
            return Ok(views::error_fragment(
                &v,
                "Access Denied",
                "#user-list-partial-error", // Use a dedicated error target for the partial
            )?);
        } else {
            return Ok(views::error_page(
                &v,
                "Access Denied",
                Some(Error::Message(
                    "You do not have permission to view this page.".to_string(),
                )),
            )?);
        }
    }

    info!(user_pid = %current_user.inner.pid, query = ?query, "Admin user accessed user list");

    // --- Start Fetching and Filtering Users (common logic for both full page & HTMX) ---
    let mut user_query = users::Entity::find();

    // Filter by status if provided and valid
    let status_filter = query.status.as_ref().map(|s| s.trim().to_lowercase());
    let valid_status = match status_filter.as_deref() {
        Some(USER_STATUS_NEW) | Some(USER_STATUS_APPROVED) | Some(USER_STATUS_REJECTED) => {
            status_filter.clone()
        }
        _ => None, // Treat "all" or invalid status as no filter
    };

    if let Some(status) = &valid_status {
        user_query = user_query.filter(users::Column::Status.eq(status.clone()));
    }

    // Apply ordering
    user_query = user_query.order_by_asc(users::Column::CreatedAt);

    // Apply pagination
    let page = query.page.unwrap_or(1);
    let paginator = user_query.paginate(&ctx.db, USERS_PER_PAGE);
    let num_items_and_pages = match paginator.num_items_and_pages().await {
        Ok(iap) => iap,
        Err(e) => {
            error!(error = ?e, "Failed to count users for pagination");
            // Return error response suitable for HTMX or full page
            if headers.contains_key("HX-Request") {
                return Ok(views::error_fragment(
                    &v,
                    "Database error counting users.",
                    "#user-list-partial-error",
                )?);
            } else {
                // For full page, we can leverage the existing error page rendering
                return Ok(views::error_page(&v, "Database Error", Some(Error::from(e)))?);
            }
        }
    };

    // Fetch the users for the current page
    let users = match paginator.fetch_page(page.saturating_sub(1)).await {
        // fetch_page is 0-indexed
        Ok(users) => users.into_iter().map(UserModel::from).collect::<Vec<_>>(), // Convert to Vec<UserModel>
        Err(e) => {
            error!(error = ?e, "Failed to fetch users for admin list page {}", page);
            // Return error response suitable for HTMX or full page
            if headers.contains_key("HX-Request") {
                return Ok(views::error_fragment(
                    &v,
                    "Database error fetching users.",
                    "#user-list-partial-error",
                )?);
            } else {
                 return Ok(views::error_page(&v, "Database Error", Some(Error::from(e)))?);
            }
        }
    };
    // --- End Fetching and Filtering Users ---

    // Determine if it's an HTMX request
    let is_htmx = headers.contains_key("HX-Request");

    // Pass the original query status string (or "all") to the view for filter button state
    let query_status_for_view = valid_status.clone().unwrap_or_else(|| "all".to_string());

    if is_htmx {
        // Render only the user list partial using the dedicated view function
        views::admin::render_user_list_partial(
            &ctx,
            // Convert Vec<UserModel> back to Vec<UserEntityModel> for the partial
            users.into_iter().map(|m| m.inner).collect(),
            num_items_and_pages,
            page,
            Some(query_status_for_view), // Pass current filter for pagination links
        )
    } else {
        // Render the full page using the layout context and page-specific data

        // 1. Get the base layout context
        let layout_ctx = current_user
            .get_layout_context(&ctx, "admin_users") // Pass AppContext, active page is "admin_users"
            .await?;
        let mut context = tera::Context::from_serialize(&layout_ctx)?;

        // 2. Prepare page-specific data
        let page_data = AdminUsersFullPageResponse {
            status_filter: Some(query_status_for_view),
        };

        // 3. Merge page-specific data into the context
        context.extend(tera::Context::from_serialize(&page_data)?);

        // 4. Render the full page template
        Ok(v.render("admin/users.html", context).into_response())
    }
}

// --- User Action Handlers ---
#[debug_handler]
async fn approve_user(
    auth: auth::JWTWithUser<UserModel>,
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<impl IntoResponse> {
    // Verify admin
    if !UserModel::is_system_admin(&ctx, &auth.user).await? {
        error!(user_pid = %auth.user.inner.pid, target_pid = %pid, "Attempt to approve user denied: not system admin");
        return Err(Error::Forbidden);
    }

    let user_model = users_model::Model::find_by_pid(&ctx.db, &pid)
        .await?
        .ok_or(Error::NotFound)?;

    // Check if already approved to prevent unnecessary updates
    if user_model.inner.status == USER_STATUS_APPROVED {
        info!(admin_pid = %auth.user.inner.pid, user_pid = %user_model.inner.pid, "User already approved, no action taken");
        return views::admin::render_user_row_partial(&ctx, user_model.inner); // Pass inner entity
    }

    let mut user_active_model: UserActiveModel = user_model.inner.into(); // Use inner entity
    user_active_model.status = Set(USER_STATUS_APPROVED.to_string());
    let updated_user = user_active_model.update(&ctx.db).await?;

    info!(admin_pid = %auth.user.inner.pid, user_pid = %updated_user.pid, "User approved");
    views::admin::render_user_row_partial(&ctx, updated_user)
}

#[debug_handler]
async fn reject_user(
    auth: auth::JWTWithUser<UserModel>,
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<impl IntoResponse> {
    // Verify admin
    if !UserModel::is_system_admin(&ctx, &auth.user).await? {
        error!(user_pid = %auth.user.inner.pid, target_pid = %pid, "Attempt to reject user denied: not system admin");
        return Err(Error::Forbidden);
    }

    let user_model = users_model::Model::find_by_pid(&ctx.db, &pid)
        .await?
        .ok_or(Error::NotFound)?;

    // Check if already rejected
    if user_model.inner.status == USER_STATUS_REJECTED {
        info!(admin_pid = %auth.user.inner.pid, user_pid = %user_model.inner.pid, "User already rejected, no action taken");
        return views::admin::render_user_row_partial(&ctx, user_model.inner); // Pass inner entity
    }

    let mut user_active_model: UserActiveModel = user_model.inner.into(); // Use inner entity
    user_active_model.status = Set(USER_STATUS_REJECTED.to_string());
    let updated_user = user_active_model.update(&ctx.db).await?;

    info!(admin_pid = %auth.user.inner.pid, user_pid = %updated_user.pid, "User rejected");
    views::admin::render_user_row_partial(&ctx, updated_user)
}

#[debug_handler]
async fn ban_user(
    auth: auth::JWTWithUser<UserModel>,
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<impl IntoResponse> {
    // Verify admin
    if !UserModel::is_system_admin(&ctx, &auth.user).await? {
        error!(user_pid = %auth.user.inner.pid, target_pid = %pid, "Attempt to ban user denied: not system admin");
        return Err(Error::Forbidden);
    }

    let user_model = users_model::Model::find_by_pid(&ctx.db, &pid)
        .await?
        .ok_or(Error::NotFound)?;

    // Ensure we don't ban the user initiating the request (self-ban)
    if user_model.inner.pid == auth.user.inner.pid {
        warn!(admin_pid = %auth.user.inner.pid, "Attempt to self-ban denied");
        // Consider returning an error fragment for HTMX here instead of a generic Error
        // For now, returning a standard error which might not be ideal for HTMX flow.
        return Err(Error::BadRequest("Cannot ban yourself.".to_string()));
    }

    // Start transaction
    let txn = ctx.db.begin().await?;

    // Set status to rejected (banned)
    let mut user_active_model: UserActiveModel = user_model.inner.clone().into(); // Use inner entity
    user_active_model.status = Set(USER_STATUS_REJECTED.to_string());
    let updated_user = match user_active_model.update(&txn).await {
        Ok(u) => u,
        Err(e) => {
            error!(error = ?e, user_id = user_model.inner.id, "Failed to update user status during ban");
            txn.rollback().await?;
            return Err(Error::from(e)); // Propagate the DB error
        }
    };

    // Remove user from all teams
    if let Err(e) = tm_model::Model::remove_user_from_all_teams(&txn, user_model.inner.id).await {
        error!(error = ?e, user_id = user_model.inner.id, "Failed to remove banned user from teams");
        txn.rollback().await?; // Rollback on error
        return Err(Error::from(e)); // Return a model error
    }

    // Commit transaction
    if let Err(e) = txn.commit().await {
        error!(error = ?e, "Failed to commit ban user transaction");
        return Err(Error::from(e)); // Propagate the DB error
    }

    info!(admin_pid = %auth.user.inner.pid, user_pid = %updated_user.pid, "User banned and removed from teams");
    views::admin::render_user_row_partial(&ctx, updated_user)
}

#[debug_handler]
async fn unban_user(
    auth: auth::JWTWithUser<UserModel>,
    State(ctx): State<AppContext>,
    Path(pid): Path<String>,
) -> Result<impl IntoResponse> {
    // Verify admin
    if !UserModel::is_system_admin(&ctx, &auth.user).await? {
        error!(user_pid = %auth.user.inner.pid, target_pid = %pid, "Attempt to unban user denied: not system admin");
        return Err(Error::Forbidden);
    }

    let user_model = users_model::Model::find_by_pid(&ctx.db, &pid)
        .await?
        .ok_or(Error::NotFound)?;

    // Check if already approved
    if user_model.inner.status == USER_STATUS_APPROVED {
        info!(admin_pid = %auth.user.inner.pid, user_pid = %user_model.inner.pid, "User already approved, no action taken for unban");
        return views::admin::render_user_row_partial(&ctx, user_model.inner); // Pass inner entity
    }

    // Set status back to approved
    let mut user_active_model: UserActiveModel = user_model.inner.into(); // Use inner entity
    user_active_model.status = Set(USER_STATUS_APPROVED.to_string());
    let updated_user = user_active_model.update(&ctx.db).await?;

    info!(admin_pid = %auth.user.inner.pid, user_pid = %updated_user.pid, "User unbanned (set to approved)");
    views::admin::render_user_row_partial(&ctx, updated_user)
}

// Define the router for this controller
pub fn routes() -> Router<AppContext> {
    Router::new()
        .route("/users", get(list_users)) // Existing GET route for the list page
        .route("/users/:pid/approve", post(approve_user)) // Add POST route for approve
        .route("/users/:pid/reject", post(reject_user)) // Add POST route for reject
        .route("/users/:pid/ban", post(ban_user)) // Add POST route for ban
        .route("/users/:pid/unban", post(unban_user)) // Add POST route for unban
}
