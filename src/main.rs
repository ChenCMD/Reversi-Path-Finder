use easy_smt::{ContextBuilder, Response, SExpr};
use std::time::Instant;

use reversi_path_finder::board::*;
use reversi_path_finder::game::*;

fn init_yices2_ctx() -> easy_smt::Context {
    ContextBuilder::new()
        .solver("yices-smt2")
        .solver_args(["--delegate=kissat"])
        .build()
        .expect("Failed to create SMT context with Yices2")
}

// ============================================================================
// SMT Constraint Generation with Bitboards
// ============================================================================

/// Helper function to compute which pieces would be flipped in a specific direction.
/// Returns a bitvector mask of pieces that would be flipped.
fn compute_flips_in_direction(
    ctx: &mut easy_smt::Context,
    move_mask: SExpr,
    player_board: SExpr,
    opponent_board: SExpr,
    shift_amount: i32,
    edge_mask: u64,
) -> SExpr {
    let zero_bv = ctx.binary(36, 0);
    let edge_mask_expr = ctx.binary(36, edge_mask);

    // Function to shift in the direction (handles both positive and negative shifts)
    let shift_fn = |pos: SExpr| -> SExpr {
        if shift_amount > 0 {
            ctx.bvshl(pos, ctx.binary(36, shift_amount as u64))
        } else {
            ctx.bvlshr(pos, ctx.binary(36, (-shift_amount) as u64))
        }
    };

    // Start from the move position
    let mut current = move_mask;

    // Collect positions that might flip
    let mut candidate_flips = Vec::new();

    // Traverse up to 5 steps (max distance on 6x6 board)
    for _ in 0..5 {
        // Apply edge mask BEFORE shifting to prevent wrapping
        current = ctx.bvand(current, ctx.bvnot(edge_mask_expr));
        current = shift_fn(current);
        candidate_flips.push(current);
    }

    // Now check which prefix of candidate_flips forms a valid sandwich
    // We need: opponent+, player (where + means 1 or more)
    let mut result = zero_bv;
    let mut accumulated_opponents = zero_bv;

    // Track whether the chain is still valid (not broken by empty cell or player piece)
    let mut chain_valid = ctx.eq(zero_bv, zero_bv); // Start with true

    for candidate_pos in candidate_flips {
        // Check what's at this position
        let has_opponent = ctx.distinct(ctx.bvand(candidate_pos, opponent_board), zero_bv);
        let has_player = ctx.distinct(ctx.bvand(candidate_pos, player_board), zero_bv);

        // Only accumulate opponents if chain is still valid
        accumulated_opponents = ctx.ite(
            ctx.and(chain_valid, has_opponent),
            ctx.bvor(accumulated_opponents, candidate_pos),
            accumulated_opponents,
        );

        // Finalize flips if chain valid and we hit a player cell
        result = ctx.ite(
            ctx.and(chain_valid, has_player),
            accumulated_opponents,
            result,
        );

        // Keep chain valid only while we see opponent cells
        chain_valid = ctx.and(chain_valid, has_opponent);
    }

    result
}

/// Computes all pieces that would be flipped by a move.
fn compute_all_flips(
    ctx: &mut easy_smt::Context,
    move_mask: SExpr,
    player_board: SExpr,
    opponent_board: SExpr,
) -> SExpr {
    // Edge masks for 6x6 board to prevent wrapping
    const LEFT_EDGE: u64 = 0x041041041; // Column A: bits 0,6,12,18,24,30
    const RIGHT_EDGE: u64 = 0x820820820; // Column F: bits 5,11,17,23,29,35

    // Compute flips in all 8 directions
    // North (-6): no left/right edge issues
    let flips_n = compute_flips_in_direction(ctx, move_mask, player_board, opponent_board, -6, 0);

    // South (+6): no left/right edge issues
    let flips_s = compute_flips_in_direction(ctx, move_mask, player_board, opponent_board, 6, 0);

    // East (+1): can't wrap from right edge
    let flips_e =
        compute_flips_in_direction(ctx, move_mask, player_board, opponent_board, 1, RIGHT_EDGE);

    // West (-1): can't wrap from left edge
    let flips_w =
        compute_flips_in_direction(ctx, move_mask, player_board, opponent_board, -1, LEFT_EDGE);

    // NE (-5 = -6+1): can't wrap from right edge
    let flips_ne =
        compute_flips_in_direction(ctx, move_mask, player_board, opponent_board, -5, RIGHT_EDGE);

    // NW (-7 = -6-1): can't wrap from left edge
    let flips_nw =
        compute_flips_in_direction(ctx, move_mask, player_board, opponent_board, -7, LEFT_EDGE);

    // SE (+7 = +6+1): can't wrap from right edge
    let flips_se =
        compute_flips_in_direction(ctx, move_mask, player_board, opponent_board, 7, RIGHT_EDGE);

    // SW (+5 = +6-1): can't wrap from left edge
    let flips_sw =
        compute_flips_in_direction(ctx, move_mask, player_board, opponent_board, 5, LEFT_EDGE);

    // Combine all flips
    let mut total = flips_n;
    total = ctx.bvor(total, flips_s);
    total = ctx.bvor(total, flips_e);
    total = ctx.bvor(total, flips_w);
    total = ctx.bvor(total, flips_ne);
    total = ctx.bvor(total, flips_nw);
    total = ctx.bvor(total, flips_se);
    total = ctx.bvor(total, flips_sw);

    total
}

