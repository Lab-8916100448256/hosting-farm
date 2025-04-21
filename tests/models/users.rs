use chrono::{offset::Local, Duration, DateTime, FixedOffset}; // Import DateTime and FixedOffset
use hosting_farm::{
    app::App,
    // Ensure the necessary model components and constants are imported
    models::users::{self, Model, RegisterParams, USER_STATUS_APPROVED, USER_STATUS_NEW},
};
use insta::assert_debug_snapshot;
// Import the full testing prelude
use loco_rs::testing::prelude::*;
// Import necessary traits and types explicitly
use loco_rs::prelude::Authenticable;
use loco_rs::model::ModelError;

// Use IntoActiveModel from sea_orm directly
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection, DbErr, IntoActiveModel, Set};
use serial_test::serial;
// Correctly import specific functions from prepare_data
use crate::requests::prepare_data::{truncate_users, approve_user}; // Import specific functions needed for status tests
use migration::{Migrator, MigratorTrait}; // Import Migrator and MigratorTrait

macro_rules! configure_insta {
    ($($expr:expr),*) => {
        let mut settings = insta::Settings::clone_current();
        settings.set_prepend_module_to_snapshot(false);
        settings.set_snapshot_suffix("users");
        let _guard = settings.bind_to_scope();
    };
}

// Helper function to set up DB for model tests
async fn setup_test_db() -> DatabaseConnection {
    let boot = boot_test::<App>() // Use boot_test from prelude
        .await
        .expect("Failed to boot test application");
    // Ensure migrations are run for tests involving new fields
    run_migrations(&boot.app_context.db)
        .await
        .expect("Failed to run migrations");
    // Truncate is often needed before seeding or running tests
    // Use the truncate function from prelude
    truncate_users(&boot.app_context.db)
        .await
        .expect("Failed to truncate");
    boot.app_context.db
}

// Helper function to run migrations - returns DbErr on failure
async fn run_migrations(db: &DatabaseConnection) -> Result<(), DbErr> {
    Migrator::up(db, None).await
}

#[tokio::test]
#[serial]
async fn test_can_validate_model() {
    configure_insta!();
    // No need to boot the full app for model validation usually, direct ActiveModel usage is fine
    // However, validation might rely on DB state in some cases. Booting ensures DB connection.
    let db = setup_test_db().await;

    let invalid_user = users::ActiveModel {
        name: ActiveValue::set("1".to_string()), // Too short
        email: ActiveValue::set("invalid-email".to_string()),
        // Ensure status is handled if needed by validation rules (not the case here)
        ..Default::default()
    };

    // Validation occurs during before_save hook, triggered by insert/update
    let res = invalid_user.insert(&db).await;

    assert!(res.is_err()); // Expecting validation error
    // Snapshotting the error can be useful
    assert_debug_snapshot!(res.unwrap_err());
}

#[tokio::test]
#[serial]
async fn can_create_with_password() {
    configure_insta!();
    let db = setup_test_db().await;
    truncate_users(&db).await; // Use imported truncate_users from prepare_data

    let params = RegisterParams {
        email: "test@framework.com".to_string(),
        password: "1234".to_string(),
        name: "framework".to_string(),
        password_confirmation: "1234".to_string(),
    };

    let res = Model::create_with_password(&db, &params).await;

    // Ensure cleanup filters are applied if needed
    // Use the correct path for cleanup function
    insta::with_settings!({
        filters => loco_rs::testing::redaction::cleanup_user_model()
    }, {
        assert_debug_snapshot!(res);
    });

    // Also assert specific fields like status
    assert!(res.is_ok());
    assert_eq!(res.unwrap().status, USER_STATUS_NEW);

    truncate_users(&db).await;
}
#[tokio::test]
#[serial]
async fn handle_create_with_password_with_duplicate() {
    configure_insta!();
    let db = setup_test_db().await;
    // Seed the DB using the imported seed function
    let boot = boot_test::<App>().await.expect("boot"); // Re-boot to ensure clean state and seed
    seed::<App>(&boot.app_context) // Use seed from prelude
        .await
        .expect("Failed to seed database");


    // Try to create a user with the same email as the seeded user
    let new_user_res = Model::create_with_password(
        &db, // Use db from setup
        &RegisterParams {
            email: "user1@example.com".to_string(), // Duplicate email
            password: "1234".to_string(),
            name: "framework".to_string(),
            password_confirmation: "1234".to_string(),
        },
    )
    .await;

    assert!(new_user_res.is_err()); // Expecting an error
    assert_debug_snapshot!(new_user_res.unwrap_err()); // Snapshot the error

    // Try to create a user with the same name as the seeded user
     let new_user_res_name = Model::create_with_password(
        &db, // Use db from setup
        &RegisterParams {
            email: "another@example.com".to_string(),
            password: "1234".to_string(),
            name: "user1".to_string(), // Duplicate name
            password_confirmation: "1234".to_string(),
        },
    )
    .await;

    assert!(new_user_res_name.is_err()); // Expecting an error
    assert_debug_snapshot!(new_user_res_name.unwrap_err()); // Snapshot the error

    truncate_users(&db).await; // Clean up using prepare_data version
}

