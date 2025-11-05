pub struct BitBoard {
    // For a bitboard to be valid, we have:
    //  - white & black == 0 (no overlapping pieces)
    //  - white & (1 << 36) == white (no white pieces outside 6x6 board)
    //  - black & (1 << 36) == black (no black pieces outside 6x6 board)
    pub white: u64,
    pub black: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BoardCellState {
    Empty = 0,
    White = 1,
    Black = 2,
}

/// Represents a 6x6 Reversi board state.
pub struct Board(pub [[BoardCellState; 6]; 6]);

impl Board {
    /// Creates a Board from a 6x6 array of 0s, 1s, and 2s.
    /// 0 = Empty, 1 = White, 2 = Black.
    pub const fn from_012_array(arr: [[u8; 6]; 6]) -> Self {
        let mut board = [[BoardCellState::Empty; 6]; 6];
        let mut i = 0;
        while i < 6 {
            let mut j = 0;
            while j < 6 {
                board[i][j] = match arr[i][j] {
                    0 => BoardCellState::Empty,
                    1 => BoardCellState::White,
                    2 => BoardCellState::Black,
                    _ => panic!("Invalid cell state specification"),
                };
                j += 1;
            }
            i += 1;
        }
        Board(board)
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

    #[rustfmt::skip]
    /// Initial Reversi configuration.
    pub const INITIAL: Board = Board::from_012_array([
        /*       A  B  C  D  E  F */
        /* 1 */ [0, 0, 0, 0, 0, 0],
        /* 2 */ [0, 0, 0, 0, 0, 0],
        /* 3 */ [0, 0, 1, 2, 0, 0],
        /* 4 */ [0, 0, 2, 1, 0, 0],
        /* 5 */ [0, 0, 0, 0, 0, 0],
        /* 6 */ [0, 0, 0, 0, 0, 0],
    ]);

    pub fn to_string_block(&self) -> String {
        let mut result = String::new();
        result.push_str("   ABCDEF\n");
        result.push_str("   ------\n");
        for (y, row) in self.0.iter().enumerate() {
            result.push_str(&format!("{:1} |", y));
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
}

pub mod example_boards {
    use super::Board;

    #[rustfmt::skip]
    pub const REACHABLE_1_STEP: Board = Board::from_012_array([
        /*       A  B  C  D  E  F */
        /* 1 */ [0, 0, 0, 0, 0, 0],
        /* 2 */ [0, 0, 0, 0, 0, 0],
        /* 3 */ [0, 2, 2, 2, 0, 0], // B3
        /* 4 */ [0, 0, 2, 1, 0, 0],
        /* 5 */ [0, 0, 0, 0, 0, 0],
        /* 6 */ [0, 0, 0, 0, 0, 0],
    ]);

    #[rustfmt::skip]
    pub const UNREACHABLE_2_STEPS: Board = Board::from_012_array([
        /*       A  B  C  D  E  F */
        /* 1 */ [0, 0, 0, 0, 0, 0],
        /* 2 */ [0, 0, 0, 0, 0, 0],
        /* 3 */ [0, 2, 2, 1, 1, 0], // must have B3E3 as the game record but B3E3 does not lead to this state
        /* 4 */ [0, 0, 2, 1, 0, 0],
        /* 5 */ [0, 0, 0, 0, 0, 0],
        /* 6 */ [0, 0, 0, 0, 0, 0],
    ]);

    #[rustfmt::skip]
    pub const UNREACHABLE_BROKEN: Board = Board::from_012_array([
        [1, 1, 1, 1, 1, 1],
        [0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0], // center cells are never empty in a valid game
        [0, 0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0, 0],
        [2, 2, 2, 2, 2, 2],
    ]);
}
