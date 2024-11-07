pub mod chapter;
pub mod manga;
pub mod scanlation_group;
pub mod tag;

use bytes::Bytes;
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use std::collections::HashMap;
use std::default::Default;

use chapter::{Chapter, ChapterDownloadMeta};
use manga::{Manga, MangaFeedQuery, MangaQuery};
use scanlation_group::ScanlationGroup;

pub trait Entity {}
impl<T: Entity> Entity for Vec<T> {}

pub trait Query: Serialize {}

#[derive(Serialize, Deserialize, Debug, Clone, Default, Copy)]
pub struct EmptyQuery {}
impl Query for EmptyQuery {}

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
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub trait ResultOk {
    fn result_ok(&self) -> Result<bool>;
}

impl ResultOk for Value {
    fn result_ok(&self) -> Result<bool> {
        let result = match self.get("result") {
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

        Ok(responded_without_errors)
    }
}

pub struct MangoClient {
    client: Client,
}

impl MangoClient {
    pub const BASE_URL: &str = "https://api.mangadex.org";

    pub fn new() -> Result<Self> {
        Ok(Self {
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

    pub async fn parse_respond_data<T>(mut resp: Value) -> Result<T>
    where
        for<'a> T: Entity + Deserialize<'a> + Serialize,
    {
        let responded_without_errors = resp.result_ok()?;

        if responded_without_errors {
            let data = match resp.get_mut("data") {
                Some(d) => d,
                None => return Err(Error::ParseError),
            };

            Ok(serde_json::from_value::<T>(data.take())?)
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
            .query(&format!("{}/manga", MangoClient::BASE_URL), data)
            .await?
            .json()
            .await?;

        MangoClient::parse_respond_data(resp).await
    }

    pub async fn search_manga_by_name(&self, name: &str) -> Result<Vec<Manga>> {
        self.search_manga(&MangaQuery {
            title: Some(name.to_string()),
            ..Default::default()
        })
        .await
    }

    pub async fn get_manga_feed(&self, id: &str, data: &MangaFeedQuery) -> Result<Vec<Chapter>> {
        let resp: Value = self
            .query(&format!("{}/manga/{id}/feed", MangoClient::BASE_URL), data)
            .await?
            .json()
            .await?;

        MangoClient::parse_respond_data(resp).await
    }

    pub async fn get_chapter_download_meta(&self, id: &str) -> Result<ChapterDownloadMeta> {
        let mut resp: Value = self
            .query(
                &format!("{}/at-home/server/{id}", MangoClient::BASE_URL),
                &EmptyQuery {},
            )
            .await?
            .json()
            .await?;

        let responded_without_errors = resp.result_ok()?;

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

    pub async fn get_scanlation_group(&self, id: &str) -> Result<ScanlationGroup> {
        let resp: Value = self
            .query(
                &format!("{}/group/{id}", MangoClient::BASE_URL),
                &EmptyQuery {},
            )
            .await?
            .json()
            .await?;

        MangoClient::parse_respond_data(resp).await
    }

    pub async fn download_page(&self, url: &str) -> Result<Bytes> {
        match self.query(url, &EmptyQuery {}).await?.bytes().await {
            Ok(res) => Ok(res),
            Err(e) => Err(Error::RequestError(e)),
        }
    }

    pub fn feed_iter(self, manga_id: &str, mut query: MangaFeedQuery) -> Result<Feed> {
        let rt = tokio::runtime::Runtime::new()?;

        query.offset = Some(0);
        query.limit = Some(30);

        let buf = Box::new(
            rt.block_on(async { self.get_manga_feed(manga_id, &query).await })?
                .into_iter(),
        );

        Ok(Feed {
            manga_id: manga_id.to_string(),
            query,
            client: self,
            buf,
            rt,
        })
    }
}

pub struct Feed {
    manga_id: String,
    query: MangaFeedQuery,
    client: MangoClient,
    buf: Box<dyn Iterator<Item = Chapter>>,
    rt: tokio::runtime::Runtime,
}

impl Iterator for Feed {
    type Item = Result<Chapter>;

    fn next(&mut self) -> Option<Self::Item> {
        let next_bufferized = self.buf.next();

        if let Some(res) = next_bufferized {
            Some(Ok(res))
        } else {
            match self.rt.block_on(async {
                self.client
                    .get_manga_feed(&self.manga_id, &self.query)
                    .await
            }) {
                Ok(res) => {
                    self.buf = Box::new(res.into_iter());
                    self.query.offset =
                        Some(self.query.offset.expect("initialized with some") + 20);

                    Some(Ok(self.buf.next()?))
                }

                Err(e) => Some(Err(e)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::manga::*;
    use super::*;

    use std::io::prelude::*;

    #[tokio::test]
    async fn test_search_manga() {
        let client = MangoClient::new().unwrap();
        let resp = client
            .search_manga(&MangaQuery {
                title: Some("Chainsaw man".to_string()),
                // status: Some(vec![MangaStatus::Ongoing]),
                // year: Some(2015),
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
        let client = MangoClient::new().unwrap();

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
            .get_manga_feed(&chainsaw_manga_id, &query_data)
            .await
            .unwrap();

        let mut out = std::fs::File::create("chapters_struct").unwrap();

        out.write(format!("{chapters:#?}").as_bytes()).unwrap();
    }

    #[tokio::test]
    async fn test_chapter_download() {
        let client = MangoClient::new().unwrap();

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
            .get_manga_feed(&chainsaw_manga_id, &query_data)
            .await
            .unwrap();

        let mut out = std::fs::File::create("chapters_meta").unwrap();

        let id = chapters[2].id.clone();

        let download_meta = client.get_chapter_download_meta(&id).await.unwrap();

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

    #[tokio::test]
    async fn test_get_scanlation_group() {
        let client = MangoClient::new().unwrap();

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
            .get_manga_feed(&chainsaw_manga_id, &query_data)
            .await
            .unwrap();

        let chapter_relatioships = chapters[2].relationships.clone();

        let mut scanlation_group_id = None;
        for relationship in chapter_relatioships {
            match relationship.entity_type {
                EntityType::ScanlationGroup => {
                    scanlation_group_id = Some(relationship.id);

                    break;
                }
                _ => {}
            }
        }

        let scanlation_group_id = scanlation_group_id.unwrap();

        let scanlation_group = client
            .get_scanlation_group(&scanlation_group_id)
            .await
            .unwrap();

        let scanlation_group_name = scanlation_group.attributes.name;

        println!("Scanlation group name: {scanlation_group_name}");
    }

    #[tokio::test]
    async fn test_pageness() {
        let client = MangoClient::new().unwrap();

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
            limit: Some(200),
            offset: Some(1),
            excluded_groups: Some(vec![
                "4f1de6a2-f0c5-4ac5-bce5-02c7dbb67deb".to_string(),
                "a38fc704-90ab-452f-9336-59d84997a9ce".to_string(),
            ]),
            ..Default::default()
        };

        let chapters = client
            .get_manga_feed(&chainsaw_manga_id, &query_data)
            .await
            .unwrap();

        let mut out = std::fs::File::create("test_pages").unwrap();

        out.write(format!("{chapters:#?}").as_bytes()).unwrap();
    }

    #[test]
    fn test_iter() {
        let client = MangoClient::new().unwrap();

        let rt = tokio::runtime::Runtime::new().unwrap();

        let chainsaw_manga_id = rt.block_on(async {
            client
                .search_manga(&MangaQuery {
                    title: Some("Chainsaw Man".to_string()),
                    available_translated_language: Some(vec![Locale::En]),
                    ..Default::default()
                })
                .await
                .unwrap()[0]
                .id
                .clone()
        });

        let mut query_sorting_options = HashMap::new();

        query_sorting_options.insert(OrderOption::Chapter, Order::Asc);

        let query_data = MangaFeedQuery {
            translated_language: Some(vec![Locale::En]),
            order: Some(query_sorting_options),
            // limit: Some(2),
            // offset: Some(1),
            excluded_groups: Some(vec![
                "4f1de6a2-f0c5-4ac5-bce5-02c7dbb67deb".to_string(),
                "a38fc704-90ab-452f-9336-59d84997a9ce".to_string(),
            ]),
            ..Default::default()
        };

        let iter = client.feed_iter(&chainsaw_manga_id, query_data).unwrap();

        let mut out = std::fs::File::create("test_iter").unwrap();

        for chapter in iter {
            out.write(format!("{:#?}\n", chapter.unwrap()).as_bytes())
                .unwrap();
        }
    }
}
