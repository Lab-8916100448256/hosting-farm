use crate::{
    middleware::auth_no_error::JWTWithUserOpt,
    models::{
        _entities::{ssh_keys, users}, // Import users entity
        ssh_keys::{CreateSshKeyParams, Model as SshKeyModel}, // Use the custom SshKeyModel alias
        team_memberships,
        teams::{Model as TeamModel, Role}, // Import TeamModel and Role
        users::{self as UserModelCustom, Model as UserModel}, // Import custom UserModel
    },
    views::{
        error_fragment, error_page, redirect, render_template, // Import render_template
        users::{PgpSectionResponse, SshKeysListResponse},
    },
};
use axum::{
    debug_handler,
    extract::{Form, Path, Query, State},
    http::{header::HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
};
use loco_rs::{app::AppContext, prelude::*};
use mailers::auth::AuthMailer; // Import AuthMailer
// Import PaginatorTrait for count() method
use sea_orm::{ActiveValue::{self, Set}, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::Deserialize;
use serde_json::json;
use tracing::{error, info, warn};
use uuid::Uuid; // Add Uuid import

// Structures for form data

#[derive(Deserialize, Validate)]
pub struct SshKeyPayload {
    // Renamed name to label to match model changes
    #[validate(length(min = 1, message = "Label cannot be empty"))]
    pub label: String,
    #[validate(length(min = 1, message = "Key cannot be empty"))]
    pub key: String,
}

#[derive(Deserialize, Validate)]
pub struct ChangePasswordPayload {
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub current_password: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub new_password: String,
    #[validate(must_match(other = "new_password", message = "Passwords must match"))]
    pub confirm_password: String,
}

#[derive(Deserialize, Validate)]
pub struct UpdateProfileParams {
    #[validate(length(min = 3, message = "Name must be at least 3 characters long"))]
    pub name: String,
    // Email update removed, handled separately
}

#[derive(Deserialize, Validate)]
pub struct UpdateEmailParams {
    #[validate(email(message = "Invalid email format."))]
    pub email: String,
}

#[derive(Deserialize, Validate)]
pub struct UpdatePgpParams {
    // Allow empty string for clearing the key
    pub pgp_key: String,
}

// User profile page and related handlers

#[debug_handler]
async fn profile_page(
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<UserModel>, // Use custom Model directly
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(user_pid = %user.pid, "Loading profile page");

    // Get user's teams
    let memberships = match team_memberships::Entity::find()
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(false))
        .find_with_related(crate::models::_entities::teams::Entity)
        .all(&ctx.db)
        .await
    {
        Ok(m) => m,
        Err(e) => {
            error!(error = ?e, "Failed to load team memberships for user {}", user.pid);
            return error_page(&v, "Could not load your team information.", Some(Error::from(e)));
        }
    };

    let teams_with_roles: Vec<_> = memberships
        .into_iter()
        .filter_map(|(membership, team_vec)| {
            team_vec.into_iter().next().map(|team| {
                json!({
                    "pid": team.pid.to_string(),
                    "name": team.name,
                    "role": membership.role
                })
            })
        })
        .collect();

    // Get user's SSH keys using the custom SshKeyModel
    let ssh_keys = match SshKeyModel::find_by_user(&ctx.db, user.id).await {
        Ok(keys) => keys,
        Err(e) => {
            error!(error = ?e, "Failed to load SSH keys for user {}", user.pid);
            return error_page(&v, "Could not load your SSH keys.", Some(Error::Model(e)));
        }
    };

    // Get user's PGP key status
    let has_pgp_key = user.pgp_key.is_some();
    let is_pgp_verified = user.pgp_verified_at.is_some();

    // Get pending invitations count using PaginatorTrait::count
    let invitations_count = match team_memberships::Entity::find()
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(true))
        .count(&ctx.db)
        .await
    {
        Ok(count) => count,
        Err(e) => {
            error!(error = ?e, "Failed to load invitation count for user {}", user.pid);
            return error_page(&v, "Could not load your invitations.", Some(Error::from(e)));
        }
    };

    render_template(
        &v,
        "users/profile.html",
        json!({
            "user": &user.inner, // Pass inner entity model to template
            "teams": teams_with_roles,
            "ssh_keys": &ssh_keys, // Pass the found SshKeyModel instances
            "has_pgp_key": has_pgp_key,
            "is_pgp_verified": is_pgp_verified,
            "active_page": "profile",
            "invitation_count": invitations_count,
        }),
    )
}

