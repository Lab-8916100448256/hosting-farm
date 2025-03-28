use loco_rs::schema::table_auto_tz;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let table = table_auto_tz(Teams::Table)
            .col(pk_auto(Teams::Id))
            .col(uuid(Teams::Pid))
            .col(string(Teams::Name))
            .col(string_null(Teams::Description))
            .to_owned();
        manager.create_table(table).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Teams::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Teams {
    Table,
    Id,
    Pid,
    Name,
    Description,
} 