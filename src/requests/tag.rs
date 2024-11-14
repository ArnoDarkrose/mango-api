use serde::{Deserialize, Serialize};

use super::{Entity, EntityType, LocalizedString, Relationship};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TagGroup {
    Content,
    Format,
    Genre,
    Theme,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TagAttributes {
    pub name: LocalizedString,
    pub description: LocalizedString,
    pub group: TagGroup,
    pub version: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tag {
    pub id: String,
    #[serde(rename(deserialize = "type"))]
    pub entity_type: EntityType,
    pub attributes: TagAttributes,
    pub relationships: Vec<Relationship>,
}

impl Entity for Tag {}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum TagsMode {
    And,
    Or,
}
