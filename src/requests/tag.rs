use serde::{Deserialize, Serialize};

use super::{EntityType, LocalizedString, Relationship};

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
    name: LocalizedString,
    description: LocalizedString,
    group: TagGroup,
    version: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tag {
    id: String,
    #[serde(rename(deserialize = "type"))]
    entity_type: EntityType,
    attributes: TagAttributes,
    relationships: Vec<Relationship>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum TagsMode {
    And,
    Or,
}