#[debug_handler]
async fn update_profile(
    auth: JWTWithUserOpt<UserModel>, // Use custom Model
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

    info!(user_pid = %user.pid, new_name = %params.name, "Attempting profile update");

    // Validate parameters
    if let Err(errors) = params.validate() {
        warn!(user_pid = %user.pid, validation_errors = ?errors, "Profile update validation failed");
        // Fix validation error processing
        let error_message = errors.field_errors().iter().flat_map(|(_, errors)| errors.iter()).filter_map(|e| e.message.as_ref().map(|m| m.to_string())).collect::<Vec<_>>().join(" ");
        return error_fragment(&v, &error_message, "#profile-error");
    }

    // Trim the name
    let trimmed_name = params.name.trim();

    // Check if name is changing and if it's unique
    if user.name != trimmed_name {
        let name_exists = match users::Entity::find()
            .filter(users::Column::Name.eq(trimmed_name))
            .filter(users::Column::Id.ne(user.id))
            .one(&ctx.db)
            .await
        {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(e) => {
                error!(error = ?e, "Database error checking username uniqueness for user {}", user.pid);
                return error_fragment(&v, "Could not verify username availability.", "#profile-error");
            }
        };

        if name_exists {
            warn!(user_pid = %user.pid, requested_name = %trimmed_name, "Profile update failed: username already exists");
            return error_fragment(&v, "Username already exists.", "#profile-error");
        }
    }

    // Update user model
    // Convert custom Model to ActiveModel
    let mut user_am: users::ActiveModel = user.inner.clone().into(); // Explicit conversion 
    user_am.name = Set(trimmed_name.to_string());

    // Get pid *before* consuming user_am in the update call
    // Safely get the pid as string, handle potential None if pid wasn't set (shouldn't happen here)
    let user_pid_for_logging = user.pid.to_string();

    match user_am.update(&ctx.db).await {
        Ok(updated_user_entity) => {
            info!(user_pid = %updated_user_entity.pid, "Profile updated successfully");
            // Re-render the profile form section with success message
            let updated_user_model = UserModel::from(updated_user_entity); // Convert back to custom model if needed
            let context = json!({
                "user": &updated_user_model.inner, // Pass inner entity
                "success_message": "Profile updated successfully."
            });
            match v.render("users/_profile_form.html", context) {
                Ok(html) => Ok(Html(html).into_response()),
                Err(e) => {
                    error!(error = ?e, "Failed to render profile form fragment after update");
                    Ok(Html("<p class=\"text-green-600\">Profile updated, but UI refresh failed.</p>".to_string()).into_response())
                }
            }
        }
        Err(e) => {
            error!(error = ?e, user_pid = %user_pid_for_logging, "Failed to update profile");
            error_fragment(&v, "Failed to update profile.", "#profile-error")
        }
    }
}

