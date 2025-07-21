#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]

mod utils;

use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::io::Write;

#[allow(non_snake_case)]
mod generated_chess {
    #![allow(warnings)]
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/chess.rs"));
    pub use chess::*;
}

use crate::generated_chess::{
    Archive, ArchiveType, Block, CastleKind, Game, GameResult, Move, Piece, Square,
};
use anyhow::Result;
use pgn_reader::{Reader, Skip, Visitor};
use planus::{Builder, Offset, WriteAsOffset};
use shakmaty::{Chess, Position, fen::Fen};
use std::fs::File;
use std::io::BufReader;
use std::ops::ControlFlow;

struct GameWriter<T: Write> {
    current_builder: Builder,
    writer: T,
    current_moves: Vec<Offset<Move>>,
    move_map: HashMap<Move, Offset<Move>>,
    games: Vec<Offset<Game>>,
    progress_bar: ProgressBar,
}

/// When serializing, we only want to include a certain amount of games per block. This enables us to
/// read the resulting file in parallel later.
const MAX_GAMES_PER_BLOCK: usize = 500_000;

impl<T: Write> GameWriter<T> {
    fn new(out_file: T) -> Self {
        let pb = ProgressBar::new_spinner();
        #[allow(clippy::literal_string_with_formatting_args)]
        pb.set_style(ProgressStyle::default_spinner()
            .template("{spinner:.green} Games processed: {pos:>} | Rate: {per_sec} | Elapsed: {elapsed_precise}")
            .unwrap());

        Self {
            current_builder: Builder::new(),
            writer: out_file,
            current_moves: vec![],
            games: vec![],
            move_map: HashMap::new(),
            progress_bar: pb,
        }
    }

    fn add_move(&mut self, move_to_add: shakmaty::Move, is_check: bool) {
        let made_move = if move_to_add.is_castle() {
            let castle_side = match move_to_add.castling_side() {
                Some(shakmaty::CastlingSide::KingSide) => CastleKind::Kingside,
                Some(shakmaty::CastlingSide::QueenSide) => CastleKind::Queenside,
                _ => unreachable!(),
            };

            Move {
                moved_piece: Piece::King,
                castle: Some(castle_side),
                ..Default::default()
            }
        } else {
            Move {
                moved_piece: utils::role_to_piece(move_to_add.role()),
                from: move_to_add
                    .from()
                    .map_or(Square::A1, utils::shakmaty_square_to_square),
                to: utils::shakmaty_square_to_square(move_to_add.to()),
                promoted_piece: move_to_add.promotion().map(utils::role_to_piece),
                is_check,
                is_capture: move_to_add.is_capture(),
                ..Default::default()
            }
        };

        let offset = self.move_map.get(&made_move).copied().unwrap_or_else(|| {
            let offset = made_move.prepare(&mut self.current_builder);
            self.move_map.insert(made_move, offset);
            offset
        });

        self.current_moves.push(offset);
    }

    fn add_game(&mut self, result: GameResult) {
        let res = Game::builder()
            .result(result)
            .start_position_as_null()
            .moves(&self.current_moves)
            .finish(&mut self.current_builder);
        self.current_moves = vec![];
        self.games.push(res);

        if self.games.len() > MAX_GAMES_PER_BLOCK {
            self.finalize();
        }
    }

    fn finalize(&mut self) {
        let archive = Archive::builder()
            .games(&self.games)
            .prepare(&mut self.current_builder);
        let archive_type = ArchiveType::builder()
            .archive(archive)
            .finish(&mut self.current_builder);
        let block = Block::builder()
            .archive(archive_type)
            .finish(&mut self.current_builder);
        let result = self.current_builder.finish(block, None);

        #[allow(clippy::cast_possible_truncation)]
        let length = result.len() as u32;

        // Write 4-byte length prefix, then the data
        self.writer.write_all(&length.to_le_bytes()).unwrap();
        self.writer.write_all(result).unwrap();
        self.reset();
    }

    fn reset(&mut self) {
        self.move_map = HashMap::new();
        self.current_moves = vec![];
        self.games = vec![];
        self.current_builder.clear();
    }
}

impl<T: Write> Visitor for GameWriter<T> {
    type Tags = Option<Chess>;
    type Movetext = Chess;
    type Output = ();

    fn begin_tags(&mut self) -> ControlFlow<Self::Output, Self::Tags> {
        ControlFlow::Continue(Option::default())
    }

    fn tag(
        &mut self,
        tags: &mut Self::Tags,
        name: &[u8],
        value: pgn_reader::RawTag<'_>,
    ) -> ControlFlow<Self::Output> {
        if name == b"FEN" {
            let fen = match Fen::from_ascii(value.as_bytes()) {
                Ok(fen) => fen,
                Err(_err) => return ControlFlow::Break(()),
            };
            let pos = match fen.into_position(shakmaty::CastlingMode::Standard) {
                Ok(pos) => pos,
                Err(_err) => return ControlFlow::Break(()),
            };
            tags.replace(pos);
        }
        ControlFlow::Continue(())
    }

    fn begin_movetext(&mut self, tags: Self::Tags) -> ControlFlow<Self::Output, Self::Movetext> {
        ControlFlow::Continue(tags.unwrap_or_default())
    }

    fn san(
        &mut self,
        movetext: &mut Self::Movetext,
        san_plus: pgn_reader::SanPlus,
    ) -> ControlFlow<Self::Output> {
        let Ok(res) = san_plus.san.to_move(movetext) else {
            return ControlFlow::Break(());
        };
        movetext.play_unchecked(res);

        self.add_move(res, movetext.is_check());

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
