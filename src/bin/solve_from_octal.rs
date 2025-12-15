use std::env;
use std::process;

use reversi_path_finder::board::{Board, PlacementMask, PlayerColor};
use reversi_path_finder::reachability_problem::{
    ReachabilityProblem, ReachabilitySolver, ReachabilitySolverResult,
};
use reversi_path_finder::yices2_kissat_reachability_solver::{
    GameTrace, SolverTrace, new_yices2_kissat_reachability_solver,
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

    for (i, step) in game_trace.steps.iter().enumerate() {
        println!("=== Step {} ===", i + 1);

        print!("{}", step.board_before.to_visual_string_block());

        // Determine whose turn it was (from solver's perspective)
        let turn_is_black = if i < solver_trace.is_black_turns.len() {
            solver_trace.is_black_turns[i]
        } else {
            true // First move is Black
        };
        let turn_player = if turn_is_black {
            PlayerColor::Black
        } else {
            PlayerColor::White
        };

        println!("Turn: {}", player_to_string(&turn_player));
        if step.is_pass {
            println!(
                "  {} passed, so {} makes the move",
                player_to_string(&turn_player),
                player_to_string(&step.player)
            );
        }
        println!("Move made by: {}", player_to_string(&step.player));
        println!("Move: {}", step.move_cell.to_string());

        // Check placement mask for the player who actually made the move
        let mask_ok = match step.player {
            PlayerColor::Black => black_mask.can_place_at_cell(step.move_cell),
            PlayerColor::White => white_mask.can_place_at_cell(step.move_cell),
        };

        if !mask_ok {
            println!(
                "  ❌ MASK VIOLATION: {} cannot place at {}",
                player_to_string(&step.player),
                step.move_cell.to_string()
            );
        }

        // Check if the TURN player (who might have passed) had available moves
        let turn_player_mask = match turn_player {
            PlayerColor::Black => black_mask,
            PlayerColor::White => white_mask,
        };
        let turn_player_moves: Vec<_> = step
            .board_before
            .moves_available(&turn_player)
            .into_iter()
            .filter(|cell| turn_player_mask.can_place_at_cell(*cell))
            .collect();
        let has_moves_actual = !turn_player_moves.is_empty();
        let has_moves_solver = solver_trace.has_moves[i];

        println!(
            "  {} has {} available moves (solver thinks: {})",
            player_to_string(&turn_player),
            turn_player_moves.len(),
            has_moves_solver
        );

        if has_moves_solver != has_moves_actual {
            println!(
                "  ❌ MISMATCH: Solver thinks {} has_moves={}, but actually has {} moves!",
                player_to_string(&turn_player),
                has_moves_solver,
                turn_player_moves.len()
            );
        }

        if step.is_pass && has_moves_actual {
            println!(
                "  ❌ INVALID PASS: {} passed but had {} available moves!",
                player_to_string(&turn_player),
                turn_player_moves.len()
            );
        } else if !step.is_pass && !has_moves_actual {
            println!(
                "  ❌ MISSING PASS: {} has no moves but didn't pass!",
                player_to_string(&turn_player)
            );
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
