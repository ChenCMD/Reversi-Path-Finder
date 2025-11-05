extern crate reversi_path_finder;

use reversi_path_finder::{board::*, game::INITIAL_BOARD};

fn generate_game_record() -> Option<String> {
    let mut board = INITIAL_BOARD.clone();
    let mut current_player = PlayerColor::Black;

    let mut hands_played: Vec<CellCoord> = Vec::new();

    while hands_played.len() < 32 {
        let actual_player = if board.moves_available(&current_player).is_empty() {
            current_player.opponent()
        } else {
            current_player
        };

        let available_moves = board.moves_available(&actual_player);
        if available_moves.is_empty() {
            // No moves available for either player; game over.
            return None;
        }

        let chosen_move = available_moves[(rand::random::<u8>() as usize) % available_moves.len()];
        board = board
            .place_disk(*chosen_move.column(), *chosen_move.row(), &actual_player)
            .unwrap();
        hands_played.push(chosen_move);
        current_player = actual_player.opponent();
    }

    Some(
        hands_played
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join(""),
    )
}

fn main() {
    println!("Generating 30 random game records:");

    for _ in 0..30 {
        let record;
        loop {
            if let Some(gr) = generate_game_record() {
                record = gr;
                break;
            }
        }
        println!("  {}", record);
    }
}
