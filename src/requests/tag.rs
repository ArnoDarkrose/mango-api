//! Utilities for tag specific server part of the application.
//!
//! The meaning of most of the confusing structs here can be found at <https://api.mangadex.org/docs/3-enumerations/#manga-links-data>
//!
//! All queries encountered here can be constructed with the builder syntax from the [bon] crate

use serde::{Deserialize, Serialize};

use super::query_utils::{LocalizedString, Relationship};
use super::{Entity, EntityType};

/// Used for serialization/deserialization
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TagGroup {
    Content,
    Format,
    Genre,
    Theme,
}

/// Used for serialization/deserialization
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TagAttributes {
    pub name: LocalizedString,
    pub description: LocalizedString,
    pub group: TagGroup,
    pub version: usize,
}

/// Main structure used for representing the response containg tag info
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tag {
    pub id: String,
    #[serde(rename(deserialize = "type"))]
    pub entity_type: EntityType,
    pub attributes: TagAttributes,
    pub relationships: Vec<Relationship>,
}

impl Entity for Tag {}

/// Used for serialization/deserialization
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum TagsMode {
    And,
    Or,
}
