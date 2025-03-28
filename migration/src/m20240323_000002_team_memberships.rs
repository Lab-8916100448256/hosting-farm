use loco_rs::schema::table_auto_tz;
use sea_orm_migration::{prelude::*, schema::*};

use crate::m20220101_000001_users::Users;
use crate::m20240323_000001_teams::Teams;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let table = table_auto_tz(TeamMemberships::Table)
            .col(pk_auto(TeamMemberships::Id))
            .col(uuid(TeamMemberships::Pid))
            .col(integer(TeamMemberships::TeamId).not_null())
            .col(integer(TeamMemberships::UserId).not_null())
            .col(string(TeamMemberships::Role))
            .col(boolean(TeamMemberships::Pending).default(true))
            .col(string_null(TeamMemberships::InvitationToken))
            .col(timestamp_with_time_zone_null(TeamMemberships::InvitationSentAt))
            .foreign_key(
                ForeignKey::create()
                    .name("fk_team_memberships_team_id")
                    .from(TeamMemberships::Table, TeamMemberships::TeamId)
                    .to(Teams::Table, Teams::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_team_memberships_user_id")
                    .from(TeamMemberships::Table, TeamMemberships::UserId)
                    .to(Users::Table, Users::Id)
                    .on_delete(ForeignKeyAction::Cascade),
            )
            .to_owned();
        
        // Create a unique constraint to prevent duplicate memberships
        manager.create_table(table).await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_unique_team_user")
                    .table(TeamMemberships::Table)
                    .col(TeamMemberships::TeamId)
                    .col(TeamMemberships::UserId)
                    .unique()
                    .to_owned(),
            )
            .await?;
        
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TeamMemberships::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum TeamMemberships {
    Table,
    Id,
    Pid,
    TeamId,
    UserId,
    Role,
    Pending,
    InvitationToken,
    InvitationSentAt,
} 