use serde::{Deserialize, Serialize};

use super::{EntityType, Locale, Relationship};

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

#[derive(Serialize, Deserialize, Debug)]
pub struct CoverArt {
    pub id: String,
    #[serde(rename(deserialize = "type"))]
    pub entity_type: EntityType,
    pub attributes: CoverArtAttributes,
    pub relationships: Vec<Relationship>,
}
