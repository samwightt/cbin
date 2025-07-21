use std::{collections::HashMap, io::Write};

use anyhow::Result;
use planus::{Builder, Offset, WriteAsOffset};

use crate::generated_chess::{Archive, ArchiveType, Block, Game, Move};

const MAX_GAMES_PER_BLOCK: usize = 500_000;

/// A serializer for the chess binary protocol.
///
/// Wraps the `planus::Builder` API with something nicer that also writes more efficiently.
/// Moves are deduplicated per block by default, resulting in smaller archives.
///
/// The serializer writes games in chunks called blocks. `FlatBuffer` serialization occurs in memory,
/// so it's important to flush this regularly using chunking logic. The serializer does this by maintaining
/// a list of added games. Once the amount of added games exceeds the `max_games_per_block` setting,
/// the serializer will end the current block and start a new one.
///
/// The output format is a sequence of the following:
///
/// ```
/// | u32 uint block length | block data |
/// ```
///
/// Decoding occurs by first parsing the 32-bit block length, then reading the following block data. Repeat
/// until the end of the archive is reached.
///
/// Note that because `FlatBuffer` uses 32-bit pointers, the maximum size of a block is 32-bit. Hence the block
/// length `u32`.
pub struct Serializer<T: Write> {
    writer: T,
    builder: Builder,
    move_map: HashMap<Move, Offset<Move>>,
    games_list: Vec<Offset<Game>>,
    max_games_per_block: usize,
}

impl<T: Write> Serializer<T> {
    /// Creates a new serializer with the given writer.
    ///
    /// By default, the maximum number of games per block is set to 500,000.
    pub fn new(writer: T) -> Self {
        let builder = Builder::new();
        let move_map = HashMap::new();
        Self {
            writer,
            builder,
            move_map,
            games_list: vec![],
            max_games_per_block: MAX_GAMES_PER_BLOCK,
        }
    }

    /// Allows setting the maximum number of games per block.
    pub const fn set_max_games_per_block(&mut self, max_games_per_block: usize) {
        self.max_games_per_block = max_games_per_block;
    }

    /// Adds a move to the serializer, returning the Planus offset.
    /// Deduplicates moves by default so that they are only serialized once.
    /// You can safely call this method multiple times with the same move and it will return the same offset.
    pub fn add_move(&mut self, game_move: &Move) -> Offset<Move> {
        self.move_map.get(game_move).copied().unwrap_or_else(|| {
            let offset = game_move.prepare(&mut self.builder);
            self.move_map.insert(game_move.clone(), offset);
            offset
        })
    }

    /// Adds a game to the serializer, returning the Planus offset.
    /// If the game count is greater than or equal to the maximum games per block,
    /// will finish serializing the current block and start a new one. Hence the Result type.
    pub fn add_game<R: WriteAsOffset<Game>>(&mut self, game: &R) -> Result<Offset<Game>> {
        let offset = game.prepare(&mut self.builder);
        self.games_list.push(offset);
        if self.games_list.len() >= self.max_games_per_block {
            self.finish_current_block()?;
        }
        Ok(offset)
    }

    fn reset(&mut self) {
        self.move_map.clear();
        self.games_list.clear();
        self.builder.clear();
    }

    /// Finishes serializing the current block, writing it to the output stream.
    ///
    /// Writing is a method that could fail, hence the Result type.
    pub fn finish_current_block(&mut self) -> Result<()> {
        let archive = Archive::builder()
            .games(&self.games_list)
            .prepare(&mut self.builder);
        let archive_type = ArchiveType::builder()
            .archive(archive)
            .finish(&mut self.builder);

        let block = Block::builder()
            .archive(archive_type)
            .finish(&mut self.builder);
        let result = self.builder.finish(block, None);

        #[allow(clippy::cast_possible_truncation)]
        let length = result.len() as u32;

        self.writer.write_all(&length.to_le_bytes())?;
        self.writer.write_all(result)?;
        self.reset();

        Ok(())
    }
}
