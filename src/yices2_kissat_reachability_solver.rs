use std::time::Instant;

use easy_smt::{ContextBuilder, Response, SExpr};

use crate::{
    board::{BitBoard, Board, CellCoord, PlacementMask},
    game::{INITIAL_BOARD, UncheckedGameProgression},
    reachability_problem::{ReachabilityProblem, ReachabilitySolver, ReachabilitySolverResult},
};

/// Trace of all intermediate SMT variables extracted from the solver
#[derive(Debug, Clone)]
pub struct SolverTrace {
    pub black_bitboards: Vec<u64>,  // black_bitboard_t for each timestep (0 to max_moves)
    pub white_bitboards: Vec<u64>,  // white_bitboard_t for each timestep
    pub is_black_turns: Vec<bool>,  // is_black_turn_t for each timestep
    pub move_positions: Vec<u8>,    // move_pos_t for each transition (0-35)
    pub passes: Vec<bool>,          // pass_t for each transition
}

/// Represents a single step in the game trace
#[derive(Clone)]
pub struct GameStep {
    pub board_before: Board,     // Board state before this move
    pub player: crate::board::PlayerColor,  // Player making the move
    pub move_cell: crate::board::CellCoord, // Position of the move
    pub is_pass: bool,           // Whether this was a pass
}

/// High-level game trace parsed from raw SMT variables
#[derive(Clone)]
pub struct GameTrace {
    pub steps: Vec<GameStep>,
    pub final_board: Board,
}

impl GameTrace {
    /// Converts raw SolverTrace into a high-level GameTrace
    pub fn from_solver_trace(trace: &SolverTrace) -> Self {
        let mut steps = Vec::new();

        for t in 0..trace.move_positions.len() {
            let board_before = Board::from_bitboards(
                trace.black_bitboards[t],
                trace.white_bitboards[t],
            );

            let prev_is_black = if t < trace.is_black_turns.len() {
                trace.is_black_turns[t]
            } else {
                true // Default to black for first move
            };

            let is_pass = trace.passes[t];
            let actual_player_is_black = if is_pass {
                !prev_is_black
            } else {
                prev_is_black
            };

            let player = if actual_player_is_black {
                crate::board::PlayerColor::Black
            } else {
                crate::board::PlayerColor::White
            };

            let move_pos = trace.move_positions[t];
            let col = move_pos % 6;
            let row = move_pos / 6;
            let move_cell = crate::board::CellCoord::new(col, row);

            steps.push(GameStep {
                board_before,
                player,
                move_cell,
                is_pass,
            });
        }

        let final_board = Board::from_bitboards(
            *trace.black_bitboards.last().unwrap(),
            *trace.white_bitboards.last().unwrap(),
        );

        GameTrace { steps, final_board }
    }
}

