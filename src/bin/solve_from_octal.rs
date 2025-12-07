use std::env;
use std::process;

use reversi_path_finder::board::{Board, PlacementMask};
use reversi_path_finder::reachability_problem::{
    ReachabilityProblem, ReachabilitySolver, ReachabilitySolverResult,
};
use reversi_path_finder::yices2_kissat_reachability_solver::new_yices2_kissat_reachability_solver;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        eprintln!(
            "Usage: {} <white-board-octal> <black-board-octal> <black-mask-octal> <white-mask-octal>",
            args[0]
        );
        process::exit(1);
    }

    let white_board_octal = &args[1];
    let black_board_octal = &args[2];
    let black_mask_octal = &args[3];
    let white_mask_octal = &args[4];

    let target_board = Board::from_octal_strings(white_board_octal, black_board_octal);
    let black_mask = PlacementMask::from_octal_string(black_mask_octal);
    let white_mask = PlacementMask::from_octal_string(white_mask_octal);

    let instance = ReachabilityProblem::new(target_board, black_mask, white_mask);

    let mut solver = new_yices2_kissat_reachability_solver();
    match solver.solve(&instance) {
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
                progression.to_game_record_string()
            );
        }
    }
}