/// Checks if a player has any legal moves on the current board.
/// Returns a boolean SExpr that is true if at least one legal move exists.
fn has_legal_move(
    ctx: &mut easy_smt::Context,
    player_board: SExpr,
    opponent_board: SExpr,
    player_can_place: SExpr,
) -> SExpr {
    let zero_bv = ctx.binary(36, 0);
    let occupied = ctx.bvor(player_board, opponent_board);
    let empty_cells = ctx.bvnot(occupied);

    let mut legal_moves = Vec::new();

    // Check all 36 positions
    for pos in 0..36 {
        let pos_mask = ctx.binary(36, 1u64 << pos);

        // Check if position is empty
        let is_empty = ctx.distinct(ctx.bvand(empty_cells, pos_mask), zero_bv);

        // Check if this move would flip any pieces
        let flips = compute_all_flips(ctx, pos_mask, player_board, opponent_board);
        let has_flips = ctx.distinct(flips, zero_bv);

        let can_place = ctx.distinct(ctx.bvand(player_can_place, pos_mask), zero_bv);

        // This position is legal if it's empty AND flips pieces AND conforms to placement mask
        let is_legal = ctx.and(ctx.and(is_empty, has_flips), can_place);

        legal_moves.push(is_legal);
    }

    // Return true if any position is legal
    legal_moves.into_iter().reduce(|a, b| ctx.or(a, b)).unwrap()
}

