use serde::{Deserialize, Serialize};

use super::{Entity, EntityType, Locale, Relationship};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChapterAttributes {
    pub title: Option<String>,
    pub volume: Option<String>,
    pub chapter: Option<String>,
    pub pages: usize,
    pub translated_language: Locale,
    pub uploader: String,
    pub external_url: Option<String>,
    pub version: usize,
    pub created_at: String,
    pub updated_at: String,
    pub publish_at: String,
    pub readable_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Chapter {
    pub id: String,
    pub entity_type: EntityType,
    pub attributes: ChapterAttributes,
    pub relationships: Vec<Relationship>,
}

impl Entity for Chapter {}