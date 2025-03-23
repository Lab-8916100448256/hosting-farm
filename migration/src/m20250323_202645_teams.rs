use loco_rs::schema::table_auto_tz;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create Teams table
        let teams_table = table_auto_tz(Teams::Table)
            .col(pk_auto(Teams::Id))
            .col(uuid(Teams::Pid))
            .col(string(Teams::Name))
            .col(string_null(Teams::Description))
            .col(integer(Teams::OwnerId).not_null())
            .foreign_key(
                ForeignKey::create()
                    .name("fk_teams_owner")
                    .from(Teams::Table, Teams::OwnerId)
                    .to(Users::Table, Users::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .to_owned();

        manager.create_table(teams_table).await?;

        // Create TeamMembers table
        let team_members_table = table_auto_tz(TeamMembers::Table)
            .col(pk_auto(TeamMembers::Id))
            .col(uuid(TeamMembers::Pid))
            .col(integer(TeamMembers::TeamId).not_null())
            .col(integer(TeamMembers::UserId).not_null())
            .col(string(TeamMembers::Role).not_null())
            .foreign_key(
                ForeignKey::create()
                    .name("fk_team_members_team")
                    .from(TeamMembers::Table, TeamMembers::TeamId)
                    .to(Teams::Table, Teams::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_team_members_user")
                    .from(TeamMembers::Table, TeamMembers::UserId)
                    .to(Users::Table, Users::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .to_owned();

        manager.create_table(team_members_table).await?;

        // Create unique constraint to prevent duplicate team memberships
        let unique_team_member = Index::create()
            .name("idx_unique_team_member")
            .table(TeamMembers::Table)
            .col(TeamMembers::TeamId)
            .col(TeamMembers::UserId)
            .unique()
            .to_owned();

        manager.create_index(unique_team_member).await?;

        // Create TeamInvitations table
        let team_invitations_table = table_auto_tz(TeamInvitations::Table)
            .col(pk_auto(TeamInvitations::Id))
            .col(uuid(TeamInvitations::Pid))
            .col(integer(TeamInvitations::TeamId).not_null())
            .col(string(TeamInvitations::Email).not_null())
            .col(string(TeamInvitations::Role).not_null())
            .col(integer(TeamInvitations::InvitedById).not_null())
            .col(string_null(TeamInvitations::Token))
            .col(timestamp_with_time_zone_null(TeamInvitations::ExpiresAt))
            .col(timestamp_with_time_zone_null(TeamInvitations::AcceptedAt))
            .col(timestamp_with_time_zone_null(TeamInvitations::RejectedAt))
            .foreign_key(
                ForeignKey::create()
                    .name("fk_team_invitations_team")
                    .from(TeamInvitations::Table, TeamInvitations::TeamId)
                    .to(Teams::Table, Teams::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .foreign_key(
                ForeignKey::create()
                    .name("fk_team_invitations_invited_by")
                    .from(TeamInvitations::Table, TeamInvitations::InvitedById)
                    .to(Users::Table, Users::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade),
            )
            .to_owned();

        manager.create_table(team_invitations_table).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop tables in reverse order to avoid foreign key constraints
        manager
            .drop_table(Table::drop().table(TeamInvitations::Table).to_owned())
            .await?;
        
        manager
            .drop_table(Table::drop().table(TeamMembers::Table).to_owned())
            .await?;
        
        manager
            .drop_table(Table::drop().table(Teams::Table).to_owned())
            .await?;

        Ok(())
    }
}

/// Reference to the "teams" table
#[derive(Iden)]
enum Teams {
    Table,
    Id,
    Pid,
    Name,
    Description,
    OwnerId,
    CreatedAt,
    UpdatedAt,
}

/// Reference to the "team_members" table
#[derive(Iden)]
enum TeamMembers {
    Table,
    Id,
    Pid,
    TeamId,
    UserId,
    Role,
    CreatedAt,
    UpdatedAt,
}

/// Reference to the "team_invitations" table
#[derive(Iden)]
enum TeamInvitations {
    Table,
    Id,
    Pid,
    TeamId,
    Email,
    Role,
    InvitedById,
    Token,
    ExpiresAt,
    AcceptedAt,
    RejectedAt,
    CreatedAt,
    UpdatedAt,
}

/// Reference to the "users" table
#[derive(Iden)]
enum Users {
    Table,
    Id,
}
