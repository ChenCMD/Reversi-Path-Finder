use std::sync::LazyLock;

use crate::board::{Board, CellCoord, PlayerColor};

#[rustfmt::skip]
/// Initial Reversi configuration.
pub const INITIAL_BOARD: LazyLock<Board> = LazyLock::new(|| {
    Board::from_012_array([
        /*       A  B  C  D  E  F */
        /* 1 */ [0, 0, 0, 0, 0, 0],
        /* 2 */ [0, 0, 0, 0, 0, 0],
        /* 3 */ [0, 0, 1, 2, 0, 0],
        /* 4 */ [0, 0, 2, 1, 0, 0],
        /* 5 */ [0, 0, 0, 0, 0, 0],
        /* 6 */ [0, 0, 0, 0, 0, 0],
    ])
});

pub struct UncheckedGameProgression(Vec<CellCoord>);

impl UncheckedGameProgression {
    pub fn from_game_record_string(game_record: &str) -> Self {
        let mut specs = vec![];
        // Parse moves like "B3B2C2..." (ABCDEF / 123456)
        for move_str in game_record.as_bytes().chunks(2) {
            let column = move_str[0] - b'A';
            let row = move_str[1] - b'1';

            if column >= 6 || row >= 6 {
                panic!(
                    "Invalid move in game record: {}",
                    std::str::from_utf8(move_str).unwrap()
                );
            }

            specs.push(CellCoord::new(column, row));
        }
        UncheckedGameProgression(specs)
    }

    pub fn play_through(&self) -> Board {
        let mut board = INITIAL_BOARD.clone();
        let mut current_player = PlayerColor::Black;

        for cell in self.0.iter() {
            let actual_player = if board.moves_available(&current_player).is_empty() {
                current_player.opponent()
            } else {
                current_player
            };

            board = board
                .place_disk(*cell.column(), *cell.row(), &actual_player)
                .unwrap();
            current_player = actual_player.opponent();
        }

        board
    }
}