#[debug_handler]
async fn update_email(
    auth: JWTWithUserOpt<UserModel>, // Use custom Model
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(params): Form<UpdateEmailParams>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    let new_email = params.email.trim().to_lowercase();
    info!(user_pid = %user.pid, new_email = %new_email, "Attempting email update");

    if user.email == new_email {
         warn!(user_pid = %user.pid, "Email update attempt with same email");
         // Return the email form fragment, maybe with a neutral message?
         let context = json!({ "user": &user.inner }); // Use inner entity
         return match v.render("users/_email_form.html", context) {
             Ok(html) => Ok(Html(html).into_response()),
             Err(e) => {
                 error!(error = ?e, "Failed to render email form fragment");
                 error_fragment(&v, "UI Error.", "#email-error")
             }
         };
    }

    // Validate parameters
    if let Err(errors) = params.validate() {
        warn!(user_pid = %user.pid, validation_errors = ?errors, "Email update validation failed");
         // Fix validation error processing
        let error_message = errors.field_errors().iter().flat_map(|(_, errors)| errors.iter()).filter_map(|e| e.message.as_ref().map(|m| m.to_string())).collect::<Vec<_>>().join(" ");
        return error_fragment(&v, &error_message, "#email-error");
    }

    // Check email uniqueness
    let email_exists = match users::Entity::find()
        .filter(users::Column::Email.eq(&new_email))
        .one(&ctx.db)
        .await
    {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(e) => {
            error!(error = ?e, "Database error checking email uniqueness for user {}", user.pid);
            return error_fragment(&v, "Could not verify email availability.", "#email-error");
        }
    };

    if email_exists {
        warn!(user_pid = %user.pid, requested_email = %new_email, "Email update failed: email already exists");
        return error_fragment(&v, "Email already exists.", "#email-error");
    }

    // Generate token BEFORE updating the user model
    let token = Uuid::new_v4().to_string();

    // Update user model: set new email, clear verification status, set new token
    let mut user_am: users::ActiveModel = user.inner.clone().into(); // Explicit conversion // Use conversion method on custom Model
    user_am.email = Set(new_email.clone());
    user_am.email_verified_at = Set(None);
    user_am.email_verification_sent_at = Set(Some(chrono::Utc::now().into()));
    user_am.email_verification_token = Set(Some(token.clone())); // Use the generated token

     // Get pid *before* consuming user_am in the update call
    // Safely get the pid as string
    let user_pid_for_logging = user.pid.to_string();

    match user_am.update(&ctx.db).await {
        Ok(updated_user_entity) => {
            info!(user_pid = %updated_user_entity.pid, "Email update initiated, verification required");

            // Send verification email to the *new* address
            let updated_user_model = UserModel::from(updated_user_entity);
            match AuthMailer::send_verification(&ctx, &updated_user_model, &token).await {
                Ok(_) => info!(user_pid = %updated_user_model.pid, email = %new_email, "Verification email sent to new address"),
                Err(e) => error!(error = ?e, user_pid = %updated_user_model.pid, email = %new_email, "Failed to send verification email to new address"),
            }

            // Re-render the profile form section with success message
            let context = json!({
                "user": &updated_user_model.inner, // Use inner entity
                "success_message": "Email change initiated. Please check your new email address for a verification link."
            });
            match v.render("users/_email_form.html", context) {
                Ok(html) => Ok(Html(html).into_response()),
                Err(e) => {
                    error!(error = ?e, "Failed to render email form fragment after update");
                    Ok(Html("<p class=\"text-green-600\">Email change initiated, but UI refresh failed.</p>".to_string()).into_response())
                }
            }
        }
        Err(e) => {
            error!(error = ?e, user_pid = %user_pid_for_logging, "Failed to update email");
            error_fragment(&v, "Failed to update email.", "#email-error")
        }
    }
}

#[debug_handler]
async fn change_password(
    auth: JWTWithUserOpt<UserModel>, // Use custom Model
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(params): Form<ChangePasswordPayload>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(user_pid = %user.pid, "Attempting password change");

    // Validate parameters
    if let Err(errors) = params.validate() {
        warn!(user_pid = %user.pid, validation_errors = ?errors, "Password change validation failed");
         // Fix validation error processing
        let error_message = errors.field_errors().iter().flat_map(|(_, errors)| errors.iter()).filter_map(|e| e.message.as_ref().map(|m| m.to_string())).collect::<Vec<_>>().join(" ");
        return error_fragment(&v, &error_message, "#password-error");
    }

    // Verify current password
    // Fix type mismatch: user.password is Option<String>
    let current_password_hash_opt: Option<String> = user.password.clone(); // Directly clone the Option<String>
    let current_password_hash: &str = match &current_password_hash_opt {
        Some(hash) => hash.as_str(),
        None => {
            error!(user_pid = %user.pid, "User has no password set, cannot change password");
            return error_fragment(&v, "Cannot change password. No current password set.", "#password-error");
        }
    };

    if let Err(e) = UserModelCustom::verify_password(current_password_hash, &params.current_password) {
        warn!(user_pid = %user.pid, "Password change failed: incorrect current password");
        // Use ModelError::Message for comparison
        let error_message = match e {
            ModelError::Message(msg) if msg == "invalid email or password" => "Incorrect current password.".to_string(),
            _ => "Failed to verify current password.".to_string(),
        };
        return error_fragment(&v, &error_message, "#password-error");
    }

    // Update password
    // Convert custom Model to ActiveModel
    let mut user_am: users::ActiveModel = user.inner.clone().into(); // Explicit conversion 
    // Fix type mismatch: password field in entity is Option<String>, so Set expects Option<String>
    user_am.password = Set(Some(params.new_password.clone())); // Hashing happens in before_save

     // Get pid *before* consuming user_am in the update call
    // Safely get the pid as string
    let user_pid_for_logging = user.pid.to_string();

    match user_am.update(&ctx.db).await {
        Ok(updated_user_entity) => {
            info!(user_pid = %updated_user_model.pid, "Password changed successfully");
            // Re-render the password form section with success message
            let context = json!({ "success_message": "Password changed successfully." });
            match v.render("users/_password_form.html", context) {
                Ok(html) => Ok(Html(html).into_response()),
                Err(e) => {
                    error!(error = ?e, "Failed to render password form fragment after update");
                    Ok(Html("<p class=\"text-green-600\">Password updated, but UI refresh failed.</p>".to_string()).into_response())
                }
            }
        }
        Err(e) => {
            error!(error = ?e, user_pid = %user_pid_for_logging, "Failed to change password");
            error_fragment(&v, "Failed to change password.", "#password-error")
        }
    }
}