/// Generates SMT constraints that model a valid Reversi game progression
/// using bitboard representation for efficient move and flip encoding.
///
/// # Arguments
/// * `ctx` - The SMT context to use for constraint generation
/// * `final_state` - The target board state to reach
/// * `black_pmask` - 6x6 array indicating where Black can place disks
/// * `white_pmask` - 6x6 array indicating where White can place disks
///
/// # Returns
/// A vector of SExpr constraints representing the game rules and progression
pub fn generate_reversi_constraints(
    ctx: &mut easy_smt::Context,
    final_state: &Board,
    black_pmask: PlacementMask,
    white_pmask: PlacementMask,
) -> (Vec<SExpr>, Vec<SExpr>) {
    let black_can_place_bv = {
        let mut bv: u64 = 0;
        for y in 0..6 {
            for x in 0..6 {
                if black_pmask.can_place(x, y) {
                    bv |= 1 << (y * 6 + x);
                }
            }
        }
        ctx.binary(36, bv)
    };
    let white_can_place_bv = {
        let mut bv: u64 = 0;
        for y in 0..6 {
            for x in 0..6 {
                if white_pmask.can_place(x, y) {
                    bv |= 1 << (y * 6 + x);
                }
            }
        }
        ctx.binary(36, bv)
    };

    let mut constraints = Vec::new();
    let mut move_positions = Vec::new();
    let max_moves = final_state.filled_cells_count() - /* initial 4 pieces */ 4;

    // Use 36-bit bitvectors for the 6x6 board (bits 0-35)
    let bv_sort = ctx.bit_vec_sort(ctx.numeral(36));
    let bool_sort = ctx.bool_sort();

    // Constants
    let zero_bv = ctx.binary(36, 0);

    // Variables describing the game state at each time step
    let mut black_boards = Vec::new();
    let mut white_boards = Vec::new();
    let mut is_black_turn = Vec::new();

    for t in 0..=max_moves {
        let black = ctx
            .declare_const(&format!("black_bitboard_{}", t), bv_sort)
            .unwrap();
        let white = ctx
            .declare_const(&format!("white_bitboard_{}", t), bv_sort)
            .unwrap();

        // Constraint: no bit should be set in both black and white
        {
            let overlap = ctx.bvand(black, white);
            constraints.push(ctx.eq(overlap, zero_bv));
        }

        black_boards.push(black);
        white_boards.push(white);

        if t < max_moves {
            let turn = ctx
                .declare_const(&format!("is_black_turn_{}", t), bool_sort)
                .unwrap();
            is_black_turn.push(turn);
        }
    }

    // Constraint: Initial board state
    {
        let BitBoard { black, white } = INITIAL_BOARD.to_bitboards();
        constraints.push(ctx.eq(black_boards[0], ctx.binary(36, black)));
        constraints.push(ctx.eq(white_boards[0], ctx.binary(36, white)));
    }

    // Constraint: Final board state
    {
        let BitBoard { black, white } = final_state.to_bitboards();
        constraints.push(ctx.eq(black_boards[max_moves], ctx.binary(36, black)));
        constraints.push(ctx.eq(white_boards[max_moves], ctx.binary(36, white)));
    }

    // Constraint: Black starts first
    if !is_black_turn.is_empty() {
        constraints.push(is_black_turn[0]);
    }

    // Game transition constraints between time steps
    for t in 0..max_moves {
        // Variables for this transition
        let move_pos = ctx
            .declare_const(&format!("move_pos_{}", t), ctx.bit_vec_sort(ctx.numeral(6)))
            .unwrap();
        move_positions.push(move_pos);
        let pass = ctx
            .declare_const(&format!("pass_{}", t), bool_sort)
            .unwrap();

        let black_plays = ctx.xor(is_black_turn[t], pass);

        let placement_mask = ctx.ite(black_plays, black_can_place_bv, white_can_place_bv);

        // Create one-hot move mask from move_pos
        let move_mask = {
            let zeros_30 = ctx.binary(30, 0);
            let move_pos_extended = ctx.concat(zeros_30, move_pos);
            ctx.bvshl(ctx.binary(36, 1), move_pos_extended)
        };

        // Get boards for actual player and opponent
        let player_board = ctx.ite(black_plays, black_boards[t], white_boards[t]);
        let opponent_board = ctx.ite(black_plays, white_boards[t], black_boards[t]);

        // Compute what pieces would be flipped by this move
        let total_flips = compute_all_flips(ctx, move_mask, player_board, opponent_board);

        // Move legality: "Move must be on an empty cell", "Move must flip at least one piece", "Move must conform to placement mask"
        {
            let occupied = ctx.bvor(black_boards[t], white_boards[t]);
            constraints.push(ctx.eq(ctx.bvand(occupied, move_mask), zero_bv));

            constraints.push(ctx.distinct(total_flips, zero_bv));

            constraints.push(ctx.eq(ctx.bvand(placement_mask, move_mask), move_mask));
        }

        // Pass validity constraint: "if pass, then current player must have no legal moves"
        {
            let current_player_board = ctx.ite(is_black_turn[t], black_boards[t], white_boards[t]);
            let current_opponent_board =
                ctx.ite(is_black_turn[t], white_boards[t], black_boards[t]);
            let current_player_can_place =
                ctx.ite(is_black_turn[t], black_can_place_bv, white_can_place_bv);
            let current_has_moves = has_legal_move(
                ctx,
                current_player_board,
                current_opponent_board,
                current_player_can_place,
            );

            // If pass, then current player has no legal moves (pass => !has_moves is equivalent to !pass || !has_moves)
            constraints.push(ctx.or(ctx.not(pass), ctx.not(current_has_moves)));
        }

        // "Next stage must reflect the move and flips"
        {
            let new_player_board = ctx.bvor(ctx.bvor(player_board, move_mask), total_flips);
            let new_opponent_board = ctx.bvand(opponent_board, ctx.bvnot(total_flips));

            let new_black = ctx.ite(black_plays, new_player_board, new_opponent_board);
            let new_white = ctx.ite(black_plays, new_opponent_board, new_player_board);

            constraints.push(ctx.eq(black_boards[t + 1], new_black));
            constraints.push(ctx.eq(white_boards[t + 1], new_white));
        }

        // "Next turn is the player who did not play this turn"
        if t + 1 < max_moves {
            constraints.push(ctx.eq(is_black_turn[t + 1], ctx.not(black_plays)));
        }
    }

    (constraints, move_positions)
}

