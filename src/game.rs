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

pub struct MoveInGame {
    pub cell: CellCoord,
    pub player: PlayerColor,
    pub turn_index: usize,
}

pub struct UncheckedGameProgression(Vec<CellCoord>);

impl UncheckedGameProgression {
    pub fn new(moves: Vec<CellCoord>) -> Self {
        UncheckedGameProgression(moves)
    }

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

    pub fn to_game_record_string(&self) -> String {
        let mut result = String::new();
        for cell in self.0.iter() {
            let column_char = (b'A' + *cell.column()) as char;
            let row_char = (b'1' + *cell.row()) as char;
            result.push(column_char);
            result.push(row_char);
        }
        result
    }

    fn play_through_and_observe_moves_sequentially(
        &self,
        observe: &mut impl FnMut(&Board, &CellCoord, &PlayerColor),
    ) -> Option<Board> {
        let mut board = INITIAL_BOARD.clone();
        let mut current_player = PlayerColor::Black;

        for cell in self.0.iter() {
            let actual_player = if board.moves_available(&current_player).is_empty() {
                current_player.opponent()
            } else {
                current_player
            };

            observe(&board, cell, &actual_player);

            board = board.place_disk(*cell.column(), *cell.row(), &actual_player)?;
            current_player = actual_player.opponent();
        }

        Some(board)
    }

    pub fn play_through(&self) -> Option<Board> {
        self.play_through_and_observe_moves_sequentially(&mut |_, _, _| {})
    }

    pub fn to_moves(&self) -> Vec<MoveInGame> {
        let mut moves = Vec::new();

        let _ = self.play_through_and_observe_moves_sequentially(&mut |_board, cell, actual_player| {
            moves.push(MoveInGame {
                cell: *cell,
                player: *actual_player,
                turn_index: moves.len(),
            })
        });

        moves
    }
}
