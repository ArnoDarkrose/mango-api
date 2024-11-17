pub mod chapter;
pub mod cover;
pub mod manga;
pub mod query_utils;
pub mod scanlation_group;
pub mod tag;

use crate::viewer::PageStatus;
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

use reqwest::{Client, Response, StatusCode};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_tracing::TracingMiddleware;

use tokio::sync::mpsc;
use tokio::task;
use tokio_stream::wrappers::ReceiverStream;
use tracing::instrument::Instrument as _;

use std::default::Default;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerResponseError {
    id: String,
    status: i32,
    title: String,
    detail: Option<String>,
    context: Option<String>,
}

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

pub type Result<T> = std::result::Result<T, Error>;

pub trait Entity {}
impl<T: Entity> Entity for Vec<T> {}

#[derive(Clone, Debug)]
pub struct MangoClient {
    client: ClientWithMiddleware,
}

impl MangoClient {
    pub const BASE_URL: &str = "https://api.mangadex.org";

    pub fn new() -> Result<Self> {
        let res = Client::builder().user_agent("Mango/1.0").build()?;
        let res = ClientBuilder::new(res)
            .with(TracingMiddleware::default())
            .build();

        Ok(Self { client: res })
    }

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

    #[tracing::instrument]
    pub async fn search_manga(&self, data: &MangaQuery) -> Result<Vec<Manga>> {
        let resp: Value = self
            .query(&format!("{}/manga", MangoClient::BASE_URL), data)
            .await?
            .json()
            .await?;

        MangoClient::parse_respond_data(resp).await
    }

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

    pub async fn search_manga_by_name(&self, name: &str) -> Result<Vec<Manga>> {
        self.search_manga(&MangaQuery {
            title: Some(name.to_string()),
            ..Default::default()
        })
        .await
    }

    pub async fn search_manga_by_name_include_cover(&self, name: &str) -> Result<Vec<Manga>> {
        self.search_manga_include_cover(&MangaQuery {
            title: Some(name.to_string()),
            ..Default::default()
        })
        .await
    }

    #[tracing::instrument]
    pub async fn get_manga_feed(&self, id: &str, data: &MangaFeedQuery) -> Result<Vec<Chapter>> {
        let resp: Value = self
            .query(&format!("{}/manga/{id}/feed", MangoClient::BASE_URL), data)
            .await?
            .json()
            .await?;

        MangoClient::parse_respond_data(resp).await
    }

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
