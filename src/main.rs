#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]
#![allow(clippy::multiple_crate_versions)]

mod converter;
mod serializer;
mod utils;

use indicatif::{ProgressBar, ProgressFinish, ProgressStyle};

#[allow(non_snake_case)]
mod generated_chess {
    #![allow(warnings)]
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/chess.rs"));
    pub use chess::*;
}

use crate::converter::Converter;
use crate::serializer::Serializer;
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::borrow::Cow;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

#[derive(Parser)]
#[command(name = "chessb")]
#[command(about = "A chess binary format library and utility")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert PGN files to chess binary format
    Convert {
        /// Input PGN file (supports .zst compression)
        input: String,
        /// Output file (defaults to input filename with .cbin extension)
        #[arg(short, long)]
        output: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Convert { input, output } => {
            let output_file = output.unwrap_or_else(|| generate_default_output_filename(&input));
            convert_file(&input, &output_file)
        }
    }
}

fn convert_file(input_file: &str, output_file: &str) -> Result<()> {
    println!("Reading from {input_file}");
    println!("Writing to {output_file}");

    let file = File::open(input_file)?;
    let len = file.metadata()?.len();
    let buf_reader = BufReader::new(file);
    let progress_wrapped = ProgressBar::new(len)
        .wrap_read(buf_reader)
        .with_message("Reading and converting...")
        .with_finish(ProgressFinish::WithMessage(Cow::from(
            "Finished converting file!",
        )))
        .with_style(ProgressStyle::with_template(
            "{msg} {percent_precise}% {bar:40.cyan/blue} [{decimal_bytes_per_sec}, {eta} left]",
        )?);

    let reader: Box<dyn Read> = if Path::new(input_file)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zst"))
    {
        Box::new(zstd::Decoder::new(progress_wrapped)?)
    } else {
        Box::new(progress_wrapped)
    };

    let out_file = File::create(output_file)?;
    let serializer = Serializer::new(out_file);
    let mut converter = Converter::new(reader, serializer);

    while converter.next_game().is_ok_and(|x| x) {}

    Ok(())
}

fn generate_default_output_filename(input_file: &str) -> String {
    let path = Path::new(input_file);
    
    let filename = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    
    // If input was .zst, strip one more extension (e.g., "file.pgn.zst" -> "file")
    let stem = if path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("zst")) {
        Path::new(filename).file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(filename)
    } else {
        filename
    };
    
    format!("{stem}.cbin")
}
