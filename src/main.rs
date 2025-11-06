use reversi_path_finder::board::{Board, PlacementMask};
use reversi_path_finder::reachability_problem::{
    ReachabilityProblem, ReachabilitySolver, ReachabilitySolverResult,
};
use reversi_path_finder::yices2_kissat_reachability_solver::new_yices2_kissat_reachability_solver;

fn randomly_generate_end_state() -> Board {
    let mut board_012_array = [[0u8; 6]; 6];
    for i in 0..36 {
        board_012_array[i / 6][i % 6] = (rand::random::<u8>() % 2) + 1;
    }
    Board::from_012_array(board_012_array)
}

fn main() {
    loop {
        let mut solver = new_yices2_kissat_reachability_solver();

        let instance = ReachabilityProblem::new(
            randomly_generate_end_state(),
            PlacementMask::from_octal_string("777777737773"),
            PlacementMask::from_octal_string("777677777677"),
        );

        print_instance(&instance);

        let result = solver.solve(&instance);

        match result {
            ReachabilitySolverResult::Unreachable => {
                println!("   → The position is NOT REACHABLE\n");
            }
            ReachabilitySolverResult::Unknown => {
                println!("   → UNKNOWN - Solver could not determine reachability\n");
            }
            ReachabilitySolverResult::Reachable(progression) => {
                assert!(instance.admits_as_solution(&progression));
                println!(
                    "   → Successfully found a progression: {}\n",
                    &progression.to_game_record_string()
                );
                return;
            }
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
