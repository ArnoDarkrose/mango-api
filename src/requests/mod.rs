// TODO: make a proper work on errors, sticking everything into one big error is bad

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
use query_utils::{EmptyQuery, EntityType, Locale, Query, ResultOk as _};
use scanlation_group::ScanlationGroup;
use tag::Tag;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use reqwest::{Client, Response};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_tracing::TracingMiddleware;

use tokio::sync::mpsc;
use tokio::task;
use tokio_stream::wrappers::ReceiverStream;
use tracing::instrument::Instrument as _;

use std::default::Default;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::{atomic::AtomicUsize, Arc};

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
    RequestWithMiddleWareError(#[from] reqwest_middleware::Error),
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
            Err(e) => Err(Error::RequestError(e)),
        }
    }

    #[tracing::instrument]
    pub async fn download_full_page(&self, url: &str) -> Result<Bytes> {
        // TODO: this a problem: when the url is invalid (for example some time passed and it invalidated)
        // i might get something like 404 reponse and i don't handle it here
        // moreover, i don't handle any possible errors that might be returned from the server
        // the problem i didn't find the documentation for this query

        let resp = self.query(url, &EmptyQuery {}).await?;
        let status = resp.status();
        if status != 200 {
            tracing::warn!("got an error from server: {resp:#?}");
        }

        match resp.bytes().await {
            Ok(res) => Ok(res),
            Err(e) => Err(Error::RequestError(e)),
        }
    }

    #[tracing::instrument]
    pub async fn get_page_chunks(&self, url: &str) -> Result<Response> {
        // TODO: this a problem: when the url is invalid (for example some time passed and it invalidated)
        // i might get something like 404 reponse and i don't handle it here
        // moreover, i don't handle any possible errors that might be returned from the server
        // the problem i didn't find the documentation for this query

        let resp = self.query(url, &EmptyQuery {}).await?;
        let status = resp.status();
        if status != 200 {
            tracing::warn!("got an error from server: {resp:#?}");
        }

        Ok(resp)
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

        let manager = task::spawn(Self::downloadings_manager(
            Arc::new(AtomicUsize::new(1)),
            Arc::clone(&buf),
            downloadings_spawner_command_sender.clone(),
            None,
            manager_command_receiver,
            max_concurrent_downloads,
            chapter_size,
        ));

        task::spawn(Self::downloadings_spawner(
            self.clone(),
            download_meta,
            downloadings_spawner_command_receiver,
            manager_command_sender,
            buf,
        ));

        if let Err(e) = manager.await {
            tracing::warn!("downloadings manager finished with error: {e:#?}");
        }

        res
    }
}

#[cfg(test)]
mod tests {
    use super::manga::*;
    use super::*;

    use tracing_subscriber::filter::EnvFilter;
    use tracing_subscriber::filter::LevelFilter;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::prelude::*;

    use query_utils::{Order, OrderOption};
    use std::collections::HashMap;

    use tokio::io::AsyncWriteExt as _;

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

        let mut out = tokio::fs::File::create("test_files/manga_struct")
            .await
            .unwrap();

        out.write_all(format!("{resp:#?}").as_bytes())
            .await
            .unwrap();
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

        let mut out = tokio::fs::File::create("test_files/chapters_struct")
            .await
            .unwrap();

        out.write_all(format!("{chapters:#?}").as_bytes())
            .await
            .unwrap();
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

        let mut out = tokio::fs::File::create("test_files/chapters_meta")
            .await
            .unwrap();

        let id = chapters[2].id.clone();

        let download_meta = client.get_chapter_download_meta(&id).await.unwrap();

        out.write_all(format!("{download_meta:#?}\n").as_bytes())
            .await
            .unwrap();

        let base_url = format!(
            "{}/data/{}",
            download_meta.base_url, download_meta.chapter.hash
        );

