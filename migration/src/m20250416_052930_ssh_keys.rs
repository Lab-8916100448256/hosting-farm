use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.create_table(
            Table::create()
                .table(SshKeys::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(SshKeys::Id)
                        .integer()
                        .not_null()
                        .auto_increment()
                        .primary_key(),
                )
                .col(ColumnDef::new(SshKeys::UserId).integer().not_null())
                .col(ColumnDef::new(SshKeys::Label).string().not_null()) // Add Label column
                .col(ColumnDef::new(SshKeys::PublicKey).text().not_null()) // Remove unique_key() here, handled by index
                .col(
                    ColumnDef::new(SshKeys::CreatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                .col(
                    ColumnDef::new(SshKeys::UpdatedAt)
                        .timestamp_with_time_zone()
                        .not_null(),
                )
                // Add Foreign Key constraint using .foreign_key()
                .foreign_key(
                    ForeignKey::create()
                        .name("fk-ssh_keys-user_id")
                        .from(SshKeys::Table, SshKeys::UserId)
                        .to(Users::Table, Users::Id)
                        .on_delete(ForeignKeyAction::Cascade) // Or SetNull/Restrict as needed
                        .on_update(ForeignKeyAction::Cascade),
                )
                 // Add unique constraint for user_id + label using .index()
                .index(
                    Index::create()
                        .name("idx-ssh_keys-user_id-label")
                        .table(SshKeys::Table)
                        .col(SshKeys::UserId)
                        .col(SshKeys::Label)
                        .unique(),
                )
                // Add unique constraint for user_id + public_key using .index()
                .index(
                    Index::create()
                        .name("idx-ssh_keys-user_id-public_key")
                        .table(SshKeys::Table)
                        .col(SshKeys::UserId)
                        .col(SshKeys::PublicKey)
                        .unique(),
                )
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.drop_table(Table::drop().table(SshKeys::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum SshKeys {
    Table,
    Id,
    UserId,
    Label,      // Add Label Iden
    PublicKey,
    CreatedAt,
    UpdatedAt,
}

// Define Iden for the related Users table if not already defined elsewhere
#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}
