use serde::{Deserialize, Serialize};

use super::query_utils::{LocalizedString, Relationship};
use super::{Entity, EntityType, Locale};

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
    // NOTE: this may be a bug as this possibly may be any string, not just locale
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ScanlationGroup {
    pub id: String,
    #[serde(rename = "type")]
    pub entity_type: EntityType,
    pub attributes: ScanlationGroupAttributes,
    pub relationships: Vec<Relationship>,
}

impl Entity for ScanlationGroup {}
