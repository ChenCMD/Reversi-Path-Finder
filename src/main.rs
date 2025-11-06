use rayon::prelude::*;
use reversi_path_finder::board::{Board, PlacementMask};
use reversi_path_finder::reachability_problem::{
    ReachabilityProblem, ReachabilitySolver, ReachabilitySolverResult,
};
use reversi_path_finder::yices2_kissat_reachability_solver::new_yices2_kissat_reachability_solver;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn randomly_generate_end_state() -> Board {
    let mut board_012_array = [[0u8; 6]; 6];
    for i in 0..36 {
        board_012_array[i / 6][i % 6] = (rand::random::<u8>() % 2) + 1;
    }
    Board::from_012_array(board_012_array)
}

fn main() {
    // Configure rayon to use half the number of CPU cores
    let num_threads = (num_cpus::get() / 2).max(1);
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .unwrap();

    println!("Using {} threads for parallel search\n", num_threads);

    let found = Arc::new(AtomicBool::new(false));

    // Use rayon to search in parallel across infinite attempts
    // std::iter::repeat creates an infinite iterator, par_bridge makes it parallel
    let result = std::iter::repeat(()).par_bridge().find_map_any(|_| {
        // Check if another thread already found a solution
        if found.load(Ordering::Relaxed) {
            return None;
        }

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
                None
            }
            ReachabilitySolverResult::Unknown => {
                println!("   → UNKNOWN - Solver could not determine reachability\n");
                None
            }
            ReachabilitySolverResult::Reachable(progression) => {
                assert!(instance.admits_as_solution(&progression));
                found.store(true, Ordering::Relaxed);
                Some((instance, progression))
            }
        }
    });

    if let Some((_instance, progression)) = result {
        println!(
            "   → Successfully found a progression: {}\n",
            &progression.to_game_record_string()
        );
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
