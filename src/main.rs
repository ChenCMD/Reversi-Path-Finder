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
        // TODO: Implement move legality, piece flipping, and turn alternation (iff not pass iff current player had one or more valid moves)
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
