use crate::parsers::itch_game_info_parser::{
    parse_itch_game_page_data, ItchRating, Link, MoreInfoTableData,
};
use anyhow::Result;
use reqwest::{Client, StatusCode};
use tokio::time::{sleep, Duration};

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
    author: Vec<String>,
    genre: Vec<String>,
    made_with: Vec<String>,
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

pub async fn scrape_itch_rss_feed(
    url: String,
    max_retries: u32,
    page_limit: i32,
) -> Result<Vec<ItchData>> {
    let client = Client::new();

    let mut itch_data_output = Vec::new();
    for page in 1..=page_limit {
        let rss_url = format!("{}?page={}", url, page);
        let rss_string = fetch_url(&client, &rss_url, max_retries).await?;

        match quick_xml::de::from_str::<Rss>(&rss_string) {
            Ok(feed) => {
                for item in feed.channel.items {
                    let game_data = fetch_url(&client, &item.link, max_retries).await?;
                    match parse_itch_game_page_data(&game_data) {
                        Ok(data) => {
                            itch_data_output.push(combine_itch_rss_and_info_data(data, item))
                        }
                        Err(err) => eprintln!("Error parsing Itch Game Page HTML: {:?}", err),
                    }
                }
            }
            Err(err) => {
                eprintln!("Error parsing RSS xml: {:?}", err);
            }
        }
    }

    Ok(itch_data_output)
}

async fn fetch_url(client: &Client, url: &str, max_retries: u32) -> Result<String, reqwest::Error> {
    let mut retries = 0;
    let mut delay = 1;

    loop {
        let response = client.get(url).send().await;

        match response {
            Ok(res) => match res.status() {
                StatusCode::OK => return Ok(res.text().await?),
                StatusCode::TOO_MANY_REQUESTS => {
                    if retries >= max_retries {
                        return Err(res.error_for_status().unwrap_err());
                    }

                    eprintln!(
                        "Rate limited while fetching {}. Retrying in {} seconds... (attempt {}/{})",
                        url,
                        delay,
                        retries + 1,
                        max_retries
                    );

                    sleep(Duration::from_secs(delay)).await;
                    delay = std::cmp::min(300, delay * 2);
                    retries += 1;
                }
                _ => return Err(res.error_for_status().unwrap_err()),
            },
            Err(err) => {
                if retries >= max_retries {
                    return Err(err);
                }

                eprintln!(
                    "Error sending request to {}. Retrying in {} seconds... (attempt {}/{})",
                    url,
                    delay,
                    retries + 1,
                    max_retries
                );

                sleep(Duration::from_secs(delay)).await;
                delay = std::cmp::min(300, delay * 2);
                retries += 1;
            }
        }
    }
}

fn combine_itch_rss_and_info_data(table_data: MoreInfoTableData, rss_data: Item) -> ItchData {
    ItchData {
        update_date: rss_data.update_date,
        create_date: rss_data.create_date,
        plain_title: rss_data.plain_title,
        link: rss_data.link,
        description: rss_data.description,
        pub_date: rss_data.pub_date,
        price: rss_data.price,
        title: rss_data.title,
        average_session: table_data.average_session,
        platforms: table_data.platforms,
        languages: table_data.languages,
        made_with: table_data.made_with,
        inputs: table_data.inputs,
        author: table_data.author,
        rating: table_data.rating,
        links: table_data.links,
        genre: table_data.genre,
        status: table_data.status,
        tags: table_data.tags,
    }
}
