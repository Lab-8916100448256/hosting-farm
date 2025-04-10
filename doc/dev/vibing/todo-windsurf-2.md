# TODO: Replace unwrap() and `?` operator in src/

This file tracks the usage of `.unwrap()` and the `?` operator within the `src/` directory.

## Files to Process:

- [x] `src/app.rs`
- [x] `src/bin/main.rs`
- [x] `src/controllers/auth_api.rs`
- [x] `src/controllers/auth_pages.rs`
- [x] `src/controllers/home_pages.rs`
- [x] `src/controllers/mod.rs`
- [x] `src/controllers/teams_api.rs`
- [x] `src/controllers/teams_pages.rs`
- [x] `src/controllers/users_pages.rs`
- [x] `src/initializers/mod.rs`
- [x] `src/initializers/view_engine.rs`
- [x] `src/lib.rs`
- [x] `src/mailers/auth.rs`
- [x] `src/mailers/mod.rs`
- [x] `src/mailers/team.rs`
- [x] `src/middleware/auth_no_error.rs`
- [ ] `src/middleware/mod.rs`
- [ ] `src/models/_entities/mod.rs`
- [ ] `src/models/_entities/prelude.rs`
- [ ] `src/models/_entities/team_memberships.rs`
- [ ] `src/models/_entities/teams.rs`
- [ ] `src/models/_entities/users.rs`
- [ ] `src/models/mod.rs`
- [ ] `src/models/team_memberships.rs`
- [ ] `src/models/teams.rs`
- [ ] `src/models/users.rs`
- [ ] `src/tasks/mod.rs`
- [ ] `src/utils/middleware.rs`
- [ ] `src/utils/mod.rs`
- [ ] `src/utils/template.rs`
- [ ] `src/views/auth.rs`
- [ ] `src/views/mod.rs`
- [ ] `src/views/teams.rs`
- [ ] `src/views/users.rs`
- [ ] `src/workers/downloader.rs`
- [ ] `src/workers/mod.rs`

## Occurrences:

### `src/app.rs`

#### `.unwrap()` calls

- [ ] Line 40: `                .unwrap_or("dev")`
- [ ] Line 87: `            pid: ActiveValue::set(uuid::Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap()),`
- [ ] Line 92: `            created_at: ActiveValue::set(chrono::DateTime::parse_from_rfc3339("2023-11-12T12:34:56.789+00:00").unwrap().into()),`
- [ ] Line 93: `            updated_at: ActiveValue::set(chrono::DateTime::parse_from_rfc3339("2023-11-12T12:34:56.789+00:00").unwrap().into()),`

#### `?` operator

*(None found in this file based on search results)*

### `src/bin/main.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/controllers/auth_api.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

- [ ] Line 63: `        .await?;`
- [ ] Line 65: `    AuthMailer::send_welcome(&ctx, &user).await?;`
- [ ] Line 74: `    let user = users::Model::find_by_verification_token(&ctx.db, &token).await?;`
- [ ] Line 80: `        let user = active_model.verified(&ctx.db).await?;`
- [ ] Line 105: `        .await?;`
- [ ] Line 107: `    AuthMailer::forgot_password(&ctx, &user).await?;`
- [ ] Line 124: `        .await?;`
- [ ] Line 132: `    let user = users::Model::find_by_email(&ctx.db, &params.email).await?;`
- [ ] Line 140: `    let jwt_secret = ctx.config.get_jwt_config()?;`
- [ ] Line 144: `        .or_else(|_| unauthorized("unauthorized!"))?;`
- [ ] Line 151: `    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;`
- [ ] Line 189: `    let user = user.into_active_model().create_magic_link(&ctx.db).await?;`
- [ ] Line 190: `    AuthMailer::send_magic_link(&ctx, &user).await?;`
- [ ] Line 206: `    let user = user.into_active_model().clear_magic_link(&ctx.db).await?;`
- [ ] Line 208: `    let jwt_secret = ctx.config.get_jwt_config()?;`
- [ ] Line 212: `        .or_else(|_| unauthorized("unauthorized!"))?;`
- [ ] Line 224: `        .body(axum::body::Body::empty())?;`

### `src/controllers/auth_pages.rs`

#### `.unwrap()` calls

- [ ] Line 311: `                // if user.reset_sent_at.unwrap() + expiry_duration < chrono::Utc::now().naive_utc() { ... handle expired ... }` (Commented out)

#### `?` operator