#[debug_handler]
async fn invitations_page(
    ViewEngine(v): ViewEngine<TeraView>,
    auth: JWTWithUserOpt<UserModel>, // Use custom Model
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<impl IntoResponse> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(user_pid = %user.pid, "Loading invitations page");

    // Get pending invitations
    let invitations_with_teams = match team_memberships::Entity::find()
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(true))
        .order_by_desc(team_memberships::Column::InvitationSentAt)
        .find_with_related(crate::models::_entities::teams::Entity) // Use Entity
        .all(&ctx.db)
        .await
    {
        Ok(invites) => invites,
        Err(e) => {
            error!(error = ?e, "Failed to load invitations for user {}", user.pid);
            return error_page(
                &v,
                "Could not load your invitations. Please try again later.",
                Some(Error::from(e)),
            );
        }
    };

    let invitations_json: Vec<_> = invitations_with_teams
        .into_iter()
        .filter_map(|(membership, team_vec)| {
            team_vec.into_iter().next().map(|team| {
                json!({
                    "token": membership.invitation_token,
                    "team_name": team.name,
                    "team_pid": team.pid.to_string(),
                    "sent_at": membership.invitation_sent_at
                })
            })
        })
        .collect();

    // Get pending invitations count (re-fetch as it might have changed) using PaginatorTrait::count
    let invitations_count = match team_memberships::Entity::find()
        .filter(team_memberships::Column::UserId.eq(user.id))
        .filter(team_memberships::Column::Pending.eq(true))
        .count(&ctx.db)
        .await
    {
        Ok(count) => count,
        Err(e) => {
            error!(error = ?e, "Failed to load invitation count for user {}", user.pid);
            return error_page(
                &v,
                "Could not load your invitation count.",
                Some(Error::from(e)),
            );
        }
    };

    render_template(
        &v,
        "users/invitations.html",
        json!({
            "user": &user.inner, // Use inner entity
            "invitations": invitations_json,
            "active_page": "invitations",
            "invitation_count": invitations_count,
        }),
    )
}

#[debug_handler]
async fn invitation_count_badge(
    auth: JWTWithUserOpt<UserModel>, // Use custom Model
    State(ctx): State<AppContext>,
) -> Result<impl IntoResponse> {
    if let Some(user) = auth.user {
        // Use PaginatorTrait::count
        let count = team_memberships::Entity::find()
            .filter(team_memberships::Column::UserId.eq(user.id))
            .filter(team_memberships::Column::Pending.eq(true))
            .count(&ctx.db)
            .await?;

        if count > 0 {
            let badge_html = format!(
                r#"<span class="ml-1 inline-flex items-center rounded-full bg-red-50 px-2 py-1 text-xs font-medium text-red-700 ring-1 ring-inset ring-red-600/10">{}</span>"#,
                count
            );
            Ok(Html(badge_html).into_response())
        } else {
            Ok(Html("".to_string()).into_response()) // Return empty if count is 0
        }
    } else {
        Ok(Html("".to_string()).into_response()) // Return empty if not logged in
    }
}

