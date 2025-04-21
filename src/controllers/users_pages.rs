use crate::{
    controllers::ssh_key_api::{is_valid_ssh_public_key, SshKeyPayload},
    mailers::auth::AuthMailer,
    middleware::auth_no_error::JWTWithUserOpt,
    models::{
        _entities::ssh_keys,
        _entities::team_memberships,
        _entities::teams,
        users,
        users::users::Column as UsersColumn, // Import Column specifically for users
    },
    views::{error_fragment, error_page, redirect, render_template},
};
use axum::http::HeaderMap;
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::{
    debug_handler,
    extract::{Form, Query, State},
    routing::{delete, get, post},
};
use loco_rs::prelude::Result;
use loco_rs::prelude::*;
use loco_rs::prelude::{TeraView, ViewEngine};
use sea_orm::QueryOrder;
use sea_orm::Set;
use sea_orm::{ActiveModelTrait, PaginatorTrait};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

/// Params for updating user profile
#[derive(Debug, Deserialize)]
pub struct UpdateProfileParams {
    pub name: String,
    pub email: String,
}

/// Params for updating user password
#[derive(Debug, Deserialize)]
pub struct UpdatePasswordParams {
    pub current_password: String,
    pub password: String,
    pub password_confirmation: String,
}

/// Renders the user profile page
#[debug_handler]
async fn profile(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        // Redirect to login if not authenticated
        return redirect("/auth/login", headers);
    };

    // Get PGP key details
    let pgp_fingerprint = user.pgp_fingerprint();
    let pgp_validity = user.pgp_validity();

    // Get user's team memberships
    let teams_result = teams::Entity::find()
        .find_with_related(team_memberships::Entity)
        .all(&ctx.db)
        .await;

    let teams_data = match teams_result {
        Ok(data) => data,
        Err(e) => {
            tracing::error!("Failed to load teams for user {}: {}", user.id, e);
            return error_page(
                &v,
                "Could not load your team information. Please try again later.",
                Some(e.into()),
            );
        }
    };

    let teams = teams_data
        .into_iter()
        .filter_map(|(team, memberships)| {
            let is_member = memberships
                .iter()
                .any(|m| m.user_id == user.id && !m.pending);
            if is_member {
                // Find user's role in this team
                let role = memberships
                    .iter()
                    .find(|m| m.user_id == user.id && !m.pending)
                    .map(|m| m.role.clone())
                    .unwrap_or_else(|| "Unknown".to_string());

                Some(json!({
                    "pid": team.pid.to_string(),
                    "name": team.name,
                    "role": role
                }))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // Get pending invitations count
    let invitations_result = team_memberships::Entity::find()
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(true))
        .count(&ctx.db)
        .await;

    let invitations = match invitations_result {
        Ok(count) => count,
        Err(e) => {
            tracing::error!(
                "Failed to load invitation count for user {}: {}",
                user.id,
                e
            );
            // Call error_page with 3 arguments: engine, message, error
            return error_page(
                &v,
                "Could not load your invitation count. Please try again later.",
                Some(e.into()),
            );
        }
    };

    // Get user's SSH keys
    let ssh_keys_result = ssh_keys::Entity::find()
        .filter(ssh_keys::Column::UserId.eq(user.id))
        .all(&ctx.db)
        .await;

    let ssh_keys = match ssh_keys_result {
        Ok(keys) => keys,
        Err(e) => {
            tracing::error!("Failed to load SSH keys for user {}: {}", user.id, e);
            return error_page(
                &v,
                "Could not load your SSH keys. Please try again later.",
                Some(e.into()),
            );
        }
    };

    // Check for the pgp_verified query parameter
    let pgp_verified_success = params.get("pgp_verified") == Some(&"true".to_string());

    // Determine PGP status flags
    let has_pgp_key = user.pgp_key.is_some();
    let is_pgp_verified = user.pgp_verified_at.is_some();

    render_template(
        &v,
        "users/profile.html",
        data!({
            "user": &user,
            "teams": &teams,
            "active_page": "profile",
            "invitation_count": &invitations,
            "ssh_keys": &ssh_keys,
            "pgp_fingerprint": &pgp_fingerprint,
            "pgp_validity": &pgp_validity,
            "pgp_verified_success": pgp_verified_success,
            "has_pgp_key": has_pgp_key,
            "is_pgp_verified": is_pgp_verified,
        }),
    )
}

/// Renders the user invitations page
#[debug_handler]
async fn invitations(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        // Redirect to login if not authenticated
        return redirect("/auth/login", headers);
    };

    // Get user's pending team invitations
    let invitations_result = team_memberships::Entity::find()
        .find_with_related(teams::Entity)
        .all(&ctx.db)
        .await;

    let invitations_data = match invitations_result {
        Ok(data) => data,
        Err(e) => {
            tracing::error!("Failed to load invitations for user {}: {}", user.id, e);
            // Return error page on failure
            return error_page(
                &v,
                "Could not load your invitations. Please try again later.",
                Some(e.into()),
            );
        }
    };

    let invitations = invitations_data
        .into_iter()
        .filter_map(|(membership, teams)| {
            if membership.user_id == user.id && membership.pending {
                // Handle potential None from teams.first() safely
                teams.first().map(|team| {
                    json!({
                        "team_name": team.name,
                        "team_description": team.description.clone(),
                        "token": membership.invitation_token,
                        "role": membership.role,
                        "sent_at": membership.created_at.format("%Y-%m-%d").to_string()
                    })
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    // Get pending invitations count (now derived from the collected invitations)
    let invitation_count = invitations.len();

    render_template(
        &v,
        "users/invitations.html",
        data!({
            "user": &user,
            "invitations": &invitations,
            "active_page": "invitations",
            "invitation_count": &invitation_count,
        }),
    )
}

/// Returns the HTML fragment for the invitation count badge
#[debug_handler]
async fn get_invitation_count(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    // Get pending invitations count
    let invitation_count = team_memberships::Entity::find()
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(true))
        .count(&ctx.db)
        .await?;

    // Use render_template to render the fragment
    render_template(
        &v,
        "users/_invitation_count_badge.html",
        data!({
            "invitation_count": &invitation_count,
        }),
    )
}

/// Updates user profile information
#[debug_handler]
async fn update_profile(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(params): Form<UpdateProfileParams>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    let mut user_model: users::ActiveModel = user.clone().into(); // Clone user to avoid move issues
    let mut email_changed = false; // Track if email was actually changed

    // --- Prepare Email Changes ---
    let trimmed_email = params.email.trim().to_string(); // Trim email

    if user.email != trimmed_email {
        email_changed = true; // Mark email as changed
                              // Check if the *new* email exists.
        match users::Model::find_by_email(&ctx.db, &trimmed_email).await {
            Ok(_) => {
                return error_fragment(&v, "Email already in use", "#notification-container");
            }
            Err(ModelError::EntityNotFound) => {
                // Email is available
                user_model.email = Set(trimmed_email.clone());
                user_model.email_verified_at = Set(None);
                user_model.pgp_verified_at = Set(None); // Reset PGP verification on email change
            }
            Err(e) => {
                tracing::error!(error = ?e, "Database error checking email availability");
                return error_fragment(
                    &v,
                    "Could not verify email availability. Please try again.",
                    "#notification-container",
                );
            }
        }
    } else {
        // Ensure the possibly-trimmed email is set even if logically unchanged
        user_model.email = Set(trimmed_email);
    }

    // --- Prepare Name Changes ---
    let new_name = params.name.trim().to_string();

    if user.name != new_name {
        // Check if the new name already exists
        match users::Entity::find()
            .filter(UsersColumn::Name.eq(&new_name)) // Use imported UsersColumn
            .filter(UsersColumn::Id.ne(user.id)) // Exclude self
            .one(&ctx.db)
            .await
        {
            Ok(Some(_)) => {
                return error_fragment(&v, "Username already in use", "#notification-container");
            }
            Ok(None) => {
                // Name is available
                user_model.name = Set(new_name.clone());
            }
            Err(e) => {
                tracing::error!(error = ?e, "Database error checking username availability");
                return error_fragment(
                    &v,
                    "Could not verify username availability. Please try again.",
                    "#notification-container",
                );
            }
        }
    } else {
        // Ensure the possibly-trimmed name is set even if logically unchanged
        user_model.name = Set(new_name);
    }

    // --- Perform the database update ---
    match user_model.update(&ctx.db).await {
        Ok(updated_user) => {
            let mut success_message = "Profile updated successfully.".to_string();
            let mut send_verification_banner = false;

            // Handle email verification steps if email was changed
            if email_changed {
                send_verification_banner = true; // Use the tracked flag
                match updated_user
                    .clone() // Clone again for the subsequent operations
                    .into_active_model()
                    .generate_email_verification_token(&ctx.db)
                    .await
                {
                    Ok(user_with_token) => {
                        match AuthMailer::send_welcome(&ctx, &user_with_token).await {
                            Ok(_) => {
                                match user_with_token
                                    .into_active_model()
                                    .set_email_verification_sent(&ctx.db)
                                    .await
                                {
                                    Ok(_) => {
                                        success_message.push_str(" A verification email has been sent to your new address.");
                                    }
                                    Err(db_err) => {
                                        tracing::error!(error = ?db_err, email = %updated_user.email, "Failed to set email verification sent status after profile update");
                                        success_message.push_str(
                                            " Failed to record verification email dispatch status.",
                                        );
                                    }
                                }
                            }
                            Err(mailer_err) => {
                                tracing::error!(error = ?mailer_err, email = %updated_user.email, "Failed to send verification email after profile update");
                                success_message
                                    .push_str(" However, failed to send verification email.");
                            }
                        }
                    }
                    Err(token_err) => {
                        tracing::error!(error = ?token_err, email = %updated_user.email, "Failed to generate verification token after profile update");
                        success_message.push_str(" Failed to generate new verification token.");
                    }
                }
            }

            // Prepare OOB swaps
            let mut oob_swaps = String::new();

            // Render success message fragment
            let success_html = v.render(
                "fragments/success_message.html",
                data!({ "message": success_message }),
            )?;

            // If email changed, render and prepare banners for OOB swap
            if send_verification_banner {
                // Render email verification banner
                let email_banner_html = v.render(
                    "users/_email_verification_banner.html",
                    data!({ "user": &updated_user }),
                )?;

                oob_swaps.push_str(&format!(
                    "<div id=\"email-verification-banner-container\" hx-swap-oob=\"outerHTML\">{}</div>",
                    email_banner_html
                ));

                // Also render PGP warning banner as email change resets PGP verification
                let has_pgp_key = updated_user.pgp_key.is_some();
                let is_pgp_verified = updated_user.pgp_verified_at.is_some(); // Will be false here
                let pgp_banner_html = v.render(
                    "users/_pgp_warning_banner.html",
                    data!({
                        "has_pgp_key": has_pgp_key,
                        "is_pgp_verified": is_pgp_verified,
                    }),
                )?;

                oob_swaps.push_str(&format!(
                    "<div id=\"pgp-warning-banner-container\" hx-swap-oob=\"outerHTML\">{}</div>",
                    pgp_banner_html
                ));
            }

            // Combine success message and OOB swaps
            let combined_html = format!("{}{}", success_html, oob_swaps);

            // Return combined response
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html")
                .body(axum::body::Body::from(combined_html))?)
        }
        Err(db_err) => {
            // Handle DbErr generically. Uniqueness errors should ideally be caught
            // by the pre-checks above, but handle other DB errors here.
            tracing::error!(error = ?db_err, user_id = user.id, "Failed to update user profile in database");
            error_fragment(
                &v,
                "Could not update profile due to a database error. Please try again.",
                "#notification-container",
            )
        }
    }
}

/// Update user password
#[debug_handler]
async fn update_password(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(params): Form<UpdatePasswordParams>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    // Verify passwords match
    if params.password != params.password_confirmation {
        return error_fragment(&v, "Passwords do not match", "#password-error-container");
    }

    // Verify current password
    if !user.verify_password(&params.current_password) {
        return error_fragment(
            &v,
            "Current password is incorrect",
            "#password-error-container",
        );
    }

    // Update password - handle result with match
    match user
        .into_active_model()
        .reset_password(&ctx.db, &params.password)
        .await
    {
        Ok(_updated_user) => {
            // Return a response that refreshes the page on success
            let response = Response::builder()
                .header("HX-Refresh", "true")
                .body(axum::body::Body::empty())?;
            Ok(response)
        }
        Err(e) => {
            // Use error_fragment on failure, target password errors
            tracing::error!("Failed to update password: {}", e);
            error_fragment(
                &v,
                "Could not update password. Please try again.",
                "#password-error-container",
            )
        }
    }
}

/// Renders the SSH keys list fragment for the profile page
#[debug_handler]
async fn ssh_keys_fragment(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        // Although this is a fragment, redirecting might still be appropriate
        // if the session expired mid-use. Alternatively, return an error fragment.
        return redirect("/auth/login", headers);
    };

    // Get user's SSH keys
    let ssh_keys_result = ssh_keys::Entity::find()
        .filter(ssh_keys::Column::UserId.eq(user.id))
        .order_by_asc(ssh_keys::Column::CreatedAt) // Order for consistent display
        .all(&ctx.db)
        .await;

    let ssh_keys = match ssh_keys_result {
        Ok(keys) => keys,
        Err(e) => {
            tracing::error!("Failed to load SSH keys for user {}: {}", user.id, e);
            // Return an error fragment instead of a full page
            return error_fragment(
                &v,
                "Could not load SSH keys.",
                "#ssh-keys-error-container", // Target for the error message
            );
        }
    };

    render_template(
        &v,
        "users/_ssh_keys_list.html",
        data!({
            "ssh_keys": &ssh_keys,
        }),
    )
}

/// Handles adding an SSH key via HTMX form submission
#[debug_handler]
async fn add_ssh_key(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap, // Needed for redirect on auth failure
    Form(params): Form<SshKeyPayload>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers); // Redirect if not logged in
    };

    // Trim whitespace from the key input
    let trimmed_key = params.public_key.trim().to_string();

    // --- Validation --- (using trimmed_key)
    if trimmed_key.is_empty() {
        return error_fragment(&v, "Public key cannot be empty", "#add-key-error");
    }
    if !is_valid_ssh_public_key(&trimmed_key) {
        return error_fragment(&v, "Invalid SSH public key format", "#add-key-error");
    }
    // Check if trimmed key already exists for this user
    match ssh_keys::Entity::find()
        .filter(ssh_keys::Column::UserId.eq(user.id))
        .filter(ssh_keys::Column::PublicKey.eq(trimmed_key.clone())) // Use cloned trimmed_key
        .one(&ctx.db)
        .await
    {
        Ok(Some(_)) => {
            return error_fragment(&v, "SSH Key already exists for this user", "#add-key-error");
        }
        Ok(None) => { /* Key does not exist, proceed */ }
        Err(e) => {
            tracing::error!("DB error checking for existing SSH key: {}", e);
            return error_fragment(
                &v,
                "Error checking for existing key. Please try again.",
                "#add-key-error",
            );
        }
    }
    // --- End Validation ---

    // --- Insert Key --- (using trimmed_key)
    let key = ssh_keys::ActiveModel {
        user_id: Set(user.id),
        public_key: Set(trimmed_key), // Use trimmed_key
        ..Default::default()
    };
    match key.insert(&ctx.db).await {
        Ok(_) => { /* Insert successful, proceed to render */ }
        Err(e) => {
            tracing::error!("DB error inserting SSH key: {}", e);
            return error_fragment(
                &v,
                "Failed to save the new key. Please try again.",
                "#add-key-error",
            );
        }
    }
    // --- End Insert Key ---

    // --- Fetch Updated Keys and Render Fragment --- (on success)
    match ssh_keys::Entity::find()
        .filter(ssh_keys::Column::UserId.eq(user.id))
        .order_by_asc(ssh_keys::Column::CreatedAt)
        .all(&ctx.db)
        .await
    {
        Ok(ssh_keys) => render_template(
            &v,
            "users/_ssh_keys_list.html",
            data!({
                "ssh_keys": &ssh_keys,
            }),
        ),
        Err(e) => {
            tracing::error!("Failed to reload SSH keys after add: {}", e);
            // Return error fragment even after successful add if reload fails
            error_fragment(
                &v,
                "Key added, but failed to refresh list.",
                "#ssh-keys-error-container", // Target the main list error container
            )
        }
    }
}