#[tokio::test]
#[serial]
async fn can_find_by_email() {
    configure_insta!();
    let db = setup_test_db().await;
    let boot = boot_test::<App>().await.expect("boot"); // Seed requires boot context
    seed::<App>(&boot.app_context) // Use seed from prelude
        .await
        .expect("Failed to seed database");

    let existing_user = Model::find_by_email(&db, "user1@example.com").await;
    let non_existing_user_results =
        Model::find_by_email(&db, "un@existing-email.com").await;

    assert!(existing_user.is_ok());
    assert_debug_snapshot!(existing_user.unwrap());
    assert!(non_existing_user_results.is_err());
    assert_debug_snapshot!(non_existing_user_results.unwrap_err());

    truncate_users(&db).await;
}

#[tokio::test]
#[serial]
async fn can_find_by_pid() {
    configure_insta!();
    let db = setup_test_db().await;
    let boot = boot_test::<App>().await.expect("boot"); // Seed requires boot context
    seed::<App>(&boot.app_context) // Use seed from prelude
        .await
        .expect("Failed to seed database");

    let existing_user =
        Model::find_by_pid(&db, "11111111-1111-1111-1111-111111111111").await;
    let non_existing_user_results =
        Model::find_by_pid(&db, "23232323-2323-2323-2323-232323232323").await;

    assert!(existing_user.is_ok());
    assert_debug_snapshot!(existing_user.unwrap());
    assert!(non_existing_user_results.is_err());
    assert_debug_snapshot!(non_existing_user_results.unwrap_err());

    truncate_users(&db).await;
}

#[tokio::test]
#[serial]
async fn can_verification_token() {
    configure_insta!();
    let db = setup_test_db().await;
    let boot = boot_test::<App>().await.expect("boot");
    seed::<App>(&boot.app_context) // Use seed from prelude
        .await
        .expect("Failed to seed database");

    let user = Model::find_by_pid(&db, "11111111-1111-1111-1111-111111111111")
        .await
        .expect("Failed to find user by PID");

    assert!(
        user.email_verification_sent_at.is_none(),
        "Expected no email verification sent timestamp"
    );
    assert!(
        user.email_verification_token.is_none(),
        "Expected no email verification token"
    );

    let result = user
        .clone() // Clone user to convert into ActiveModel
        .into_active_model()
        .generate_email_verification_token(&db)
        .await;

    assert!(
        result.is_ok(),
        "Failed to generate email verification token"
    );

    // Need to refetch or use the returned model for the next step
    let user_with_token = result.unwrap();

    let result_sent = user_with_token
        .into_active_model()
        .set_email_verification_sent(&db)
        .await;

    assert!(result_sent.is_ok(), "Failed to set email verification sent");

    let final_user = Model::find_by_pid(&db, "11111111-1111-1111-1111-111111111111")
        .await
        .expect("Failed to find user by PID after setting verification sent");

    assert!(
        final_user.email_verification_sent_at.is_some(),
        "Expected email verification sent timestamp to be present"
    );
    assert!(
        final_user.email_verification_token.is_some(),
        "Expected email verification token to be present"
    );
     truncate_users(&db).await;
}

#[tokio::test]
#[serial]
async fn can_set_forgot_password_sent() {
    configure_insta!();
    let db = setup_test_db().await;
    let boot = boot_test::<App>().await.expect("boot");
    seed::<App>(&boot.app_context) // Use seed from prelude
        .await
        .expect("Failed to seed database");

    let user = Model::find_by_pid(&db, "11111111-1111-1111-1111-111111111111")
        .await
        .expect("Failed to find user by PID");

    assert!(
        user.reset_sent_at.is_none(),
        "Expected no reset sent timestamp"
    );
    assert!(user.reset_token.is_none(), "Expected no reset token");

    let result = user
        .into_active_model()
        .set_forgot_password_sent(&db)
        .await;

    assert!(result.is_ok(), "Failed to set forgot password sent");

    let final_user = Model::find_by_pid(&db, "11111111-1111-1111-1111-111111111111")
        .await
        .expect("Failed to find user by PID after setting forgot password sent");

    assert!(
        final_user.reset_sent_at.is_some(),
        "Expected reset sent timestamp to be present"
    );
    assert!(
        final_user.reset_token.is_some(),
        "Expected reset token to be present"
    );
    truncate_users(&db).await;
}

