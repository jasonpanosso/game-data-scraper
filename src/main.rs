use crate::scrapers::itch_rss_scraper::scrape_itch_rss_feed;
use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::path::PathBuf;
use std::{fs, io, io::Write};

mod parsers;
mod scrapers;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[arg(short, long, value_enum, value_name = "SITE")]
    site: Site,

    #[arg(short, long, value_enum, value_name = "URL")]
    url: String,

    #[arg(short, long, value_name = "FILE PATH")]
    outfile: Option<PathBuf>,

    #[arg(short, long, value_name = "INTEGER")]
    page_limit: Option<i32>,

    #[arg(short, long, value_name = "INTEGER")]
    max_retries: Option<u32>,
}

#[derive(Debug, ValueEnum, Clone)]
enum Site {
    Itch,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let page_limit = args.page_limit.unwrap_or(300);
    let max_retries = args.max_retries.unwrap_or(20);

    let rt = tokio::runtime::Runtime::new()?;
    let json = match args.site {
        Site::Itch => {
            let itch_data = rt.block_on(scrape_itch_rss_feed(args.url, max_retries, page_limit))?;
            serde_json::to_string(&itch_data)?
        }
    };

    match args.outfile {
        Some(file) => {
            fs::write(file, json)?;
        }
        None => {
            io::stdout().write_all(json.as_bytes())?;
        }
    }

    Ok(())
}
