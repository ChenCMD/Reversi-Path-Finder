use std::env;
use std::process;

use reversi_path_finder::board::{Board, CellCoord, PlacementMask, PlayerColor};
use reversi_path_finder::game::{INITIAL_BOARD, UncheckedGameProgression};
use reversi_path_finder::reachability_problem::ReachabilityProblem;

fn player_to_string(player: &PlayerColor) -> &'static str {
    match player {
        PlayerColor::Black => "Black",
        PlayerColor::White => "White",
    }
}

fn cell_in_list(cell: &CellCoord, list: &[CellCoord]) -> bool {
    list.iter()
        .any(|c| *c.column() == *cell.column() && *c.row() == *cell.row())
}

fn print_placement_mask_visual(mask: &PlacementMask, name: &str) {
    println!("{} (✓ = allowed, ✗ = forbidden):", name);
    println!("   A B C D E F");
    for row in 0..6 {
        print!("{} ", row + 1);
        for col in 0..6 {
            let cell = CellCoord::new(col, row);
            if mask.can_place_at_cell(cell) {
                print!(" ✓");
            } else {
                print!(" ✗");
            }
        }
        println!();
    }
}

fn print_board_visual(board: &Board) {
    use reversi_path_finder::board::BoardCellState;

    println!("   A B C D E F");
    for row in 0..6 {
        print!("{} ", row + 1);
        for col in 0..6 {
            let cell_state = board.0[row][col];
            let symbol = match cell_state {
                BoardCellState::Empty => "·",
                BoardCellState::Black => "○",
                BoardCellState::White => "●",
            };
            print!(" {}", symbol);
        }
        println!();
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 6 {
        eprintln!(
            "Usage: {} <white-board-octal> <black-board-octal> <black-mask-octal> <white-mask-octal> <progression>",
            args[0]
        );
        process::exit(1);
    }

    let white_board_octal = &args[1];
    let black_board_octal = &args[2];
    let black_mask_octal = &args[3];
    let white_mask_octal = &args[4];
    let progression_str = &args[5];

    let target_board = Board::from_octal_strings(white_board_octal, black_board_octal);
    let black_mask = PlacementMask::from_octal_string(black_mask_octal);
    let white_mask = PlacementMask::from_octal_string(white_mask_octal);

    let instance = ReachabilityProblem::new(target_board, black_mask, white_mask);

    let progression = UncheckedGameProgression::from_game_record_string(progression_str);

    println!("=== DEBUGGING SOLUTION ===\n");
    println!("Progression: {}\n", progression_str);
    println!("Target board:");
    print_board_visual(&target_board);
    println!("\n=== STEPPING THROUGH MOVES ===\n");

    let mut board = INITIAL_BOARD.clone();
    let mut current_player = PlayerColor::Black;
    let mut move_number = 0;

    println!("Initial board:");
    print_board_visual(&board);
    println!();

    // Parse the progression manually to step through it
    for move_str in progression_str.as_bytes().chunks(2) {
        let column = move_str[0] - b'A';
        let row = move_str[1] - b'1';
        let cell = CellCoord::new(column, row);

        move_number += 1;

        // Determine actual player (accounting for skipped turns)
        let actual_player = if board.moves_available(&current_player).is_empty() {
            println!("  [Turn skipped for {}]", player_to_string(&current_player));
            current_player.opponent()
        } else {
            current_player
        };

        println!(
            "Move {}: {} by {}",
            move_number,
            std::str::from_utf8(move_str).unwrap(),
            player_to_string(&actual_player)
        );

        // Check if move respects placement mask
        let mask_ok = match actual_player {
            PlayerColor::Black => black_mask.can_place_at_cell(cell),
            PlayerColor::White => white_mask.can_place_at_cell(cell),
        };

        if !mask_ok {
            println!(
                "  ❌ MASK VIOLATION: {} cannot place at {} (not in placement mask)",
                player_to_string(&actual_player),
                std::str::from_utf8(move_str).unwrap()
            );
            println!("\n=== PLACEMENT MASKS ===\n");
            print_placement_mask_visual(&black_mask, "Black placement mask");
            println!();
            print_placement_mask_visual(&white_mask, "White placement mask");
            process::exit(1);
        }

        // Check if move is legal
        let available_moves = board.moves_available(&actual_player);
        if !cell_in_list(&cell, &available_moves) {
            println!(
                "  ❌ ILLEGAL MOVE: {} is not a valid move for {}",
                std::str::from_utf8(move_str).unwrap(),
                player_to_string(&actual_player)
            );
            println!(
                "  Available moves: {:?}",
                available_moves
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
            );
            println!("\n  Current board:");
            print_board_visual(&board);
            process::exit(1);
        }

        // Try to place the disk
        match board.place_disk(column, row, &actual_player) {
            Some(new_board) => {
                board = new_board;
                println!("  ✓ Move successful");
                print_board_visual(&board);
                println!();
                current_player = actual_player.opponent();
            }
            None => {
                println!("  ❌ MOVE FAILED: place_disk returned None");
                println!("\n  Current board:");
                print_board_visual(&board);
                process::exit(1);
            }
        }
    }

    println!("\n=== FINAL VALIDATION ===\n");
    println!("Final board after playing through:");
    print_board_visual(&board);
    println!("\nTarget board:");
    print_board_visual(&target_board);

    if board == target_board {
        println!("\n✓ Final board MATCHES target board");
    } else {
        println!("\n❌ Final board DOES NOT MATCH target board");
        process::exit(1);
    }

    // Now check using the official method
    println!("\n=== OFFICIAL VALIDATION ===\n");
    if instance.admits_as_solution(&progression) {
        println!("✓ Solution is valid according to admits_as_solution()");
    } else {
        println!("❌ Solution is INVALID according to admits_as_solution()");
        process::exit(1);
    }

    println!("\n=== SUCCESS ===");
    println!("The progression is completely valid!");
}
