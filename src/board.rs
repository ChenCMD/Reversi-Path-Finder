use derive_getters::Getters;

pub struct BitBoard {
    // For a bitboard to be valid, we have:
    //  - white & black == 0 (no overlapping pieces)
    //  - white & (1 << 36) == white (no white pieces outside 6x6 board)
    //  - black & (1 << 36) == black (no black pieces outside 6x6 board)
    pub white: u64,
    pub black: u64,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BoardCellState {
    Empty = 0,
    White = 1,
    Black = 2,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PlayerColor {
    White,
    Black,
}

impl PlayerColor {
    pub fn opponent(&self) -> PlayerColor {
        match self {
            PlayerColor::White => PlayerColor::Black,
            PlayerColor::Black => PlayerColor::White,
        }
    }

    pub fn corresponding_cell_state(&self) -> BoardCellState {
        match self {
            PlayerColor::White => BoardCellState::White,
            PlayerColor::Black => BoardCellState::Black,
        }
    }
}

#[derive(Clone, Copy, Getters)]
pub struct CellCoord {
    column: u8,
    row: u8,
}

impl CellCoord {
    pub fn new(column: u8, row: u8) -> Self {
        if column >= 6 || row >= 6 {
            panic!("CellCoord out of bounds: column {}, row {}", column, row);
        }
        CellCoord { column, row }
    }

    pub fn to_string(&self) -> String {
        let col_char = (b'A' + self.column) as char;
        let row_char = (b'1' + self.row) as char;
        format!("{}{}", col_char, row_char)
    }
}

/// Converts an octal string representation of a bitboard into a 6x6 boolean array.
///
/// The octal string should represent 36 bits each, corresponding to the 6x6 board.
/// The mapping from string (`s`) to board cells is as follows (s[i] is the octal digit at position i):
///
/// Board:
///                  A              B              C              D              E              F
///   1  [s[0] & 0b100] [s[0] & 0b010] [s[0] & 0b001] [s[1] & 0b100] [s[1] & 0b010] [s[1] & 0b001]
///   2  [s[2] & 0b100] [s[2] & 0b010] [s[2] & 0b001] [s[3] & 0b100] [s[3] & 0b010] [s[3] & 0b001]
///   3  [s[4] & 0b100] [s[4] & 0b010] [s[4] & 0b001] [s[5] & 0b100] [s[5] & 0b010] [s[5] & 0b001]
///   4  [s[6] & 0b100] [s[6] & 0b010] [s[6] & 0b001] [s[7] & 0b100] [s[7] & 0b010] [s[7] & 0b001]
///   5  [s[8] & 0b100] [s[8] & 0b010] [s[8] & 0b001] [s[9] & 0b100] [s[9] & 0b010] [s[9] & 0b001]
///   6  [s[10]& 0b100] [s[10]& 0b010] [s[10]& 0b001] [s[11]& 0b100] [s[11]& 0b010] [s[11]& 0b001]
fn octal_str_to_boolean_board(octal_str: &str) -> [[bool; 6]; 6] {
    if octal_str.len() != 12 {
        panic!("Octal string must be 12 characters long to represent 36 bits");
    }

    let bits = u64::from_str_radix(octal_str, 8).expect("Invalid octal string for bitboard");

    let mut board = [[false; 6]; 6];

    for y in 0..6 {
        for x in 0..6 {
            let pos = (y * 6 + x) as u64;
            board[y][x] = (bits >> (35 - pos)) & 1 == 1;
        }
    }

    board
}

/// Represents a 6x6 Reversi board state.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Board(pub [[BoardCellState; 6]; 6]);

impl Board {
    /// Creates a Board from a 6x6 array of 0s, 1s, and 2s.
    /// 0 = Empty, 1 = White, 2 = Black.
    pub fn from_012_array(arr: [[u8; 6]; 6]) -> Self {
        let mut board = [[BoardCellState::Empty; 6]; 6];

        for i in 0..6 {
            for j in 0..6 {
                board[i][j] = match arr[i][j] {
                    0 => BoardCellState::Empty,
                    1 => BoardCellState::White,
                    2 => BoardCellState::Black,
                    _ => panic!("Invalid cell state specification"),
                };
            }
        }
        Board(board)
    }

    /// Creates a Board from two octal strings representing black and white bitboards.
    ///
    /// The octal strings should represent 36 bits each, corresponding to the 6x6 board.
    /// The mapping from string (`s`) to board cells is as follows (s[i] is the octal digit at position i):
    ///
    /// Board:
    ///                  A              B              C              D              E              F
    ///   1  [s[0] & 0b100] [s[0] & 0b010] [s[0] & 0b001] [s[1] & 0b100] [s[1] & 0b010] [s[1] & 0b001]
    ///   2  [s[2] & 0b100] [s[2] & 0b010] [s[2] & 0b001] [s[3] & 0b100] [s[3] & 0b010] [s[3] & 0b001]
    ///   3  [s[4] & 0b100] [s[4] & 0b010] [s[4] & 0b001] [s[5] & 0b100] [s[5] & 0b010] [s[5] & 0b001]
    ///   4  [s[6] & 0b100] [s[6] & 0b010] [s[6] & 0b001] [s[7] & 0b100] [s[7] & 0b010] [s[7] & 0b001]
    ///   5  [s[8] & 0b100] [s[8] & 0b010] [s[8] & 0b001] [s[9] & 0b100] [s[9] & 0b010] [s[9] & 0b001]
    ///   6  [s[10]& 0b100] [s[10]& 0b010] [s[10]& 0b001] [s[11]& 0b100] [s[11]& 0b010] [s[11]& 0b001]
    pub fn from_octal_strings(white_board_octal: &str, black_board_octal: &str) -> Self {
        let white_board = octal_str_to_boolean_board(white_board_octal);
        let black_board = octal_str_to_boolean_board(black_board_octal);
        let mut board = [[BoardCellState::Empty; 6]; 6];

        for y in 0..6 {
            for x in 0..6 {
                let is_black = black_board[y][x];
                let is_white = white_board[y][x];

                board[y][x] = if is_black && is_white {
                    panic!(
                        "Invalid bitboard: overlapping pieces at position ({}, {})",
                        x, y
                    )
                } else if is_black {
                    BoardCellState::Black
                } else if is_white {
                    BoardCellState::White
                } else {
                    BoardCellState::Empty
                }
            }
        }

        Board(board)
    }

    /// Creates a Board from black and white bitboards (u64 values).
    /// Each bit in the bitboard represents a cell position (0-35 for the 6x6 board).
    pub fn from_bitboards(black_bitboard: u64, white_bitboard: u64) -> Self {
        let mut cells = [[BoardCellState::Empty; 6]; 6];
        for row in 0..6 {
            for col in 0..6 {
                let bit_index = row * 6 + col;
                let bit_mask = 1u64 << bit_index;

                if black_bitboard & bit_mask != 0 {
                    cells[row][col] = BoardCellState::Black;
                } else if white_bitboard & bit_mask != 0 {
                    cells[row][col] = BoardCellState::White;
                }
            }
        }
        Board(cells)
    }

    pub fn black_to_octal_string(&self) -> String {
        let bb = self.to_bitboards();
        format!("{:012o}", bb.black.reverse_bits() >> 28)
    }

    pub fn white_to_octal_string(&self) -> String {
        let bb = self.to_bitboards();
        format!("{:012o}", bb.white.reverse_bits() >> 28)
    }

    pub fn to_bitboards(&self) -> BitBoard {
        // We use the following mapping.
        //
        // Board:
        //          A      B      C      D      E      F
        //   1  [0][0] [0][1] [0][2] [0][3] [0][4] [0][5]
        //   2  [1][0] [1][1] [1][2] [1][3] [1][4] [1][5]
        //   3  [2][0] [2][1] [2][2] [2][3] [2][4] [2][5]
        //   4  [3][0] [3][1] [3][2] [3][3] [3][4] [3][5]
        //   5  [4][0] [4][1] [4][2] [4][3] [4][4] [4][5]
        //   6  [5][0] [5][1] [5][2] [5][3] [5][4] [5][5]
        //
        // Bitboard bits:
        //          A      B      C      D      E      F
        //   1   [00]    [01]   [02]   [03]   [04]   [05]
        //   2   [06]    [07]   [08]   [09]   [10]   [11]
        //   3   [12]    [13]   [14]   [15]   [16]   [17]
        //   4   [18]    [19]   [20]   [21]   [22]   [23]
        //   5   [24]    [25]   [26]   [27]   [28]   [29]
        //   6   [30]    [31]   [32]   [33]   [34]   [35]

        let mut white: u64 = 0;
        let mut black: u64 = 0;
        for y in 0..6 {
            for x in 0..6 {
                let pos = (y * 6 + x) as u64;
                match self.0[y][x] {
                    BoardCellState::White => white |= 1 << pos,
                    BoardCellState::Black => black |= 1 << pos,
                    BoardCellState::Empty => {}
                }
            }
        }
        BitBoard { white, black }
    }

    pub fn filled_cells_count(&self) -> usize {
        self.0
            .iter()
            .flatten()
            .filter(|&&cell| cell != BoardCellState::Empty)
            .count()
    }

    pub fn to_string_block(&self) -> String {
        let mut result = String::new();
        result.push_str("   ABCDEF\n");
        result.push_str("   ------\n");
        for (y, row) in self.0.iter().enumerate() {
            result.push_str(&format!("{:1} |", y + 1));
            for &cell in row {
                result.push(match cell {
                    BoardCellState::Empty => '.',
                    BoardCellState::White => 'W',
                    BoardCellState::Black => 'B',
                });
            }
            result.push('\n');
        }
        result
    }

    /// Returns a visual string representation of the board using unicode symbols.
    /// Uses · for empty, ○ for black, ● for white.
    pub fn to_visual_string_block(&self) -> String {
        let mut result = String::new();
        result.push_str("   A B C D E F\n");
        for (y, row) in self.0.iter().enumerate() {
            result.push_str(&format!("{} ", y + 1));
            for &cell in row {
                let symbol = match cell {
                    BoardCellState::Empty => "·",
                    BoardCellState::Black => "○",
                    BoardCellState::White => "●",
                };
                result.push(' ');
                result.push_str(symbol);
            }
            result.push('\n');
        }
        result
    }

    pub fn moves_available(&self, playing_color: &PlayerColor) -> Vec<CellCoord> {
        let mut moves = Vec::new();
        for y in 0..6 {
            for x in 0..6 {
                if self.can_place_disk(x, y, playing_color) {
                    moves.push(CellCoord {
                        column: x as u8,
                        row: y as u8,
                    });
                }
            }
        }
        moves
    }

    pub fn can_place_disk(&self, x: u8, y: u8, playing_color: &PlayerColor) -> bool {
        self.0[y as usize][x as usize] == BoardCellState::Empty
            && !self.get_flipped_disks(x, y, playing_color).is_empty()
    }

    /// Returns a vector of CellCoord that would be flipped if a disk of playing_color is placed at (x, y).
    pub fn get_flipped_disks(&self, x: u8, y: u8, playing_color: &PlayerColor) -> Vec<CellCoord> {
        let mut flipped = Vec::new();

        let player_state = playing_color.corresponding_cell_state();
        let opponent_state = playing_color.opponent().corresponding_cell_state();

        // All 8 directions: (dx, dy)
        #[rustfmt::skip]
        let directions = [
            (-1, -1), (0, -1), (1, -1),  // up-left, up, up-right
            (-1, 0),           (1, 0),   // left, right
            (-1, 1),  (0, 1),  (1, 1),   // down-left, down, down-right
        ];

        for (dx, dy) in directions {
            let mut flipped_in_this_direction = Vec::new();
            let mut current_x = x;
            let mut current_y = y;

            // Look for opponent disks in this direction unless we are about to go out of bounds
            while (dx >= 0 || current_x > 0)
                && (dx <= 0 || current_x < 5)
                && (dy >= 0 || current_y > 0)
                && (dy <= 0 || current_y < 5)
            {
                current_x = (current_x as i32 + dx) as u8;
                current_y = (current_y as i32 + dy) as u8;

                let cell = self.0[current_y as usize][current_x as usize];

                if cell == opponent_state {
                    flipped_in_this_direction.push(CellCoord::new(current_x, current_y));
                } else if cell == player_state {
                    // Found a disk of our color, so all the opponent disks in between get flipped
                    flipped.extend(flipped_in_this_direction);
                    break;
                } else {
                    // Empty cell, no flips in this direction
                    break;
                }
            }
        }

        flipped
    }

    pub fn place_disk(&self, x: u8, y: u8, playing_color: &PlayerColor) -> Option<Board> {
        let flipped_disks = self.get_flipped_disks(x, y, playing_color);
        if flipped_disks.is_empty() {
            return None; // Invalid move
        } else {
            let mut new_board = self.0.clone();

            let player_state = playing_color.corresponding_cell_state();

            // Place the new disk and flip the disks
            new_board[y as usize][x as usize] = player_state;
            for coord in flipped_disks {
                new_board[coord.row as usize][coord.column as usize] = player_state;
            }

            Some(Board(new_board))
        }
    }
}

/// A mask indicating where a player can place disks.
/// Each entry is true if placement is allowed, false otherwise.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PlacementMask(pub [[bool; 6]; 6]);

impl PlacementMask {
    pub fn from_bool_array(arr: [[bool; 6]; 6]) -> Self {
        PlacementMask(arr)
    }

    pub fn can_place(&self, x: u8, y: u8) -> bool {
        self.0[y as usize][x as usize]
    }

    pub fn can_place_at_cell(&self, cell: CellCoord) -> bool {
        self.can_place(*cell.column(), *cell.row())
    }

    pub fn allow_everywhere() -> Self {
        PlacementMask([[true; 6]; 6])
    }

    /// Creates a PlacementMask from an octal string representation of a bitboard.
    ///
    /// The mapping from string (`s := octal_str`) to board cells is as follows (s[i] is the octal digit at position i):
    ///
    /// Board:
    ///                  A              B              C              D              E              F
    ///   1  [s[0] & 0b100] [s[0] & 0b010] [s[0] & 0b001] [s[1] & 0b100] [s[1] & 0b010] [s[1] & 0b001]
    ///   2  [s[2] & 0b100] [s[2] & 0b010] [s[2] & 0b001] [s[3] & 0b100] [s[3] & 0b010] [s[3] & 0b001]
    ///   3  [s[4] & 0b100] [s[4] & 0b010] [s[4] & 0b001] [s[5] & 0b100] [s[5] & 0b010] [s[5] & 0b001]
    ///   4  [s[6] & 0b100] [s[6] & 0b010] [s[6] & 0b001] [s[7] & 0b100] [s[7] & 0b010] [s[7] & 0b001]
    ///   5  [s[8] & 0b100] [s[8] & 0b010] [s[8] & 0b001] [s[9] & 0b100] [s[9] & 0b010] [s[9] & 0b001]
    ///   6  [s[10]& 0b100] [s[10]& 0b010] [s[10]& 0b001] [s[11]& 0b100] [s[11]& 0b010] [s[11]& 0b001]
    pub fn from_octal_string(octal_str: &str) -> Self {
        PlacementMask(octal_str_to_boolean_board(octal_str))
    }

    pub fn to_octal_string(&self) -> String {
        let mut bits: u64 = 0;
        for y in 0..6 {
            for x in 0..6 {
                let pos = (y * 6 + x) as u64;
                if self.0[y][x] {
                    bits |= 1 << pos;
                }
            }
        }
        format!("{:012o}", bits.reverse_bits() >> 28)
    }

    pub fn to_string_block(&self) -> String {
        let mut result = String::new();
        result.push_str("   ABCDEF\n");
        result.push_str("   ------\n");
        for (y, row) in self.0.iter().enumerate() {
            result.push_str(&format!("{:1} |", y + 1));
            for &cell in row {
                result.push(if cell { '.' } else { 'X' });
            }
            result.push('\n');
        }
        result
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_board_place_disk_at_se_corner() {
        let before = Board::from_012_array([
            /*       A  B  C  D  E  F */
            /* 1 */ [1, 1, 2, 2, 1, 2],
            /* 2 */ [0, 1, 1, 1, 1, 1],
            /* 3 */ [2, 0, 1, 1, 1, 2],
            /* 4 */ [1, 2, 2, 2, 1, 2],
            /* 5 */ [2, 2, 2, 2, 2, 1],
            /* 6 */ [0, 2, 1, 1, 1, 0],
        ]);
        let expected = Board::from_012_array([
            /*       A  B  C  D  E  F */
            /* 1 */ [1, 1, 2, 2, 1, 2],
            /* 2 */ [0, 1, 1, 1, 1, 1],
            /* 3 */ [2, 0, 1, 1, 1, 2],
            /* 4 */ [1, 2, 2, 2, 1, 2],
            /* 5 */ [2, 2, 2, 2, 2, 2],
            /* 6 */ [0, 2, 2, 2, 2, 2],
        ]);

        assert!(before.place_disk(5, 5, &PlayerColor::Black).unwrap() == expected);
    }

    #[test]
    fn test_from_octal_strings_initial() {
        let board = Board::from_octal_strings("000010040000", "000004100000");
        let expected = Board::from_012_array([
            /*       A  B  C  D  E  F */
            /* 1 */ [0, 0, 0, 0, 0, 0],
            /* 2 */ [0, 0, 0, 0, 0, 0],
            /* 3 */ [0, 0, 1, 2, 0, 0],
            /* 4 */ [0, 0, 2, 1, 0, 0],
            /* 5 */ [0, 0, 0, 0, 0, 0],
            /* 6 */ [0, 0, 0, 0, 0, 0],
        ]);
        assert_eq!(board, expected);
    }
}

pub mod example_boards {
    use std::sync::LazyLock;

    use super::Board;

    #[rustfmt::skip]
    pub const UNREACHABLE_2_STEPS: LazyLock<Board> = LazyLock::new(|| {
        Board::from_012_array([
            /*       A  B  C  D  E  F */
            /* 1 */ [0, 0, 0, 0, 0, 0],
            /* 2 */ [0, 0, 0, 0, 0, 0],
            /* 3 */ [0, 2, 2, 1, 1, 0], // must have B3E3 as the game record but B3E3 does not lead to this state
            /* 4 */ [0, 0, 2, 1, 0, 0],
            /* 5 */ [0, 0, 0, 0, 0, 0],
            /* 6 */ [0, 0, 0, 0, 0, 0],
        ])
    });

    #[rustfmt::skip]
    pub const UNREACHABLE_BROKEN: LazyLock<Board> = LazyLock::new(|| {
        Board::from_012_array([
            [1, 1, 1, 1, 1, 1],
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0], // center cells are never empty in a valid game
            [0, 0, 0, 0, 0, 0],
            [0, 0, 0, 0, 0, 0],
            [2, 2, 2, 2, 2, 2],
        ])
    });
}
