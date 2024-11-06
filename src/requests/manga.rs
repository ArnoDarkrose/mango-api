use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use super::tag::Tag;
use super::{
    ContentRating, Entity, EntityType, Locale, LocalizedString, PublicationDemographic,
    Relationship,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum MangaStatus {
    Completed,
    Ongoing,
    Cancelled,
    Hiatus,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, std::hash::Hash, Clone)]
#[serde(rename_all = "camelCase")]
pub enum MangaLinkSource {
    Al,
    Ap,
    Bw,
    Mu,
    Nu,
    Kt,
    Amz,
    Ebj,
    Mal,
    Cdj,
    Raw,
    Engtl,
}

pub type MangaLinks = HashMap<MangaLinkSource, String>;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum MangaState {
    Draft,
    Submitted,
    Published,
    Rejected,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MangaAttributes {
    pub title: LocalizedString,
    pub alt_titles: Vec<LocalizedString>,
    pub description: LocalizedString,
    pub is_locked: bool,
    pub links: MangaLinks,
    pub original_language: Locale,
    pub last_volume: Option<String>,
    pub last_chapter: Option<String>,
    pub publication_demographic: Option<PublicationDemographic>,
    pub status: MangaStatus,
    pub year: Option<isize>,
    pub content_rating: ContentRating,
    pub chapter_numbers_reset_on_new_volume: bool,
    pub available_translated_languages: Vec<Locale>,
    pub latest_uploaded_chapter: String,
    pub tags: Vec<Tag>,
    pub state: MangaState,
    pub version: usize,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Manga {
    pub id: String,
    #[serde(rename(deserialize = "type"))]
    pub entity_type: EntityType,
    pub attributes: MangaAttributes,
    pub relationships: Vec<Relationship>,
}

impl Entity for Manga {}
