use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use super::tag::Tag;
use super::{
    ContentRating, EntityType, Locale, LocalizedString, PublicationDemographic, Relationship,
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
    title: LocalizedString,
    alt_titles: Vec<LocalizedString>,
    description: LocalizedString,
    is_locked: bool,
    links: MangaLinks,
    original_language: Locale,
    last_volume: Option<String>,
    last_chapter: Option<String>,
    publication_demographic: Option<PublicationDemographic>,
    status: MangaStatus,
    year: Option<isize>,
    content_rating: ContentRating,
    chapter_numbers_reset_on_new_volume: bool,
    available_translated_languages: Vec<Locale>,
    latest_uploaded_chapter: String,
    tags: Vec<Tag>,
    state: MangaState,
    version: usize,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Manga {
    id: String,
    #[serde(rename(deserialize = "type"))]
    entity_type: EntityType,
    attributes: MangaAttributes,
    relationships: Vec<Relationship>,
}
