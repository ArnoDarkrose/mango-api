//! Placement for most of the internal structs and functions related to the [ChapterViewer]

use crate::requests::chapter::ChapterDownloadMeta;
use crate::requests::{Error, Result};
use crate::MangoClient;

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;

use tokio::io::AsyncWriteExt as _;
use tokio::sync::mpsc::{self, Sender};
use tokio::task::{self, JoinSet};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt as _;

use bon::bon;
use bytes::Bytes;
use reqwest::Response;
use tracing::instrument::Instrument as _;

#[derive(Debug, Clone)]
pub(crate) enum PageStatus {
    Loaded(PathBuf),
    Loading(usize),
    Idle,
}

#[derive(Debug, Clone)]
pub(crate) enum ManagerCommand {
    SwitchPage { page_num: usize },
    DownloadedSuccessfully { page_num: usize },
    DownloadError { page_num: usize },
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum DownloadingsSpawnerCommand {
    Shutdown,
    NewDownload { page_num: usize },
}

/// The struct used for getting access to chapter pages that are being downloaded via
/// [MangoClient::chapter_viewer]
#[derive(Debug)]
pub struct ChapterViewer {
    pub(crate) statuses: Arc<Mutex<Vec<PageStatus>>>,
    pub(crate) downloadings: ReceiverStream<usize>,
    pub(crate) submit_switch: Sender<ManagerCommand>,
}

impl ChapterViewer {
    #[tracing::instrument(skip(self))]
    /// Returns the path to the desired page
    pub async fn open_page(&mut self, page_num: usize) -> PathBuf {
        tracing::trace!("entered");

        tracing::trace!("acquiring mutex lock");

        let status = {
            let statuses = self.statuses.lock();
            statuses[page_num - 1].clone()
        };

        tracing::trace!("released mutex lock");

        let sender = self.submit_switch.clone();

        task::spawn(async move {
            // NOTE: if the receiver is dropped, then all pages are downloaded and we don't care in
            // delivering this message
            let _ = sender.send(ManagerCommand::SwitchPage { page_num }).await;
        });

        match status {
            PageStatus::Loaded(path) => path,
            _ => {
                tracing::trace!("starting waiting for page {page_num}");

                let mut res = None;
                while let Some(downloaded_page_num) = self.downloadings.next().await {
                    tracing::trace!("got signal that page {downloaded_page_num} loaded");

                    if downloaded_page_num == page_num {
                        tracing::trace!("acquiring mutex lock");

                        let status = {
                            let statuses = self.statuses.lock();
                            statuses[page_num - 1].clone()
                        };

                        tracing::trace!("returned mutex lock");

                        match status {
                            PageStatus::Loaded(path) => {
                                res = Some(path);
                                break;
                            }
                            _ => panic!(
                                "got message that page was downloaded while it actually was not"
                            ),
                        }
                    }
                }
                res.expect("mananager task shutdowned before desired page was downloaded")
            }
        }
    }
}

#[bon]
impl MangoClient {
    pub(crate) fn determine_next_download_page(
        statuses: &mut parking_lot::MutexGuard<'_, Vec<PageStatus>>,
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

                        if page == opened_page - 1 {
                            break;
                        }
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

    #[builder]
    #[tracing::instrument(
        skip(
            statuses,
            spawner_command_sender,
            downloadings_sender,
            command_receiver
        ),
        name = "downloadings_manager"
    )]
    pub(crate) async fn downloadings_manager(
        mut opened_page: usize,
        statuses: Arc<Mutex<Vec<PageStatus>>>,
        spawner_command_sender: mpsc::Sender<DownloadingsSpawnerCommand>,
        downloadings_sender: Option<mpsc::Sender<usize>>,
        mut command_receiver: ReceiverStream<ManagerCommand>,
        max_concurrent_downloads: usize,
        chapter_size: usize,
    ) {
        {
            let mut statuses = statuses.lock();

            for i in 0..chapter_size.min(max_concurrent_downloads) {
                statuses[i] = PageStatus::Loading(0);
            }
        }

        for i in 0..chapter_size.min(max_concurrent_downloads) {
            spawner_command_sender
                .send(DownloadingsSpawnerCommand::NewDownload { page_num: i + 1 })
                .await
                .expect("task spawner shutdowned before downloading all pages");
        }

        while let Some(command) = command_receiver.next().await {
            tracing::debug!("got command: \n{command:#?}\n");
            tracing::trace!("\nstatuses: {statuses:#?}\n");

            match command {
                ManagerCommand::SwitchPage { page_num } => {
                    opened_page = page_num;
                }
                ManagerCommand::DownloadError { page_num } => {
                    let new_download_command = DownloadingsSpawnerCommand::NewDownload { page_num };
                    spawner_command_sender
                        .send(new_download_command)
                        .await
                        .expect("downloadings_spawner shutdowned before downloading all pages");

                    tracing::trace!("manager sent {new_download_command:#?} command to set");
                }
                ManagerCommand::DownloadedSuccessfully { page_num } => {
                    // NOTE: if the receiver is dropped, than nobody is interested in downloading this
                    // chapter at the moment, so we can shutdown
                    if let Some(sender) = &downloadings_sender {
                        if let Err(e) = sender.send(page_num).await {
                            tracing::debug!(
                                "got error {e}: downloadings receiver dropped, shutting down downloadings manager"
                            );

                            let command = DownloadingsSpawnerCommand::Shutdown;
                            spawner_command_sender.send(command).await.expect(
                                "downloadings_spawner shutdowned before the respectful command",
                            );

                            tracing::trace!("manager sent {command:#?} command to set");

                            break;
                        }
                    };

                    let (next_download_page, found_loading_pages) =
                        Self::determine_next_download_page(&mut statuses.lock(), opened_page);

                    match next_download_page {
                        Some(page) => {
                            let command =
                                DownloadingsSpawnerCommand::NewDownload { page_num: page };

                            spawner_command_sender.send(command).await.expect(
                                "downloadings_spawner shutdowned before downloading all pages",
                            );

                            tracing::trace!("manager sent {command:#?} command to set");
                        }
                        None => {
                            if !found_loading_pages {
                                let command = DownloadingsSpawnerCommand::Shutdown;
                                spawner_command_sender.send(command).await.expect(
                                    "join_set task shutdowned before the respectful command",
                                );

                                tracing::trace!("manager sent {command:#?} command to set");

                                break;
                            }
                        }
                    };
                }
            }
        }
        tracing::debug!("shutdowned");
    }

