use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(Iden)]
enum SshPublicKeys {
    Table,
    Id,
    UserId,
    Key,
    Label,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
}

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(SshPublicKeys::Table).if_exists().to_owned()).await?;
        m.create_table(
            Table::create()
                .table(SshPublicKeys::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(SshPublicKeys::Id)
                        .integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(
                    ColumnDef::new(SshPublicKeys::UserId)
                        .integer()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(SshPublicKeys::Key)
                        .text()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(SshPublicKeys::Label)
                        .string()
                        .null(),
                )
                .col(
                    ColumnDef::new(SshPublicKeys::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .col(
                    ColumnDef::new(SshPublicKeys::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null()
                        .default(Expr::current_timestamp()),
                )
                .foreign_key(
                    ForeignKey::create()
                        .name("fk_ssh_public_keys_user_id")
                        .from(SshPublicKeys::Table, SshPublicKeys::UserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade),
                )
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "ssh_public_keys").await
    }
}
