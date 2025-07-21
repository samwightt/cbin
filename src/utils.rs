use crate::generated_chess::{File, GameResult, Piece, Rank, Square};

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

macro_rules! square_match {
    ($input:expr, $($square:ident),*) => {
        match $input {
            $(
                shakmaty::Square::$square => Square::$square,
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
