use loco_rs::controller::middleware;

/// Re-exports auth middleware for convenience
pub fn auth() -> middleware::Auth {
    middleware::auth()
} 