use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add gpg_key_verification_token
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(
                        ColumnDef::new(Users::GpgKeyVerificationToken)
                            .string()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Add gpg_key_verification_sent_at
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(
                        ColumnDef::new(Users::GpgKeyVerificationSentAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Add gpg_key_verified_at
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(
                        ColumnDef::new(Users::GpgKeyVerifiedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop gpg_key_verified_at
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::GpgKeyVerifiedAt)
                    .to_owned(),
            )
            .await?;

        // Drop gpg_key_verification_sent_at
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::GpgKeyVerificationSentAt)
                    .to_owned(),
            )
            .await?;

        // Drop gpg_key_verification_token
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::GpgKeyVerificationToken)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    #[sea_orm(iden = "gpg_key_verification_token")]
    GpgKeyVerificationToken,
    #[sea_orm(iden = "gpg_key_verification_sent_at")]
    GpgKeyVerificationSentAt,
    #[sea_orm(iden = "gpg_key_verified_at")]
    GpgKeyVerifiedAt,
}