#[tokio::test]
#[serial]
async fn can_verified() {
    configure_insta!();
    let db = setup_test_db().await;
    let boot = boot_test::<App>().await.expect("boot");
    seed::<App>(&boot.app_context) // Use seed from prelude
        .await
        .expect("Failed to seed database");

    let user = Model::find_by_pid(&db, "11111111-1111-1111-1111-111111111111")
        .await
        .expect("Failed to find user by PID");

    assert!(
        user.email_verified_at.is_none(),
        "Expected email to be unverified"
    );

    let result = user
        .into_active_model()
        .verified(&db)
        .await;

    assert!(result.is_ok(), "Failed to mark email as verified");

    let final_user = Model::find_by_pid(&db, "11111111-1111-1111-1111-111111111111")
        .await
        .expect("Failed to find user by PID after verification");

    assert!(
        final_user.email_verified_at.is_some(),
        "Expected email to be verified"
    );
    truncate_users(&db).await;
}

#[tokio::test]
#[serial]
async fn can_reset_password() {
    configure_insta!();
    let db = setup_test_db().await;
    let boot = boot_test::<App>().await.expect("boot");
    seed::<App>(&boot.app_context) // Use seed from prelude
        .await
        .expect("Failed to seed database");

    let user = Model::find_by_pid(&db, "11111111-1111-1111-1111-111111111111")
        .await
        .expect("Failed to find user by PID");

    // Note: The seeded password hash is for "password"
    assert!(
        user.verify_password("password"), // Use the actual seeded password
        "Password verification failed for original password"
    );

    let result = user
        .clone()
        .into_active_model()
        .reset_password(&db, "new-password")
        .await;

    assert!(result.is_ok(), "Failed to reset password");

    let final_user = Model::find_by_pid(&db, "11111111-1111-1111-1111-111111111111")
        .await
        .expect("Failed to find user by PID after password reset");

    assert!(
        final_user.verify_password("new-password"),
        "Password verification failed for new password"
    );
    truncate_users(&db).await;
}

#[tokio::test]
#[serial]
async fn magic_link() {
     configure_insta!();
    let db = setup_test_db().await;
    let boot = boot_test::<App>().await.expect("boot");
    seed::<App>(&boot.app_context).await.unwrap(); // Use seed from prelude

    let user = Model::find_by_pid(&db, "11111111-1111-1111-1111-111111111111")
        .await
        .unwrap();

    // Seeded user should be 'approved' based on src/app.rs
    assert_eq!(user.status, USER_STATUS_APPROVED);

    assert!(
        user.magic_link_token.is_none(),
        "Magic link token should be initially unset"
    );
    assert!(
        user.magic_link_expiration.is_none(),
        "Magic link expiration should be initially unset"
    );

    let create_result = user
        .clone() // Clone before moving into active model
        .into_active_model()
        .create_magic_link(&db)
        .await;

    assert!(
        create_result.is_ok(),
        "Failed to create magic link: {:?}",
        create_result.unwrap_err() // Use unwrap_err here
    );

    let updated_user =
        Model::find_by_pid(&db, "11111111-1111-1111-1111-111111111111")
            .await
            .expect("Failed to refetch user after magic link creation");

    assert!(
        updated_user.magic_link_token.is_some(),
        "Magic link token should be set after creation"
    );

    let magic_link_token = updated_user.magic_link_token.as_ref().unwrap(); // Borrow token
    assert_eq!(
        magic_link_token.len(),
        users::MAGIC_LINK_LENGTH as usize,
        "Magic link token length does not match expected length"
    );

    assert!(
        updated_user.magic_link_expiration.is_some(),
        "Magic link expiration should be set after creation"
    );

    let now: DateTime<Local> = Local::now();
    // Use fixed offset for comparison if possible, or allow slight variation
    let should_expired_at: DateTime<Local> = now + Duration::minutes(users::MAGIC_LINK_EXPIRATION_MIN.into());
    let actual_expiration: DateTime<FixedOffset> = updated_user.magic_link_expiration.unwrap();


    // Explicitly convert `now` to the same type as `actual_expiration` (FixedOffset)
    assert!(
        actual_expiration >= now.fixed_offset(),
        "Magic link expiration should be in the future or now. Actual: {}, Now: {}", actual_expiration, now.fixed_offset()
    );


    // Explicitly convert the added time to FixedOffset
    let max_expected_expiration = (should_expired_at + Duration::seconds(5)).fixed_offset();
    assert!(
        actual_expiration <= max_expected_expiration,
        "Magic link expiration exceeds expected maximum expiration time. Actual: {}, Max Expected: {}", actual_expiration, max_expected_expiration
    );


     // Test finding by magic link
     let found_by_magic = Model::find_by_magic_token(&db, magic_link_token).await;
     assert!(found_by_magic.is_ok());
     assert_eq!(found_by_magic.unwrap().id, updated_user.id);

     // Test clearing the magic link
     let clear_result = updated_user.into_active_model().clear_magic_link(&db).await;
     assert!(clear_result.is_ok());

     let cleared_user = Model::find_by_pid(&db, "11111111-1111-1111-1111-111111111111")
         .await
         .expect("Failed to refetch user after clearing magic link");

     assert!(cleared_user.magic_link_token.is_none());
     assert!(cleared_user.magic_link_expiration.is_none());

    truncate_users(&db).await;
}

