use crate::m20220101_000001_users::Users; // Import the Users entity definition
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Use manager.create_index() to add a unique index on Users::Name
        // Example: manager.create_index(Index::create().unique().name("idx-users-name-unique").table(Users::Table).col(Users::Name)).await
        manager
            .create_index(
                Index::create()
                    .unique()
                    .name("idx-users-name-unique") // Choose a descriptive name
                    .table(Users::Table)
                    .col(Users::Name)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Use manager.drop_index() to remove the index created in 'up'
        // Example: manager.drop_index(Index::drop().name("idx-users-name-unique").table(Users::Table)).await
        manager
            .drop_index(
                Index::drop()
                    .name("idx-users-name-unique") // Must match the name used in 'up'
                    .table(Users::Table)
                    .to_owned(),
            )
            .await
    }
}
