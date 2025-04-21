use hosting_farm::{app::App, models::users};
use insta::{assert_debug_snapshot, with_settings};
use loco_rs::testing::prelude::*;
use rstest::rstest;
use sea_orm::{ActiveModelTrait, Set}; // Import ActiveModelTrait and Set
use serial_test::serial;

use super::prepare_data;

// TODO: see how to dedup / extract this to app-local test utils
// not to framework, because that would require a runtime dep on insta
macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("auth_request");
        let _guard = settings.bind_to_scope();
    };
}

#[tokio::test]
#[serial]
async fn can_register() {
    configure_insta!();

    request::<App, _, _>(|request, ctx| async move {
        let email = "test@loco.com";
        let payload = serde_json::json!({
            "name": "loco",
            "email": email,
            "password": "12341234",
            "password_confirmation": "12341234"
        });

        let response = request.post("/api/auth/register").json(&payload).await;
        assert_eq!(
            response.status_code(),
            200,
            "Register request should succeed"
        );
        let saved_user = users::Model::find_by_email(&ctx.db, email).await;

        with_settings!({
            filters => cleanup_user_model()
        }, {
            assert_debug_snapshot!(saved_user);
        });

        let deliveries = ctx.mailer.unwrap().deliveries();
        assert_eq!(deliveries.count, 1, "Exactly one email should be sent");

        // with_settings!({
        //     filters => cleanup_email()
        // }, {
        //     assert_debug_snapshot!(ctx.mailer.unwrap().deliveries());
        // });
    })
    .await;
}

