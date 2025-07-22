use crate::generated_chess::{File, GameResult, MoveRef, Piece, Rank, Square};
use anyhow::Result;

/// Converts a `shakmaty::Role` into a corresponding `Piece`.
pub const fn role_to_piece(role: shakmaty::Role) -> Piece {
    match role {
        shakmaty::Role::King => Piece::King,
        shakmaty::Role::Queen => Piece::Queen,
        shakmaty::Role::Rook => Piece::Rook,
        shakmaty::Role::Bishop => Piece::Bishop,
        shakmaty::Role::Knight => Piece::Knight,
        shakmaty::Role::Pawn => Piece::Pawn,
    }
}

/// Converts a `Piece` into a corresponding `shakmaty::Role`.
pub const fn piece_to_role(piece: Piece) -> shakmaty::Role {
    match piece {
        Piece::King => shakmaty::Role::King,
        Piece::Queen => shakmaty::Role::Queen,
        Piece::Rook => shakmaty::Role::Rook,
        Piece::Bishop => shakmaty::Role::Bishop,
        Piece::Knight => shakmaty::Role::Knight,
        Piece::Pawn => shakmaty::Role::Pawn,
    }
}

macro_rules! square_match {
    ($input:expr, $($square:ident),*) => {
        match $input {
            $(
                shakmaty::Square::$square => Square::$square,
            )*
        }
    }
}

macro_rules! reverse_square_match {
    ($input:expr, $($square:ident),*) => {
        match $input {
            $(
                Square::$square => shakmaty::Square::$square,
            )*
        }
    }
}

pub const fn shakmaty_square_to_square(s_square: shakmaty::Square) -> Square {
    // Macro to save us lines for a converter and without having to use unsafe.
    // Zero clue how shakmaty::Square is actually implemented so we're doing this.
    square_match!(
        s_square, A1, B1, C1, D1, E1, F1, G1, H1, A2, B2, C2, D2, E2, F2, G2, H2, A3, B3, C3, D3,
        E3, F3, G3, H3, A4, B4, C4, D4, E4, F4, G4, H4, A5, B5, C5, D5, E5, F5, G5, H5, A6, B6, C6,
        D6, E6, F6, G6, H6, A7, B7, C7, D7, E7, F7, G7, H7, A8, B8, C8, D8, E8, F8, G8, H8
    )
}

pub const fn outcome_to_game_result(outcome: shakmaty::Outcome) -> GameResult {
    use shakmaty::{Color, KnownOutcome, Outcome};
    match outcome {
        Outcome::Known(outcome) => match outcome {
            KnownOutcome::Decisive {
                winner: Color::White,
            } => GameResult::WhiteWin,
            KnownOutcome::Decisive {
                winner: Color::Black,
            } => GameResult::BlackWin,
            KnownOutcome::Draw => GameResult::Draw,
        },
        Outcome::Unknown => GameResult::Unknown,
    }
}

pub const fn shakmaty_file_to_file(s_file: pgn_reader::shakmaty::File) -> File {
    use pgn_reader::shakmaty;
    match s_file {
        shakmaty::File::A => File::A,
        shakmaty::File::B => File::B,
        shakmaty::File::C => File::C,
        shakmaty::File::D => File::D,
        shakmaty::File::E => File::E,
        shakmaty::File::F => File::F,
        shakmaty::File::G => File::G,
        shakmaty::File::H => File::H,
    }
}

pub const fn shakmaty_rank_to_rank(s_rank: pgn_reader::shakmaty::Rank) -> Rank {
    use pgn_reader::shakmaty;
    match s_rank {
        shakmaty::Rank::First => Rank::First,
        shakmaty::Rank::Second => Rank::Second,
        shakmaty::Rank::Third => Rank::Third,
        shakmaty::Rank::Fourth => Rank::Fourth,
        shakmaty::Rank::Fifth => Rank::Fifth,
        shakmaty::Rank::Sixth => Rank::Sixth,
        shakmaty::Rank::Seventh => Rank::Seventh,
        shakmaty::Rank::Eighth => Rank::Eighth,
    }
}

pub const fn square_to_shakmaty_square(square: Square) -> shakmaty::Square {
    reverse_square_match!(
        square, A1, B1, C1, D1, E1, F1, G1, H1, A2, B2, C2, D2, E2, F2, G2, H2, A3, B3, C3, D3, E3,
        F3, G3, H3, A4, B4, C4, D4, E4, F4, G4, H4, A5, B5, C5, D5, E5, F5, G5, H5, A6, B6, C6, D6,
        E6, F6, G6, H6, A7, B7, C7, D7, E7, F7, G7, H7, A8, B8, C8, D8, E8, F8, G8, H8
    )
}

pub fn move_ref_to_san(move_ref: &MoveRef) -> Result<shakmaty::san::San> {
    use shakmaty::san::San;
    use shakmaty::CastlingSide;

    // Handle castling first
    if let Some(castle_kind) = move_ref.castle()? {
        return Ok(San::Castle(match castle_kind {
            crate::generated_chess::CastleKind::Kingside => CastlingSide::KingSide,
            crate::generated_chess::CastleKind::Queenside => CastlingSide::QueenSide,
        }));
    }

    let role = piece_to_role(move_ref.moved_piece()?);
    let to = square_to_shakmaty_square(move_ref.to()?);
    let capture = move_ref.is_capture()?;

    let promotion = move_ref.promoted_piece()?.map(piece_to_role);

    // Convert disambiguation info
    let from_file = move_ref.from_file()?.map(|f| match f {
        File::A => shakmaty::File::A,
        File::B => shakmaty::File::B,
        File::C => shakmaty::File::C,
        File::D => shakmaty::File::D,
        File::E => shakmaty::File::E,
        File::F => shakmaty::File::F,
        File::G => shakmaty::File::G,
        File::H => shakmaty::File::H,
    });

    let from_rank = move_ref.from_rank()?.map(|r| match r {
        Rank::First => shakmaty::Rank::First,
        Rank::Second => shakmaty::Rank::Second,
        Rank::Third => shakmaty::Rank::Third,
        Rank::Fourth => shakmaty::Rank::Fourth,
        Rank::Fifth => shakmaty::Rank::Fifth,
        Rank::Sixth => shakmaty::Rank::Sixth,
        Rank::Seventh => shakmaty::Rank::Seventh,
        Rank::Eighth => shakmaty::Rank::Eighth,
    });

    Ok(San::Normal {
        role,
        file: from_file,
        rank: from_rank,
        capture,
        to,
        promotion,
    })
}
