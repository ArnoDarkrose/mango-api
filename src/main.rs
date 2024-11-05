use reqwest::Client;
use std::collections::HashMap;
use std::io::prelude::*;

#[tokio::main]
async fn main() -> reqwest::Result<()> {
    let base_url = "https://api.mangadex.org";

    let client = Client::builder().user_agent("Mango/1.0").build()?;

    let mut params = HashMap::new();
    params.insert("title", "Kanojyo to Himitsu to Koimoyou");

    let body = client
        .get(format!("{base_url}/manga"))
        .query(&params)
        .send()
        .await?
        .text()
        .await?;

    let info: serde_json::Value = match serde_json::from_str(&body) {
        Ok(val) => val,
        Err(_) => panic!("Deserialization error"),
    };

    let results = info.get("data").unwrap();

    let mut full_res = std::fs::File::create("full_res").unwrap();

    full_res.write(format!("{results:#?}").as_bytes()).unwrap();

    let results = results.as_array().unwrap();

    let mut entries = Vec::new();

    for result in results {
        let cur_id = result.get("id").unwrap();
        let cur_title = result.get("attributes").unwrap();
        let cur_title = cur_title.get("title").unwrap();
        let cur_title = cur_title.get("en").unwrap();
        entries.push((cur_id.as_str().unwrap(), cur_title.as_str().unwrap()));
    }

    let (needed_id, _) = entries[2];

    let chapters = client
        .get(format!("{base_url}/manga/{needed_id}/feed"))
        .send()
        .await?
        .text()
        .await?;

    let chapters: serde_json::Value = serde_json::from_str(&chapters).unwrap();
    let chapters = chapters.get("data").unwrap().as_array().unwrap();

    let mut chapter_id = None;
    for chapter in chapters {
        let num = chapter
            .get("attributes")
            .unwrap()
            .get("chapter")
            .unwrap()
            .as_str()
            .unwrap();

        if num.parse::<f32>().unwrap() == 1.0 {
            chapter_id = Some(chapter.get("id").unwrap().as_str().unwrap());

            break;
        }
    }

    let Some(chapter_id) = chapter_id else {
        panic!("No first chapter")
    };

    let chapter_meta = client
        .get(format!("{base_url}/at-home/server/{chapter_id}"))
        .send()
        .await?
        .text()
        .await?;

    let chapter_meta: serde_json::Value = serde_json::from_str(&chapter_meta).unwrap();

    let download_url = chapter_meta.get("baseUrl").unwrap().as_str().unwrap();
    let chapter_meta = chapter_meta.get("chapter").unwrap();
    let hash = chapter_meta.get("hash").unwrap().as_str().unwrap();
    let data = chapter_meta.get("data").unwrap().as_array().unwrap();

    let mut pages = Vec::new();
    for page in data {
        let page_hash = page.as_str().unwrap();

        let cur_url = format!("{download_url}/data/{hash}/{page_hash}");
        let cur_page = client.get(cur_url).send().await?.bytes().await?;

        pages.push(cur_page);
    }

    let mut out = std::fs::File::create("out").unwrap();

    for entry in entries {
        let (id, title) = entry;
        out.write(format!("id: {id}, title: {title}\n").as_bytes())
            .unwrap();
    }

    out.write("\n".as_bytes()).unwrap();
    out.write(format!("{chapters:#?}").as_bytes()).unwrap();

    if !std::fs::exists("pages").unwrap() {
        std::fs::create_dir("pages").unwrap();
    }

    for i in 0..pages.len() {
        let mut page_out = std::fs::File::create(format!("pages/page{}.jpg", i + 1)).unwrap();

        page_out.write(pages[i].as_ref()).unwrap();
    }

    Ok(())
}