/// Resend verification email
#[debug_handler]
async fn resend_verification_email(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    if user.email_verified_at.is_some() {
        // Instead of rendering profile, return a success message indicating it's already verified
        return render_template(
            &v,
            "fragments/success_message.html",
            data!({
                "message": "Your email is already verified.",
                "target": "notification-container",
            }),
        );
    }

    // Generate email verification token
    match user
        .clone()
        .into_active_model()
        .generate_email_verification_token(&ctx.db)
        .await
    {
        Ok(user_with_token) => {
            // Send verification email first
            match AuthMailer::send_welcome(&ctx, &user_with_token).await {
                Ok(_) => {
                    // Email sent successfully, now update verification status
                    match user_with_token
                        .clone()
                        .into_active_model()
                        .set_email_verification_sent(&ctx.db)
                        .await
                    {
                        Ok(_) => {
                            // All good, render success message
                            format::render().view(
                                &v,
                                "fragments/success_message.html",
                                data!({
                                    "message": "Verification email sent successfully.",
                                    "target": "notification-container",
                                }),
                            )
                        }
                        Err(err) => {
                            tracing::error!(
                                message =
                                    "Failed to set email verification status after sending email",
                                user_email = &user_with_token.email,
                                error = err.to_string(),
                            );
                            error_fragment(
                                &v,
                                "Failed to update verification status. Please try again.",
                                "#notification-container",
                            )
                        }
                    }
                }
                Err(err) => {
                    tracing::error!(
                        message = "Failed to send welcome email",
                        user_email = &user_with_token.email,
                        error = err.to_string(),
                    );
                    error_fragment(
                        &v,
                        "Could not send verification email. Please try again.",
                        "#notification-container",
                    )
                }
            }
        }
        Err(err) => {
            tracing::error!(
                message = "Failed to generate email verification token",
                user_email = &user.email,
                error = err.to_string(),
            );
            error_fragment(
                &v,
                "Could not generate verification token. Please try again.",
                "#notification-container",
            )
        }
    }
}