//=============================================//
//  NEW STATUS TESTS                          //
//=============================================//

#[tokio::test]
#[serial]
async fn test_create_user_sets_default_status_new() {
    configure_insta!();
    let db = setup_test_db().await;
    truncate_users(&db).await;

    let params = RegisterParams {
        email: "newuser@example.com".to_string(),
        name: "New User".to_string(),
        password: "password123".to_string(),
        password_confirmation: "password123".to_string(),
    };

    let user = Model::create_with_password(&db, &params).await.unwrap();

    assert_eq!(user.status, USER_STATUS_NEW);

    truncate_users(&db).await;
}

#[tokio::test]
#[serial]
async fn test_find_by_pid_ignores_status() {
     configure_insta!();
    let db = setup_test_db().await;
    truncate_users(&db).await;

    // Create a user (status will be 'new')
    let params = RegisterParams {
        email: "statuscheck@example.com".to_string(),
        name: "Status Check".to_string(),
        password: "password123".to_string(),
        password_confirmation: "password123".to_string(),
    };
    let created_user = Model::create_with_password(&db, &params).await.unwrap();
    assert_eq!(created_user.status, USER_STATUS_NEW);

    // Find by PID should succeed regardless of status
    let found_user = Model::find_by_pid(&db, &created_user.pid.to_string()).await;
    assert!(found_user.is_ok());
    assert_eq!(found_user.unwrap().id, created_user.id);

    // Approve user and test again
    let mut user_am = created_user.into_active_model();
    user_am.status = Set(USER_STATUS_APPROVED.to_string());
    let approved_user = user_am.update(&db).await.unwrap();

    let found_approved_user = Model::find_by_pid(&db, &approved_user.pid.to_string()).await;
    assert!(found_approved_user.is_ok());
    assert_eq!(found_approved_user.unwrap().id, approved_user.id);

    truncate_users(&db).await;
}

#[tokio::test]
#[serial]
async fn test_find_by_claims_key_requires_approved_status() {
    configure_insta!();
    let db = setup_test_db().await;
    truncate_users(&db).await;

    // Create a user (status will be 'new')
    let params = RegisterParams {
        email: "claimscheck@example.com".to_string(),
        name: "Claims Check".to_string(),
        password: "password123".to_string(),
        password_confirmation: "password123".to_string(),
    };
    let created_user = Model::create_with_password(&db, &params).await.unwrap();
    assert_eq!(created_user.status, USER_STATUS_NEW);
    let pid_string = created_user.pid.to_string(); // Get PID as string

    // Attempt to find by claims key (PID) - should fail for 'new' user
    // Need to use Authenticable trait for find_by_claims_key
    let find_new_result = Model::find_by_claims_key(&db, &pid_string).await;
    assert!(find_new_result.is_err());
    assert!(matches!(find_new_result.unwrap_err(), ModelError::EntityNotFound));

    // Approve the user using the helper function from prepare_data
    let approved_user = approve_user(&db, &pid_string).await.unwrap(); // Use imported approve_user

    // Attempt to find by claims key again - should succeed for 'approved' user
    let find_approved_result = Model::find_by_claims_key(&db, &approved_user.pid.to_string()).await;
    assert!(find_approved_result.is_ok());
    assert_eq!(find_approved_result.unwrap().id, approved_user.id);

    truncate_users(&db).await;
}

