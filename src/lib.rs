// TODO: add timeout to open_page
// TODO: test error handling
// TODO: test the program with model when instead sharing downloadings buffer across tasks,
// this buffer is only held by manager and open_page only gets page information after submitting
// request to manager

pub mod requests;
pub mod viewer;

#[cfg(test)]
mod tests {
    use super::*;
    use requests::{manga::*, query_utils::*, *};

    use tracing_subscriber::filter::EnvFilter;
    use tracing_subscriber::filter::LevelFilter;
    use tracing_subscriber::layer::SubscriberExt as _;
    use tracing_subscriber::prelude::*;

    use std::collections::HashMap;

    use tokio::io::AsyncWriteExt as _;

    #[tokio::test]
    async fn test_search_manga() {
        let client = MangoClient::new().unwrap();
        let query = MangaQuery::builder()
            .title("Chainsaw man")
            // .status(vec![MangaStatus::Ongoing])
            // .original_language(vec![Locale::En])
            .build();
        let resp = client.search_manga(&query).await.unwrap();

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

        let query = MangaQuery::builder()
            .title("Chainsaw Man")
            .available_translated_language(vec![Locale::En])
            .build();

        let chainsaw_manga_id = client.search_manga(&query).await.unwrap()[0].id.clone();

        let mut query_sorting_options = HashMap::new();

        query_sorting_options.insert(OrderOption::Chapter, Order::Asc);

        let query_data = MangaFeedQuery::builder()
            .translated_language(vec![Locale::En])
            .order(query_sorting_options)
            .build();

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

        let query = MangaQuery::builder()
            .title("Chainsaw man")
            .available_translated_language(vec![Locale::En])
            .build();

        let chainsaw_manga_id = client.search_manga(&query).await.unwrap()[0].id.clone();

        let mut query_sorting_options = HashMap::new();
        query_sorting_options.insert(OrderOption::Chapter, Order::Asc);

        let query_data = MangaFeedQuery::builder()
            .translated_language(vec![Locale::En])
            .order(query_sorting_options)
            .build();

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

        let query = MangaQuery::builder()
            .title("Chainsaw man")
            .available_translated_language(vec![Locale::En])
            .build();

        let chainsaw_manga_id = client.search_manga(&query).await.unwrap()[0].id.clone();

        let mut query_sorting_options = HashMap::new();

        query_sorting_options.insert(OrderOption::Chapter, Order::Asc);

        let query_data = MangaFeedQuery::builder()
            .translated_language(vec![Locale::En])
            .order(query_sorting_options)
            .build();

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

        let query = MangaQuery::builder()
            .title("Chainsaw man")
            .available_translated_language(vec![Locale::En])
            .build();

        let chainsaw_manga_id = client.search_manga(&query).await.unwrap()[0].id.clone();

        let mut query_sorting_options = HashMap::new();

        query_sorting_options.insert(OrderOption::Chapter, Order::Asc);

        let query_data = MangaFeedQuery::builder()
            .translated_language(vec![Locale::En])
            .order(query_sorting_options)
            .limit(200)
            .offset(1)
            .excluded_groups(vec![
                "4f1de6a2-f0c5-4ac5-bce5-02c7dbb67deb".to_owned(),
                "a38fc704-90ab-452f-9336-59d84997a9ce".to_owned(),
            ])
            .build();

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

        let (_writer, _guard) = tracing_appender::non_blocking(
            std::fs::File::options().append(true).open("logs").unwrap(),
        );

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_test_writer()
                    // .with_writer(_writer)
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

        let query_data = MangaFeedQuery::builder()
            .translated_language(vec![Locale::En])
            .order(query_sorting_options)
            .build();

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

        let query = MangaQuery::builder()
            .title("Chainsaw man")
            .available_translated_language(vec![Locale::En])
            .build();

        let chainsaw_manga_id = client.search_manga(&query).await.unwrap()[0].id.clone();

        let mut query_sorting_options = HashMap::new();

        query_sorting_options.insert(OrderOption::Chapter, Order::Asc);

        let query_data = MangaFeedQuery::builder()
            .translated_language(vec![Locale::En])
            .order(query_sorting_options)
            .build();

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
        let query = MangaQuery::builder()
            .title("Chainsaw man")
            // .status(vec![MangaStatus::Ongoing])
            // .original_language(vec![Locale::En])
            .build();

        let resp = client.search_manga_include_cover(&query).await.unwrap();

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

        let query = MangaQuery::builder()
            .title("Chainsaw man")
            // .status(vec![MangaStatus::Ongoing])
            // .original_language(vec![Locale::En])
            .build();

        let resp = client.search_manga_with_cover(&query).await.unwrap();

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
