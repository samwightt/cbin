#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]

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
use std::borrow::Cow;
use std::fs::File;
use std::io::{BufReader, Read};

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    let input_file = if args.len() > 1 {
        &args[1]
    } else {
        panic!("Usage: {} <input.fbs>", args[0]);
    };

    println!("Reading from {input_file}");

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
    
    let reader: Box<dyn Read> = if input_file.ends_with(".zst") {
        Box::new(zstd::Decoder::new(progress_wrapped)?)
    } else {
        Box::new(progress_wrapped)
    };

    let out_file = File::create("out.cbin")?;
    let serializer = Serializer::new(out_file);
    let mut converter = Converter::new(reader, serializer);

    while converter.next_game().is_ok_and(|x| x) {}

    Ok(())
}