// SSH Key Management Handlers

#[debug_handler]
async fn add_ssh_key(
    auth: JWTWithUserOpt<UserModel>, // Use custom Model
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap, // Needed for redirect on auth failure
    Form(params): Form<SshKeyPayload>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(user_pid = %user.pid, key_label = %params.label, "Attempting to add SSH key"); // Use label

    // Validate parameters
    if let Err(errors) = params.validate() {
        warn!(user_pid = %user.pid, validation_errors = ?errors, "Add SSH key validation failed");
         // Fix validation error processing
        let error_message = errors.field_errors().iter().flat_map(|(_, errors)| errors.iter()).filter_map(|e| e.message.as_ref().map(|m| m.to_string())).collect::<Vec<_>>().join(" ");
        return error_fragment(&v, &error_message, "#ssh-key-error");
    }

    // Basic validation for SSH key format (starts with ssh-rsa, ssh-ed25519, etc.)
    let trimmed_key = params.key.trim();
    if !trimmed_key.starts_with("ssh-rsa") && !trimmed_key.starts_with("ssh-ed25519") && !trimmed_key.starts_with("ecdsa-sha2-nistp") {
         warn!(user_pid = %user.pid, "Add SSH key failed: invalid format");
         return error_fragment(&v, "Invalid SSH key format.", "#ssh-key-error");
    }

    let create_params = CreateSshKeyParams {
        label: params.label.trim().to_string(), // Use label
        key: trimmed_key.to_string(),
    };

    // Use the custom SshKeyModel for create_key
    match SshKeyModel::create_key(&ctx.db, user.id, &create_params).await {
        Ok(_) => {
            info!(user_pid = %user.pid, key_label = %create_params.label, "SSH key added successfully"); // Use label
            // Fetch updated keys using custom SshKeyModel and render the list fragment
            let ssh_keys = SshKeyModel::find_by_user(&ctx.db, user.id).await?;
            let context = json!({ "ssh_keys": ssh_keys });
            format::render().view(&v, "users/_ssh_keys_list.html", context)
        }
        Err(e) => {
            error!(error = ?e, user_pid = %user.pid, "Failed to add SSH key");
            let error_message = match e {
                 ModelError::Message(msg) => msg, // For uniqueness errors
                 _ => "Failed to add SSH key due to an unexpected error.".to_string(),
            };
            error_fragment(&v, &error_message, "#ssh-key-error")
        }
    }
}

#[debug_handler]
async fn delete_ssh_key(
    auth: JWTWithUserOpt<UserModel>, // Use custom Model
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap, // Needed for redirect on auth failure
    Path(key_id): Path<i32>, // Expect key ID (primary key)
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(user_pid = %user.pid, key_id = %key_id, "Attempting to delete SSH key");

    // Use the custom SshKeyModel for find_by_id
    match SshKeyModel::find_by_id(&ctx.db, key_id).await {
        Ok(key) => {
            // Verify ownership
            if key.user_id != user.id {
                warn!(user_pid = %user.pid, key_id = %key_id, "Unauthorized attempt to delete SSH key");
                return error_fragment(&v, "Unauthorized.", "#ssh-key-error");
            }

            match key.delete(&ctx.db).await {
                Ok(_) => {
                    info!(user_pid = %user.pid, key_id = %key_id, "SSH key deleted successfully");
                    // Fetch updated keys using custom SshKeyModel and render the list fragment
                    let ssh_keys = SshKeyModel::find_by_user(&ctx.db, user.id).await?;
                    let context = json!({ "ssh_keys": ssh_keys });
                    format::render().view(&v, "users/_ssh_keys_list.html", context)
                }
                Err(e) => {
                    error!(error = ?e, user_pid = %user.pid, key_id = %key_id, "Failed to delete SSH key");
                    error_fragment(
                        &v,
                        "Failed to delete SSH key.",
                        "#ssh-key-error",
                    )
                }
            }
        }
        Err(ModelError::EntityNotFound) => {
            warn!(user_pid = %user.pid, key_id = %key_id, "Attempt to delete non-existent SSH key");
            error_fragment(&v, "Key not found.", "#ssh-key-error")
        }
        Err(e) => {
            error!(error = ?e, user_pid = %user.pid, key_id = %key_id, "Error finding SSH key for deletion");
            error_fragment(&v, "Error finding key.", "#ssh-key-error")
        }
    }
}

