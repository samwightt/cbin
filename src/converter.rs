use anyhow::{Context, Result};
use std::{
    io::{Read, Write},
    ops::ControlFlow,
};

use pgn_reader::Visitor;
use planus::Offset;

use crate::{
    generated_chess::{CastleKind, Game, Move, Piece},
    serializer::Serializer,
    utils::{self, role_to_piece, shakmaty_square_to_square},
};

struct ConverterVisitor<W: Write> {
    serializer: Serializer<W>,
    current_moves: Vec<Offset<Move>>,
}

impl<W: Write> Visitor for ConverterVisitor<W> {
    type Tags = ();

    type Movetext = ();

    type Output = ();

    fn begin_tags(&mut self) -> std::ops::ControlFlow<Self::Output, Self::Tags> {
        ControlFlow::Continue(())
    }

    fn san(
        &mut self,
        _movetext: &mut Self::Movetext,
        san_plus: pgn_reader::SanPlus,
    ) -> ControlFlow<Self::Output> {
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
                from_file: file.map(crate::utils::shakmaty_file_to_file),
                from_rank: rank.map(crate::utils::shakmaty_rank_to_rank),
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
        ControlFlow::Continue(())
    }

    fn outcome(
        &mut self,
        _movetext: &mut Self::Movetext,
        outcome: shakmaty::Outcome,
    ) -> ControlFlow<Self::Output> {
        let result = utils::outcome_to_game_result(outcome);
        let res = Game::builder()
            .result(result)
            .start_position_as_null()
            .moves(&self.current_moves);
        self.serializer.add_game(&res).unwrap();
        self.current_moves = vec![];
        ControlFlow::Continue(())
    }

    fn begin_movetext(
        &mut self,
        _tags: Self::Tags,
    ) -> std::ops::ControlFlow<Self::Output, Self::Movetext> {
        ControlFlow::Continue(())
    }

    fn end_game(&mut self, _movetext: Self::Movetext) -> Self::Output {}
}

/// Given a reader and a serializer, reads PGN from the serializer and converts it to
/// a chess binary.
pub struct Converter<W: Write, R: Read> {
    visitor: ConverterVisitor<W>,
    pgn_parser: pgn_reader::Reader<R>,
    game_count: usize,
}

impl<W: Write, R: Read> Converter<W, R> {
    /// Creates a new converter instance from the given reader and serializer.
    ///
    /// Note that it must own both the reader and the serializer.
    pub fn new(reader: R, serializer: Serializer<W>) -> Self {
        Self {
            visitor: ConverterVisitor {
                serializer,
                current_moves: vec![],
            },
            pgn_parser: pgn_reader::Reader::new(reader),
            game_count: 0,
        }
    }

    /// Returns true if there are more games to be read from the PGN file.
    /// Note that this requires some parsing from the pgn library, which is why
    /// it has `&mut self` in there. Might throw if there are IO errors.
    pub fn has_more(&mut self) -> Result<bool> {
        self.pgn_parser
            .has_more()
            .with_context(|| "Failed to tell whether there are more games.")
    }

    /// Reads the next game the PGN file and converts it into the chess binary.
    ///
    /// Returns true if there was a game to read, false if there are no more games.
    pub fn next_game(&mut self) -> Result<bool> {
        let return_val = self.pgn_parser.read_game(&mut self.visitor)?.is_some();

        self.game_count += 1;

        Ok(return_val)
    }

    /// Flushes all the games converted so far to the output stream. Finishes the current block.
    pub fn flush(&mut self) -> Result<()> {
        self.visitor.serializer.finish_current_block()
    }

    /// Gets the number of games that have been converted from the PGN file into
    /// the chess binary.
    pub const fn game_count(&self) -> usize {
        self.game_count
    }
}

// Blanket implementation so we don't forget to flush the last value.
impl<W: Write, R: Read> Drop for Converter<W, R> {
    fn drop(&mut self) {
        self.flush().unwrap();
    }
}
