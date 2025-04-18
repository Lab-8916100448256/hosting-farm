<!-- Generated by SpecStory -->

# Profile Page Feature Implementation Issues (2025-03-26 13:40:06)

_**User**_

On the profile page, there are some features that are not implemented yet : 
- the button to validate the profile information modification is not doing what it should
- the button to validate password modification is not doing what it should
- it is currently possible to create a team only if the user team list is empty. The user needs to be able to create a new team even if he is already memeber of one or several teams.
Implement these features and use the rules while doing so.


---

_**Assistant**_



---

# Implementing Profile Page Features

I'll implement the missing features for the profile page. Based on the provided rules, we need to:

1. Implement profile information modification functionality
2. Implement password modification functionality 
3. Allow team creation even when the user is already a member of teams

To follow the Rust web app development rules and endpoint structure rules, I'll ensure we:
- Keep human-facing endpoints in `_pages.rs` files
- Keep API endpoints in `.rs` files
- Use server-side rendering with HTMX
- Use Tera templates for HTML
- Follow proper response handling patterns

## 1. Profile Information Update

First, let's implement the API endpoint for processing profile information updates:

```rust:src/controllers/user.rs
// ... existing code ...

pub async fn update_profile(
    State(ctx): State<AppContext>,
    auth: auth::JWT,
    Form(params): Form<ProfileUpdateParams>,
) -> Result<impl IntoResponse> {
    let user = auth.current_user(&ctx).await?;
    
    // Update user information
    let user = models::users::ActiveModel {
        id: Set(user.id),
        name: Set(params.name.unwrap_or(user.name)),
        email: Set(params.email.unwrap_or(user.email)),
        // Keep other fields unchanged
        ..Default::default()
    };
    
    // Save changes
    let updated_user = user.update(&ctx.db).await?;
    
    Ok(format::json(updated_user))
}

// ... existing code ...
```

Now, let's update the human-facing endpoint:

```rust:src/controllers/user_pages.rs
// ... existing code ...

pub async fn profile_page(
    State(ctx): State<AppContext>,
    auth: auth::JWT,
) -> Result<impl IntoResponse> {
    let user = auth.current_user(&ctx).await?;
    
    // Get user's teams
    let teams = models::teams::Entity::find()
        .join(
            sea_orm::JoinType::InnerJoin,
            models::teams::Entity::belongs_to(models::team_users::Entity)
                .from(models::teams::Column::Id)
                .to(models::team_users::Column::TeamId)
                .into(),
        )
        .filter(models::team_users::Column::UserId.eq(user.id))
        .all(&ctx.db)
        .await?;
    
    // Render template with user data and success message if any
    render_template(
        "profile.html.tera",
        context! {
            user => user,
            teams => teams,
            // Allow team creation regardless of existing teams
            can_create_team => true, 
        },
    )
}

pub async fn update_profile_action(
    State(ctx): State<AppContext>,
    auth: auth::JWT,
    Form(params): Form<ProfileUpdateParams>,
) -> Result<impl IntoResponse> {
    let user = auth.current_user(&ctx).await?;
    
    // Update user information
    let mut user_model = models::users::ActiveModel {
        id: Set(user.id),
        ..Default::default()
    };
    
    if let Some(name) = params.name {
        user_model.name = Set(name);
    }
    
    if let Some(email) = params.email {
        user_model.email = Set(email);
    }
    
    // Save changes
    user_model.update(&ctx.db).await?;
    
    // Redirect to profile page with success message
    Ok(Redirect::to("/profile?success=profile_updated"))
}

// ... existing code ...
```

## 2. Password Modification

Let's implement the password update functionality:

```rust:src/controllers/user.rs
// ... existing code ...

pub async fn update_password(
    State(ctx): State<AppContext>,
    auth: auth::JWT,
    Form(params): Form<PasswordUpdateParams>,
) -> Result<impl IntoResponse> {
    let user = auth.current_user(&ctx).await?;
    
    // Verify current password
    if !auth::verify_password(&params.current_password, &user.password_hash)? {
        return Err(Error::Unauthorized("Invalid current password".into()));
    }
    
    // Validate new password
    if params.new_password != params.confirm_password {
        return Err(Error::BadRequest("Passwords do not match".into()));
    }
    
    // Update password
    let password_hash = auth::hash_password(&params.new_password)?;
    let user = models::users::ActiveModel {
        id: Set(user.id),
        password_hash: Set(password_hash),
        // Keep other fields unchanged
        ..Default::default()
    };
    
    // Save changes
    user.update(&ctx.db).await?;
    
    Ok(format::json(serde_json::json!({ "success": true })))
}

// ... existing code ...
```

Now, let's update the human-facing endpoint:

```rust:src/controllers/user_pages.rs
// ... existing code ...

pub async fn update_password_action(
    State(ctx): State<AppContext>,
    auth: auth::JWT,
    Form(params): Form<PasswordUpdateParams>,
) -> Result<impl IntoResponse> {
    let user = auth.current_user(&ctx).await?;
    
    // Verify current password
    if !auth::verify_password(&params.current_password, &user.password_hash)? {
        // Render template with error
        return Ok(render_template(
            "profile.html.tera",
            context! {
                user => user,
                error => "Current password is incorrect",
                can_create_team => true,
            },
        ));
    }
    
    // Validate new password
    if params.new_password != params.confirm_password {
        // Render template with error
        return Ok(render_template(
            "profile.html.tera",
            context! {
                user => user,
                error => "New passwords do not match",
                can_create_team => true,
            },
        ));
    }
    
    // Update password
    let password_hash = auth::hash_password(&params.new_password)?;
    let user_model = models::users::ActiveModel {
        id: Set(user.id),
        password_hash: Set(password_hash),
        ..Default::default()
    };
    
    // Save changes
    user_model.update(&ctx.db).await?;
    
    // Redirect to profile page with success message
    Ok(Redirect::to("/profile?success=password_updated"))
}

// ... existing code ...
```

## 3. Team Creation Regardless of Existing Teams

Let's modify the team creation functionality to allow users to create teams even when they already belong to other teams:

```rust:src/controllers/team_pages.rs
// ... existing code ...

pub async fn create_team_page(
    State(ctx): State<AppContext>,
    auth: auth::JWT,
) -> Result<impl IntoResponse> {
    let user = auth.current_user(&ctx).await?;
    
    // Always allow team creation
    Ok(render_template(
        "create_team.html.tera",
        context! {
            user => user,
        },
    ))
}

pub async fn create_team_action(
    State(ctx): State<AppContext>,
    auth: auth::JWT,
    Form(params): Form<TeamCreateParams>,
) -> Result<impl IntoResponse> {
    let user = auth.current_user(&ctx).await?;
    
    // Create new team
    let team = models::teams::ActiveModel {
        name: Set(params.name),
        description: Set(params.description),
        created_by: Set(user.id),
        ..Default::default()
    };
    
    let team = team.insert(&ctx.db).await?;
    
    // Add user as team member with admin role
    let team_user = models::team_users::ActiveModel {
        team_id: Set(team.id),
        user_id: Set(user.id),
        role: Set("admin".to_string()),
        ..Default::default()
    };
    
    team_user.insert(&ctx.db).await?;
    
    // Redirect to team page
    Ok(Redirect::to(&format!("/teams/{}", team.id)))
}

// ... existing code ...
```

Also, we need to update the profile page template to ensure the "Create Team" button is always available:

```html:assets/views/profile.html.tera
<!-- ... existing code ... -->

<div class="mt-8">
  <h2 class="text-xl font-bold">Teams</h2>
  
  <!-- Always show the create team button -->
  <div class="mt-4">
    <a href="/teams/create" class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded">
      Create New Team
    </a>
  </div>
  
  <!-- List existing teams -->
  {% if teams and teams | length > 0 %}
    <div class="mt-4 grid grid-cols-1 gap-4">
      {% for team in teams %}
        <div class="p-4 border rounded">
          <h3 class="font-bold">{{ team.name }}</h3>
          <p>{{ team.description }}</p>
          <a href="/teams/{{ team.id }}" class="text-blue-500 hover:underline">View Team</a>
        </div>
      {% endfor %}
    </div>
  {% else %}
    <p class="mt-4">You are not a member of any teams yet.</p>
  {% endif %}
</div>

<!-- ... existing code ... -->
```

## Add the Necessary Routes

Finally, let's make sure all these endpoints are properly registered:

```rust:src/controllers/user_pages.rs
// ... existing code ...

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/profile")
        .add("/", get(profile_page))
        .add("/update", post(update_profile_action))
        .add("/update-password", post(update_password_action))
}

// ... existing code ...
```

```rust:src/controllers/user.rs
// ... existing code ...

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api/user")
        .add("/profile", put(update_profile))
        .add("/password", put(update_password))
}

// ... existing code ...
```

```rust:src/controllers/team_pages.rs
// ... existing code ...

pub fn routes() -> Routes {
    Routes::new()
        .prefix("/teams")
        .add("/", get(list_teams))
        .add("/create", get(create_team_page))
        .add("/create", post(create_team_action))
        .add("/{id}", get(team_details))
}

// ... existing code ...
```

These changes implement all three required features while following the rules for human-facing endpoints and API endpoints. The profile page now allows updating profile information, changing passwords, and creating teams regardless of existing team membership.

---



---

