// #[allow(non_snake_case)]
// #[path = "../target/flatbuffers/main_generated.rs"]
// mod main_flatbuffers;

#[allow(non_snake_case)]
mod generated_chess {
    include!(concat!(env!("OUT_DIR"), "/chess.rs"));
    pub use chess::*;
}

// use crate::main_flatbuffers::sample::{
//     Color, Equipment, Monster, MonsterArgs, Vec3, Weapon, WeaponArgs,
// };

use std::fs::File;

use planus::{Builder, WriteAsOffset};

use crate::generated_chess::{Archive, ArchiveType, Block, Game, GameResult, Move, Piece, Square};

fn main() {

    let mut builder = Builder::new();
    let example_move = Move {
        moved_piece: Piece::Pawn,
        to: Square::A6,
        ..Default::default()
    };

    let offset = example_move.prepare(&mut builder);

    let other_move = Move {
        moved_piece: Piece::Bishop,
        to: Square::F8,
        ..Default::default()
    };
    let other_offset = other_move.prepare(&mut builder);

    let game = Game::builder()
        .result(GameResult::BlackWin)
        .moves(vec![example_move, other_move])
        .finish(&mut builder);
    let other_game = Game::builder()
        .result(GameResult::WhiteWin)
        .moves(vec![offset, other_offset])
        .finish(&mut builder);

    let archive = Archive::builder()
        .games(vec![game, other_game])
        .finish(&mut builder);

    let block = Block::builder().archive(ArchiveType::create_archive(&mut builder, archive));

    let result = builder.finish(block, None);

    println!("{:?}", result);
}