#[tokio::test]
#[serial]
async fn test_generate_jwt_requires_approved_status() {
    configure_insta!();
    let db = setup_test_db().await;
    truncate_users(&db).await;

    // Create a user (status will be 'new')
    let params = RegisterParams {
        email: "jwtcheck@example.com".to_string(),
        name: "JWT Check".to_string(),
        password: "password123".to_string(),
        password_confirmation: "password123".to_string(),
    };
    let user_new = Model::create_with_password(&db, &params).await.unwrap();
    assert_eq!(user_new.status, USER_STATUS_NEW);

    // Attempt JWT generation for 'new' user - should fail
    let jwt_new_result = user_new.generate_jwt("test_secret", &3600);
    assert!(jwt_new_result.is_err());
    assert!(matches!(jwt_new_result.unwrap_err(), ModelError::EntityNotFound));

    // Approve the user using the helper function from prepare_data
     let approved_user = approve_user(&db, &user_new.pid.to_string()).await.unwrap(); // Use imported approve_user


    // Attempt JWT generation for 'approved' user - should succeed
    let jwt_approved_result = approved_user.generate_jwt("test_secret", &3600);
    assert!(jwt_approved_result.is_ok());

    truncate_users(&db).await;
}

#[tokio::test]
#[serial]
async fn test_find_by_api_key_requires_approved_status() {
    configure_insta!();
    let db = setup_test_db().await;
    truncate_users(&db).await;

    // Create a user (status 'new')
    let params = RegisterParams {
        email: "apikeytest@example.com".to_string(),
        name: "API Key Test".to_string(),
        password: "password123".to_string(),
        password_confirmation: "password123".to_string(),
    };
    let user_new = Model::create_with_password(&db, &params).await.unwrap();
    let api_key_new = user_new.api_key.clone();

    // Try find by API key for 'new' user
    // Need to use Authenticable trait for find_by_api_key
    let find_new = Model::find_by_api_key(&db, &api_key_new).await;
    assert!(find_new.is_err());
    assert!(matches!(find_new.unwrap_err(), ModelError::EntityNotFound));

     // Approve the user using the helper function from prepare_data
     let approved_user = approve_user(&db, &user_new.pid.to_string()).await.unwrap(); // Use imported approve_user
    let api_key_approved = approved_user.api_key.clone();

    // Try find by API key for 'approved' user
    let find_approved = Model::find_by_api_key(&db, &api_key_approved).await;
    assert!(find_approved.is_ok());
    assert_eq!(find_approved.unwrap().id, approved_user.id);

    truncate_users(&db).await;
}

#[tokio::test]
#[serial]
async fn test_find_by_magic_token_requires_approved_status() {
    configure_insta!();
    let db = setup_test_db().await;
    truncate_users(&db).await;

    // Create a 'new' user
    let params = RegisterParams {
        email: "magiclinktest@example.com".to_string(),
        name: "Magic Link Test".to_string(),
        password: "password123".to_string(),
        password_confirmation: "password123".to_string(),
    };
    let user_new = Model::create_with_password(&db, &params).await.unwrap();

    // Try to create magic link for 'new' user - should fail
    let create_magic_new_res = user_new.clone().into_active_model().create_magic_link(&db).await;
    assert!(create_magic_new_res.is_err());
    assert!(matches!(create_magic_new_res.unwrap_err(), ModelError::EntityNotFound)); // Check error type


    // Approve the user using the helper function from prepare_data
     let approved_user = approve_user(&db, &user_new.pid.to_string()).await.unwrap(); // Use imported approve_user


    // Create magic link for 'approved' user - should succeed
    let create_magic_approved_res = approved_user.clone().into_active_model().create_magic_link(&db).await;
    assert!(create_magic_approved_res.is_ok());
    let user_with_link = create_magic_approved_res.unwrap();
    let magic_token = user_with_link.magic_link_token.unwrap();

    // Find by the created magic token - should succeed
    let find_magic_res = Model::find_by_magic_token(&db, &magic_token).await;
    assert!(find_magic_res.is_ok());
    assert_eq!(find_magic_res.unwrap().id, approved_user.id);

    truncate_users(&db).await;
}