        if !std::fs::exists("test_files/pages").unwrap() {
            std::fs::create_dir("test_files/pages").unwrap();
        }
        for (i, page_url) in download_meta.chapter.data.into_iter().enumerate().take(3) {
            let url = format!("{base_url}/{page_url}");

            let bytes = client.download_full_page(&url).await.unwrap();

            let mut out_page = tokio::fs::File::create(format!("test_files/pages/{i}.png"))
                .await
                .unwrap();

            out_page.write_all(&bytes).await.unwrap();
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

        let mut out = tokio::fs::File::create("test_files/scanlation_group_name_test")
            .await
            .unwrap();

        out.write_all(format!("Scanlation group name: {scanlation_group_name}").as_bytes())
            .await
            .unwrap();
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

        let mut out = tokio::fs::File::create("test_files/test_pages")
            .await
            .unwrap();

        out.write_all(format!("{chapters:#?}").as_bytes())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_viewer() {
        {
            std::fs::File::create("logs").unwrap();
        }

        let filter = EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env_lossy();

        let (writer, _guard) = tracing_appender::non_blocking(
            std::fs::File::options().append(true).open("logs").unwrap(),
        );

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_test_writer()
                    // .with_writer(writer)
                    .pretty()
                    .compact(),
            )
            .with(filter)
            .init();

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

        let first_chapter = chapters[2].clone();

        let mut viewer = client.chapter_viewer(&first_chapter.id, 8).await.unwrap();

        let chapter_len = first_chapter.attributes.pages;

        let mut page_paths = Vec::new();
        for i in 0..chapter_len {
            page_paths.push(viewer.open_page(i + 1).await);
        }

        let mut out = tokio::fs::File::create("test_files/page_paths")
            .await
            .unwrap();

        out.write(format!("{page_paths:#?}").as_bytes())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_download_full_chapter() {
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

        let second_chapter = chapters[4].clone();

        let chapter_path = client
            .download_full_chapter(&second_chapter.id, 8)
            .await
            .unwrap();

        let mut out = tokio::fs::File::create("test_files/chapter_path_test")
            .await
            .unwrap();

        out.write(format!("{chapter_path:#?}").as_bytes())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_get_manga_include_cover() {
        let client = MangoClient::new().unwrap();
        let resp = client
            .search_manga_include_cover(&MangaQuery {
                title: Some("Chainsaw man".to_string()),
                // status: Some(vec![MangaStatus::Ongoing]),
                // year: Some(2015),
                // original_language: Some(vec![Locale::En]),
                ..Default::default()
            })
            .await
            .unwrap();

        let manga = resp[0].clone();

        let mut cover = None;

        for relation in manga.relationships {
            match relation.entity_type {
                EntityType::CoverArt => {
                    cover = Some(serde_json::from_value::<cover::CoverArtAttributes>(
                        relation.attributes.unwrap(),
                    ));
                }
                _ => {}
            }
        }

        let cover = cover.unwrap().unwrap();

        let cover = client
            .download_full_cover(&manga.id, &cover.file_name)
            .await
            .unwrap();

        let mut out_cover = tokio::fs::File::create("test_files/cover.png")
            .await
            .unwrap();
        out_cover.write_all(cover.as_ref()).await.unwrap();

        let mut out = tokio::fs::File::create("test_files/manga_feed_with_cover_test")
            .await
            .unwrap();

        out.write_all(format!("{resp:#?}").as_bytes())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_get_manga_with_cover() {
        let client = MangoClient::new().unwrap();
        let resp = client
            .search_manga_with_cover(&MangaQuery {
                title: Some("Chainsaw man".to_string()),
                // status: Some(vec![MangaStatus::Ongoing]),
                // year: Some(2015),
                // original_language: Some(vec![Locale::En]),
                ..Default::default()
            })
            .await
            .unwrap();

        let (chainsaw_manga, chainsaw_cover) = resp[0].clone();

        let mut out_manga = tokio::fs::File::create("test_files/manga_with_cover")
            .await
            .unwrap();

        let mut out_manga_cover = tokio::fs::File::create("test_files/maga_with_cover_cover.png")
            .await
            .unwrap();

        out_manga
            .write_all(format!("{chainsaw_manga:#?}").as_bytes())
            .await
            .unwrap();

        out_manga_cover
            .write_all(chainsaw_cover.as_ref())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_get_tags() {
        let client = MangoClient::new().unwrap();
        let resp = client.get_tags().await.unwrap();

        let mut out_tags = tokio::fs::File::create("test_files/tags").await.unwrap();

        out_tags
            .write_all(format!("{resp:#?}").as_bytes())
            .await
            .unwrap();
    }
}