    pub(crate) async fn handle_downloading_page_error(
        error: Error,
        manager_command_sender: &mpsc::Sender<ManagerCommand>,
        page_num: usize,
    ) {
        tracing::warn!("got response from the server: {error:#?}");

        manager_command_sender
            .send(ManagerCommand::DownloadError { page_num })
            .await
            .expect("manager shutdowned before getting the signal from last downloading task");
    }

    /// Queries next chunk, handles possible errors and returnes this chunk if no errors
    /// encountered or None if errors were met or the source is drained along with the boolean
    /// value that represents if errores were emitted
    pub(crate) async fn get_next_chunk(
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
                Self::handle_downloading_page_error(
                    Error::ReqwestError(e),
                    manager_command_sender,
                    page_num,
                )
                .in_current_span()
                .await;

                let with_errors = true;
                (None, with_errors)
            }
        }
    }

    #[builder]
    #[tracing::instrument(
        skip(client, statuses, manager_command_sender),
        name = "downloading_page"
    )]
    pub(crate) async fn downloading_page(
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
                        let mut statuses = statuses.lock();
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
                    let mut statuses = statuses.lock();
                    statuses[page_num - 1] = PageStatus::Loaded(out_page_filename.into());
                }

                manager_command_sender
                    .send(ManagerCommand::DownloadedSuccessfully { page_num })
                    .await
                    .expect(
                        "manager shutdowned before getting the signal from last downloading task",
                    );
            }

            Err(e) => {
                Self::handle_downloading_page_error(e, &manager_command_sender, page_num)
                    .in_current_span()
                    .await;
            }
        }
    }

    #[builder]
    #[tracing::instrument(skip_all, name = "downloadings_spawner")]
    pub(crate) async fn downloadings_spawner(
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
                    // HACK: we don't care about the result of the computations, this is simply
                    // for the sake of buffer not overflowing
                    let _ = set.try_join_next();

                    let download_url = format!(
                        "{}/data/{}/{}",
                        &meta.base_url,
                        &meta.chapter.hash,
                        &meta.chapter.data[page_num - 1]
                    );

                    let downloading_page = Self::downloading_page()
                        .client(client.clone())
                        .chapter_hash(meta.chapter.hash.clone())
                        .page_num(page_num)
                        .url(download_url)
                        .statuses(Arc::clone(&statuses))
                        .manager_command_sender(manager_command_sender.clone())
                        .call();

                    set.spawn(downloading_page);

                    tracing::trace!("spawned new download task");
                }
            }
        }

        tracing::debug!("shutdowned");
    }

    /// Setups new [ChapterViewer] instance.
    /// The gist of how this works is as following.
    ///
    /// This function creates two working green threads:
    /// - downloadings_manager
    /// - downloadings_spawner
    ///
    /// These threads interact with each other via channels.
    /// Manager is responsible for deciding what to download next and an overall process status.
    /// Spawner is responsible for creating small green thread downloaders for each page.
    /// The maximum number of concurrently downloading pages is specified by `max_concurrent_downloads` .
    /// The function returnes an instance of [ChapterViewer] that can be used to interact with
    /// the manager task by means of the [`open_page`](ChapterViewer::open_page) function.
    /// Working threads shutdown when the [ChapterViewer] is dropped or when all pages are downloaded
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

        let (downloadings_spawner_command_sender, downloadings_spawner_command_receiver) =
            mpsc::channel(10);
        let downloadings_spawner_command_receiver =
            ReceiverStream::new(downloadings_spawner_command_receiver);

        let res = ChapterViewer {
            downloadings: downloading_receiver,
            statuses: buf,
            submit_switch: manager_command_sender.clone(),
        };

        let chapter_download_dir = format!("tmp/{}", download_meta.chapter.hash);
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
            .statuses(Arc::clone(&res.statuses))
            .spawner_command_sender(downloadings_spawner_command_sender.clone())
            .downloadings_sender(downloadings_sender)
            .command_receiver(manager_command_receiver)
            .max_concurrent_downloads(max_concurrent_downloads)
            .chapter_size(chapter_size)
            .call();

        task::spawn(manager);

        let spawner = Self::downloadings_spawner()
            .client(self.clone())
            .meta(download_meta)
            .command_receiver(downloadings_spawner_command_receiver)
            .manager_command_sender(manager_command_sender)
            .statuses(Arc::clone(&res.statuses))
            .call();

        task::spawn(spawner);

        return Ok(res);
    }
}
