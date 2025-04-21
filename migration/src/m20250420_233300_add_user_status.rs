use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add the 'status' column to the 'users' table
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(
                        ColumnDef::new(Users::Status)
                            .string() // Using string for status ('new', 'approved', 'rejected')
                            .not_null()
                            .default("new"), // Default status for new users
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove the 'status' column from the 'users' table
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::Status)
                    .to_owned(),
            )
            .await
    }
}

// Define the table and column names
#[derive(DeriveIden)]
enum Users {
    Table,
    Status, // The new column identifier
}
