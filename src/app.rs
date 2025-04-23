use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Hooks},
    boot::{create_app, BootResult, StartMode},
    controller::AppRoutes,
    db::{self, truncate_table},
    environment::Environment,
    task::Tasks,
    worker::{Processor, Workers},
    config::Config, // Import Config
    Result,

};
use migration::{Migrator, MigratorTrait}; // Import MigratorTrait
use sea_orm::DatabaseConnection;
use std::path::Path; // Import std::path::Path

use crate::{
    controllers::{
        admin_api, admin_pages, auth_api, auth_pages, home_pages, pgp_pages, ssh_key_api,
        teams_api, teams_pages, users_pages,
    },
    initializers::view_engine,
    models::_entities::{ssh_keys, team_memberships, teams, users},
    tasks,
    // workers::downloader::DownloadWorker, // Commented out if unused
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
            option_env!("BUILD_SHA").unwrap_or("dev")
        )
    }

    // Correct the boot signature to include the config parameter to match the trait
    async fn boot(mode: StartMode, environment: &Environment, _config: &Config) -> Result<BootResult> {
        // create_app internally loads the config based on the environment,
        // so we don't need to explicitly pass the config parameter here.
        // The _config parameter is present only to match the trait signature.
        create_app::<Self, Migrator>(mode, environment).await
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::empty()
            // -- Add page routes below --
            .prefix("") // Reset prefix for top-level pages
            .add_router(home_pages::routes())
            .add_router(auth_pages::routes())
            .add_router(users_pages::routes())
            .add_router(teams_pages::routes())
            .add_router(pgp_pages::routes())
            // -- Add Admin routes (API and Pages) --
            // Merging admin_api and admin_pages under /admin prefix
            .prefix("/admin")
            .add_router(admin_api::routes().merge(admin_pages::routes())) // Ensure merge happens correctly
            // -- Add API routes below --
            // Prefix /api AFTER admin routes to avoid conflict
            .prefix("/api")
            .add_router(auth_api::routes())
            .add_router(ssh_key_api::routes())
            .add_router(teams_api::routes())
    }

    fn connect_workers<'a>(_p: &'a mut Processor, _ctx: &'a AppContext) {
        // p.register(DownloadWorker::build(ctx));
    }

    fn register_tasks(t: &mut Tasks) {
        t.register(tasks::user_report::UserReport);
        t.register(tasks::seed_task::SeedTask);
    }

    async fn migrate(db: &DatabaseConnection) {
        // Use MigratorTrait::up
        Migrator::up(db, None).await.unwrap();
    }

    async fn after_context(ctx: AppContext) -> Result<AppContext> {
        // Must initialize the view engine instance before using it
        let initialized_ctx = view_engine::initialize(&ctx).unwrap();
        Ok(initialized_ctx)
    }

    async fn truncate(db: &DatabaseConnection) -> Result<()> {
        // Need to truncate in reverse order of foreign key constraints
        truncate_table(db, team_memberships::Entity).await?;
        truncate_table(db, ssh_keys::Entity).await?;
        truncate_table(db, teams::Entity).await?;
        truncate_table(db, users::Entity).await?;
        Ok(())
    }

    async fn seed(db: &DatabaseConnection, _base_path: &str) -> Result<()> {
        // Use Path::new(_base_path).join()
        db::seed::<users::ActiveModel>(
            db,
            &Path::new(_base_path)
                .join("users.yaml")
                .display()
                .to_string(),
        )
        .await?;
        // db::seed::<teams::ActiveModel>(db, &Path::new(_base_path).join("teams.yaml").display().to_string()).await?;
        // db::seed::<team_memberships::ActiveModel>(db, &Path::new(_base_path).join("team_memberships.yaml").display().to_string()).await?;
        Ok(())
    }
}
