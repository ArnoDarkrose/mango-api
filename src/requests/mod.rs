pub mod chapter;
pub mod manga;
pub mod tag;

use bytes::Bytes;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use std::collections::HashMap;
use std::default::Default;

use chapter::{Chapter, ChapterDownloadMeta};
use manga::{Manga, MangaStatus};
use tag::{Tag, TagsMode};

pub trait Entity {}
pub trait Query: Serialize {}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Copy)]
pub struct EmptyQuery {}
impl Query for EmptyQuery {}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
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

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, std::hash::Hash, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum Locale {
    Ab,
    Aa,
    Af,
    Ak,
    Sq,
    Am,
    Ar,
    An,
    Hy,
    Av,
    Ae,
    Ay,
    Az,
    Bm,
    Ba,
    Eu,
    Be,
    Bn,
    Bi,
    Bs,
    Br,
    Bg,
    My,
    Ca,
    Ch,
    Ce,
    Ny,
    Zh,
    Cu,
    Cv,
    Kw,
    Co,
    Cr,
    Hr,
    Cs,
    Da,
    Dv,
    Dz,
    En,
    Eo,
    Et,
    Ee,
    Fo,
    Fj,
    Fr,
    Fi,
    Fy,
    Ff,
    Gd,
    Gl,
    Lg,
    Ka,
    De,
    El,
    Kl,
    Gn,
    Gu,
    Hu,
    Ht,
    Ha,
    He,
    Hi,
    Ho,
    Is,
    Io,
    Ig,
    Id,
    Ia,
    Ie,
    Iu,
    Ik,
    Ga,
    It,
    Ja,
    Jv,
    Kn,
    Kr,
    Ks,
    Kk,
    Km,
    Ki,
    Rw,
    Ky,
    Kv,
    Kg,
    Ko,
    Kj,
    Ku,
    Lo,
    La,
    Lv,
    Li,
    Ln,
    Lt,
    Lu,
    Lb,
    Mk,
    Mg,
    Ms,
    Ml,
    Mt,
    Gv,
    Mi,
    Mr,
    Mh,
    Mn,
    Na,
    Nv,
    Nd,
    Nr,
    Ng,
    Ne,
    No,
    Nb,
    Nn,
    Nl,
    Oc,
    Oj,
    Or,
    Om,
    Os,
    Pi,
    Ps,
    Fa,
    Pl,
    Pt,
    Pa,
    Qu,
    Ro,
    Rm,
    Rn,
    Ru,
    Se,
    Sm,
    Sg,
    Sa,
    Sc,
    Sr,
    Sn,
    Sd,
    Si,
    Sk,
    Sl,
    So,
    St,
    Es,
    Su,
    Sw,
    Ss,
    Sv,
    Tl,
    Ty,
    Tg,
    Ta,
    Tt,
    Te,
    Th,
    Bo,
    Ti,
    To,
    Ts,
    Tn,
    Tr,
    Tk,
    Tw,
    Ug,
    Uk,
    Ur,
    Uz,
    Ve,
    Vi,
    Vo,
    Wa,
    Cy,
    Wo,
    Xh,
    Ii,
    Yi,
    Yo,
    Za,
    Zu,
    As,
    ZhHk,
    PtBr,
    EsLa,
    JaRo,
    KoRo,
    ZhRo,
}

pub type LocalizedString = HashMap<Locale, String>;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum EntityType {
    Manga,
    CoverArt,
    Chapter,
    Author,
    ApiClient,
    ReportReason,
    ScanlationGroup,
    User,
    Tag,
    Artist,
    Creator,
}

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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Relationship {
    id: String,
    #[serde(rename = "type")]
    entity_type: EntityType,
    related: Option<MangaRelation>,
    attributes: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum Order {
    Asc,
    Desc,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, std::hash::Hash)]
