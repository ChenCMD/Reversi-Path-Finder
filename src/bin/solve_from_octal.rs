use std::env;
use std::process;

use reversi_path_finder::board::{Board, PlacementMask, PlayerColor};
use reversi_path_finder::game::INITIAL_BOARD;
use reversi_path_finder::reachability_problem::{
    ReachabilityProblem, ReachabilitySolver, ReachabilitySolverResult,
};
use reversi_path_finder::yices2_kissat_reachability_solver::{
    new_yices2_kissat_reachability_solver, GameTrace, SolverTrace,
};

fn player_to_string(player: &PlayerColor) -> &'static str {
    match player {
        PlayerColor::Black => "Black",
        PlayerColor::White => "White",
    }
}

fn debug_invalid_solution(
    game_trace: &GameTrace,
    solver_trace: &SolverTrace,
    black_mask: &PlacementMask,
    white_mask: &PlacementMask,
) {
    println!("   → ❌ WARNING: Solution validation FAILED!");
    println!("\n=== SMT SOLVER TRACE (What the solver thinks happened) ===\n");
    println!("SMT Solver generated {} steps\n", game_trace.steps.len());

    println!("=== Initial Board ===");
    print!("{}", INITIAL_BOARD.to_visual_string_block());
    println!();

    for (i, step) in game_trace.steps.iter().enumerate() {
        println!("=== Step {} ===", i + 1);
        println!("Player: {}", player_to_string(&step.player));
        println!("Move: {}", step.move_cell.to_string());
        println!("Pass: {}", step.is_pass);

        // Check placement mask
        let mask_ok = match step.player {
            PlayerColor::Black => black_mask.can_place_at_cell(step.move_cell),
            PlayerColor::White => white_mask.can_place_at_cell(step.move_cell),
        };

        if !step.is_pass && !mask_ok {
            println!(
                "  ❌ MASK VIOLATION: {} cannot place at {}",
                player_to_string(&step.player),
                step.move_cell.to_string()
            );
        }

        // Check if player had available moves
        let available_moves = step.board_before.moves_available(&step.player);
        let has_moves_actual = !available_moves.is_empty();
        let has_moves_solver = solver_trace.has_moves[i];

        println!("  Solver thinks player has moves: {}", has_moves_solver);
        println!("  Actual available moves: {}", available_moves.len());

        if has_moves_solver != has_moves_actual {
            println!(
                "  ❌ MISMATCH: Solver thinks has_moves={}, but actually has {} moves!",
                has_moves_solver,
                available_moves.len()
            );
        }

        if step.is_pass && has_moves_actual {
            println!(
                "  ❌ INVALID PASS: Player passed but had {} available moves!",
                available_moves.len()
            );
        } else if !step.is_pass && !has_moves_actual {
            println!("  ❌ MISSING PASS: Player has no moves but didn't pass!");
        }

        println!();
    }

    println!("=== Final Board (from SMT) ===");
    print!("{}", game_trace.final_board.to_visual_string_block());
}

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
        ReachabilitySolverResult::Unreachable(_) => {
            println!("   → The position is NOT REACHABLE\n");
        }
        ReachabilitySolverResult::Unknown => {
            println!("   → UNKNOWN - Solver could not determine reachability\n");
        }
        ReachabilitySolverResult::Reachable(progression, trace) => {
            println!(
                "   → The position is REACHABLE. Progression: {}",
                progression.to_game_record_string()
            );

            if !instance.admits_as_solution(&progression) {
                let game_trace = GameTrace::from_solver_trace(&trace);
                debug_invalid_solution(&game_trace, &trace, &black_mask, &white_mask);
                process::exit(1);
            }

            println!("   → ✓ Solution validated successfully\n");
        }
    }
}
