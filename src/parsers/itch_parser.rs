use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use scraper::{ElementRef, Html, Selector};
use thiserror::Error;

#[derive(Default, Debug)]
struct ItchData {
    last_update_date: DateTime<Utc>,
    publish_date: DateTime<Utc>,
    status: String,
    platforms: Vec<String>,
    rating: ItchRating,
    author: String,
    genre: String,
    made_with: String,
    tags: Vec<String>,
    average_session: String,
    languages: Vec<String>,
    inputs: Vec<String>,
    links: Vec<Link>,
}

#[derive(Default, Debug)]
struct Link {
    name: String,
    url: String,
}

#[derive(Default, Debug)]
struct ItchRating {
    score: f32,
    count: i32,
}

#[derive(Error, Debug)]
pub enum ItchHTMLDataFormatError {
    #[error("Unknown data type found in Itch.io TD element: {data:?})")]
    UnknownDataType { data: String },

    #[error("Unable to locate TD elements while parsing itch HTML data")]
    MissingElements,

    #[error("Attempted to locate data within accompanying data element to Itch.io TD element {data_type:?} and failed to find data")]
    MissingData { data_type: ItchTableData },

    #[error("Invalid data format found for type {data_type:?}, found: {found:?}")]
    InvalidData {
        data_type: ItchTableData,
        found: String,
    },
}

pub fn parse_itch_data(raw_html: &str) -> Result<String, ItchHTMLDataFormatError> {
    let document = Html::parse_document(raw_html);
    let tr_selector = Selector::parse("div.game_info_panel_widget table tbody tr").unwrap();
    let td_selector = Selector::parse("td").unwrap();

    let mut itch_data = ItchData::default();

    for tr in document.select(&tr_selector) {
        let tds: Vec<ElementRef<'_>> = tr.select(&td_selector).collect();
        if tds.len() != 2 {
            return Err(ItchHTMLDataFormatError::MissingElements);
        }

        let data_type = parse_row_data_type(tds[0])?;
        let data = tds[1];

        match data_type {
            ItchTableData::UpdatedDate => {
                itch_data.last_update_date = parse_date_element(data, data_type)?
            }
            ItchTableData::PublishDate => {
                itch_data.publish_date = parse_date_element(data, data_type)?
            }
            ItchTableData::Status => {
                itch_data.status = data.text().collect::<String>().trim().to_owned()
            }
            ItchTableData::Platforms => {
                itch_data.platforms = parse_anchor_separated_strings(data);
            }
            ItchTableData::Rating => itch_data.rating = parse_rating_element(data, data_type)?,
            ItchTableData::Author => {
                itch_data.author = data.text().collect::<String>().trim().to_owned()
            }
            ItchTableData::Genre => {
                itch_data.genre = data.text().collect::<String>().trim().to_owned()
            }
            ItchTableData::MadeWith => {
                itch_data.made_with = data.text().collect::<String>().trim().to_owned()
            }
            ItchTableData::Tags => itch_data.tags = parse_anchor_separated_strings(data),
            ItchTableData::AverageSession => {
                itch_data.average_session = data.text().collect::<String>().trim().to_owned()
            }
            ItchTableData::Languages => {
                itch_data.languages = parse_anchor_separated_strings(data);
            }
            ItchTableData::Inputs => {
                itch_data.inputs = parse_anchor_separated_strings(data);
            }
            ItchTableData::Links => {
                itch_data.links = parse_links(data)?;
            }
        }
    }

    println!("{:?}", itch_data);

    Ok(raw_html.to_string())
}

#[derive(Debug)]
pub enum ItchTableData {
    UpdatedDate,
    PublishDate,
    Status,
    Platforms,
    Rating,
    Author,
    Genre,
    MadeWith,
    Tags,
    AverageSession,
    Languages,
    Inputs,
    Links,
}

