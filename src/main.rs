#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]

mod serializer;
mod utils;

use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;

#[allow(non_snake_case)]
mod generated_chess {
    #![allow(warnings)]
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/chess.rs"));
    pub use chess::*;
}

use crate::generated_chess::{CastleKind, Game, GameResult, Move, Piece};
use crate::serializer::Serializer;
use crate::utils::{role_to_piece, shakmaty_square_to_square};
use anyhow::Result;
use pgn_reader::{Reader, Skip, Visitor};
use planus::Offset;
use std::fs::File;
use std::io::BufReader;
use std::ops::ControlFlow;

struct GameWriter<T: Write> {
    serializer: Serializer<T>,
    current_moves: Vec<Offset<Move>>,
    progress_bar: ProgressBar,
}

impl<T: Write> GameWriter<T> {
    fn new(out_file: T) -> Self {
        let pb = ProgressBar::new_spinner();
        #[allow(clippy::literal_string_with_formatting_args)]
        pb.set_style(ProgressStyle::default_spinner()
            .template("{spinner:.green} Games processed: {pos:>} | Rate: {per_sec} | Elapsed: {elapsed_precise}")
            .unwrap());

        Self {
            serializer: Serializer::new(out_file),
            current_moves: vec![],
            progress_bar: pb,
        }
    }

    fn add_move(&mut self, san_plus: pgn_reader::SanPlus) {
        use pgn_reader::shakmaty::{CastlingSide, san::San};
        let made_move = match san_plus.san {
            San::Normal {
                role,
                file,
                rank,
                capture,
                to,
                promotion,
            } => Move {
                moved_piece: role_to_piece(role),
                to: shakmaty_square_to_square(to),
                is_capture: capture,
                promoted_piece: promotion.map(role_to_piece),
                castle: None,
                from_file: file.map(utils::shakmaty_file_to_file),
                from_rank: rank.map(utils::shakmaty_rank_to_rank),
            },
            San::Castle(side) => {
                let castle_side = match side {
                    CastlingSide::KingSide => CastleKind::Kingside,
                    CastlingSide::QueenSide => CastleKind::Queenside,
                };

                Move {
                    moved_piece: Piece::King,
                    castle: Some(castle_side),
                    ..Default::default()
                }
            }
            _ => panic!("Unsupported move type."),
        };

        let offset = self.serializer.add_move(&made_move);

        self.current_moves.push(offset);
    }

    fn add_game(&mut self, result: GameResult) {
        let res = Game::builder()
            .result(result)
            .start_position_as_null()
            .moves(&self.current_moves);
        self.serializer.add_game(&res).unwrap();
        self.current_moves = vec![];
    }

    fn finalize(&mut self) {
        self.serializer.finish_current_block().unwrap();
    }
}

impl<T: Write> Visitor for GameWriter<T> {
    type Tags = ();
    type Movetext = ();
    type Output = ();

    fn begin_tags(&mut self) -> ControlFlow<Self::Output, Self::Tags> {
        ControlFlow::Continue(())
    }

    fn tag(
        &mut self,
        _tags: &mut Self::Tags,
        _name: &[u8],
        _value: pgn_reader::RawTag<'_>,
    ) -> ControlFlow<Self::Output> {
        ControlFlow::Continue(())
    }

    fn begin_movetext(&mut self, _tags: Self::Tags) -> ControlFlow<Self::Output, Self::Movetext> {
        ControlFlow::Continue(())
    }

    fn san(
        &mut self,
        _movetext: &mut Self::Movetext,
        san_plus: pgn_reader::SanPlus,
    ) -> ControlFlow<Self::Output> {
        self.add_move(san_plus);

        ControlFlow::Continue(())
    }

    fn nag(
        &mut self,
        _movetext: &mut Self::Movetext,
        _nag: pgn_reader::Nag,
    ) -> ControlFlow<Self::Output> {
        ControlFlow::Continue(())
    }

    fn comment(
        &mut self,
        _movetext: &mut Self::Movetext,
        _comment: pgn_reader::RawComment<'_>,
    ) -> ControlFlow<Self::Output> {
        ControlFlow::Continue(())
    }

    fn begin_variation(
        &mut self,
        _movetext: &mut Self::Movetext,
    ) -> ControlFlow<Self::Output, Skip> {
        ControlFlow::Continue(Skip(true))
    }

    fn end_variation(&mut self, _movetext: &mut Self::Movetext) -> ControlFlow<Self::Output> {
        ControlFlow::Continue(())
    }

    fn outcome(
        &mut self,
        _movetext: &mut Self::Movetext,
        outcome: pgn_reader::Outcome,
    ) -> ControlFlow<Self::Output> {
        self.add_game(utils::outcome_to_game_result(outcome));
        ControlFlow::Continue(())
    }

    fn end_game(&mut self, _movetext: Self::Movetext) -> Self::Output {
        self.progress_bar.inc(1);
    }
}

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
    let mut pgn_reader = Reader::new(buf_reader);

    let out_file = File::create("out.cbin")?;
    let mut visitor = GameWriter::new(out_file);

    pgn_reader.visit_all_games(&mut visitor)?;

    visitor.finalize();
    visitor
        .progress_bar
        .finish_with_message("Processing complete!");

    Ok(())
}
