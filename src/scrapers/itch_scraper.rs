use crate::parsers::itch_parser::{parse_itch_game_page_data, ItchRating, Link, MoreInfoTableData};
use anyhow::Result;
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::Client;

#[derive(Default, Debug, serde::Serialize)]
pub struct ItchData {
    link: String,
    title: String,
    plain_title: String,
    price: String,
    description: String,
    pub_date: String,
    create_date: String,
    update_date: String,
    rating: ItchRating,
    author: String,
    genre: String,
    made_with: String,
    tags: Vec<String>,
    average_session: String,
    languages: Vec<String>,
    inputs: Vec<String>,
    links: Vec<Link>,
    status: String,
    platforms: Vec<String>,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
struct Item {
    guid: String,
    title: String,
    #[serde(rename = "plainTitle")]
    plain_title: String,
    link: String,
    price: String,
    description: String,
    #[serde(rename = "pubDate")]
    pub_date: String,
    #[serde(rename = "createDate")]
    create_date: String,
    #[serde(rename = "updateDate")]
    update_date: String,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
struct Channel {
    #[serde(rename = "item")]
    items: Vec<Item>,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
struct Rss {
    channel: Channel,
}

pub async fn scrape_itch_rss_feed(url: String, page_limit: i32) -> Result<Vec<ItchData>> {
    let urls: Vec<String> = (0..page_limit)
        .map(|i| url.clone() + "?page=" + &i.to_string())
        .collect();

    let client = Client::new();

    let futures = stream::iter(urls)
        .map(|url| fetch_url(&client, url))
        .buffer_unordered(50)
        .then(|result| async {
            let client = client.clone();
            match result {
                Ok(body) => process_rss_feed(&client, body).await,
                Err(err) => Err(err.into()),
            }
        });

    let itch_data: Result<Vec<Vec<ItchData>>, _> = futures.try_collect().await;
    match itch_data {
        Ok(data) => Ok(data.into_iter().flatten().collect()),
        Err(err) => Err(err.into()),
    }
}

async fn process_rss_feed(client: &Client, body: String) -> Result<Vec<ItchData>> {
    if let Ok(feed) = quick_xml::de::from_str::<Rss>(&body) {
        let futures = stream::iter(feed.channel.items)
            .map(|item| get_item_data(client, item))
            .buffer_unordered(50);

        let output: Vec<ItchData> = futures
            .try_fold(Vec::new(), |mut acc, (result, item)| async move {
                if let Ok(data) = parse_itch_game_page_data(&result) {
                    acc.push(combine_itch_rss_and_info_data(data, item));
                }
                Ok(acc)
            })
            .await?;

        Ok(output)
    } else {
        Ok(Vec::new())
    }
}

fn combine_itch_rss_and_info_data(table_data: MoreInfoTableData, rss_data: Item) -> ItchData {
    ItchData {
        average_session: table_data.average_session,
        update_date: rss_data.update_date,
        plain_title: rss_data.plain_title,
        link: rss_data.link,
        platforms: table_data.platforms,
        create_date: rss_data.create_date,
        description: rss_data.description,
        languages: table_data.languages,
        made_with: table_data.made_with,
        pub_date: rss_data.pub_date,
        inputs: table_data.inputs,
        author: table_data.author,
        rating: table_data.rating,
        links: table_data.links,
        genre: table_data.genre,
        price: rss_data.price,
        title: rss_data.title,
        status: table_data.status,
        tags: table_data.tags,
    }
}

async fn get_item_data(client: &Client, item: Item) -> Result<(String, Item)> {
    let data = fetch_url(client, item.link.clone()).await?;

    Ok((data, item))
}

async fn fetch_url(client: &Client, url: String) -> Result<String, reqwest::Error> {
    let response = client.get(url).send().await?;
    let body = response.text().await?;
    Ok(body)
}