/// Refreshes the user's PGP key from a keyserver
#[debug_handler]
async fn refresh_pgp(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers); // Redirect if not logged in
    };

    // Use the new model method to fetch and update the key
    let fetch_result = user
        .clone() // Clone user to get ActiveModel without consuming original
        .into_active_model()
        .fetch_and_update_pgp_key(&ctx.db)
        .await;

    match fetch_result {
        Ok(updated_user) => {
            // Get updated details from the returned model
            let pgp_fingerprint = updated_user.pgp_fingerprint();
            let pgp_validity = updated_user.pgp_validity();

            // Determine the notification message
            let notification_message = if updated_user.pgp_key.is_some() {
                "PGP key updated successfully from server."
            } else {
                "No PGP key found on server for your email."
            }
            .to_string();

            // Render the PGP section partial, now including the notification
            let pgp_section_html = v.render(
                "users/_pgp_section.html",
                data!({
                    "pgp_fingerprint": &pgp_fingerprint,
                    "pgp_validity": &pgp_validity,
                    "notification_message": &notification_message, // Pass message
                    "user": &updated_user, // Pass the updated user object
                }),
            )?;

            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html")
                .body(axum::body::Body::from(pgp_section_html))?)
        }
        Err(e) => {
            tracing::error!(user_id = user.id, error = ?e, "Error fetching/updating PGP key.");
            error_fragment(
                &v,
                &format!("Error refreshing PGP key: {}", e),
                "#pgp-section",
            )
        }
    }
}

