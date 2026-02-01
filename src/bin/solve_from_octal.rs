use std::env;
use std::process;

use reversi_path_finder::board::{Board, CellCoord, PlacementMask};
use reversi_path_finder::game::initial_board_at;
use reversi_path_finder::reachability_problem::{
    ReachabilityProblem, ReachabilitySolver, ReachabilitySolverResult,
};
use reversi_path_finder::yices2_kissat_reachability_solver::new_yices2_kissat_reachability_solver;
use serde_json::json;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 6 {
        let payload = json!({
            "bin": "solve_from_octal",
            "status": "error",
            "error": "usage",
            "usage": format!("{} <origin> <white-board-octal> <black-board-octal> <black-mask-octal> <white-mask-octal>", args[0]),
        });
        println!("{}", serde_json::to_string(&payload).unwrap());
        process::exit(1);
    }

    let origin_cell = parse_cell(&args[1]);
    let white_board_octal = &args[2];
    let black_board_octal = &args[3];
    let black_mask_octal = &args[4];
    let white_mask_octal = &args[5];

    let target_board = Board::from_octal_strings(white_board_octal, black_board_octal);
    let black_mask = PlacementMask::from_octal_string(black_mask_octal);
    let white_mask = PlacementMask::from_octal_string(white_mask_octal);
    let initial_board = initial_board_at(origin_cell);

    let instance =
        ReachabilityProblem::new(target_board, black_mask, white_mask, initial_board.clone());

    let mut solver = new_yices2_kissat_reachability_solver();
    let input = json!({
        "white_board_octal": white_board_octal,
        "black_board_octal": black_board_octal,
        "black_mask_octal": black_mask_octal,
        "white_mask_octal": white_mask_octal,
        "origin": origin_cell.to_string(),
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
            let progression_with_passes = {
                let mut board = initial_board.clone();
                let mut current_player = reversi_path_finder::board::PlayerColor::Black;
                let mut with_passes = String::new();

                for cell in progression.to_moves(
                    &instance.black_placement_mask,
                    &instance.white_placement_mask,
                    &instance.initial_board,
                ) {
                    let moves_current = board
                        .moves_available(&current_player)
                        .into_iter()
                        .filter(|mv| {
                            let mask = match current_player {
                                reversi_path_finder::board::PlayerColor::Black => {
                                    &instance.black_placement_mask
                                }
                                reversi_path_finder::board::PlayerColor::White => {
                                    &instance.white_placement_mask
                                }
                            };
                            mask.can_place_at_cell(*mv)
                        })
                        .collect::<Vec<_>>();
                    if moves_current.is_empty() {
                        with_passes.push_str("--");
                    }

                    with_passes.push_str(&cell.cell.to_string());
                    board = board
                        .place_disk(*cell.cell.column(), *cell.cell.row(), &cell.player)
                        .expect("place_disk failed");
                    current_player = cell.player.opponent();
                }

                with_passes
            };
            if !instance.admits_as_solution(&progression) {
                let payload = json!({
                    "bin": "solve_from_octal",
                    "status": "error",
                    "error": "invalid_progression",
                    "input": input,
                    "progression": progression_with_passes,
                    "solver_trace_steps": trace.is_black_turns.len(),
                });
                println!("{}", serde_json::to_string(&payload).unwrap());
                process::exit(1);
            }

            let payload = json!({
                "bin": "solve_from_octal",
                "status": "reachable",
                "input": input,
                "progression": progression_with_passes,
            });
            println!("{}", serde_json::to_string(&payload).unwrap());
        }
    }
}

fn parse_cell(s: &str) -> CellCoord {
    let bytes = s.as_bytes();
    if bytes.len() != 2 {
        panic!("Origin must be like C3");
    }
    let col = bytes[0].to_ascii_uppercase() - b'A';
    let row = bytes[1] - b'1';
    if col >= 6 || row >= 6 {
        panic!("Origin out of board: {}", s);
    }
    if col > 4 || row > 4 {
        panic!("Origin must allow 2x2 block to fit (A1-E5): {}", s);
    }
    CellCoord::new(col, row)
}
