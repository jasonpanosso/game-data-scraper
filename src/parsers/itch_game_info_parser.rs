use anyhow::Result;
use scraper::{ElementRef, Html, Selector};
use serde::Serialize;
use thiserror::Error;

#[derive(Default, Debug, Serialize)]
pub struct MoreInfoTableData {
    pub status: String,
    pub release_date: String,
    pub platforms: Vec<String>,
    pub rating: ItchRating,
    pub authors: Vec<String>,
    pub genres: Vec<String>,
    pub made_with: Vec<String>,
    pub tags: Vec<String>,
    pub average_session: String,
    pub languages: Vec<String>,
    pub inputs: Vec<String>,
    pub links: Vec<Link>,
    pub accessibility: Vec<String>,
}

#[derive(Default, Debug, Serialize)]
pub struct Link {
    pub name: String,
    pub url: String,
}

#[derive(Default, Debug, Serialize)]
pub struct ItchRating {
    pub score: f32,
    pub count: i32,
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

#[derive(Debug)]
pub enum ItchTableData {
    ReleaseDate,
    Status,
    Platforms,
    Rating,
    Authors,
    Genres,
    MadeWith,
    Tags,
    AverageSession,
    Languages,
    Inputs,
    Links,
    Accessibility,
}

impl ItchTableData {
    fn from_str(s: &str) -> Option<ItchTableData> {
        match s {
            "Status" => Some(ItchTableData::Status),
            "Release date" => Some(ItchTableData::ReleaseDate),
            "Accessibility" => Some(ItchTableData::Accessibility),
            "Platforms" => Some(ItchTableData::Platforms),
            "Rating" => Some(ItchTableData::Rating),
            "Author" => Some(ItchTableData::Authors),
            "Authors" => Some(ItchTableData::Authors),
            "Genre" => Some(ItchTableData::Genres),
            "Genres" => Some(ItchTableData::Genres),
            "Made with" => Some(ItchTableData::MadeWith),
            "Tag" => Some(ItchTableData::Tags),
            "Tags" => Some(ItchTableData::Tags),
            "Average session" => Some(ItchTableData::AverageSession),
            "Languages" => Some(ItchTableData::Languages),
            "Language" => Some(ItchTableData::Languages),
            "Inputs" => Some(ItchTableData::Inputs),
            "Links" => Some(ItchTableData::Links),
            _ => None,
        }
    }
}

pub fn parse_itch_game_page_data(
    raw_html: &str,
) -> Result<MoreInfoTableData, ItchHTMLDataFormatError> {
    let document = Html::parse_document(raw_html);
    let tr_selector = Selector::parse("div.game_info_panel_widget table tbody tr").unwrap();
    let td_selector = Selector::parse("td").unwrap();

    let mut itch_data = MoreInfoTableData::default();

    for tr in document.select(&tr_selector) {
        let tds: Vec<ElementRef> = tr.select(&td_selector).collect();
        if tds.len() != 2 {
            return Err(ItchHTMLDataFormatError::MissingElements);
        }

        let data_type = match parse_row_data_type(tds[0]) {
            Ok(data) => data,
            Err(_) => continue,
        };
        let data = tds[1];

        match data_type {
            ItchTableData::ReleaseDate => {
                itch_data.release_date = data.text().collect::<String>().trim().to_owned()
            }
            ItchTableData::Status => {
                itch_data.status = data.text().collect::<String>().trim().to_owned()
            }
            ItchTableData::Accessibility => {
                itch_data.accessibility = parse_anchor_separated_strings(data)
            }
            ItchTableData::Platforms => {
                itch_data.platforms = parse_anchor_separated_strings(data);
            }
            ItchTableData::Rating => itch_data.rating = parse_rating_element(data, data_type)?,
            ItchTableData::Authors => itch_data.authors = parse_anchor_separated_strings(data),
            ItchTableData::Genres => itch_data.genres = parse_anchor_separated_strings(data),
            ItchTableData::MadeWith => itch_data.made_with = parse_anchor_separated_strings(data),
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

    Ok(itch_data)
}

fn parse_row_data_type(el: ElementRef) -> Result<ItchTableData, ItchHTMLDataFormatError> {
    let inner_html = el.inner_html();

    if let Some(table_data) = ItchTableData::from_str(&inner_html) {
        Ok(table_data)
    } else {
        Err(ItchHTMLDataFormatError::UnknownDataType { data: inner_html }.into())
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
