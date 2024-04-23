use clap::Parser;
use std::path::PathBuf;
use std::{fs, io};

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[arg(short, long, value_name = "FILE PATH")]
    infile: Option<PathBuf>,

    #[arg(short, long, value_name = "FILE PATH")]
    outfile: Option<PathBuf>,
}

enum Mode {
    Stream,
    File,
}

fn main() -> io::Result<()> {
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

    println!("{buffer}");

    let output_mode: Mode = match args.outfile {
        Some(_) => Mode::File,
        None => Mode::Stream,
    };

    Ok(())
}
