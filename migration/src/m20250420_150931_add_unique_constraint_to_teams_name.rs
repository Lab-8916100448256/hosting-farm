use crate::m20240323_000001_teams::Teams;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Use manager.create_index() to add a unique index on Teams::Name
        manager
            .create_index(
                Index::create()
                    .unique()
                    .name("idx-teams-name-unique") // Choose a descriptive name
                    .table(Teams::Table)
                    .col(Teams::Name)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Use manager.drop_index() to remove the index created in 'up'
        manager
            .drop_index(
                Index::drop()
                    .name("idx-teams-name-unique") // Must match the name used in 'up'
                    .table(Teams::Table)
                    .to_owned(),
            )
            .await
    }
}