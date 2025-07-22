#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]

mod converter;
mod serializer;
mod utils;

use indicatif::{ProgressBar, ProgressStyle};

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
use std::fs::File;
use std::io::BufReader;

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let input_file = if args.len() > 1 {
        &args[1]
    } else {
        panic!("Usage: {} <input.fbs>", args[0]);
    };

    println!("Reading from {input_file}");

    let file = File::open(input_file)?;
    let buf_reader = BufReader::new(file);

    let out_file = File::create("out.cbin")?;
    let serializer = Serializer::new(out_file);
    let mut converter = Converter::new(buf_reader, serializer);

    let pb = ProgressBar::new_spinner();
    #[allow(clippy::literal_string_with_formatting_args)]
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} Games processed: {pos:>} | Rate: {per_sec} | Elapsed: {elapsed_precise}")
        .unwrap());

    while converter.next_game().is_ok_and(|x| x) {
        pb.inc(1);
    }

    pb.finish_with_message("Processing complete!");

    Ok(())
}
