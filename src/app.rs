use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    bgworker::{BackgroundWorker, Queue},
    boot::{create_app, BootResult, StartMode},
    config::Config,
    controller::AppRoutes,
    db::{self, truncate_table},
    environment::Environment,
    task::Tasks,
    Result,
};
use migration::Migrator;
use std::path::Path;
use sea_orm::{ActiveModelTrait, ActiveValue, DatabaseConnection};
use std::sync::Arc;
use uuid;

#[allow(unused_imports)]
use crate::{
    controllers, initializers, models::_entities::{users, teams, team_memberships}, tasks, workers::downloader::DownloadWorker,
};

pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(vec![Box::new(
            initializers::view_engine::ViewEngineInitializer,
        )])
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // controller routes below
            .add_route(controllers::auth::routes())
            .add_route(controllers::teams::routes())
            .add_route(controllers::home_pages::routes())
            .add_route(controllers::users::routes())
            .add_route(controllers::auth_pages::routes())
            .add_route(controllers::teams_pages::routes())
    }

    fn register_tasks(tasks: &mut Tasks) {
        // tasks-inject (do not remove)
    }

    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue.register(DownloadWorker::build(ctx)).await?;
        Ok(())
    }

    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, team_memberships::Entity).await?;
        truncate_table(&ctx.db, teams::Entity).await?;
        truncate_table(&ctx.db, users::Entity).await?;
        Ok(())
    }

    async fn seed(ctx: &AppContext, _base: &Path) -> Result<()> {
        let user = users::ActiveModel {
            id: ActiveValue::set(1),
            pid: ActiveValue::set(uuid::Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap()),
            email: ActiveValue::set("user1@example.com".to_string()),
            password: ActiveValue::set("$argon2id$v=19$m=19456,t=2,p=1$ETQBx4rTgNAZhSaeYZKOZg$eYTdH26CRT6nUJtacLDEboP0li6xUwUF/q5nSlQ8uuc".to_string()),
            api_key: ActiveValue::set("lo-95ec80d7-cb60-4b70-9b4b-9ef74cb88758".to_string()),
            name: ActiveValue::set("user1".to_string()),
            created_at: ActiveValue::set(chrono::DateTime::parse_from_rfc3339("2023-11-12T12:34:56.789+00:00").unwrap().into()),
            updated_at: ActiveValue::set(chrono::DateTime::parse_from_rfc3339("2023-11-12T12:34:56.789+00:00").unwrap().into()),
            reset_token: ActiveValue::NotSet,
            reset_sent_at: ActiveValue::NotSet,
            email_verification_token: ActiveValue::NotSet,
            email_verification_sent_at: ActiveValue::NotSet,
            email_verified_at: ActiveValue::NotSet,
            magic_link_token: ActiveValue::NotSet,
            magic_link_expiration: ActiveValue::NotSet,
        };
        user.insert(&ctx.db).await?;
        Ok(())
    }
}
