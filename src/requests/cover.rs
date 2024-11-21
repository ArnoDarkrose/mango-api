//! Utilities for scanlation group specific server part of the application.
//!
//! The meaning of most of the confusing structs here can be found at <https://api.mangadex.org/docs/3-enumerations/#manga-links-data>
//!
//! All queries encountered here can be constructed with the builder syntax from the [bon] crate

use serde::{Deserialize, Serialize};

use super::query_utils::Relationship;
use super::{EntityType, Locale};

/// Used for serialization/deserialization
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CoverArtAttributes {
    pub volume: Option<String>,
    pub file_name: String,
    pub description: Option<String>,
    pub locale: Option<Locale>,
    pub version: usize,
    pub created_at: String,
    pub updated_at: String,
}

/// Main structure used for representing the response containg cover info
#[derive(Serialize, Deserialize, Debug)]
pub struct CoverArt {
    pub id: String,
    #[serde(rename(deserialize = "type"))]
    pub entity_type: EntityType,
    pub attributes: CoverArtAttributes,
    pub relationships: Vec<Relationship>,
}