#[serde(rename_all = "camelCase")]
pub enum OrderOption {
    Title,
    Year,
    CreatedAt,
    UpdatedAt,
    LatestUploadedChapter,
    FollowedCount,
    Relevance,
    Chapter,
}

pub type SortingOptions = HashMap<OrderOption, Order>;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ContentRating {
    Safe,
    Suggestive,
    Erotica,
    Pornographic,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PublicationDemographic {
    Shounen,
    Shoujo,
    Josei,
    Seinen,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BadResponseError {
    id: String,
    status: i32,
    title: String,
    detail: Option<String>,
    context: Option<String>,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    RequestError(#[from] reqwest::Error),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error("error while parsing json value")]
    ParseError,
    #[error("400 server respond")]
    BadResponseError(Vec<BadResponseError>),
    #[error(transparent)]
    QsError(#[from] serde_qs::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct MangaClient {
    client: Client,
}

impl MangaClient {
    pub const BASE_URL: &str = "https://api.mangadex.org";

    pub fn new() -> Result<MangaClient> {
        Ok(MangaClient {
            client: Client::builder().user_agent("Mango/1.0").build()?,
        })
    }

    pub async fn query(&self, base_url: &str, query: &impl Query) -> Result<Response> {
        let query_data = match serde_qs::to_string(query) {
            Ok(res) => res,
            Err(e) => return Err(Error::QsError(e)),
        };

        let url = format!("{base_url}?{query_data}");
        match self.client.get(url).send().await {
            Ok(res) => Ok(res),
            Err(e) => Err(Error::RequestError(e)),
        }
    }

    pub async fn parse_respond<T>(mut resp: Value) -> Result<Vec<T>>
    where
        for<'a> T: Entity + Deserialize<'a> + Serialize,
    {
        let result = match resp.get("result") {
            Some(status) => status,
            None => return Err(Error::ParseError),
        };

        let responded_without_errors;

        if result.is_string() {
            let result = result.as_str().expect("verified to be a string");

            if result == "ok" {
                responded_without_errors = true;
            } else {
                responded_without_errors = false;
            }
        } else {
            return Err(Error::ParseError);
        }

        if responded_without_errors {
            let data = match resp.get_mut("data") {
                Some(d) => d,
                None => return Err(Error::ParseError),
            };

            Ok(serde_json::from_value::<Vec<T>>(data.take())?)
        } else {
            let errors = match resp.get_mut("errors") {
                Some(d) => d,
                None => return Err(Error::ParseError),
            };

            let err: Vec<BadResponseError> = serde_json::from_value(errors.take())?;

            Err(Error::BadResponseError(err))
        }
    }

    pub async fn search_manga(&self, data: &MangaQuery) -> Result<Vec<Manga>> {
        let resp: Value = self
            .query(&format!("{}/manga", MangaClient::BASE_URL), data)
            .await?
            .json()
            .await?;

        MangaClient::parse_respond(resp).await
    }

    pub async fn search_manga_by_name(&self, name: &str) -> Result<Vec<Manga>> {
        self.search_manga(&MangaQuery {
            title: Some(name.to_string()),
            ..Default::default()
        })
        .await
    }

    pub async fn get_manga_feed(&self, id: String, data: &MangaFeedQuery) -> Result<Vec<Chapter>> {
        let resp: Value = self
            .query(&format!("{}/manga/{id}/feed", MangaClient::BASE_URL), data)
            .await?
            .json()
            .await?;

        MangaClient::parse_respond(resp).await
    }

    pub async fn get_chapter_download_meta(&self, id: String) -> Result<ChapterDownloadMeta> {
        let mut resp: Value = self
            .query(
                &format!("{}/at-home/server/{id}", MangaClient::BASE_URL),
                &EmptyQuery {},
            )
            .await?
            .json()
            .await?;

        let result = match resp.get("result") {
            Some(status) => status,
            None => return Err(Error::ParseError),
        };

        let responded_without_errors;

        if result.is_string() {
            let result = result.as_str().expect("verified to be a string");

            if result == "ok" {
                responded_without_errors = true;
            } else {
                responded_without_errors = false;
            }
        } else {
            return Err(Error::ParseError);
        }

        if responded_without_errors {
            Ok(serde_json::from_value(resp)?)
        } else {
            Err(Error::BadResponseError(serde_json::from_value::<
                Vec<BadResponseError>,
            >(
                resp["errors"].take()
            )?))
        }
    }

    pub async fn download_page(&self, url: &str) -> Result<Bytes> {
        match self.query(url, &EmptyQuery {}).await?.bytes().await {
            Ok(res) => Ok(res),
            Err(e) => Err(Error::RequestError(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::manga::*;
    use super::*;

    use std::io::prelude::*;

    #[tokio::test]
    async fn test_find_by_name() {
        let client = MangaClient::new().unwrap();
        let resp = client
            .search_manga(&MangaQuery {
                // title: Some("Chainsaw man".to_string()),
                status: Some(vec![MangaStatus::Ongoing]),
                year: Some(2015),
                // original_language: Some(vec![Locale::En]),
                ..Default::default()
            })
            .await
            .unwrap();

        let mut out = std::fs::File::create("manga_struct").unwrap();

        out.write(format!("{resp:#?}").as_bytes()).unwrap();
    }

    #[tokio::test]
    async fn test_get_manga_feed() {
        let client = MangaClient::new().unwrap();

        let chainsaw_manga_id = client
            .search_manga(&MangaQuery {
                title: Some("Chainsaw Man".to_string()),
                available_translated_language: Some(vec![Locale::En]),
                ..Default::default()
            })
            .await
            .unwrap()[0]
            .id
            .clone();

        let mut query_sorting_options = HashMap::new();

        query_sorting_options.insert(OrderOption::Chapter, Order::Asc);

        let query_data = MangaFeedQuery {
            translated_language: Some(vec![Locale::En]),
            order: Some(query_sorting_options),
            ..Default::default()
        };

        let chapters = client
            .get_manga_feed(chainsaw_manga_id, &query_data)
            .await
            .unwrap();

        let mut out = std::fs::File::create("chapters_struct").unwrap();

        out.write(format!("{chapters:#?}").as_bytes()).unwrap();
    }

    #[tokio::test]
    async fn test_chapter_download() {
        let client = MangaClient::new().unwrap();

        let chainsaw_manga_id = client
            .search_manga(&MangaQuery {
                title: Some("Chainsaw Man".to_string()),
                available_translated_language: Some(vec![Locale::En]),
                ..Default::default()
            })
            .await
            .unwrap()[0]
            .id
            .clone();

        let mut query_sorting_options = HashMap::new();

        query_sorting_options.insert(OrderOption::Chapter, Order::Asc);

        let query_data = MangaFeedQuery {
            translated_language: Some(vec![Locale::En]),
            order: Some(query_sorting_options),
            ..Default::default()
        };

        let chapters = client
            .get_manga_feed(chainsaw_manga_id, &query_data)
            .await
            .unwrap();

        let mut out = std::fs::File::create("chapters_meta").unwrap();

        let id = chapters[2].id.clone();

        let download_meta = client.get_chapter_download_meta(id).await.unwrap();

        out.write(format!("{download_meta:#?}\n").as_bytes())
            .unwrap();

        let base_url = format!(
            "{}/data/{}",
            download_meta.base_url, download_meta.chapter.hash
        );

        if !std::fs::exists("pages").unwrap() {
            std::fs::create_dir("pages").unwrap();
        }
        for (i, page_url) in kdam::tqdm!(download_meta.chapter.data.into_iter().enumerate().take(3))
        {
            let bytes = client
                .download_page(&format!("{base_url}/{page_url}"))
                .await
                .unwrap();

            let mut out_page = std::fs::File::create(format!("pages/{i}.png")).unwrap();

            out_page.write(&bytes).unwrap();
        }
    }
}
