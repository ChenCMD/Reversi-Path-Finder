use easy_smt::{ContextBuilder, Response, SExpr};

mod board;
use board::*;

fn init_yices2_ctx() -> easy_smt::Context {
    ContextBuilder::new()
        .solver("yices-smt2")
        .solver_args(["--incremental"])
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
            ctx.bvlshr(pos, ctx.binary(36, shift_amount as u64))
        } else {
            ctx.bvshl(pos, ctx.binary(36, (-shift_amount) as u64))
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
    let mut chain_valid = ctx.not(ctx.eq(zero_bv, zero_bv)); // Start with true

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

        // Break chain if we hit non-opponent cell
        chain_valid = ctx.and(chain_valid, ctx.not(has_opponent));
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

        // This position is legal if it's empty AND flips pieces
        let is_legal = ctx.and(is_empty, has_flips);

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
///
/// # Returns
/// A vector of SExpr constraints representing the game rules and progression
pub fn generate_reversi_constraints(
    ctx: &mut easy_smt::Context,
    final_state: &Board,
) -> Vec<SExpr> {
    let mut constraints = Vec::new();
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
        let BitBoard { black, white } = Board::INITIAL.to_bitboards();
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
        let pass = ctx
            .declare_const(&format!("pass_{}", t), bool_sort)
            .unwrap();

        let black_plays = ctx.xor(is_black_turn[t], pass);

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

        // Move legality: "Move must be on an empty cell" and "Move must flip at least one piece"
        {
            let occupied = ctx.bvor(black_boards[t], white_boards[t]);
            constraints.push(ctx.eq(ctx.bvand(occupied, move_mask), zero_bv));

            constraints.push(ctx.distinct(total_flips, zero_bv));
        }

        // Pass validity constraint: "if pass, then current player must have no legal moves"
        {
            let current_player_board = ctx.ite(is_black_turn[t], black_boards[t], white_boards[t]);
            let current_opponent_board =
                ctx.ite(is_black_turn[t], white_boards[t], black_boards[t]);
            let current_has_moves =
                has_legal_move(ctx, current_player_board, current_opponent_board);

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

    constraints
}

fn main() {
    println!("Testing Example Final State 1 (Black plays at (2,1))...");
    assert!(test_reachability(&example_boards::REACHABLE_1_STEP) == Response::Sat);

    println!("\n{}\n", "=".repeat(60));

    println!("Testing Example Final State 2 (two moves)...");
    assert!(test_reachability(&example_boards::UNREACHABLE_2_STEPS) == Response::Unsat);

    println!("\n{}\n", "=".repeat(60));

    println!("Testing Unreachable State (expect UNSAT)...");
    assert!(test_reachability(&example_boards::UNREACHABLE_BROKEN) == Response::Unsat);
}

fn test_reachability(final_state: &Board) -> Response {
    let mut ctx = init_yices2_ctx();
    ctx.set_logic("QF_BV").expect("Failed to set logic");

    println!("Target board state:");
    println!("{}", final_state.to_string_block());

    let constraints = generate_reversi_constraints(&mut ctx, final_state);

    println!("Adding {} constraints to solver...", constraints.len());
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

    println!("Checking satisfiability...");
    match ctx.check() {
        Ok(Response::Sat) => {
            println!("✓ Result: SAT - This position IS REACHABLE!");
            println!("\nThe final state can be reached through valid Reversi play.");
            Response::Sat
        }
        Ok(Response::Unsat) => {
            println!("✗ Result: UNSAT - This position is NOT REACHABLE");
            Response::Unsat
        }
        Ok(Response::Unknown) => {
            println!("? Result: UNKNOWN - Solver could not determine reachability");
            Response::Unknown
        }
        Err(e) => {
            panic!("Error during solving: {}", e);
        }
    }
}
