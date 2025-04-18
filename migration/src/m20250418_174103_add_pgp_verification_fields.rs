use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        add_column(m, "users", "pgp_verification_token", ColType::TextNull).await?;
        add_column(
            m,
            "users",
            "pgp_verification_sent_at",
            ColType::TimestampWithTimeZoneNull,
        )
        .await?;
        add_column(
            m,
            "users",
            "pgp_verified_at",
            ColType::TimestampWithTimeZoneNull,
        )
        .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        remove_column(m, "users", "pgp_verified_at").await?;
        remove_column(m, "users", "pgp_verification_sent_at").await?;
        remove_column(m, "users", "pgp_verification_token").await?;
        Ok(())
    }
}
