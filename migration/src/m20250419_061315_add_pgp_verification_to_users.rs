// use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Users::Table)
                .add_column(ColumnDef::new(Users::PgpVerificationToken).string().null())
                .to_owned(),
        )
        .await?;
        m.alter_table(
            Table::alter()
                .table(Users::Table)
                .add_column(ColumnDef::new(Users::PgpVerifiedAt).date_time().null())
                .to_owned(),
        )
        .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            Table::alter()
                .table(Users::Table)
                .drop_column(Users::PgpVerificationToken)
                .to_owned(),
        )
        .await?;
        m.alter_table(
            Table::alter()
                .table(Users::Table)
                .drop_column(Users::PgpVerifiedAt)
                .to_owned(),
        )
        .await?;
        Ok(())
    }
}

// Helper struct to define the table and column names
#[derive(Iden)]
enum Users {
    Table,
    PgpVerificationToken,
    PgpVerifiedAt,
}
