use rand::seq::IndexedRandom;
use reversi_path_finder::board::{Board, PlacementMask};
use reversi_path_finder::game::INITIAL_BOARD;
use reversi_path_finder::reachability_problem::{
    ReachabilityProblem, ReachabilitySolver, ReachabilitySolverResult,
};
use reversi_path_finder::yices2_kissat_reachability_solver::new_yices2_kissat_reachability_solver;

fn randomly_play_in_conformance_to_masks(
    black_mask: &PlacementMask,
    white_mask: &PlacementMask,
) -> Board {
    let mask_for = |player: &reversi_path_finder::board::PlayerColor| match player {
        reversi_path_finder::board::PlayerColor::Black => black_mask,
        reversi_path_finder::board::PlayerColor::White => white_mask,
    };

    let moves_available_for = |board: &Board, player: &reversi_path_finder::board::PlayerColor| {
        board
            .moves_available(player)
            .into_iter()
            .filter(|cell| mask_for(player).can_place_at_cell(*cell))
            .collect::<Vec<_>>()
    };

    'outer: loop {
        let mut board = INITIAL_BOARD.clone();
        let mut current_player = reversi_path_finder::board::PlayerColor::Black;

        for _ in 0..32 {
            let moves_available_current = moves_available_for(&board, &current_player);

            let actual_player = if moves_available_current.is_empty() {
                current_player.opponent()
            } else {
                current_player
            };

            let randomly_picked_move = {
                let moves_available_actual = if actual_player == current_player {
                    moves_available_current
                } else {
                    moves_available_for(&board, &current_player.opponent())
                };

                if moves_available_actual.is_empty() {
                    continue 'outer;
                }

                moves_available_actual
                    .choose(&mut rand::rng())
                    .unwrap()
                    .clone()
            };

            board = board
                .place_disk(
                    *randomly_picked_move.column(),
                    *randomly_picked_move.row(),
                    &actual_player,
                )
                .unwrap();
            current_player = actual_player.opponent();
        }

        return board;
    }
}

fn main() {
    let mut solver = new_yices2_kissat_reachability_solver();

    let instance = {
        let black_mask = PlacementMask::from_octal_string("737777737752");
        let white_mask = PlacementMask::from_octal_string("377675777677");
        ReachabilityProblem::new(
            randomly_play_in_conformance_to_masks(&black_mask, &white_mask),
            black_mask,
            white_mask,
        )
    };

    print_instance(&instance);

    let result = solver.solve(&instance);

    match result {
        ReachabilitySolverResult::Unreachable(_) => {
            println!("   → The position is NOT REACHABLE\n");
        }
        ReachabilitySolverResult::Unknown => {
            println!("   → UNKNOWN - Solver could not determine reachability\n");
        }
        ReachabilitySolverResult::Reachable(progression, _) => {
            assert!(instance.admits_as_solution(&progression));
            println!(
                "   → Successfully found a progression: {}\n",
                &progression.to_game_record_string()
            );
        }
    }
}

fn print_instance(instance: &ReachabilityProblem) {
    println!(
        "Target board state ({}:{}):\n{}",
        instance.target_board.white_to_octal_string(),
        instance.target_board.black_to_octal_string(),
        instance
            .target_board
            .to_string_block()
            .split("\n")
            .map(|line| format!("\t{}", line))
            .collect::<Vec<_>>()
            .join("\n")
    );
    println!(
        "Black placement mask ({}):\n{}",
        instance.black_placement_mask.to_octal_string(),
        instance
            .black_placement_mask
            .to_string_block()
            .split("\n")
            .map(|line| format!("\t{}", line))
            .collect::<Vec<_>>()
            .join("\n"),
    );
    println!(
        "White placement mask ({}):\n{}",
        instance.white_placement_mask.to_octal_string(),
        instance
            .white_placement_mask
            .to_string_block()
            .split("\n")
            .map(|line| format!("\t{}", line))
            .collect::<Vec<_>>()
            .join("\n"),
    );
}
