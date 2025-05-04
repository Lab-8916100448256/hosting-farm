#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users;
mod m20240323_000001_teams;
mod m20240323_000002_team_memberships;

mod m20250416_052930_ssh_keys;
mod m20250416_173257_add_pgp_key_to_users;
mod m20250419_061315_add_pgp_verification_to_users;
mod m20250420_150931_add_unique_constraint_to_teams_name;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_users::Migration),
            Box::new(m20240323_000001_teams::Migration),
            Box::new(m20240323_000002_team_memberships::Migration),
            Box::new(m20250416_052930_ssh_keys::Migration),
            Box::new(m20250416_173257_add_pgp_key_to_users::Migration),
            Box::new(m20250419_061315_add_pgp_verification_to_users::Migration),
            Box::new(m20250420_150931_add_unique_constraint_to_teams_name::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
