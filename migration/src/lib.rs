#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users;
mod m20240323_000001_teams;
mod m20240323_000002_team_memberships;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_users::Migration),
            Box::new(m20240323_000001_teams::Migration),
            Box::new(m20240323_000002_team_memberships::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