// Email Verification Banner & Resend Logic

#[debug_handler]
async fn resend_verification_email(
    auth: JWTWithUserOpt<UserModel>, // Use custom Model
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(user_pid = %user.pid, "Resending verification email");

    if user.email_verified_at.is_some() {
        warn!(user_pid = %user.pid, "Verification email resend requested for already verified email");
        // Optionally return a message indicating it's already verified
        return Ok(Html("<div class=\"text-yellow-600 text-sm\"><p>Your email is already verified.</p></div>".to_string()).into_response());
    }

    // Check if a token was sent recently (e.g., within the last 5 minutes)
    if let Some(sent_at) = user.email_verification_sent_at {
         if sent_at + chrono::Duration::minutes(5) > chrono::Utc::now() {
            warn!(user_pid = %user.pid, "Verification email resend requested too soon");
            return Ok(Html("<div class=\"text-yellow-600 text-sm\"><p>Verification email recently sent. Please check your inbox or wait a few minutes before trying again.</p></div>".to_string()).into_response());
         }
    }

    // Generate new token using the custom UserModel method
    match user
        .generate_email_verification_token(&ctx.db)
        .await
    {
        Ok(token) => {
            // Send the email
            // Refetch user to get the updated token/sent_at for the mailer
            match UserModel::find_by_pid(&ctx.db, &user.pid.to_string()).await {
                Ok(user_with_token) => {
                    match AuthMailer::send_verification(&ctx, &user_with_token, &token).await {
                        Ok(_) => {
                            info!(user_pid = %user.pid, "Verification email resent successfully");
                            Ok(Html("<div class=\"text-green-600 text-sm\"><p>Verification email resent. Please check your inbox.</p></div>".to_string()).into_response())
                        }
                        Err(e) => {
                            error!(error = ?e, user_pid = %user.pid, "Failed to send verification email");
                             // Reset sent_at time if sending failed?
                             // Call method on user_with_token, then convert to ActiveModel for update
                             // Convert custom Model to ActiveModel
                             let mut user_am = user_with_token.clone().into_active_model();
                             user_am.email_verification_sent_at = Set(None);

                              // Get pid *before* consuming user_am in the update call
                              // Safely get the pid as string
                             let user_pid_for_logging = user_with_token.pid.to_string(); // Use user_with_token here

                             if let Err(update_err) = user_am.update(&ctx.db).await {
                                 error!(error = ?update_err, user_pid = %user_pid_for_logging, "Failed to reset email_verification_sent_at after send failure");
                             }
                            Ok(Html("<div class=\"text-red-600 text-sm\"><p>Failed to send verification email. Please try again later.</p></div>".to_string()).into_response())
                        }
                    }
                }
                Err(e) => {
                    error!(error = ?e, user_pid = %user.pid, "Failed to refetch user after generating verification token");
                     Ok(Html("<div class=\"text-red-600 text-sm\"><p>Failed to resend email due to an internal error.</p></div>".to_string()).into_response())
                }
            }
        }
        Err(e) => {
            error!(error = ?e, user_pid = %user.pid, "Failed to generate email verification token");
            Ok(Html("<div class=\"text-red-600 text-sm\"><p>Failed to resend email due to an internal error.</p></div>".to_string()).into_response())
        }
    }
}

// PGP Key Management Handlers

