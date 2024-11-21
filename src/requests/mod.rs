//! Structs and utilities for making requests to mangadex servers

pub mod chapter;
pub mod cover;
pub mod manga;
pub mod query_utils;
pub mod scanlation_group;
pub mod tag;

use crate::viewer::PageStatus;
use crate::MangoClient;
use chapter::{Chapter, ChapterDownloadMeta};
use cover::CoverArtAttributes;
use manga::{Manga, MangaFeedQuery, MangaQuery};
use query_utils::{EmptyQuery, EntityType, Locale, Query, ResponseResultOk as _};
use scanlation_group::ScanlationGroup;
use tag::Tag;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use reqwest::{Response, StatusCode};

use tokio::sync::mpsc;
use tokio::task;
use tokio_stream::wrappers::ReceiverStream;
use tracing::instrument::Instrument as _;

use std::default::Default;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;

/// Used to deserialize errors returned from server
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerResponseError {
    id: String,
    status: i32,
    title: String,
    detail: Option<String>,
    context: Option<String>,
}

/// Custom error type that contains all errors that this can be emitted by this crate's functions
#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    RequestWithMiddleWareError(#[from] reqwest_middleware::Error),
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
    #[error("error while parsing json value")]
    ParseError,
    #[error("104 server response")]
    ConnectionResetByPeerError(Vec<ServerResponseError>),
    #[error("400 server response")]
    BadRequestError(Vec<ServerResponseError>),
    #[error("404 server response")]
    NotFoundError(Vec<ServerResponseError>),
    #[error("403 server respose")]
    ForbiddenError(Vec<ServerResponseError>),
    #[error(transparent)]
    QsError(#[from] serde_qs::Error),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

/// Type alias for the [`Result`](std::result::Result) that is used in the crate's functions
pub type Result<T> = std::result::Result<T, Error>;

/// [Entity] is implemented for all structs that represent Entity types in terms used by mangadex servers
pub trait Entity {}
impl<T: Entity> Entity for Vec<T> {}

impl MangoClient {
    pub const BASE_URL: &str = "https://api.mangadex.org";

    /// Lowest level function that executes arbitrary [Query] and returnes its response
    #[tracing::instrument]
    pub async fn query(&self, base_url: &str, query: &impl Query) -> Result<Response> {
        let query_data = match serde_qs::to_string(query) {
            Ok(res) => res,
            Err(e) => return Err(Error::QsError(e)),
        };

        let url = format!("{base_url}?{query_data}");
        match self.client.get(url).send().await {
            Ok(res) => Ok(res),
            Err(e) => Err(Error::RequestWithMiddleWareError(e)),
        }
    }

    /// Deserializes responses that can be deserialized into [Entity] or a [`Vec`] of entities
    pub async fn parse_respond_data<T>(mut resp: Value) -> Result<T>
    where
        for<'a> T: Entity + Deserialize<'a> + Serialize,
    {
        let responded_without_errors = resp.response_result_ok()?;

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

            let err: Vec<ServerResponseError> = serde_json::from_value(errors.take())?;

            Err(Error::BadRequestError(err))
        }
    }

    /// Searches for manga with parameteres, specified by data
    #[tracing::instrument]
    pub async fn search_manga(&self, data: &MangaQuery) -> Result<Vec<Manga>> {
        let resp: Value = self
            .query(&format!("{}/manga", MangoClient::BASE_URL), data)
            .await?
            .json()
            .await?;

        MangoClient::parse_respond_data(resp).await
    }

    /// Essentially the same as [`search_manga`](MangoClient::search_manga) but the returned response would also contain information
    /// about covers for each entry
    #[tracing::instrument]
    pub async fn search_manga_include_cover(&self, data: &MangaQuery) -> Result<Vec<Manga>> {
        let mut data = data.clone();
        data.includes = Some(serde_json::json!(["cover_art"]));

        let resp: Value = self
            .query(&format!("{}/manga", MangoClient::BASE_URL), &data)
            .await?
            .json()
            .await?;

        MangoClient::parse_respond_data(resp).await
    }

    /// Executes [`search_manga_include_cover`](MangoClient::search_manga_include_cover) and downloads cover
    /// for each entry. Returns each search entry paired with byte respresentation of its cover
    #[tracing::instrument]
    pub async fn search_manga_with_cover(&self, data: &MangaQuery) -> Result<Vec<(Manga, Bytes)>> {
        let resp = self.search_manga_include_cover(data).await?;

        let mut res = Vec::new();
        for manga in resp {
            let mut cover = None;
            for relation in manga.relationships.iter() {
                if let EntityType::CoverArt = relation.entity_type {
                    cover = Some(serde_json::from_value::<CoverArtAttributes>(
                        relation
                            .clone()
                            .attributes
                            .expect("didn't get relation attributes from server"),
                    )?);

                    break;
                }
            }

            let cover = cover.expect("server didn't send the required cover art info");

            let bytes = self
                .download_full_cover(&manga.id, &cover.file_name)
                .await?;

            res.push((manga, bytes));
        }

        Ok(res)
    }

    /// Shorthand for searching manga just by name
    pub async fn search_manga_by_name(&self, name: &str) -> Result<Vec<Manga>> {
        self.search_manga(&MangaQuery {
            title: Some(name.to_string()),
            ..Default::default()
        })
        .await
    }

    /// The same as [`search_manga_by_name`](MangoClient::search_manga_by_name) combined with
    /// [`search_manga_include_cover`](MangoClient::search_manga_include_cover)
    pub async fn search_manga_by_name_include_cover(&self, name: &str) -> Result<Vec<Manga>> {
        self.search_manga_include_cover(&MangaQuery {
            title: Some(name.to_string()),
            ..Default::default()
        })
        .await
    }

    /// Queries for the feed of the manga with the given `id` and parameteres specified by `data`
    #[tracing::instrument]
    pub async fn get_manga_feed(&self, id: &str, data: &MangaFeedQuery) -> Result<Vec<Chapter>> {
        let resp: Value = self
            .query(&format!("{}/manga/{id}/feed", MangoClient::BASE_URL), data)
            .await?
            .json()
            .await?;

        MangoClient::parse_respond_data(resp).await
    }

    /// Queries for the meta info about downloading chapter with the given `id`
    #[tracing::instrument]
    pub async fn get_chapter_download_meta(&self, id: &str) -> Result<ChapterDownloadMeta> {
        let mut resp: Value = self
            .query(
                &format!("{}/at-home/server/{id}", MangoClient::BASE_URL),
                &EmptyQuery {},
            )
            .await?
            .json()
            .await?;

        let responded_without_errors = resp.response_result_ok()?;

        if responded_without_errors {
            Ok(serde_json::from_value(resp)?)
        } else {
            Err(Error::BadRequestError(serde_json::from_value::<
                Vec<ServerResponseError>,
            >(resp["errors"].take())?))
        }
    }

    /// Queries for the info about the scanlation group with the specified `id`
    #[tracing::instrument(skip(self))]
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

    /// Queries for available tags
    pub async fn get_tags(&self) -> Result<Vec<Tag>> {
        let resp: Value = self
            .query(
                &format!("{}/manga/tag", MangoClient::BASE_URL),
                &EmptyQuery {},
            )
            .await?
            .json()
            .await?;

        MangoClient::parse_respond_data(resp).await
    }

    /// Given the name of the cover filename (on mangadex server) and the manga `id`,
    /// downloads the needed cover art
    #[tracing::instrument(skip(self))]
    pub async fn download_full_cover(&self, manga_id: &str, cover_filename: &str) -> Result<Bytes> {
        let url = format!("https://uploads.mangadex.org/covers/{manga_id}/{cover_filename}");

        let resp = self.query(&url, &EmptyQuery {}).await?;
        let status = resp.status();
        if status != 200 {
            tracing::warn!("got an error from server: {resp:#?}");
        }

        match resp.bytes().await {
            Ok(res) => Ok(res),
            Err(e) => Err(Error::ReqwestError(e)),
        }
    }

    /// Shorthand for deserializing server error response
    async fn deserialize_reponse_error<T: std::fmt::Debug>(resp: Response) -> Result<T> {
        let status = resp.status();

        let errors = resp.json::<Value>().await?["errors"].take();
        let e: Vec<ServerResponseError> = serde_json::from_value(errors)?;

        let res = match status.as_str() {
            "400" => Err(Error::BadRequestError(e)),
            "403" => Err(Error::ForbiddenError(e)),
            "404" => Err(Error::NotFoundError(e)),
            "104" => Err(Error::ConnectionResetByPeerError(e)),
            _ => {
                unreachable!()
            }
        };

        tracing::warn!("got {res:#?} from server");

        res
    }

    /// Downloads page from the specified `url`
    #[tracing::instrument]
    pub async fn download_full_page(&self, url: &str) -> Result<Bytes> {
        let resp = self.query(url, &EmptyQuery {}).await?;

        if resp.status() != StatusCode::OK {
            Self::deserialize_reponse_error(resp)
                .in_current_span()
                .await
        } else {
            match resp.bytes().await {
                Ok(res) => Ok(res),
                Err(e) => Err(Error::ReqwestError(e)),
            }
        }
    }

    /// Queries for chunked downloading of the page from the specified `url`.
    /// The returned [Response] can then be used to download the page chunk by chunk    
    #[tracing::instrument]
    pub async fn get_page_chunks(&self, url: &str) -> Result<Response> {
        let resp = self.query(url, &EmptyQuery {}).await?;

        if resp.status() != StatusCode::OK {
            Self::deserialize_reponse_error(resp)
                .in_current_span()
                .await
        } else {
            Ok(resp)
        }
    }

    /// Downloads the chapter with the specified `id`, uses maximum `max_concurrent downloads` pages
    /// downloading at each moment of time. Downloaded chapter pages are stored in the directory
    /// "tmp/{chapter_id}"
    #[tracing::instrument(skip(self))]
    pub async fn download_full_chapter(
        &self,
        chapter_id: &str,
        mut max_concurrent_downloads: usize,
    ) -> Result<PathBuf> {
        max_concurrent_downloads = max_concurrent_downloads.max(1);

        let download_meta = self
            .get_chapter_download_meta(chapter_id)
            .in_current_span()
            .await?;

        let chapter_size = download_meta.chapter.data.len();
        let buf = Arc::new(Mutex::new(vec![PageStatus::Idle; chapter_size]));

        let (manager_command_sender, manager_command_receiver) = mpsc::channel(10);
        let manager_command_receiver = ReceiverStream::new(manager_command_receiver);

        let (downloadings_spawner_command_sender, downloadings_spawner_command_receiver) =
            mpsc::channel(10);
        let downloadings_spawner_command_receiver =
            ReceiverStream::new(downloadings_spawner_command_receiver);

        let res = Ok(format!("tmp/{}", &download_meta.chapter.hash).into());

        let chapter_download_dir = format!("tmp/{}", &download_meta.chapter.hash);
        if !std::fs::exists(&chapter_download_dir).expect("failed to get info about tmp directory")
        {
            std::fs::create_dir_all(&chapter_download_dir)
                .expect("failed to create directory for storing pages");
        } else {
            let dir_meta = std::fs::metadata(&chapter_download_dir)
                .expect("failed to create directory for storing pages");

            if !dir_meta.is_dir() {
                panic!("failed to create directory for storing pages: file already exists, not a directory");
            }
        }

        let manager = Self::downloadings_manager()
            .opened_page(1)
            .statuses(Arc::clone(&buf))
            .spawner_command_sender(downloadings_spawner_command_sender.clone())
            .command_receiver(manager_command_receiver)
            .max_concurrent_downloads(max_concurrent_downloads)
            .chapter_size(chapter_size)
            .call();

        let manager = task::spawn(manager);

        let spawner = Self::downloadings_spawner()
            .client(self.clone())
            .meta(download_meta)
            .command_receiver(downloadings_spawner_command_receiver)
            .manager_command_sender(manager_command_sender)
            .statuses(buf)
            .call();

        task::spawn(spawner);

        if let Err(e) = manager.await {
            tracing::warn!("downloadings manager finished with error: {e:#?}");
        }

        res
    }
}
