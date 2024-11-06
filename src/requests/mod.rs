pub mod manga;
pub mod tag;

use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use std::collections::HashMap;
use std::default::Default;

use manga::{Manga, MangaStatus};
use tag::{Tag, TagsMode};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct MangaQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub title: Option<String>,
    pub author_or_artist: Option<String>,
    #[serde(rename = "authors[]")]
    pub authors: Option<Vec<String>>,
    #[serde(rename = "artists[]")]
    pub artists: Option<Vec<String>>,
    pub year: Option<usize>,
    #[serde(rename = "includedTags[]")]
    pub included_tags: Option<Vec<Tag>>,
    #[serde(rename = "includedTagsMode[]")]
    pub included_tags_mode: Option<TagsMode>,
    #[serde(rename = "excludedTags[]")]
    pub excluded_tags: Option<Vec<Tag>>,
    #[serde(rename = "excludedTagsMode[]")]
    pub excluded_tags_mode: Option<TagsMode>,
    #[serde(rename = "status[]")]
    pub status: Option<Vec<MangaStatus>>,
    #[serde(rename = "originalLanguage[]")]
    pub original_language: Option<Vec<Locale>>,
    #[serde(rename = "excludedOriginalLanguage[]")]
    pub excluded_original_language: Option<Vec<Locale>>,
    #[serde(rename = "availableTrandlatedLanguage[]")]
    pub available_translated_language: Option<Vec<Locale>>,
    #[serde(rename = "publicationDemographic[]")]
    pub publication_demographic: Option<Vec<PublicationDemographic>>,
    #[serde(rename = "ids[]")]
    pub ids: Option<Vec<String>>,
    #[serde(rename = "contentRating[]")]
    pub content_rating: Option<Vec<ContentRating>>,
    pub created_at_since: Option<String>,
    pub updated_at_since: Option<String>,
    #[serde(rename = "order[]")]
    pub order: Option<Vec<SortingOptions>>,
    pub includes: Option<Value>,
    pub has_available_chapters: Option<String>,
    pub group: Option<String>,
}

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
    #[serde(rename(deserialize = "type"))]
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

    pub async fn query_manga(&self, data: &MangaQuery) -> reqwest::Result<Response> {
        self.client
            .get(format!("{}/manga", MangaClient::BASE_URL))
            .json(data)
            .send()
            .await
    }

    pub async fn search_manga(&self, data: &MangaQuery) -> Result<Vec<Manga>> {
        let mut resp: Value = self.query_manga(data).await?.json().await?;
        // let mut resp: Value = self.query_manga(data).await.unwrap().json().await?;

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

            Ok(serde_json::from_value(data.take())?)
        } else {
            let errors = match resp.get_mut("errors") {
                Some(d) => d,
                None => return Err(Error::ParseError),
            };

            let err: Vec<BadResponseError> = serde_json::from_value(errors.take())?;

            Err(Error::BadResponseError(err))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::manga::*;
    use super::tag::*;
    use super::*;

    use std::io::prelude::*;

    #[tokio::test]
    async fn test_find_by_name() {
        let client = MangaClient::new().unwrap();
        let resp = client
            .search_manga(&MangaQuery {
                // title: Some("Chainsaw man".to_string()),
                status: Some(vec![MangaStatus::Completed]),
                // year: Some(1200),
                // original_language: Some(vec![Locale::En]),
                ..Default::default()
            })
            .await
            .unwrap();

        let mut out = std::fs::File::create("manga_struct").unwrap();

        out.write(format!("{resp:#?}").as_bytes()).unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn test_queries() {
        let client = reqwest::Client::builder()
            .user_agent("Mango/1.0")
            .build()
            .unwrap();

        let mut data = HashMap::new();
        // data.insert("status[]", "ongoing");
        data.insert("status[]", vec!["ongoing", "completed"]);

        let json1 = serde_json::json!(data);

        println!("{:#?}", json1.to_string());
        let resp: Value = client
            .get(format!("{}/manga", MangaClient::BASE_URL))
            .json(&data)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        for entry in resp["data"].as_array().unwrap() {
            println!("{:#?}", entry["attributes"]["status"])
        }
    }
}
