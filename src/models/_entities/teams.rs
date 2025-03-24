//! `SeaORM` Entity for teams
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "teams")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key)]
    pub id: i32,
    pub pid: Uuid,
    pub name: String,
    pub description: Option<String>,
    #[sea_orm(unique)]
    pub slug: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::team_memberships::Entity")]
    TeamMemberships,
}

impl Related<super::team_memberships::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TeamMemberships.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
