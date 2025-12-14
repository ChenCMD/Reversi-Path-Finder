use rand::seq::{IndexedRandom, SliceRandom};
use reversi_path_finder::board::{Board, PlacementMask, PlayerColor};
use reversi_path_finder::game::INITIAL_BOARD;

fn main() {
    let (black_mask, white_mask, target_board) = loop {
        let (black_mask, white_mask) = generate_balanced_disjoint_masks();
        let board = play_random_game_with_masks(&black_mask, &white_mask);
        if board.filled_cells_count() == 36 {
            break (black_mask, white_mask, board);
        }
    };

    println!("Generated reachable instance for solve_from_octal:");
    println!(
        "cargo run --bin solve_from_octal -- {} {} {} {}",
        target_board.white_to_octal_string(),
        target_board.black_to_octal_string(),
        black_mask.to_octal_string(),
        white_mask.to_octal_string()
    );
    println!(
        "\nVisual board:\n{}",
        target_board
            .to_string_block()
            .split('\n')
            .map(|line| format!("\t{}", line))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Create masks where every cell is assigned to exactly one player and totals are balanced (18 each).
fn generate_balanced_disjoint_masks() -> (PlacementMask, PlacementMask) {
    let excluded = [(2, 2), (3, 2), (2, 3), (3, 3)]; // initial center 4
    let mut coords: Vec<(usize, usize)> = (0..6)
        .flat_map(|y| (0..6).map(move |x| (x, y)))
        .filter(|c| !excluded.contains(c))
        .collect();
    coords.shuffle(&mut rand::rng());

    let mut black = [[false; 6]; 6];
    let mut white = [[false; 6]; 6];
    let split = coords.len() / 2; // should be 16 with 32 usable cells
    for (i, (x, y)) in coords.into_iter().enumerate() {
        if i < split {
            black[y][x] = true;
        } else {
            white[y][x] = true;
        }
    }

    (PlacementMask(black), PlacementMask(white))
}

/// Play a random game that respects the provided placement masks.
fn play_random_game_with_masks(black_mask: &PlacementMask, white_mask: &PlacementMask) -> Board {
    let mut board = INITIAL_BOARD.clone();
    let mut current_player = PlayerColor::Black;

    // Play until neither side can move; cap at 36 plies.
    for _ in 0..36 {
        let moves_current =
            moves_available_with_mask(&board, &current_player, black_mask, white_mask);
        let (actual_player, moves_actual) = if moves_current.is_empty() {
            let opp = current_player.opponent();
            let opp_moves = moves_available_with_mask(&board, &opp, black_mask, white_mask);
            if opp_moves.is_empty() {
                break;
            }
            (opp, opp_moves)
        } else {
            (current_player, moves_current)
        };

        let chosen = moves_actual
            .choose(&mut rand::rng())
            .expect("moves_actual is empty")
            .clone();

        board = board
            .place_disk(*chosen.column(), *chosen.row(), &actual_player)
            .expect("move should be legal");
        current_player = actual_player.opponent();
    }

    board
}

fn moves_available_with_mask(
    board: &Board,
    player: &PlayerColor,
    black_mask: &PlacementMask,
    white_mask: &PlacementMask,
) -> Vec<reversi_path_finder::board::CellCoord> {
    let mask = match player {
        PlayerColor::Black => black_mask,
        PlayerColor::White => white_mask,
    };
    board
        .moves_available(player)
        .into_iter()
        .filter(|cell| mask.can_place_at_cell(*cell))
        .collect()
}
