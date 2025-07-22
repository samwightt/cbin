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
use crate::generated_chess::BlockRef;
use crate::serializer::Serializer;
use anyhow::Result;
use clap::{Parser, Subcommand};
use memmap2::Mmap;
use num_format::{Locale, ToFormattedString};
use planus::ReadAsRoot;
use rayon::prelude::*;
use shakmaty::Position;
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
    /// Read and analyze chess binary files
    Read {
        /// Input chess binary file (.cbin)
        input: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Convert { input, output } => {
            let output_file = output.unwrap_or_else(|| generate_default_output_filename(&input));
            convert_file(&input, &output_file)
        }
        Commands::Read { input } => read_file(&input),
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

    let filename = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    // If input was .zst, strip one more extension (e.g., "file.pgn.zst" -> "file")
    let stem = if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("zst"))
    {
        Path::new(filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(filename)
    } else {
        filename
    };

    format!("{stem}.cbin")
}

struct BlockIterator<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> BlockIterator<'a> {
    const fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }
}

impl<'a> Iterator for BlockIterator<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + 4 > self.data.len() {
            return None;
        }

        // Read the 4-byte block length (little-endian u32)
        let length_bytes = &self.data[self.offset..self.offset + 4];
        let block_length = u32::from_le_bytes([
            length_bytes[0],
            length_bytes[1],
            length_bytes[2],
            length_bytes[3],
        ]) as usize;

        // Move past the length header
        self.offset += 4;

        // Check if we have enough bytes for the block data
        if self.offset + block_length > self.data.len() {
            return None;
        }

        let block_data = &self.data[self.offset..self.offset + block_length];
        self.offset += block_length;

        Some(block_data)
    }
}

fn get_games_from_block(
    block_data: &[u8],
) -> Result<planus::vectors::Iter<'_, Result<generated_chess::GameRef<'_>, planus::Error>>> {
    let block = BlockRef::read_as_root(block_data)?;
    let archive = block.archive()?;

    let generated_chess::ArchiveTypeRef::Archive(archive_ref) = archive;
    let games = archive_ref.games()?;
    Ok(games.iter())
}

fn is_white_win(game: &generated_chess::GameRef) -> Result<bool> {
    use crate::utils::move_ref_to_san;

    let mut chess = shakmaty::Chess::default();

    for move_item in game.moves()? {
        let move_ref = move_item?;
        let san = move_ref_to_san(&move_ref)?;
        let mv = san.to_move(&chess)?;
        chess = chess.play(mv)?;
    }

    match chess.outcome() {
        shakmaty::Outcome::Known(shakmaty::KnownOutcome::Decisive {
            winner: shakmaty::Color::White,
        }) => Ok(true),
        _ => Ok(false),
    }
}

fn read_file(input_file: &str) -> Result<()> {
    println!("Reading chess binary file: {input_file}");

    let file = File::open(input_file)?;
    let mmap = unsafe { Mmap::map(&file)? };

    // First pass: count total games for progress bar
    let (block_count, total_games): (usize, usize) = BlockIterator::new(&mmap)
        .par_bridge()
        .map(|block_data| {
            (
                1,
                get_games_from_block(block_data).map_or(0, Iterator::count),
            )
        })
        .reduce(|| (0, 0), |a, b| (a.0 + b.0, a.1 + b.1));

    println!("Total blocks: {block_count}");
    println!("Total games: {total_games}");

    let start_time = std::time::Instant::now();

    // Second pass: calculate average moves per game
    let moves_progress_bar = ProgressBar::new(total_games as u64);
    moves_progress_bar.set_style(ProgressStyle::with_template(
        "{msg} {percent}% {bar:40.cyan/blue} [{pos:>} / {len:>} games, {per_sec}, {eta} left]",
    )?);
    moves_progress_bar.set_message("Calculating average moves");

    let total_moves: usize = BlockIterator::new(&mmap)
        .par_bridge()
        .flat_map_iter(|block_data| {
            get_games_from_block(block_data).unwrap_or_else(|_| planus::Vector::new_empty().iter())
        })
        .filter_map(std::result::Result::ok)
        .map(|game| {
            moves_progress_bar.inc(1);
            game.moves().map_or(0, |moves| moves.len())
        })
        .sum();

    moves_progress_bar.finish_with_message("Average moves calculation complete");

    let average_moves_per_game = total_moves as f64 / total_games as f64;
    println!(
        "Average moves per game: {:.2}",
        average_moves_per_game
    );

    // Set up progress bar for game analysis
    let progress_bar = ProgressBar::new(total_games as u64);
    progress_bar.set_style(ProgressStyle::with_template(
        "{msg} {percent}% {bar:40.cyan/blue} [{pos:>} / {len:>} games, {per_sec}, {eta} left]",
    )?);
    progress_bar.set_message("Analyzing games");

    // Third pass: analyze games with progress tracking
    let white_wins = BlockIterator::new(&mmap)
        .par_bridge()
        .flat_map_iter(|block_data| {
            get_games_from_block(block_data).unwrap_or_else(|_| planus::Vector::new_empty().iter())
        })
        .filter_map(std::result::Result::ok)
        .filter(|game| {
            progress_bar.inc(1);
            is_white_win(game).unwrap_or(false)
        })
        .count();

    let elapsed = start_time.elapsed();
    progress_bar.finish_with_message(format!(
        "Analysis complete! Processed {} games in {:.2}s",
        total_games.to_formatted_string(&Locale::en),
        elapsed.as_secs_f64()
    ));

    println!(
        "White wins: {}",
        white_wins.to_formatted_string(&Locale::en)
    );

    Ok(())
}
