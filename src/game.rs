use std::sync::LazyLock;

use crate::board::{Board, CellCoord, PlacementMask, PlayerColor};

/// Build the initial Reversi board with a 2x2 block whose top-left corner is `origin`.
/// Color layout (same as standard Reversi):
///   origin       -> White
///   origin + (1,0)-> Black
///   origin + (0,1)-> Black
///   origin + (1,1)-> White
pub fn initial_board_at(origin: CellCoord) -> Board {
    if *origin.column() > 4 || *origin.row() > 4 {
        panic!("Initial block origin must fit within 6x6 board (A1-E5)");
    }

    let mut arr = [[0u8; 6]; 6];
    let x = *origin.column() as usize;
    let y = *origin.row() as usize;

    arr[y][x] = 1; // White
    arr[y][x + 1] = 2; // Black
    arr[y + 1][x] = 2; // Black
    arr[y + 1][x + 1] = 1; // White

    Board::from_012_array(arr)
}

/// Default initial board (origin at C3 as before).
pub static INITIAL_BOARD: LazyLock<Board> =
    LazyLock::new(|| initial_board_at(CellCoord::new(2, 2)));

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
        black_mask: &PlacementMask,
        white_mask: &PlacementMask,
        initial_board: &Board,
        observe: &mut impl FnMut(&Board, &CellCoord, &PlayerColor),
    ) -> Option<Board> {
        let mut board = initial_board.clone();
        let mut current_player = PlayerColor::Black;

        for cell in self.0.iter() {
            // Check if current player has any legal moves
            let has_moves = {
                let mask = match current_player {
                    PlayerColor::Black => black_mask,
                    PlayerColor::White => white_mask,
                };
                board
                    .moves_available(&current_player)
                    .iter()
                    .any(|mv| mask.can_place_at_cell(*mv))
            };

            let actual_player = if has_moves {
                current_player
            } else {
                current_player.opponent()
            };

            observe(&board, cell, &actual_player);

            board = board.place_disk(*cell.column(), *cell.row(), &actual_player)?;
            current_player = actual_player.opponent();
        }

        Some(board)
    }

    pub fn play_through(
        &self,
        black_mask: &PlacementMask,
        white_mask: &PlacementMask,
        initial_board: &Board,
    ) -> Option<Board> {
        self.play_through_and_observe_moves_sequentially(
            black_mask,
            white_mask,
            initial_board,
            &mut |_, _, _| {},
        )
    }

    pub fn to_moves(
        &self,
        black_mask: &PlacementMask,
        white_mask: &PlacementMask,
        initial_board: &Board,
    ) -> Vec<MoveInGame> {
        let mut moves = Vec::new();

        let _ = self.play_through_and_observe_moves_sequentially(
            black_mask,
            white_mask,
            initial_board,
            &mut |_board, cell, actual_player| {
                moves.push(MoveInGame {
                    cell: *cell,
                    player: *actual_player,
                    turn_index: moves.len(),
                })
            },
        );

        moves
    }
}