- [ ] Line 66: `                .await?;`
- [ ] Line 68: `            AuthMailer::send_welcome(&ctx, &user).await?;`
- [ ] Line 73: `                .body(axum::body::Body::empty())?;`
- [ ] Line 166: `            let jwt_secret = ctx.config.get_jwt_config()?;`
- [ ] Line 194: `            let response = response_builder.body(axum::body::Body::empty())?;`
- [ ] Line 291: `        .body(axum::body::Body::empty())?;`
- [ ] Line 361: `                let _user = active_model.verified(&ctx.db).await?;`
- [ ] Line 397: `        .body(axum::body::Body::empty())?;`
- [ ] Line 428: `                            .body(axum::body::Body::empty())?;`

### `src/controllers/home_pages.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

- [ ] Line 25: `                .await?;`

### `src/controllers/mod.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/controllers/teams_api.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

- [ ] Line 31: `    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;`
- [ ] Line 34: `    let team = TeamModel::create_team(&ctx.db, user.id, &params).await?;`
- [ ] Line 41: `    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;`
- [ ] Line 47: `        .await?`
- [ ] Line 70: `    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;`
- [ ] Line 71: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 74: `    let has_access = team.has_role(&ctx.db, user.id, "Observer").await?;`
- [ ] Line 89: `    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;`
- [ ] Line 90: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 93: `    let is_owner = team.has_role(&ctx.db, user.id, "Owner").await?;`
- [ ] Line 98: `    let updated_team = team.update(&ctx.db, &params).await?;`
- [ ] Line 109: `    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;`
- [ ] Line 110: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 113: `    let is_owner = team.has_role(&ctx.db, user.id, "Owner").await?;`
- [ ] Line 119: `    team.delete(&ctx.db).await?;`
- [ ] Line 125: `        .body(axum::body::Body::empty())?;`
- [ ] Line 136: `    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;`
- [ ] Line 137: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 140: `    let has_access = team.has_role(&ctx.db, user.id, "Observer").await?;`
- [ ] Line 150: `        .await?;`
- [ ] Line 157: `            .await?;`
- [ ] Line 178: `    let user = users::Model::find_by_pid(&ctx.db, &auth.claims.pid).await?;`
- [ ] Line 179: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`

### `src/controllers/teams_pages.rs`

#### `.unwrap()` calls

- [ ] Line 31: `    let user = auth.user.unwrap();`
- [ ] Line 59: `    let user = auth.user.unwrap();`
- [ ] Line 77: `                    .unwrap_or_else(|| "Unknown".to_string());`
- [ ] Line 120: `    let user = auth.user.unwrap();`
- [ ] Line 251: `    let user = auth.user.unwrap();`
- [ ] Line 324: `    let user = auth.user.unwrap();`
- [ ] Line 397: `    let user = auth.user.unwrap();`
- [ ] Line 446: `    let user = auth.user.unwrap();`
- [ ] Line 492: `    let user = auth.user.unwrap();`
- [ ] Line 539: `    let user = auth.user.unwrap();`
- [ ] Line 585: `    let user = auth.user.unwrap();`
- [ ] Line 691: `    let current_user = auth.user.unwrap();`
- [ ] Line 776: `    let current_user = auth.user.unwrap();`
- [ ] Line 886: `    let user = auth.user.unwrap();`
- [ ] Line 923: `    let user = auth.user.unwrap();`

#### `?` operator

- [ ] Line 51: `        .await?;`
- [ ] Line 105: `        .await?;`
- [ ] Line 106: `        .await?;`
- [ ] Line 162: `        .await?;`
- [ ] Line 163: `        .await?;`
- [ ] Line 189: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 191: `    let is_owner = team.has_role(&ctx.db, user.id, "Owner").await?;`
- [ ] Line 217: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 219: `    let is_owner = team.has_role(&ctx.db, user.id, "Owner").await?;`
- [ ] Line 243: `    let team = team.update(&ctx.db, &params).await?;`
- [ ] Line 281: `        .await?;`
- [ ] Line 299: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 300: `    let is_owner = team.has_role(&ctx.db, user.id, "Owner").await?;`
- [ ] Line 351: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 352: `    let is_owner = team.has_role(&ctx.db, user.id, "Owner").await?;`
- [ ] Line 372: `    team.delete(&ctx.db).await?;`
- [ ] Line 374: `        .body(axum::body::Body::empty())?;`
- [ ] Line 428: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 429: `    let is_owner = team.has_role(&ctx.db, user.id, "Owner").await?;`
- [ ] Line 478: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 480: `    let is_owner = team.has_role(&ctx.db, user.id, "Owner").await?;`
- [ ] Line 526: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 528: `    let is_owner = team.has_role(&ctx.db, user.id, "Owner").await?;`
- [ ] Line 575: `    let _membership = team.update_membership(&ctx.db, member_id, &params).await?;`
- [ ] Line 601: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 602: `    let is_owner = team.has_role(&ctx.db, user.id, "Owner").await?;`
- [ ] Line 617: `    let _membership = team.leave(&ctx.db, user.id).await?;`
- [ ] Line 619: `        .body(axum::body::Body::empty())?;`
- [ ] Line 644: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 645: `    let is_owner = team.has_role(&ctx.db, user.id, "Owner").await?;`
- [ ] Line 673: `    let _membership = team.remove_member(&ctx.db, user_id).await?;`
- [ ] Line 675: `        .body(axum::body::Body::empty())?;`
- [ ] Line 719: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 720: `    let has_access = team.has_role(&ctx.db, current_user.id, "Owner").await?;`
- [ ] Line 743: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 744: `    let has_access = team.has_role(&ctx.db, current_user.id, "Owner").await?;`
- [ ] Line 767: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 804: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 805: `    let has_access = team.has_role(&ctx.db, current_user.id, "Owner").await?;`
- [ ] Line 832: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 833: `    let has_access = team.has_role(&ctx.db, current_user.id, "Owner").await?;`
- [ ] Line 860: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 861: `    let has_access = team.has_role(&ctx.db, current_user.id, "Owner").await?;`
- [ ] Line 911: `    let team = TeamModel::find_by_pid(&ctx.db, &team_pid).await?;`
- [ ] Line 912: `    let has_access = team.has_role(&ctx.db, user.id, "Owner").await?;`

### `src/controllers/users_pages.rs`

#### `.unwrap()` calls

- [ ] Line 41: `    let user = auth.user.unwrap(); // User is guaranteed to be Some here`
- [ ] Line 59: `                    .unwrap_or_else(|| "Unknown".to_string());`
- [ ] Line 100: `    let user = auth.user.unwrap(); // User is guaranteed to be Some here`
- [ ] Line 145: `    let user = auth.user.unwrap(); // User is guaranteed to be Some here`
- [ ] Line 171: `    let user = auth.user.unwrap(); // User is guaranteed to be Some here`
- [ ] Line 206: `    let user = auth.user.unwrap(); // User is guaranteed to be Some here`

