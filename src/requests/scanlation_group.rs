//! Utilities for scanlation group specific server part of the application.
//!
//! The meaning of most of the confusing structs here can be found at <https://api.mangadex.org/docs/3-enumerations/#manga-links-data>
//!
//! All queries encountered here can be constructed with the builder syntax from the [bon] crate

use serde::{Deserialize, Serialize};

use super::query_utils::{LocalizedString, Relationship};
use super::{Entity, EntityType, Locale};

/// Used for serialization/deserialization
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ScanlationGroupAttributes {
    pub name: String,
    pub alt_names: Vec<LocalizedString>,
    pub website: Option<String>,
    pub irc_server: Option<String>,
    pub discord: Option<String>,
    pub contact_email: Option<String>,
    pub description: Option<String>,
    pub twitter: Option<String>,
    pub manga_updates: Option<String>,
    pub focused_language: Option<Vec<Locale>>,
    pub locked: bool,
    pub official: bool,
    pub verified: bool,
    pub inactive: bool,
    pub ex_licensed: bool,
    pub publish_delay: Option<String>,
    pub version: usize,
    pub created_at: String,
    pub updated_at: String,
}

/// Main structure used for representing the response containg scanlation group info
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScanlationGroup {
    pub id: String,
    #[serde(rename = "type")]
    pub entity_type: EntityType,
    pub attributes: ScanlationGroupAttributes,
    pub relationships: Vec<Relationship>,
}

impl Entity for ScanlationGroup {}