impl ItchTableData {
    fn from_str(s: &str) -> Option<ItchTableData> {
        match s {
            "Updated" => Some(ItchTableData::UpdatedDate),
            "Published" => Some(ItchTableData::PublishDate),
            "Status" => Some(ItchTableData::Status),
            "Platforms" => Some(ItchTableData::Platforms),
            "Rating" => Some(ItchTableData::Rating),
            "Author" => Some(ItchTableData::Author),
            "Genre" => Some(ItchTableData::Genre),
            "Made with" => Some(ItchTableData::MadeWith),
            "Tags" => Some(ItchTableData::Tags),
            "Average session" => Some(ItchTableData::AverageSession),
            "Languages" => Some(ItchTableData::Languages),
            "Inputs" => Some(ItchTableData::Inputs),
            "Links" => Some(ItchTableData::Links),
            _ => None,
        }
    }
}

fn parse_row_data_type(el: ElementRef) -> Result<ItchTableData, ItchHTMLDataFormatError> {
    let inner_html = el.inner_html();

    if let Some(table_data) = ItchTableData::from_str(&inner_html) {
        Ok(table_data)
    } else {
        Err(ItchHTMLDataFormatError::UnknownDataType { data: inner_html }.into())
    }
}

fn parse_date_element(
    el: ElementRef,
    data_type: ItchTableData,
) -> Result<DateTime<Utc>, ItchHTMLDataFormatError> {
    let selector = Selector::parse("abbr").unwrap();

    match el.select(&selector).next() {
        Some(abbr) => {
            if let Some(title) = abbr.value().attr("title") {
                if let Ok(date) = NaiveDateTime::parse_from_str(title, "%d %B %Y @ %H:%M UTC") {
                    Ok(date.and_utc())
                } else {
                    Err(ItchHTMLDataFormatError::InvalidData {
                        data_type,
                        found: title.to_string(),
                    }
                    .into())
                }
            } else {
                Err(ItchHTMLDataFormatError::MissingData { data_type }.into())
            }
        }
        None => Err(ItchHTMLDataFormatError::MissingData { data_type }.into()),
    }
}

fn parse_rating_element(
    el: ElementRef,
    data_type: ItchTableData,
) -> Result<ItchRating, ItchHTMLDataFormatError> {
    let value_selector = Selector::parse(r#"div[itemprop="ratingValue"]"#).unwrap();
    let count_selector = Selector::parse(r#"span[itemprop="ratingCount"]"#).unwrap();

    let mut rating = ItchRating::default();

    match el.select(&value_selector).next() {
        Some(div) => {
            if let Some(score_str) = div.value().attr("content") {
                if let Ok(score) = score_str.parse() {
                    rating.score = score;
                } else {
                    return Err(ItchHTMLDataFormatError::InvalidData {
                        data_type,
                        found: score_str.to_string(),
                    }
                    .into());
                }
            } else {
                return Err(ItchHTMLDataFormatError::MissingData { data_type }.into());
            }
        }
        None => return Err(ItchHTMLDataFormatError::MissingData { data_type }.into()),
    }

    match el.select(&count_selector).next() {
        Some(span) => {
            if let Some(rating_count) = span.value().attr("content") {
                if let Ok(count) = rating_count.parse() {
                    rating.count = count;
                } else {
                    return Err(ItchHTMLDataFormatError::InvalidData {
                        data_type,
                        found: rating_count.to_string(),
                    }
                    .into());
                }
            } else {
                return Err(ItchHTMLDataFormatError::MissingData { data_type }.into());
            }
        }
        None => return Err(ItchHTMLDataFormatError::MissingData { data_type }.into()),
    }

    Ok(rating)
}

fn parse_anchor_separated_strings(el: ElementRef) -> Vec<String> {
    el.text()
        .flat_map(|s| s.split("\n"))
        .map(|s| s.trim().to_string())
        .filter(|s| s != "," && s.len() > 0)
        .collect()
}

fn parse_links(el: ElementRef) -> Result<Vec<Link>, ItchHTMLDataFormatError> {
    let anchor_selector = Selector::parse("a").unwrap();

    let mut links: Vec<Link> = Vec::new();

    for anchor in el.select(&anchor_selector) {
        if let Some(href) = anchor.value().attr("href") {
            let name: String = anchor.text().collect();
            links.push(Link {
                name,
                url: href.to_string(),
            });
        } else {
            return Err(ItchHTMLDataFormatError::MissingData {
                data_type: ItchTableData::Links,
            }
            .into());
        }
    }

    Ok(links)
}