#### `?` operator

- [ ] Line 87: `        .await?;`
- [ ] Line 131: `        .await?;`
- [ ] Line 132: `    let user = users::Model::update(&user.into_active_model(), &ctx.db, &params).await?;`
- [ ] Line 163: `        .await?;`
- [ ] Line 194: `    let team = users::Model::update(&user.into_active_model(), &ctx.db, &params).await?;`
- [ ] Line 230: `        .await?;`
- [ ] Line 231: `    let _user = users::Model::update(&user.into_active_model(), &ctx.db, &params).await?;`

### `src/initializers/mod.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/initializers/view_engine.rs`

#### `.unwrap()` calls

- [ ] Line 38: `                    .unwrap_or(0) as usize;`
- [ ] Line 42: `                    .unwrap_or(0) as usize;`
- [ ] Line 55: `                    .unwrap_or("%Y");`
- [ ] Line 79: `                    .unwrap_or(0) as usize;`
- [ ] Line 83: `                    .unwrap_or(0) as usize;`
- [ ] Line 96: `                    .unwrap_or("%Y");`

#### `?` operator

- [ ] Line 45: `            .ok_or_else(|| tera::Error::msg("Value must be a string"))?;`
- [ ] Line 48: `            .map_err(|e| tera::Error::msg(format!("Failed to parse date: {}", e)))?;`
- [ ] Line 86: `            .ok_or_else(|| tera::Error::msg("Value must be a string"))?;`
- [ ] Line 107: `    let mut tera = Tera::new(&format!("{}/src/**/*.tera.html", folder))?;`

### `src/lib.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/mailers/auth.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

- [ ] Line 45: `        Self::mail_template(ctx, &welcome, args).await?;`
- [ ] Line 76: `        Self::mail_template(ctx, &forgot, args).await?;`
- [ ] Line 93: `                ))?;,`
- [ ] Line 109: `        Self::mail_template(ctx, &magic_link, args).await?;`

### `src/mailers/mod.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/mailers/team.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

- [ ] Line 36: `        Self::mail_template(ctx, &invitation, args).await?;`

### `src/middleware/auth_no_error.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/middleware/mod.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/models/_entities/mod.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/models/_entities/prelude.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/models/_entities/team_memberships.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/models/_entities/teams.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/models/_entities/users.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/models/mod.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/models/team_memberships.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/models/teams.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/models/users.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/tasks/mod.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/utils/middleware.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/utils/mod.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/utils/template.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/views/auth.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/views/mod.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/views/teams.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/views/users.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/workers/downloader.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*

### `src/workers/mod.rs`

#### `.unwrap()` calls

*(None found in this file based on search results)*

#### `?` operator

*(None found in this file based on search results)*
