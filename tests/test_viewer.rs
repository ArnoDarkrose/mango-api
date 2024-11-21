use mango_api::requests::manga::{MangaFeedQuery, MangaQuery};
use mango_api::requests::query_utils::{Locale, Order, OrderOption};
use mango_api::MangoClient;

use std::collections::HashMap;

#[tokio::test]
async fn test_viewer_integrated() {
    // Acquire client for intercating with the server
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

    println!("{page_paths:#?}");
}