#[rstest]
#[case("login_with_valid_password", "12341234")]
#[case("login_with_invalid_password", "invalid-password")]
#[tokio::test]
#[serial]
async fn can_login_with_verify(#[case] test_name: &str, #[case] password: &str) {
    configure_insta!();

    request::<App, _, _>(|request, ctx| async move {
        let email = "test@loco.com";
        let register_payload = serde_json::json!({
            "name": "loco",
            "email": email,
            "password": "12341234",
            "password_confirmation": "12341234"
        });

        //Creating a new user
        let register_response = request
            .post("/api/auth/register")
            .json(&register_payload)
            .await;

        assert_eq!(
            register_response.status_code(),
            200,
            "Register request should succeed"
        );

        let user = users::Model::find_by_email(&ctx.db, email).await.unwrap();
        let email_verification_token = user
            .email_verification_token
            .expect("Email verification token should be generated");
        request
            .get(&format!("/api/auth/verify/{email_verification_token}"))
            .await;

        //verify user request
        let response = request
            .post("/api/auth/login")
            .json(&serde_json::json!({
                "email": email,
                "password": password
            }))
            .await;

        // Make sure email_verified_at is set
        let user = users::Model::find_by_email(&ctx.db, email)
            .await
            .expect("Failed to find user by email");

        assert!(
            user.email_verified_at.is_some(),
            "Expected the email to be verified, but it was not. User: {:?}",
            user
        );

        with_settings!({
            filters => cleanup_user_model()
        }, {
            assert_debug_snapshot!(test_name, (response.status_code(), response.text()));
        });
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_login_without_verify() {
    configure_insta!();

    request::<App, _, _>(|request, ctx| async move {
        // Ensure clean state
        prepare_data::truncate_users(&ctx.db).await.expect("Failed to truncate users");

        // 1. Register a dummy first user to prevent auto-approval
        let dummy_payload = serde_json::json!({
            "name": "Dummy First User Login",
            "email": "dummy.login@first.com",
            "password": "password123",
            "password_confirmation": "password123"
        });
        let dummy_register_response = request
            .post("/auth/register") // Use page endpoint for registration
            .form(&dummy_payload)
            .await;
        assert_eq!(dummy_register_response.status_code(), 303, "Dummy registration should redirect");

        // 2. Register the actual test user
        let email = "test.no.verify@loco.com";
        let password = "12341234";
        let register_payload = serde_json::json!({
            "name": "loco no verify",
            "email": email,
            "password": password,
            "password_confirmation": password
        });
        let register_response = request
            .post("/auth/register") // Use page endpoint
            .form(&register_payload)
            .await;
        assert_eq!(register_response.status_code(), 303, "Actual user registration should redirect");


        // 3. Verify user status is 'new'
        let user = users::Model::find_by_email(&ctx.db, email)
            .await
            .expect("Failed to find registered user");
        assert_eq!(user.status, users::USER_STATUS_NEW, "User status should be 'new' after registration");


        // 4. Attempt login - should fail because status is 'new'
        let login_response = request
            .post("/auth/login") // Use page endpoint
            .form(&serde_json::json!({
                "email": email,
                "password": password
            }))
            .await;

        // Assert login fails with correct message
        assert_eq!(login_response.status_code(), 200, "Login attempt should return OK status even on failure");
        let response_text = login_response.text();
        assert!(response_text.contains("Log in failed: Your account is pending approval."), "Expected pending approval message in response HTML");

        // Snapshot the response containing the error message
        with_settings!({
            filters => cleanup_user_model()
        }, {
            assert_debug_snapshot!("login_without_verify_fail", response_text); // New snapshot name
        });
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_reset_password() {
    configure_insta!();

    request::<App, _, _>(|request, ctx| async move {
        let login_data = prepare_data::init_user_login(&request, &ctx).await;

        let forgot_payload = serde_json::json!({
            "email": login_data.user.email,
        });
        let forget_response = request.post("/api/auth/forgot").json(&forgot_payload).await;
        assert_eq!(
            forget_response.status_code(),
            200,
            "Forget request should succeed"
        );

        let user = users::Model::find_by_email(&ctx.db, &login_data.user.email)
            .await
            .expect("Failed to find user by email");

        assert!(
            user.reset_token.is_some(),
            "Expected reset_token to be set, but it was None. User: {user:?}"
        );
        assert!(
            user.reset_sent_at.is_some(),
            "Expected reset_sent_at to be set, but it was None. User: {user:?}"
        );

        let new_password = "new-password";
        let reset_payload = serde_json::json!({
            "token": user.reset_token,
            "password": new_password,
        });

        let reset_response = request.post("/api/auth/reset").json(&reset_payload).await;
        assert_eq!(
            reset_response.status_code(),
            200,
            "Reset password request should succeed"
        );

        let user = users::Model::find_by_email(&ctx.db, &user.email)
            .await
            .unwrap();

        assert!(user.reset_token.is_none());
        assert!(user.reset_sent_at.is_none());

        assert_debug_snapshot!(reset_response.text());

        let login_response = request
            .post("/api/auth/login")
            .json(&serde_json::json!({
                "email": user.email,
                "password": new_password
            }))
            .await;

        assert_eq!(
            login_response.status_code(),
            200,
            "Login request should succeed"
        );

        let deliveries = ctx.mailer.unwrap().deliveries();
        assert_eq!(deliveries.count, 2, "Exactly one email should be sent");
        // with_settings!({
        //     filters => cleanup_email()
        // }, {
        //     assert_debug_snapshot!(deliveries.messages);
        // });
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_get_current_user() {
    configure_insta!();

    request::<App, _, _>(|request, ctx| async move {
        let user = prepare_data::init_user_login(&request, &ctx).await;

        let (auth_key, auth_value) = prepare_data::auth_header(&user.token);
        let response = request
            .get("/api/auth/current")
            .add_header(auth_key, auth_value)
            .await;

        assert_eq!(
            response.status_code(),
            200,
            "Current request should succeed"
        );

        with_settings!({
            filters => cleanup_user_model()
        }, {
            assert_debug_snapshot!((response.status_code(), response.text()));
        });
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_auth_with_magic_link() {
    configure_insta!();
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();

        let payload = serde_json::json!({
            "email": "user1@example.com",
        });
        let response = request.post("/api/auth/magic-link").json(&payload).await;
        assert_eq!(
            response.status_code(),
            200,
            "Magic link request should succeed"
        );

        let deliveries = ctx.mailer.unwrap().deliveries();
        assert_eq!(deliveries.count, 1, "Exactly one email should be sent");

        // let redact_token = format!("[a-zA-Z0-9]{{{}}}", users::MAGIC_LINK_LENGTH);
        // with_settings!({
        //      filters => {
        //          let mut combined_filters = cleanup_email().clone();
        //         combined_filters.extend(vec![(r"(\\r\\n|=\\r\\n)", ""), (redact_token.as_str(), "[REDACT_TOKEN]") ]);
        //         combined_filters
        //     }
        // }, {
        //     assert_debug_snapshot!(deliveries.messages);
        // });

        let user = users::Model::find_by_email(&ctx.db, "user1@example.com")
            .await
            .expect("User should be found");

        let magic_link_token = user
            .magic_link_token
            .expect("Magic link token should be generated");
        let magic_link_response = request
            .get(&format!("/api/auth/magic-link/{magic_link_token}"))
            .await;
        assert_eq!(
            magic_link_response.status_code(),
            200,
            "Magic link authentication should succeed"
        );

        with_settings!({
            filters => cleanup_user_model()
        }, {
            assert_debug_snapshot!(magic_link_response.text());
        });
    })
    .await;
}

#[tokio::test]
#[serial]
async fn can_reject_invalid_email() {
    configure_insta!();
    request::<App, _, _>(|request, _ctx| async move {
        let invalid_email = "user1@temp-mail.com";
        let payload = serde_json::json!({
            "email": invalid_email,
        });
        let response = request.post("/api/auth/magic-link").json(&payload).await;
        assert_eq!(
            response.status_code(),
            400,
            "Expected request with invalid email '{invalid_email}' to be blocked, but it was allowed."
        );
    })
    .await;
}

#[tokio::test]
#[serial]
async fn cannot_login_if_rejected() {
    configure_insta!();

    request::<App, _, _>(|request, ctx| async move {
        let email = "rejected@loco.com";
        let password = "password123";
        let register_payload = serde_json::json!({
            "name": "Rejected User",
            "email": email,
            "password": password,
            "password_confirmation": password
        });

        // 1. Register the user
        let register_response = request
            .post("/auth/register") // Using page endpoint
            .form(&register_payload)
            .await;
        // Expect a redirect on successful registration page submission
        assert_eq!(register_response.status_code(), 303, "Registration should redirect");

        // 2. Find the user and manually set status to rejected
        let mut user = users::Model::find_by_email(&ctx.db, email)
            .await
            .expect("Failed to find registered user");

        let mut user_active: users::ActiveModel = user.clone().into();
        user_active.status = sea_orm::Set(users::USER_STATUS_REJECTED.to_string());
        user = user_active.update(&ctx.db).await.expect("Failed to update user status");
        assert_eq!(user.status, users::USER_STATUS_REJECTED);

        // 3. Attempt to log in
        let login_response = request
            .post("/auth/login") // Using page endpoint
            .form(&serde_json::json!({
                "email": email,
                "password": password
            }))
            .await;

        // 4. Assert login failed and correct error message is shown
        // Login page returns 200 OK but renders an error fragment
        assert_eq!(login_response.status_code(), 200, "Login attempt should return OK status");
        let response_text = login_response.text();
        assert!(response_text.contains("Log in failed: Your account has been rejected."), "Expected rejection message in response HTML");

        // Optional: Snapshot the response fragment
        // with_settings!({}, {
        //     assert_debug_snapshot!("cannot_login_if_rejected", response_text);
        // });
    })
    .await;
}

#[tokio::test]
#[serial]
async fn cannot_login_if_new() {
    configure_insta!();

    request::<App, _, _>(|request, ctx| async move {
        let email = "new@loco.com";
        let password = "password123";
        let register_payload = serde_json::json!({
            "name": "New User",
            "email": email,
            "password": password,
            "password_confirmation": password
        });

        // Ensure clean state
        prepare_data::truncate_users(&ctx.db).await.expect("Failed to truncate users");

        // 1. Register a dummy first user to prevent auto-approval of the test user
        let dummy_payload = serde_json::json!({
            "name": "Dummy First User",
            "email": "dummy@first.com",
            "password": "password123",
            "password_confirmation": "password123"
        });
        let dummy_register_response = request
            .post("/auth/register")
            .form(&dummy_payload)
            .await;
        assert_eq!(dummy_register_response.status_code(), 303, "Dummy registration should redirect");


        // 2. Register the actual test user (status should now default to 'new')
        let register_response = request
            .post("/auth/register") // Using page endpoint
            .form(&register_payload)
            .await;
        assert_eq!(register_response.status_code(), 303, "Registration should redirect"); // Assuming successful registration redirects

        // 2. Verify user status is 'new'
        let user = users::Model::find_by_email(&ctx.db, email)
            .await
            .expect("Failed to find registered user");
        assert_eq!(user.status, users::USER_STATUS_NEW, "User status should be 'new' after registration");

        // 3. Attempt to log in
        let login_response = request
            .post("/auth/login") // Using page endpoint
            .form(&serde_json::json!({
                "email": email,
                "password": password
            }))
            .await;

        // 4. Assert login failed and correct error message is shown
        assert_eq!(login_response.status_code(), 200, "Login attempt should return OK status");
        let response_text = login_response.text();
        assert!(response_text.contains("Log in failed: Your account is pending approval."), "Expected pending approval message in response HTML");

    })
    .await;
}


#[tokio::test]
#[serial]
async fn can_reject_invalid_magic_link_token() {
    configure_insta!();
    request::<App, _, _>(|request, ctx| async move {
        seed::<App>(&ctx).await.unwrap();

        let magic_link_response = request.get("/api/auth/magic-link/invalid-token").await;
        assert_eq!(
            magic_link_response.status_code(),
            401,
            "Magic link authentication should be rejected"
        );
    })
    .await;
}
