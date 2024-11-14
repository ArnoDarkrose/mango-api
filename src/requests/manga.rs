use serde::{Deserialize, Serialize};
use serde_json::Value;

use std::collections::HashMap;

use super::query_utils::{
    ContentRating, LocalizedString, PublicationDemographic, Query, Relationship, SortingOptions,
};
use super::tag::{Tag, TagsMode};
use super::{Entity, EntityType, Locale};

use bon::Builder;

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Manga {
    pub id: String,
    #[serde(rename(deserialize = "type"))]
    pub entity_type: EntityType,
    pub attributes: MangaAttributes,
    pub relationships: Vec<Relationship>,
}

impl Entity for Manga {}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Builder)]
#[builder(on(String, into))]
#[serde(rename_all = "camelCase")]
pub struct MangaQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub title: Option<String>,
    pub author_or_artist: Option<String>,
    pub authors: Option<Vec<String>>,
    pub artists: Option<Vec<String>>,
    pub year: Option<usize>,
    pub included_tags: Option<Vec<Tag>>,
    pub included_tags_mode: Option<TagsMode>,
    pub excluded_tags: Option<Vec<Tag>>,
    pub excluded_tags_mode: Option<TagsMode>,
    pub status: Option<Vec<MangaStatus>>,
    pub original_language: Option<Vec<Locale>>,
    pub excluded_original_language: Option<Vec<Locale>>,
    pub available_translated_language: Option<Vec<Locale>>,
    pub publication_demographic: Option<Vec<PublicationDemographic>>,
    pub ids: Option<Vec<String>>,
    pub content_rating: Option<Vec<ContentRating>>,
    pub created_at_since: Option<String>,
    pub updated_at_since: Option<String>,
    pub order: Option<SortingOptions>,
    pub includes: Option<Value>,
    pub has_available_chapters: Option<String>,
    pub group: Option<String>,
}

impl Query for MangaQuery {}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Builder)]
#[builder(on(String, into))]
#[serde(rename_all = "camelCase")]
pub struct MangaFeedQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub translated_language: Option<Vec<Locale>>,
    pub original_language: Option<Vec<Locale>>,
    pub excluded_original_language: Option<Vec<Locale>>,
    pub content_rating: Option<Vec<ContentRating>>,
    pub excluded_groups: Option<Vec<String>>,
    pub include_future_updates: Option<String>,
    pub created_at_since: Option<String>,
    pub updated_at_since: Option<String>,
    pub publish_at_since: Option<String>,
    pub order: Option<SortingOptions>,
    pub includes: Option<Value>,
    pub include_empty_pages: Option<usize>,
    pub include_future_publish_at: Option<usize>,
    pub include_external_url: Option<usize>,
}

impl Query for MangaFeedQuery {}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum MangaRelation {
    Monochrome,
    MainStory,
    AdaptedFrom,
    BasedOn,
    Prequel,
    SideStory,
    Doujinshi,
    SameFranchise,
    SharedUniverse,
    Sequel,
    SpinOff,
    AlternateStory,
    AlternateVersion,
    Preserialization,
    Colored,
    Serialization,
}