fn main() {
    let valid_game_records = vec![
        "B3B4D5B2A4B5",
        "B3B4D5D2D1E5B5A4",
        "E4C5B2E3E5B3B4E6F2A3B5A1E2E1B6C6D5D6F1C2B1C1F4A4A2A6F6F3D1F5A5D2",
        "C2B2B3D2B1A1E3F2E1A3C1D5A2D1A4A5E2F1E4B4B5B6F3E5E6C6A6C5D6F4F6F5",
        "D5E3D2C5B3C2B1E6B4A2C6A4F2C1D1E2A3A5E5F3B5F1E1D6A1E4F6F5A6B6B2F4",
        "B3B4C5D6B5C2D5A4C1A3A2B2A1E3E2E5A5D1C6F3F6F5D2E1F4A6B6B1E4E6F2F1",
        "B3D2E4F5D1C2E2C5B5A5C1F1F2B1A1A3B6C6A6E1D5E3A4D6F4B4F6B2F3E5E6A2",
        "D5E3E2E5C2D2F3D6E1B4C5B5B6B3F6C6A4F1F2C1E6A5A6B2D1F4B1F5A3A1A2E4",
        "B3B4A5E2C5A3E3E4F4D5E1F2C6B6B5E6B2F1D2A1D6C2A6F3D1E5A2B1F5A4F6C1",
        "C2D2E5D5E1F5E2B3F6D1B2B1F4F3C5B6F2E6A1F1B5B4A5C6D6E4A3A4E3A6C1A2",
        "D5C5B2E6D6C2B5A1F6E3E2E5F3F2E4B6C1B4A2A3A6E1F1F4A4C6D1A5F5B3D2B1",
        "D5E3E2E5E4C5B6B5F6F2A5F4F3F1D2A6B4C1D1C6F5E6D6B3A2A4B1C2B2E1A1A3",
        "D5C5B5E3B2D6E6B3B4A6B6A4E5A3C2D1A2C6A5F6E2E4F2B1A1F3D2E1F1C1F4F5",
        "C2B2A2C5D5A1B6D2B5D6E2E3E6C6E4E5B4F6F5A6E1F3A3A4D1F1C1B1A5B3F4F2",
        "B3D2E3F4E1A3A2B5F3F2A4D1E5F1D5D6E2B2E4C5C2C1B4B1B6C6E6F6A6F5A5A1",
        "D5E3D2E5E4C5F3D1E6F2C6F6E2F1C2B4B3B2A4D6F5B5A3F4A6A2A1B1C1A5B6E1",
        "E4E5E6B4B3C2B2E2D2C1F2E1B1A3B5A1D1F6D5B6A2F4A4F1E3F3C6F5A5A6C5D6",
        "E4C5B2E2B4B3D6C6B6E5E6F5F4F6A2A6C2B1D2A1D5B5A5E3F2A3A4C1D1F3E1F1",
        "E4E3B2E5F4B3E6F5A2C5D5A3D2E2C2B1B6B4F2D1A5D6A1C1F6E1F1F3C6A4B5A6",
        "C2D2E2B2D5E1D1E3E4C5F3F2B4A3C6B5C1B1A6D6A5E5A1A4F1B3F5F6E6B6F4A2",
        "B3B4A5B2D2B5A3D1C5A4E2F2E5C6B1E4F5F4E3A6D5F3B6E6D6A2C1C2F1A1E1F6",
        "E4E3F2B5B2F4C5F3E2B6E5D5C6C2D2F1E1A2A4D6A1A6A3B3F5B4D1B1C1E6A5F6",
        "C2B2E4C5A1F4C6D2E2B1E3B4D5E1A3B6D6B3D1B5E5C1A2F5E6F6F3A4A6F2A5F1",
        "E4E3D2E5B4D1F4C5E2E1F3B5C1A5B6D5A4B3A6F2C2B2D6B1A2F5A1A3C6F6E6F1",
        "E4E5B3E3F6E6D6C5C6B2F5F4B1A3F3B6A6D2E2D5C1F1D1E1C2A2B4A4A5B5F2A1",
        "D5C5B4E3C6D6E5F6D2B3C2E6F4F3F2A3B6E1E4A6D1B5A4B2A1E2F1A2A5F5C1B1",
        "E4E3C2C5D5E5F5D2F6B2D1E2B6F3F2F1B3A2B4B1F4B5A5D6A4C6E6A3C1A6A1E1",
        "E4E3D2C5B3C1C6F4E5D6F6B4C2A3E6B5F5B6E2F3F2F1A4E1A6B1D1A5A1B2A2D5",
        "D5E3C2C5F3B2B3B4B5F2A2B1E5E4C1D1E2C6D2F4A3D6F6F1B6F5E6A6E1A4A5A1",
        "D5C5B2E2B5E4E3C6E1A4A6B3A3D2B6C2D1E5F5A2C1F4A1E6A5B1F6F2F3F1B4D6",
    ];
    for record in valid_game_records {
        let progression = UncheckedGameProgression::from_game_record_string(record);
        let final_board = progression.play_through();

        let result = test_reachability(
            &final_board,
            PlacementMask::allow_everywhere(),
            PlacementMask::allow_everywhere(),
        );
        assert!(result.is_some());
        assert_eq!(final_board, result.unwrap().play_through());
    }
}