/// Handler to initiate PGP email sending verification
#[debug_handler]
async fn verify_pgp_sending(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    //_headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        // Should not happen if UI prevents this, but handle defensively
        // Pass a default target selector instead of None
        return error_fragment(&v, "Authentication required.", "#notification-container");
    };

    // Ensure user has a PGP key configured
    if user.pgp_key.is_none() {
        return error_fragment(
            &v,
            "No PGP key configured. Cannot send verification email.",
            "#notification-container", // Pass a default target selector
        );
    }

    // 1. Generate PGP verification token
    // Clone user before converting to ActiveModel to avoid move
    let active_user: users::ActiveModel = user.clone().into();
    let updated_user_res = active_user.generate_pgp_verification_token(&ctx.db).await;

    let updated_user = match updated_user_res {
        Ok(u) => u,
        Err(e) => {
            // Access user.id before it's potentially moved/borrow error occurs
            let user_id = user.id;
            tracing::error!("Failed to generate PGP token for user {}: {}", user_id, e);
            // Pass target selector, rely on logging for error details
            return error_fragment(
                &v,
                "Failed to start PGP verification process.",
                "#notification-container",
            );
        }
    };

    // 2. Send PGP encrypted verification email
    // Ensure the token exists before sending
    if let Some(token) = &updated_user.pgp_verification_token {
        match AuthMailer::send_pgp_verification(&ctx, &updated_user, token).await {
            Ok(_) => {
                tracing::info!("PGP verification email sent to: {}", updated_user.email);
                // Return a success message fragment for the notification area
                render_template(
                    &v,
                    "fragments/success_message.html",
                    data!({
                        "message": "PGP verification email sent successfully. Please check your inbox.",
                        "target": "#notification-container"
                    }),
                )
            }
            Err(e) => {
                tracing::error!(
                    "Failed to send PGP verification email to {}: {}",
                    updated_user.email,
                    e
                );
                // Pass target selector, rely on logging for error details
                error_fragment(
                    &v,
                    "Failed to send PGP verification email.",
                    "#notification-container",
                )
            }
        }
    } else {
        // This should not happen if token generation succeeded
        tracing::error!(
            "PGP token missing after generation for user {}",
            updated_user.id
        );
        // Pass a default target selector instead of None
        error_fragment(
            &v,
            "Internal error: PGP token generation failed.",
            "#notification-container",
        )
    }
}

