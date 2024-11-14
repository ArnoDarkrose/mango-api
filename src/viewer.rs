use crate::requests::chapter::ChapterDownloadMeta;
use crate::requests::{Error, MangoClient, Result};

use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use tokio::io::AsyncWriteExt as _;
use tokio::sync::mpsc::{self, Sender};
use tokio::task::{self, JoinSet};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt as _;

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

#[derive(Debug)]
pub struct ChapterViewer {
    pub(crate) opened_page: Arc<AtomicUsize>,
    pub(crate) statuses: Arc<Mutex<Vec<PageStatus>>>,
    pub(crate) meta: ChapterDownloadMeta,
    pub(crate) downloadings: ReceiverStream<usize>,
    pub(crate) submit_switch: Sender<ManagerCommand>,
}

impl ChapterViewer {
    #[tracing::instrument(skip(self))]
    pub async fn open_page(&mut self, page_num: usize) -> PathBuf {
        tracing::trace!("entered");

        tracing::trace!("acquiring mutex lock");

        let status = {
            let statuses = self.statuses.lock().expect("mutex poisoned");
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
                            let statuses = self.statuses.lock().expect("mutex poisoned");
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

impl MangoClient {
    pub(crate) fn determine_next_download_page(
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

    #[tracing::instrument(skip(
        statuses,
        set_command_sender,
        downloadings_sender,
        command_receiver
    ))]
    pub(crate) async fn downloadings_manager(
        opened_page: Arc<AtomicUsize>,
        statuses: Arc<Mutex<Vec<PageStatus>>>,
        set_command_sender: mpsc::Sender<DownloadingsSpawnerCommand>,
        downloadings_sender: Option<mpsc::Sender<usize>>,
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
                    if let Some(sender) = &downloadings_sender {
                        let _ = sender.send(page_num).await;
                    };

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

    pub(crate) async fn handle_downloading_page_request_error(
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
                Self::handle_downloading_page_request_error(e, manager_command_sender, page_num)
                    .in_current_span()
                    .await;

                let with_errors = true;
                (None, with_errors)
            }
        }
    }

    #[tracing::instrument(skip(client, statuses, manager_command_sender))]
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

        let (downloadings_spawner_command_sender, downloadings_spawner_command_receiver) =
            mpsc::channel(10);
        let downloadings_spawner_command_receiver =
            ReceiverStream::new(downloadings_spawner_command_receiver);

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
            downloadings_spawner_command_sender.clone(),
            Some(downloadings_sender),
            manager_command_receiver,
            max_concurrent_downloads,
            chapter_size,
        ));

        task::spawn(Self::downloadings_spawner(
            self.clone(),
            res.meta.clone(),
            downloadings_spawner_command_receiver,
            manager_command_sender,
            Arc::clone(&res.statuses),
        ));

        return Ok(res);
    }
}
