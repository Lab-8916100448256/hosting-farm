use loco_rs::prelude::*;

use crate::models::users;

/// Renders the home page for an authenticated user.
pub fn index(
    v: &TeraView,
    user: users::Model,
    invitation_count: usize,
    is_pending_approval: bool,
) -> Result<Response> {
    format::render().view(
        v,
        "home/index.html",
        data!({
            "user": &user,
            "is_logged_in": &true, // Add flag for template logic
            "active_page": "home",
            "invitation_count": &invitation_count,
            "is_pending_approval": &is_pending_approval, // Pass the new flag
        }),
    )
}

/// Renders the home page for a non-authenticated user.
pub fn index_logged_out(v: &TeraView) -> Result<Response> {
    format::render().view(
        v,
        "home/index.html",
        data!({
            "is_logged_in": &false, // Add flag for template logic
            "active_page": "home",
        }),
    )
}
