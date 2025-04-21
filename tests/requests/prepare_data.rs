use axum::http::{HeaderName, HeaderValue};
use hosting_farm::{models::users, views::auth::LoginResponse};
// Removed unused ModelError import
use loco_rs::{app::AppContext, prelude::*, TestServer};
use sea_orm::{DatabaseConnection, DbErr, EntityTrait, Set}; // Add DbErr and EntityTrait

const USER_EMAIL: &str = "test@loco.com";
const USER_PASSWORD: &str = "1234";

pub struct LoggedInUser {
    pub user: users::Model,
    pub token: String,
}

// Helper function to truncate the users table (often needed in tests)
pub async fn truncate_users(db: &DatabaseConnection) -> Result<(), DbErr> {
    users::Entity::delete_many().exec(db).await?;
    Ok(())
}

// Helper function to approve a user by PID
pub async fn approve_user(db: &DatabaseConnection, user_pid: &str) -> ModelResult<users::Model> {
    let user = users::Model::find_by_pid(db, user_pid).await?;
    let mut user_am = user.into_active_model();
    user_am.status = Set(users::USER_STATUS_APPROVED.to_string());
    // Map DbErr to ModelError for consistency with ModelResult
    user_am.update(db).await.map_err(ModelError::DbErr)
}

// Helper function to reject a user by PID
pub async fn reject_user(db: &DatabaseConnection, user_pid: &str) -> ModelResult<users::Model> {
    let user = users::Model::find_by_pid(db, user_pid).await?;
    let mut user_am = user.into_active_model();
    user_am.status = Set(users::USER_STATUS_REJECTED.to_string());
    // Map DbErr to ModelError for consistency with ModelResult
    user_am.update(db).await.map_err(ModelError::DbErr)
}

pub async fn init_user_login(request: &TestServer, ctx: &AppContext) -> LoggedInUser {
    // Ensure users table is clean before creating test user
    truncate_users(&ctx.db).await.expect("Failed to truncate users before init_user_login");

    let register_payload = serde_json::json!({
        "name": "loco test user", // Ensure name is unique enough for tests
        "email": USER_EMAIL,
        "password": USER_PASSWORD,
        "password_confirmation": USER_PASSWORD
    });

    // Creating a new user
    let register_response = request
        .post("/api/auth/register")
        .json(&register_payload)
        .await;

    // Check if registration was successful (status 2xx)
    if !register_response.status_code().is_success() {
        panic!("Failed to register user in init_user_login: {}", register_response.text());
    }


    let user = users::Model::find_by_email(&ctx.db, USER_EMAIL)
        .await
        .expect("Failed to find user immediately after registration");

     // --- Auto-approve user for testing login ---
     // In a real scenario with manual approval, this would not happen here.
     // For testing endpoints that require login, we approve the test user.
     approve_user(&ctx.db, &user.pid.to_string()).await.expect("Failed to approve test user");
     // --- End Auto-approval ---


    // NOTE: Verification step is skipped here because the user is now auto-approved.
    // If verification were required before approval, the verification API call would be needed.
    /*
    let verify_payload = serde_json::json!({
        "token": user.email_verification_token.as_ref().expect("Verification token missing"),
    });

    let verify_response = request.post("/api/auth/verify").json(&verify_payload).await;
    if !verify_response.status_code().is_success() {
        panic!("Failed to verify user in init_user_login: {}", verify_response.text());
    }
    */


    let response = request
        .post("/api/auth/login")
        .json(&serde_json::json!({
            "email": USER_EMAIL,
            "password": USER_PASSWORD
        }))
        .await;

     // Check if login was successful (status 2xx)
    if !response.status_code().is_success() {
        panic!("Failed to login user in init_user_login: {}", response.text());
    }


    let login_response: LoginResponse = serde_json::from_str(&response.text())
        .expect("Failed to parse login response");

    // Refetch user to ensure status is updated if approval happened
    let final_user = users::Model::find_by_email(&ctx.db, USER_EMAIL)
        .await
        .expect("Failed to refetch user after login attempt");

    LoggedInUser {
        user: final_user,
        token: login_response.token,
    }
}

pub fn auth_header(token: &str) -> (HeaderName, HeaderValue) {
    let auth_header_value = HeaderValue::from_str(&format!("Bearer {}", &token)).unwrap();

    (HeaderName::from_static("authorization"), auth_header_value)
}
