#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::cargo)]

use std::collections::HashMap;
use std::io::Write;
use indicatif::{ProgressBar, ProgressStyle};

#[allow(non_snake_case)]
mod generated_chess {
    #![allow(warnings)]
    #![allow(clippy::all)]
    include!(concat!(env!("OUT_DIR"), "/chess.rs"));
    pub use chess::*;
}

use std::fs::File;
use std::io::{BufReader};
use std::ops::ControlFlow;
use planus::{Builder, Offset, WriteAsOffset};
use anyhow::Result;
use pgn_reader::{Reader, Skip, Visitor};
use shakmaty::{Chess, Position, fen::Fen};
use crate::generated_chess::{Archive, ArchiveType, Block, CastleKind, Game, GameResult, Move, Piece, Square};


struct GameWriter {
    current_builder: Builder,
    out_file: File,
    current_moves: Vec<Offset<Move>>,
    move_map: HashMap<Move, Offset<Move>>,
    games: Vec<Offset<Game>>,
    progress_bar: ProgressBar,
}

const fn role_to_piece(role: shakmaty::Role) -> Piece {
    match role {
        shakmaty::Role::King => Piece::King,
        shakmaty::Role::Queen => Piece::Queen,
        shakmaty::Role::Rook => Piece::Rook,
        shakmaty::Role::Bishop => Piece::Bishop,
        shakmaty::Role::Knight => Piece::Knight,
        shakmaty::Role::Pawn => Piece::Pawn
    }
}

const fn s_square_to_square(s_square: shakmaty::Square) -> Square {
    match s_square {
        shakmaty::Square::A1 => Square::A1,
        shakmaty::Square::B1 => Square::B1,
        shakmaty::Square::C1 => Square::C1,
        shakmaty::Square::D1 => Square::D1,
        shakmaty::Square::E1 => Square::E1,
        shakmaty::Square::F1 => Square::F1,
        shakmaty::Square::G1 => Square::G1,
        shakmaty::Square::H1 => Square::H1,
        shakmaty::Square::A2 => Square::A2,
        shakmaty::Square::B2 => Square::B2,
        shakmaty::Square::C2 => Square::C2,
        shakmaty::Square::D2 => Square::D2,
        shakmaty::Square::E2 => Square::E2,
        shakmaty::Square::F2 => Square::F2,
        shakmaty::Square::G2 => Square::G2,
        shakmaty::Square::H2 => Square::H2,
        shakmaty::Square::A3 => Square::A3,
        shakmaty::Square::B3 => Square::B3,
        shakmaty::Square::C3 => Square::C3,
        shakmaty::Square::D3 => Square::D3,
        shakmaty::Square::E3 => Square::E3,
        shakmaty::Square::F3 => Square::F3,
        shakmaty::Square::G3 => Square::G3,
        shakmaty::Square::H3 => Square::H3,
        shakmaty::Square::A4 => Square::A4,
        shakmaty::Square::B4 => Square::B4,
        shakmaty::Square::C4 => Square::C4,
        shakmaty::Square::D4 => Square::D4,
        shakmaty::Square::E4 => Square::E4,
        shakmaty::Square::F4 => Square::F4,
        shakmaty::Square::G4 => Square::G4,
        shakmaty::Square::H4 => Square::H4,
        shakmaty::Square::A5 => Square::A5,
        shakmaty::Square::B5 => Square::B5,
        shakmaty::Square::C5 => Square::C5,
        shakmaty::Square::D5 => Square::D5,
        shakmaty::Square::E5 => Square::E5,
        shakmaty::Square::F5 => Square::F5,
        shakmaty::Square::G5 => Square::G5,
        shakmaty::Square::H5 => Square::H5,
        shakmaty::Square::A6 => Square::A6,
        shakmaty::Square::B6 => Square::B6,
        shakmaty::Square::C6 => Square::C6,
        shakmaty::Square::D6 => Square::D6,
        shakmaty::Square::E6 => Square::E6,
        shakmaty::Square::F6 => Square::F6,
        shakmaty::Square::G6 => Square::G6,
        shakmaty::Square::H6 => Square::H6,
        shakmaty::Square::A7 => Square::A7,
        shakmaty::Square::B7 => Square::B7,
        shakmaty::Square::C7 => Square::C7,
        shakmaty::Square::D7 => Square::D7,
        shakmaty::Square::E7 => Square::E7,
        shakmaty::Square::F7 => Square::F7,
        shakmaty::Square::G7 => Square::G7,
        shakmaty::Square::H7 => Square::H7,
        shakmaty::Square::A8 => Square::A8,
        shakmaty::Square::B8 => Square::B8,
        shakmaty::Square::C8 => Square::C8,
        shakmaty::Square::D8 => Square::D8,
        shakmaty::Square::E8 => Square::E8,
        shakmaty::Square::F8 => Square::F8,
        shakmaty::Square::G8 => Square::G8,
        shakmaty::Square::H8 => Square::H8,
    }
}

/// When serializing, we only want to include a certain amount of games per block. This enables us to
/// read the resulting file in parallel later
const MAX_GAMES_PER_BLOCK: usize = 500_000;

