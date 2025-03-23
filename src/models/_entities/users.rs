//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.7

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    pub pid: Uuid,
    #[sea_orm(unique)]
    pub email: String,
    pub password: String,
    #[sea_orm(unique)]
    pub api_key: String,
    pub name: String,
    pub reset_token: Option<String>,
    pub reset_sent_at: Option<DateTimeWithTimeZone>,
    pub email_verification_token: Option<String>,
    pub email_verification_sent_at: Option<DateTimeWithTimeZone>,
    pub email_verified_at: Option<DateTimeWithTimeZone>,
    pub magic_link_token: Option<String>,
    pub magic_link_expiration: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::team_invitations::Entity")]
    TeamInvitations,
    #[sea_orm(has_many = "super::team_members::Entity")]
    TeamMembers,
    #[sea_orm(has_many = "super::teams::Entity")]
    Teams,
}

impl Related<super::team_invitations::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TeamInvitations.def()
    }
}

impl Related<super::team_members::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TeamMembers.def()
    }
}

impl Related<super::teams::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Teams.def()
    }
}
