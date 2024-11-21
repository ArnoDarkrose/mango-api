//! Utilities for chapter specific server part of the application.
//!
//! The meaning of most of the confusing structs here can be found at <https://api.mangadex.org/docs/3-enumerations/#manga-links-data>
//!
//! All queries encountered here can be constructed with the builder syntax from the [bon] crate

use serde::{Deserialize, Serialize};

use super::query_utils::Relationship;
use super::{Entity, EntityType, Locale};

/// Used for serialization/deserialization
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChapterAttributes {
    pub title: Option<String>,
    pub volume: Option<String>,
    pub chapter: Option<String>,
    pub pages: usize,
    pub translated_language: Locale,
    pub uploader: Option<String>,
    pub external_url: Option<String>,
    pub version: usize,
    pub created_at: String,
    pub updated_at: String,
    pub publish_at: String,
    pub readable_at: String,
}

/// Main structure used for representing the response containg chapter info
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Chapter {
    pub id: String,
    #[serde(rename = "type")]
    pub entity_type: EntityType,
    pub attributes: ChapterAttributes,
    pub relationships: Vec<Relationship>,
}

impl Entity for Chapter {}

/// Main structure used for representing the response containg chapter meta info
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChapterMeta {
    pub hash: String,
    pub data: Vec<String>,
    pub data_saver: Vec<String>,
}

/// Main structure used for representing the response containg chapter download meta info
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChapterDownloadMeta {
    pub result: String,
    pub base_url: String,
    pub chapter: ChapterMeta,
}