#[debug_handler]
async fn update_pgp_key(
    auth: JWTWithUserOpt<UserModel>, // Use custom Model
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    Form(params): Form<UpdatePgpParams>,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(user_pid = %user.pid, "Attempting to update PGP key");

    // Validate parameters (basic check for now)
    // You might want more robust PGP key validation (e.g., using a crate)
    let trimmed_key = params.pgp_key.trim();

    if trimmed_key.is_empty() {
        // Clearing the PGP key
        info!(user_pid = %user.pid, "Clearing PGP key");
        // Convert custom Model to ActiveModel
        let mut user_am: users::ActiveModel = user.inner.clone().into(); // Explicit conversion 
        user_am.pgp_key = Set(None);
        user_am.pgp_verified_at = Set(None); // Clear verification status too
        user_am.pgp_verification_token = Set(None);

         // Get pid *before* consuming user_am in the update call
         // Safely get the pid as string
        let user_pid_for_logging = user.pid.to_string();

        return match user_am.update(&ctx.db).await {
            Ok(updated_user_entity) => {
                info!(user_pid = %updated_user_entity.pid, "PGP key cleared successfully");
                let updated_user = UserModel::from(updated_user_entity);
                let context = json!({
                    "user": &updated_user.inner,
                    "has_pgp_key": false,
                    "is_pgp_verified": false,
                    "success_message": "PGP key removed."
                });
                format::render().view(&v, "users/_pgp_section.html", context)
            }
            Err(e) => {
                error!(error = ?e, user_pid = %user_pid_for_logging, "Failed to clear PGP key");
                error_fragment(&v, "Failed to remove PGP key.", "#pgp-error")
            }
        };
    }

    // Setting or updating the PGP key
    if !trimmed_key.starts_with("-----BEGIN PGP PUBLIC KEY BLOCK-----") {
        warn!(user_pid = %user.pid, "Update PGP key failed: invalid format");
        return error_fragment(&v, "Invalid PGP public key format.", "#pgp-error");
    }

    // Convert custom Model to ActiveModel
    let mut user_am: users::ActiveModel = user.inner.clone().into(); // Explicit conversion 
    user_am.pgp_key = Set(Some(trimmed_key.to_string()));
    user_am.pgp_verified_at = Set(None); // Reset verification on key change
    user_am.pgp_verification_token = Set(None);

     // Get pid *before* consuming user_am in the update call
     // Safely get the pid as string
    let user_pid_for_logging = user.pid.to_string();

    match user_am.update(&ctx.db).await {
        Ok(updated_user_entity) => {
            info!(user_pid = %updated_user_entity.pid, "PGP key updated successfully, verification needed");
            let updated_user = UserModel::from(updated_user_entity);
             let context = json!({
                "user": &updated_user.inner,
                "has_pgp_key": true,
                "is_pgp_verified": false,
                "success_message": "PGP key updated. Please verify it."
            });
             format::render().view(&v, "users/_pgp_section.html", context)
        }
        Err(e) => {
            error!(error = ?e, user_pid = %user_pid_for_logging, "Failed to update PGP key");
            error_fragment(&v, "Failed to update PGP key.", "#pgp-error")
        }
    }
}

#[debug_handler]
async fn send_pgp_verification(
    auth: JWTWithUserOpt<UserModel>, // Use custom Model
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    headers: HeaderMap,
) -> Result<Response> {
    let user = if let Some(user) = auth.user {
        user
    } else {
        return redirect("/auth/login", headers);
    };

    info!(user_pid = %user.pid, "Requesting PGP verification email");

    if user.pgp_key.is_none() {
        warn!(user_pid = %user.pid, "PGP verification requested but no key set");
        return error_fragment(&v, "No PGP key set.", "#pgp-error");
    }
    if user.pgp_verified_at.is_some() {
        warn!(user_pid = %user.pid, "PGP verification requested for already verified key");
        return error_fragment(&v, "PGP key already verified.", "#pgp-error");
    }

    // TODO: Add rate limiting check? Similar to email verification resend?

    // Generate token using the custom UserModel method and send email
    match user.generate_pgp_verification_token(&ctx.db).await {
        Ok(token) => {
             // Refetch user to ensure we have the latest state (with the token)
            match UserModel::find_by_pid(&ctx.db, &user.pid.to_string()).await {
                Ok(user_with_token) => {
                    match AuthMailer::send_pgp_verification(&ctx, &user_with_token, &token).await {
                        Ok(_) => {
                            info!(user_pid = %user.pid, "PGP verification email sent successfully");
                             let context = json!({
                                "user": &user_with_token.inner,
                                "has_pgp_key": true,
                                "is_pgp_verified": false,
                                "info_message": "Verification email sent. Check your inbox."
                            });
                            format::render().view(&v, "users/_pgp_section.html", context)
                        }
                        Err(e) => {
                            error!(error = ?e, user_pid = %user.pid, "Failed to send PGP verification email");
                             // Reset token if send failed?
                              // Call method on user_with_token, then convert to ActiveModel for update
                              // Convert custom Model to ActiveModel
                             let mut user_am = user_with_token.clone().into_active_model();
                             user_am.pgp_verification_token = Set(None);

                              // Get pid *before* consuming user_am in the update call
                              // Safely get the pid as string
                             let user_pid_for_logging = user_with_token.pid.to_string(); // Use user_with_token here

                             if let Err(update_err) = user_am.update(&ctx.db).await {
                                  error!(error = ?update_err, user_pid = %user_pid_for_logging, "Failed to reset pgp_verification_token after send failure");
                             }
                            error_fragment(
                                &v,
                                "Failed to send verification email. Please try again later.",
                                "#pgp-error",
                            )
                        }
                    }
                }
                 Err(e) => {
                     error!(error = ?e, user_pid = %user.pid, "Failed to refetch user after generating PGP token");
                      error_fragment(&v, "Failed to send email due to an internal error.", "#pgp-error")
                 }
            }
        }
        Err(e) => {
            error!(error = ?e, user_pid = %user.pid, "Failed to generate PGP verification token");
            error_fragment(
                &v,
                "Failed to initiate PGP verification. Please try again later.",
                "#pgp-error",
            )
        }
    }
}