/// Deletes an SSH key
#[debug_handler]
async fn delete_ssh_key(
    auth: JWTWithUserOpt<users::Model>,
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(params): Form<HashMap<String, String>>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    let key_id_str = match params.get("key_id") {
        Some(id) => id,
        None => {
            return error_fragment(&v, "Missing key ID", "#ssh-key-error-container");
        }
    };

    let key_id = match key_id_str.parse::<i32>() {
        Ok(id) => id,
        Err(_) => {
            return error_fragment(&v, "Invalid key ID format", "#ssh-key-error-container");
        }
    };

    // Find the key
    let key_result = ssh_keys::Entity::find_by_id(key_id).one(&ctx.db).await?;

    let key = match key_result {
        Some(k) => k,
        None => {
            return error_fragment(&v, "SSH Key not found", "#ssh-key-error-container");
        }
    };

    // Verify ownership
    if key.user_id != user.id {
        return error_fragment(
            &v,
            "You do not own this SSH key",
            "#ssh-key-error-container",
        );
    }

    // Delete the key
    let key_model: ssh_keys::ActiveModel = key.into();
    match key_model.delete(&ctx.db).await {
        Ok(_) => {
            // Return an empty response with OK status code
            // HTMX will remove the element based on hx-target="closest div"
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html") // Ensure correct content type for HTMX
                .body(axum::body::Body::empty())?)
        }
        Err(e) => {
            tracing::error!("Failed to delete SSH key {}: {}", key_id, e);
            error_fragment(&v, "Failed to delete SSH key.", "#ssh-key-error-container")
        }
    }
}

/// User routes
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/users")
        .add("/profile", get(profile).post(update_profile))
        .add("/profile/ssh_keys_fragment", get(ssh_keys_fragment))
        .add("/profile/ssh_keys", post(add_ssh_key))
        .add("/profile/password", post(update_password))
        .add("/invitations", get(invitations))
        .add("/invitations/count", get(get_invitation_count))
        .add(
            "/profile/resend-verification",
            post(resend_verification_email),
        )
        .add("/profile/refresh-pgp", post(refresh_pgp))
        .add("/profile/verify-pgp", post(verify_pgp_sending))
        .add("/profile/ssh_keys", delete(delete_ssh_key))
}
