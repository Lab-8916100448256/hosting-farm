use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

use crate::m20220101_000001_users::Users;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserSshKeys::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserSshKeys::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(UserSshKeys::Pid)
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(UserSshKeys::Name).string().not_null())
                    .col(ColumnDef::new(UserSshKeys::Key).text().not_null())
                    .col(ColumnDef::new(UserSshKeys::UserId).integer().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-user_ssh_keys-user_id")
                            .from(UserSshKeys::Table, UserSshKeys::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .col(
                        ColumnDef::new(UserSshKeys::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserSshKeys::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserSshKeys::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum UserSshKeys {
    Table,
    Id,
    Pid,
    Name,
    Key,
    UserId,
    CreatedAt,
    UpdatedAt,
}
