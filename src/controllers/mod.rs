// Remove unused prelude import

// Import controllers - make them public
pub mod admin_pages;
pub mod auth_api;
pub mod auth_pages;
pub mod home_pages;
pub mod pgp_pages;
pub mod ssh_key_api;
pub mod teams_api;
pub mod teams_pages;
pub mod users_pages;

// This function might not be needed if routes are added directly in app.rs
// pub fn routes() -> Routes {
//     // ... implementation ...
// }