#[derive(Deserialize)]
pub struct PgpVerifyQuery {
    token: String,
}

#[debug_handler]
async fn verify_pgp_page(
    ViewEngine(v): ViewEngine<TeraView>,
    State(ctx): State<AppContext>,
    Query(query): Query<PgpVerifyQuery>,
) -> Result<impl IntoResponse> {
    info!(token = %query.token, "Attempting PGP verification via link");

    let user = match users::Entity::find()
        .filter(users::Column::PgpVerificationToken.eq(query.token.clone()))
        .one(&ctx.db)
        .await
    {
        Ok(Some(user_entity)) => UserModel::from(user_entity),
        Ok(None) => {
            warn!(token = %query.token, "Invalid or expired PGP verification token");
            return render_template(
                &v,
                "users/pgp_verify.html", // Create this template
                json!({ "title": "PGP Verification Failed", "success": false, "message": "Invalid or expired verification link." }),
            );
        }
        Err(e) => {
            error!(error = ?e, "Database error during PGP verification lookup");
            return error_page(&v, "An unexpected error occurred during verification.", Some(Error::from(e)));
        }
    };

    match user.verify_pgp(&ctx.db, &query.token).await {
        Ok(_) => {
            info!(user_pid = %user.pid, "PGP key verified successfully");
            render_template(
                &v,
                 "users/pgp_verify.html",
                 json!({ "title": "PGP Key Verified", "success": true, "message": "Your PGP key has been verified successfully!" }),
            )
        }
        Err(e) => {
            error!(error = ?e, user_pid = %user.pid, "PGP verification failed");
            // Use ModelError::Message for comparison
            let message = match e {
                 ModelError::Message(msg) if msg == "invalid token" => "Invalid or expired PGP verification token.".to_string(),
                _ => "PGP key verification failed due to an unexpected error.".to_string(),
            };
            render_template(
                &v,
                 "users/pgp_verify.html",
                json!({ "title": "PGP Verification Failed", "success": false, "message": message }),
            )
        }
    }
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/users")
        .add("/profile", get(profile_page))
        .add("/profile/update", post(update_profile)) // For name update
        .add("/profile/email", post(update_email)) // For email update
        .add("/profile/password", post(change_password))
        .add("/invitations", get(invitations_page))
        .add("/invitations/count", get(invitation_count_badge))
        .add("/ssh_keys", post(add_ssh_key)) // Add SSH key
        .add("/ssh_keys/{key_id}", delete(delete_ssh_key)) // Delete SSH key by ID
        .add("/profile/resend-verification", post(resend_verification_email))
        .add("/profile/pgp", post(update_pgp_key)) // Update/set PGP key
        .add("/profile/pgp/send-verification", post(send_pgp_verification))
        .add("/pgp/verify", get(verify_pgp_page))
}
