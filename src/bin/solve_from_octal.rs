use std::env;
use std::process;

use reversi_path_finder::board::{Board, PlacementMask};
use reversi_path_finder::reachability_problem::{
    ReachabilityProblem, ReachabilitySolver, ReachabilitySolverResult,
};
use reversi_path_finder::yices2_kissat_reachability_solver::new_yices2_kissat_reachability_solver;
use serde_json::json;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        let payload = json!({
            "bin": "solve_from_octal",
            "status": "error",
            "error": "usage",
            "usage": format!("{} <white-board-octal> <black-board-octal> <black-mask-octal> <white-mask-octal>", args[0]),
        });
        println!("{}", serde_json::to_string(&payload).unwrap());
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
    let input = json!({
        "white_board_octal": white_board_octal,
        "black_board_octal": black_board_octal,
        "black_mask_octal": black_mask_octal,
        "white_mask_octal": white_mask_octal,
    });

    match solver.solve(&instance) {
        ReachabilitySolverResult::Unreachable(_) => {
            let payload = json!({
                "bin": "solve_from_octal",
                "status": "unreachable",
                "input": input,
            });
            println!("{}", serde_json::to_string(&payload).unwrap());
        }
        ReachabilitySolverResult::Unknown => {
            let payload = json!({
                "bin": "solve_from_octal",
                "status": "unknown",
                "input": input,
            });
            println!("{}", serde_json::to_string(&payload).unwrap());
        }
        ReachabilitySolverResult::Reachable(progression, trace) => {
            let progression_str = progression.to_game_record_string();
            if !instance.admits_as_solution(&progression) {
                let payload = json!({
                    "bin": "solve_from_octal",
                    "status": "error",
                    "error": "invalid_progression",
                    "input": input,
                    "progression": progression_str,
                    "solver_trace_steps": trace.is_black_turns.len(),
                });
                println!("{}", serde_json::to_string(&payload).unwrap());
                process::exit(1);
            }

            let payload = json!({
                "bin": "solve_from_octal",
                "status": "reachable",
                "input": input,
                "progression": progression_str,
            });
            println!("{}", serde_json::to_string(&payload).unwrap());
        }
    }
}