/// Parse a bitvector value from SMT solver output
/// Handles formats like "#b000000", "#x00", "0", etc.
fn parse_bitvector_value(s: &str) -> u8 {
    let s = s.trim();
    if s.starts_with("#b") {
        // Binary format
        u8::from_str_radix(&s[2..], 2).expect("Failed to parse binary bitvector")
    } else if s.starts_with("#x") {
        // Hexadecimal format
        u8::from_str_radix(&s[2..], 16).expect("Failed to parse hex bitvector")
    } else {
        // Decimal format
        s.parse::<u8>().expect("Failed to parse decimal bitvector")
    }
}

fn test_reachability(
    final_state: &Board,
    black_pmask: PlacementMask,
    white_pmask: PlacementMask,
) -> Option<UncheckedGameProgression> {
    let mut ctx = init_yices2_ctx();
    ctx.set_logic("QF_BV").expect("Failed to set logic");

    println!("Target board state:");
    println!("{}", final_state.to_string_block());

    let (constraints, move_positions) =
        generate_reversi_constraints(&mut ctx, final_state, black_pmask, white_pmask);

    println!("Adding {} constraints to solver...", constraints.len());
    {
        let assert_start = Instant::now();
        for (i, constraint) in constraints.iter().enumerate() {
            if let Err(e) = ctx.assert(*constraint) {
                panic!(
                    "Error asserting constraint {}: {}, Constraint: {}",
                    i,
                    e,
                    ctx.display(*constraint)
                );
            }
        }
        println!("Constraints asserted in {:.2?}", assert_start.elapsed());
    }

    let result = {
        println!("Checking satisfiability...");
        let check_start = Instant::now();
        let r = ctx.check();
        println!(
            "Satisfiability check completed in {:.2?}",
            check_start.elapsed()
        );
        r
    };

    match result {
        Ok(Response::Sat) => {
            // Extract model values for move positions
            let model_values = ctx
                .get_value(move_positions)
                .expect("Failed to get model values");

            // Parse the model values to extract move positions
            let mut moves = Vec::new();
            for (_var, value_expr) in model_values {
                // The value_expr should be a bitvector constant
                // We need to parse it to get the integer position (0-35)
                let pos_str = ctx.display(value_expr).to_string();
                // Parse bitvector format (e.g., "#b000000" or "#x00")
                let pos = parse_bitvector_value(&pos_str);
                let column = (pos % 6) as u8;
                let row = (pos / 6) as u8;
                moves.push(CellCoord::new(column, row));
            }
            let progression = UncheckedGameProgression::new(moves);

            println!(
                "✓ Result: SAT - This position IS REACHABLE ({})! The final state can be reached through valid Reversi play.",
                progression.to_game_record_string()
            );

            Some(progression)
        }
        Ok(Response::Unsat) => {
            println!("✗ Result: UNSAT - This position is NOT REACHABLE");
            None
        }
        Ok(Response::Unknown) => {
            println!("? Result: UNKNOWN - Solver could not determine reachability");
            None
        }
        Err(e) => {
            panic!("Error during solving: {}", e);
        }
    }
}