impl GameWriter {
    fn new(out_file: File) -> Self {
        let pb = ProgressBar::new_spinner();
        #[allow(clippy::literal_string_with_formatting_args)]
        pb.set_style(ProgressStyle::default_spinner()
            .template("{spinner:.green} Games processed: {pos:>} | Rate: {per_sec} | Elapsed: {elapsed_precise}")
            .unwrap());

        Self {
            current_builder: Builder::new(),
            out_file,
            current_moves: vec![],
            games: vec![],
            move_map: HashMap::new(),
            progress_bar: pb
        }
    }

    fn add_move(&mut self, move_to_add: shakmaty::Move, is_check: bool) {
        let made_move = if move_to_add.is_castle() {
            let castle_side = match move_to_add.castling_side() {
                Some(shakmaty::CastlingSide::KingSide) => CastleKind::Kingside,
                Some(shakmaty::CastlingSide::QueenSide) => CastleKind::Queenside,
                _ => unreachable!()
            };

            Move {
                moved_piece: Piece::King,
                castle: Some(castle_side),
                ..Default::default()
            }
        } else {
            Move {
                moved_piece: role_to_piece(move_to_add.role()),
                from: move_to_add.from().map_or(Square::A1, s_square_to_square),
                to: s_square_to_square(move_to_add.to()),
                promoted_piece: move_to_add.promotion().map(role_to_piece),
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
        let archive_type = ArchiveType::builder().archive(archive).finish(&mut self.current_builder);
        let block = Block::builder().archive(archive_type).finish(&mut self.current_builder);
        let result = self.current_builder.finish(block, None);

        #[allow(clippy::cast_possible_truncation)]
        let length = result.len() as u32;

        // Write 4-byte length prefix, then the data
        self.out_file.write_all(&length.to_le_bytes()).unwrap();
        self.out_file.write_all(result).unwrap();
        self.reset();
    }

    fn reset(&mut self) {
        self.move_map = HashMap::new();
        self.current_moves = vec![];
        self.games = vec![];
        self.current_builder.clear();
    }
}

const fn outcome_to_game_result(outcome: shakmaty::Outcome) -> GameResult {
    use shakmaty::{Outcome, KnownOutcome, Color};
    match outcome {
        Outcome::Known(outcome) => match outcome {
            KnownOutcome::Decisive { winner: Color::White } => GameResult::WhiteWin,
            KnownOutcome::Decisive { winner: Color::Black } => GameResult::BlackWin,
            KnownOutcome::Draw => GameResult::Draw
        },
        Outcome::Unknown => GameResult::Unknown
    }
}

impl Visitor for GameWriter {
    type Tags = Option<Chess>;
    type Movetext = Chess;
    type Output = ();

    fn begin_tags(&mut self) -> ControlFlow<Self::Output, Self::Tags> {
        ControlFlow::Continue(Option::default())
    }

    fn tag(&mut self, tags: &mut Self::Tags, name: &[u8], value: pgn_reader::RawTag<'_>) -> ControlFlow<Self::Output> {
        if name == b"FEN" {
            let fen = match Fen::from_ascii(value.as_bytes()) {
                Ok(fen) => fen,
                Err(_err) => return ControlFlow::Break(())
            };
            let pos = match fen.into_position(shakmaty::CastlingMode::Standard) {
                Ok(pos) => pos,
                Err(_err) => return ControlFlow::Break(())
            };
            tags.replace(pos);
        }
        ControlFlow::Continue(())
    }

    fn begin_movetext(&mut self, tags: Self::Tags) -> ControlFlow<Self::Output, Self::Movetext> {
        ControlFlow::Continue(tags.unwrap_or_default())
    }

    fn san(&mut self, movetext: &mut Self::Movetext, san_plus: pgn_reader::SanPlus) -> ControlFlow<Self::Output> {
        let Ok(res) = san_plus.san.to_move(movetext) else { return ControlFlow::Break(()) };
        movetext.play_unchecked(res);

        self.add_move(res, movetext.is_check());

        ControlFlow::Continue(())
    }

    fn nag(&mut self, _movetext: &mut Self::Movetext, _nag: pgn_reader::Nag) -> ControlFlow<Self::Output> {
        ControlFlow::Continue(())
    }

    fn comment(&mut self, _movetext: &mut Self::Movetext, _comment: pgn_reader::RawComment<'_>) -> ControlFlow<Self::Output> {
        ControlFlow::Continue(())
    }

    fn begin_variation(&mut self, _movetext: &mut Self::Movetext) -> ControlFlow<Self::Output, Skip> {
        ControlFlow::Continue(Skip(true))
    }

    fn end_variation(&mut self, _movetext: &mut Self::Movetext) -> ControlFlow<Self::Output> {
        ControlFlow::Continue(())
    }

    fn outcome(&mut self, _movetext: &mut Self::Movetext, outcome: pgn_reader::Outcome) -> ControlFlow<Self::Output> {
        self.add_game(outcome_to_game_result(outcome));
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
    visitor.progress_bar.finish_with_message("Processing complete!");

    Ok(())

}
