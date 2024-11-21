This crate provides useful functions and structs for interacting with the <https://mangadex.org> API.
One of the most useful features: asyncrounous chapter downloader that changes its behaviour depending on the currently opened chapter page.

In order to fully understand how to use this library, please look through <https://api.mangadex.org/docs>. This is an official documentation of <https://mangadex.org> API.

## Example
```rust
use mango_api::requests::manga::{MangaFeedQuery, MangaQuery};
use mango_api::requests::query_utils::{Locale, Order, OrderOption};
use mango_api::MangoClient;

use std::collections::HashMap;

#[tokio::main]
async fn main() {  
    // Acquire client for interacting with the server
    let client = MangoClient::new().unwrap();

    // Get the id of the desired manga
    let chainsaw_manga_id = client
        .search_manga(&MangaQuery {
            title: Some("Chainsaw Man".to_string()),
            available_translated_language: Some(vec![Locale::En]),
            ..Default::default()
        })
        .await
        .unwrap()[0] // we pick the first search entry
        .id
        .clone();

    let mut query_sorting_options = HashMap::new();

    // We will sort chapters by its number in ascending order
    query_sorting_options.insert(OrderOption::Chapter, Order::Asc);

    // Build query for the manga feed.
    // We include only entries with english locale and sort using defined query_sorting_options
    let query_data = MangaFeedQuery::builder()
        .translated_language(vec![Locale::En])
        .order(query_sorting_options)
        .build();

    // Execute query
    let chapters = client
        .get_manga_feed(&chainsaw_manga_id, &query_data)
        .await
        .unwrap();

    // We pick the third entry
    let first_chapter = chapters[2].clone();

    // Create chapter viewer that will only have 8 concurrently downloading pages
    // at maximum 
    let mut viewer = client.chapter_viewer(&first_chapter.id, 8).await.unwrap();

    let chapter_len = first_chapter.attributes.pages;

    // For each page get the path at which it was stored
    let mut page_paths = Vec::new();
    for i in 0..chapter_len {
        page_paths.push(viewer.open_page(i + 1).await);
    }

    println!("{page_paths:#?}");
}
```
### Special thanks
Big thanks to the <https://mangadex.org> dev team for providing such a useful API.
