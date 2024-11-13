// TODO: add download and store full chapter function
// TODO: make a proper work on errors, sticking everything into one big error is bad
// TODO: change all span::in_scope to calling .instrument on every future

pub mod chapter;
pub mod manga;
pub mod scanlation_group;
pub mod tag;

use crate::viewer::{ChapterViewer, DownloadingsSpawnerCommand, ManagerCommand, PageStatus};
use chapter::{Chapter, ChapterDownloadMeta};
use manga::{Manga, MangaFeedQuery, MangaQuery};
use scanlation_group::ScanlationGroup;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use reqwest::{Client, Response};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_tracing::TracingMiddleware;

use tokio::io::AsyncWriteExt as _;
use tokio::sync::mpsc;
use tokio::task::{self, JoinSet};

use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt as _;

use tracing::instrument::Instrument;

use std::collections::HashMap;
use std::default::Default;
use std::sync::Mutex;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

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

pub trait ResultOk {
    fn result_ok(&self) -> Result<bool>;
}

impl ResultOk for Value {
    fn result_ok(&self) -> Result<bool> {
        let result = match self.get("result") {
            Some(status) => status,
            None => return Err(Error::ParseError),
        };

        let responded_without_errors = if result.is_string() {
            let result = result.as_str().expect("verified to be a string");

            result == "ok"
        } else {
            return Err(Error::ParseError);
        };

        Ok(responded_without_errors)
    }
}

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

    pub async fn get_manga_feed_val(self, id: &str, data: &MangaFeedQuery) -> Result<Vec<Chapter>> {
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

    #[tracing::instrument]
    pub async fn download_full_page(&self, url: &str) -> Result<Bytes> {
        // TODO: this a problem
        // when the url is invalid (for example some time passed and it invalidated)
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
        // TODO: this a problem
        // when the url is invalid (for example some time passed and it invalidated)
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

    fn determine_next_download_page(
        statuses: &mut std::sync::MutexGuard<'_, Vec<PageStatus>>,
        opened_page: usize,
    ) -> (Option<usize>, bool) {
        let mut found_loading_pages = false;

        match statuses[opened_page - 1] {
            PageStatus::Idle => (Some(opened_page), found_loading_pages),
            _ => {
                let mut page = opened_page;

                while page != opened_page - 1 {
                    if page == statuses.len() {
                        page = 0;
                    }

                    match statuses[page] {
                        PageStatus::Idle => {
                            break;
                        }
                        PageStatus::Loading(_) => {
                            found_loading_pages = true;
                        }
                        _ => {}
                    };

                    page += 1;
                }

                if page == opened_page - 1 {
                    if let PageStatus::Loading(_) = statuses[page] {
                        found_loading_pages = true;
                    }

                    (None, found_loading_pages)
                } else {
                    statuses[page] = PageStatus::Loading(0);
                    (Some(page + 1), found_loading_pages)
                }
            }
        }
    }

    #[tracing::instrument(skip(
        statuses,
        set_command_sender,
        downloadings_sender,
        command_receiver
    ))]
    async fn downloadings_manager(
        opened_page: Arc<AtomicUsize>,
        statuses: Arc<Mutex<Vec<PageStatus>>>,
        set_command_sender: mpsc::Sender<DownloadingsSpawnerCommand>,
        downloadings_sender: mpsc::Sender<usize>,
        mut command_receiver: ReceiverStream<ManagerCommand>,
        max_concurrent_downloads: usize,
        chapter_size: usize,
    ) {
        {
            let mut statuses = statuses.lock().expect("mutex poisoned");

            for i in 0..chapter_size.min(max_concurrent_downloads) {
                statuses[i] = PageStatus::Loading(0);
            }
        }

        for i in 0..chapter_size.min(max_concurrent_downloads) {
            set_command_sender
                .send(DownloadingsSpawnerCommand::NewDownload { page_num: i + 1 })
                .await
                .expect("task spawner shutdowned before downloading all pages");
        }

        while let Some(command) = command_receiver.next().await {
            tracing::debug!("got command: \n{command:#?}\n");
            tracing::trace!("\nstatuses: {statuses:#?}\n");

            match command {
                ManagerCommand::SwitchPage { page_num } => {
                    opened_page.store(page_num, Ordering::Release);
                }
                ManagerCommand::DownloadError { page_num } => {
                    let new_download_command = DownloadingsSpawnerCommand::NewDownload { page_num };
                    set_command_sender
                        .send(new_download_command)
                        .await
                        .expect("join_set task shutdowned before downloading all pages");

                    tracing::trace!("manager sent {new_download_command:#?} command to set");
                }
                ManagerCommand::DownloadedSuccessfully { page_num } => {
                    // NOTE: we don't care if the receiver is no longer interested in this
                    // information
                    //
                    // TODO: maybe this is a bug
                    let _ = downloadings_sender.send(page_num).await;

                    // TODO: i'm not sure if this scope is required
                    let (next_download_page, found_loading_pages) = {
                        Self::determine_next_download_page(
                            &mut statuses.lock().expect("mutex poisoined"),
                            opened_page.load(Ordering::Acquire),
                        )
                    };

                    match next_download_page {
                        Some(page) => {
                            let command =
                                DownloadingsSpawnerCommand::NewDownload { page_num: page };

                            set_command_sender
                                .send(command)
                                .await
                                .expect("join_set task shutdowned before downloading all pages");

                            tracing::trace!("manager sent {command:#?} command to set");
                        }
                        None => {
                            if !found_loading_pages {
                                set_command_sender
                                    .send(DownloadingsSpawnerCommand::Shutdown)
                                    .await
                                    .expect(
                                        "join_set task shutdowned before the respectful command",
                                    );

                                tracing::trace!("manager sent shutdown command to set");

                                break;
                            }
                        }
                    };
                }
            }
        }
        tracing::debug!("shutdowned");
    }

    async fn handle_downloading_page_request_error(
        e: reqwest::Error,
        manager_command_sender: &mpsc::Sender<ManagerCommand>,
        page_num: usize,
    ) {
        // TODO: handle possible errors

        tracing::warn!("got respond from the server: {e:#?}");

        manager_command_sender
            .send(ManagerCommand::DownloadError { page_num })
            .await
            .expect("manager shutdowned before getting the signal from last downloading task");
    }

    /// Queries next chunk, handles possible errors and returnes this chunk if no errors
    /// encountered or None if errors were met or the source is drained along with the boolean
    /// value that represents if errores were emitted
    async fn get_next_chunk(
        resp: &mut Response,
        manager_command_sender: &mpsc::Sender<ManagerCommand>,
        page_num: usize,
    ) -> (Option<Bytes>, bool) {
        match resp.chunk().in_current_span().await {
            Ok(chunk) => {
                let with_errors = false;
                (chunk, with_errors)
            }
            Err(e) => {
                Self::handle_downloading_page_request_error(e, manager_command_sender, page_num)
                    .in_current_span()
                    .await;

                let with_errors = true;
                (None, with_errors)
            }
        }
    }

    #[tracing::instrument(skip(client, statuses, manager_command_sender))]
    async fn downloading_page(
        client: MangoClient,
        chapter_hash: String,
        page_num: usize,
        url: String,
        statuses: Arc<Mutex<Vec<PageStatus>>>,
        manager_command_sender: mpsc::Sender<ManagerCommand>,
    ) {
        let out_page_filename = format!("tmp/{}/{}.png", chapter_hash, page_num);

        let mut out_page = tokio::fs::File::create(&out_page_filename)
            .await
            .expect("failed to open file to save page");

        let resp_res = client.get_page_chunks(&url).in_current_span().await;

        match resp_res {
            Ok(mut resp) => {
                let total_size = resp
                    .content_length()
                    .expect("could not get the content_length of page");

                while let (Some(chunk), with_error) =
                    Self::get_next_chunk(&mut resp, &manager_command_sender, page_num)
                        .in_current_span()
                        .await
                {
                    if with_error {
                        return;
                    }

                    let cur_size = chunk.len();

                    out_page
                        .write_all(chunk.as_ref())
                        .await
                        .expect("failed to save page");

                    {
                        let mut statuses = statuses.lock().expect("mutex poisoned");
                        let already_loaded = match statuses[page_num - 1] {
                            PageStatus::Loading(percent) => percent,
                            _ => unreachable!(),
                        };

                        statuses[page_num - 1] = PageStatus::Loading(
                            already_loaded + 100 * cur_size / total_size as usize,
                        );
                    }
                }

                {
                    let mut statuses = statuses.lock().expect("mutex poisoned");
                    statuses[page_num - 1] = PageStatus::Loaded(out_page_filename.into());
                }

                manager_command_sender
                    .send(ManagerCommand::DownloadedSuccessfully { page_num })
                    .await
                    .expect(
                        "manager shutdowned before getting the signal from last downloading task",
                    );
            }

            Err(e) => match e {
                Error::RequestError(e) => {
                    Self::handle_downloading_page_request_error(
                        e,
                        &manager_command_sender,
                        page_num,
                    )
                    .in_current_span()
                    .await;
                }
                _ => {
                    panic!("Got an unexpected error from download page process")
                }
            },
        }
    }

    #[tracing::instrument(skip_all)]
    async fn downloadings_spawner(
        client: MangoClient,
        meta: ChapterDownloadMeta,
        mut command_receiver: ReceiverStream<DownloadingsSpawnerCommand>,
        manager_command_sender: mpsc::Sender<ManagerCommand>,
        statuses: Arc<Mutex<Vec<PageStatus>>>,
    ) {
        let mut set = JoinSet::new();

        while let Some(command) = command_receiver.next().await {
            tracing::debug!("got command: \n{command:#?}\n");

            match command {
                DownloadingsSpawnerCommand::Shutdown => {
                    set.shutdown().await;

                    break;
                }
                DownloadingsSpawnerCommand::NewDownload { page_num } => {
                    // NOTE: we don't care about the result of the computations, this is simply
                    // for the sake of buffer not overflowing
                    let _ = set.try_join_next();

                    let download_url = format!(
                        "{}/data/{}/{}",
                        &meta.base_url,
                        &meta.chapter.hash,
                        &meta.chapter.data[page_num - 1]
                    );

                    set.spawn(Self::downloading_page(
                        client.clone(),
                        meta.chapter.hash.clone(),
                        page_num,
                        download_url,
                        Arc::clone(&statuses),
                        manager_command_sender.clone(),
                    ));

                    tracing::trace!("spawned new download task");
                }
            }
        }

        tracing::debug!("shutdowned");
    }

    #[tracing::instrument(skip(self))]
    pub async fn chapter_viewer(
        &self,
        chapter_id: &str,
        mut max_concurrent_downloads: usize,
    ) -> Result<ChapterViewer> {
        max_concurrent_downloads = max_concurrent_downloads.max(1);

        let download_meta = self
            .get_chapter_download_meta(chapter_id)
            .in_current_span()
            .await?;

        let chapter_size = download_meta.chapter.data.len();
        let buf = Arc::new(Mutex::new(vec![PageStatus::Idle; chapter_size]));

        let (downloadings_sender, downloading_receiver) = mpsc::channel(chapter_size);
        let downloading_receiver = ReceiverStream::new(downloading_receiver);

        let (manager_command_sender, manager_command_receiver) = mpsc::channel(10);
        let manager_command_receiver = ReceiverStream::new(manager_command_receiver);

        let (set_command_sender, set_command_receiver) = mpsc::channel(10);
        let set_command_receiver = ReceiverStream::new(set_command_receiver);

        let res = ChapterViewer {
            opened_page: Arc::new(AtomicUsize::new(1)),
            downloadings: downloading_receiver,
            meta: download_meta,
            statuses: buf,
            submit_switch: manager_command_sender.clone(),
        };

        let chapter_download_dir = format!("tmp/{}", res.meta.chapter.hash);
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

        task::spawn(Self::downloadings_manager(
            Arc::clone(&res.opened_page),
            Arc::clone(&res.statuses),
            set_command_sender.clone(),
            downloadings_sender,
            manager_command_receiver,
            max_concurrent_downloads,
            chapter_size,
        ));

        task::spawn(Self::downloadings_spawner(
            self.clone(),
            res.meta.clone(),
            set_command_receiver,
            manager_command_sender,
            Arc::clone(&res.statuses),
        ));

        return Ok(res);
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

        let mut out = tokio::fs::File::create("manga_struct").await.unwrap();

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

        let mut out = tokio::fs::File::create("chapters_struct").await.unwrap();

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

        let mut out = tokio::fs::File::create("chapters_meta").await.unwrap();

        let id = chapters[2].id.clone();

        let download_meta = client.get_chapter_download_meta(&id).await.unwrap();

        out.write_all(format!("{download_meta:#?}\n").as_bytes())
            .await
            .unwrap();

        let base_url = format!(
            "{}/data/{}",
            download_meta.base_url, download_meta.chapter.hash
        );

        if !std::fs::exists("pages").unwrap() {
            std::fs::create_dir("pages").unwrap();
        }
        for (i, page_url) in download_meta.chapter.data.into_iter().enumerate().take(3) {
            let url = format!("{base_url}/{page_url}");

            let bytes = client.download_full_page(&url).await.unwrap();

            let mut out_page = tokio::fs::File::create(format!("pages/{i}.png"))
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

        let mut out = tokio::fs::File::create("scanlation_group_name_test")
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

        let mut out = tokio::fs::File::create("test_pages").await.unwrap();

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
            .with_default_directive(LevelFilter::TRACE.into())
            .from_env_lossy();

        let (writer, _guard) = tracing_appender::non_blocking(
            std::fs::File::options().append(true).open("logs").unwrap(),
        );

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_test_writer()
                    .with_writer(writer)
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
            page_paths.push(viewer.get_page(i + 1).await);
        }

        let mut out = tokio::fs::File::create("page_paths").await.unwrap();

        out.write(format!("{page_paths:#?}").as_bytes())
            .await
            .unwrap();
    }
}