/// Result of generating Reversi constraints for SMT solving
pub struct ReversiConstraints {
    pub constraints: Vec<SExpr>,
    pub move_positions: Vec<SExpr>,
    pub black_boards: Vec<SExpr>,
    pub white_boards: Vec<SExpr>,
    pub is_black_turn: Vec<SExpr>,
    pub passes: Vec<SExpr>,
}

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
) -> ReversiConstraints {
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
    let mut passes = Vec::new();
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
        passes.push(pass);

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

    ReversiConstraints {
        constraints,
        move_positions,
        black_boards,
        white_boards,
        is_black_turn,
        passes,
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

/// Parse a 36-bit bitboard value from SMT solver output
fn parse_bitboard_value(s: &str) -> u64 {
    let s = s.trim();
    if s.starts_with("#b") {
        u64::from_str_radix(&s[2..], 2).expect("Failed to parse binary bitboard")
    } else if s.starts_with("#x") {
        u64::from_str_radix(&s[2..], 16).expect("Failed to parse hex bitboard")
    } else {
        s.parse::<u64>().expect("Failed to parse decimal bitboard")
    }
}

/// Parse a boolean value from SMT solver output
fn parse_bool_value(s: &str) -> bool {
    let s = s.trim();
    match s {
        "true" => true,
        "false" => false,
        _ => panic!("Failed to parse boolean value: {}", s),
    }
}

pub struct Yices2KissatSolver;

impl ReachabilitySolver for Yices2KissatSolver {
    type ExtraTraceDataOnSAT = SolverTrace;
    type ExtraTraceDataOnUNSAT = ();

    fn solve(
        &mut self,
        problem: &ReachabilityProblem,
    ) -> ReachabilitySolverResult<Self::ExtraTraceDataOnSAT, Self::ExtraTraceDataOnUNSAT> {
        let mut ctx = ContextBuilder::new()
            .solver("yices-smt2")
            .solver_args(["--delegate=kissat"])
            .build()
            .expect("Failed to create SMT context with Yices2");
        ctx.set_logic("QF_BV").expect("Failed to set logic");

        let reversi_constraints = generate_reversi_constraints(
            &mut ctx,
            &problem.target_board,
            problem.black_placement_mask,
            problem.white_placement_mask,
        );

        let constraints = &reversi_constraints.constraints;
        let move_positions = reversi_constraints.move_positions;
        let black_boards = reversi_constraints.black_boards;
        let white_boards = reversi_constraints.white_boards;
        let is_black_turn_vars = reversi_constraints.is_black_turn;
        let passes = reversi_constraints.passes;

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
                    .get_value(move_positions.clone())
                    .expect("Failed to get model values");

                // Parse the model values to extract move positions
                let mut moves = Vec::new();
                let mut move_pos_values = Vec::new();
                for (_var, value_expr) in model_values {
                    let pos_str = ctx.display(value_expr).to_string();
                    let pos = parse_bitvector_value(&pos_str);
                    move_pos_values.push(pos);
                    let column = (pos % 6) as u8;
                    let row = (pos / 6) as u8;
                    moves.push(CellCoord::new(column, row));
                }
                let progression = UncheckedGameProgression::new(moves);

                // Extract all intermediate variables for the trace
                let black_bitboard_values = black_boards
                    .iter()
                    .map(|&var| {
                        let model = ctx.get_value(vec![var]).expect("Failed to get bitboard");
                        let val_str = ctx.display(model[0].1).to_string();
                        parse_bitboard_value(&val_str)
                    })
                    .collect();

                let white_bitboard_values = white_boards
                    .iter()
                    .map(|&var| {
                        let model = ctx.get_value(vec![var]).expect("Failed to get bitboard");
                        let val_str = ctx.display(model[0].1).to_string();
                        parse_bitboard_value(&val_str)
                    })
                    .collect();

                let is_black_turn_values = is_black_turn_vars
                    .iter()
                    .map(|&var| {
                        let model = ctx.get_value(vec![var]).expect("Failed to get turn");
                        let val_str = ctx.display(model[0].1).to_string();
                        parse_bool_value(&val_str)
                    })
                    .collect();

                let pass_values = passes
                    .iter()
                    .map(|&var| {
                        let model = ctx.get_value(vec![var]).expect("Failed to get pass");
                        let val_str = ctx.display(model[0].1).to_string();
                        parse_bool_value(&val_str)
                    })
                    .collect();

                let trace = SolverTrace {
                    black_bitboards: black_bitboard_values,
                    white_bitboards: white_bitboard_values,
                    is_black_turns: is_black_turn_values,
                    move_positions: move_pos_values,
                    passes: pass_values,
                };

                ReachabilitySolverResult::Reachable(progression, trace)
            }
            Ok(Response::Unsat) => ReachabilitySolverResult::Unreachable(()),
            Ok(Response::Unknown) => ReachabilitySolverResult::Unknown,
            Err(e) => {
                panic!("Error during solving: {}", e);
            }
        }
    }
}

pub fn new_yices2_kissat_reachability_solver() -> Yices2KissatSolver {
    Yices2KissatSolver
}
