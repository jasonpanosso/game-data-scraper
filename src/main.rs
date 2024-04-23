use anyhow::Result;
use clap::{Parser, ValueEnum};
use parsers::itch_parser::parse_itch_data;
use std::path::PathBuf;
use std::{fs, io};

mod parsers;
mod scrapers;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[arg(short, long, value_name = "FILE PATH")]
    infile: Option<PathBuf>,

    #[arg(short, long, value_name = "FILE PATH")]
    outfile: Option<PathBuf>,

    #[arg(short, long, value_enum, value_name = "SITE NAME")]
    site: Site,
}

#[derive(Debug, ValueEnum, Clone)]
enum Site {
    Itch,
}

enum Mode {
    Stream,
    File,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut buffer = String::new();
    match args.infile {
        Some(path) => {
            buffer = fs::read_to_string(path)?;
        }
        None => {
            io::stdin().read_line(&mut buffer)?;
        }
    };

    assert!(buffer.len() > 0);

    let parsed = match args.site {
        Site::Itch => parse_itch_data(&buffer),
    }?;

    let output_mode: Mode = match args.outfile {
        Some(_) => Mode::File,
        None => Mode::Stream,
    };

    Ok(())
}
