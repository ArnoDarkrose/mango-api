use crate::requests::Result;
use crate::requests::{chapter::ChapterDownloadMeta, MangoClient};

use std::path::PathBuf;
use std::sync::{atomic::AtomicUsize, Arc, Mutex};

use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt as _;

#[derive(Debug, Clone)]
pub(crate) enum PageStatus {
    Loaded(PathBuf),
    Loading,
    Idle,
}

#[derive(Debug, Clone)]
pub(crate) enum ManagerCommand {
    SwitchPage { page_num: usize },
    DownloadedSuccessfully { page_num: usize },
    DownloadError { page_num: usize },
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SetCommand {
    Shutdown,
    NewDownload { page_num: usize },
}

// TODO: change Mutex to Arc<Mutex>, remove client
#[derive(Debug)]
pub struct ChapterViewer {
    pub(crate) opened_page: Arc<AtomicUsize>,
    pub(crate) statuses: Arc<Mutex<Vec<PageStatus>>>,
    pub(crate) meta: ChapterDownloadMeta,
    pub(crate) client: MangoClient,
    pub(crate) downloadings: ReceiverStream<usize>,
    pub(crate) submit_switch: Sender<ManagerCommand>,
}

impl ChapterViewer {
    pub async fn get_page(&mut self, page_num: usize) -> PathBuf {
        let statuses = self.statuses.lock().expect("mutex poisoned");
        let status = statuses[page_num - 1].clone();

        drop(statuses);

        let sender = self.submit_switch.clone();

        task::spawn(async move {
            // NOTE: if the receiver is dropped, then all pages are downloaded and we don't care in
            // delivering this message
            let _ = sender.send(ManagerCommand::SwitchPage { page_num }).await;
        });

        let res = match status {
            PageStatus::Loaded(path) => path,
            _ => {
                let mut res = None;
                while let Some(downloaded_page_num) = self.downloadings.next().await {
                    if downloaded_page_num == page_num {
                        let statuses = self.statuses.lock().expect("mutex poisoned");
                        let status = statuses[page_num - 1].clone();

                        drop(statuses);

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
        };

        res
    }
}
