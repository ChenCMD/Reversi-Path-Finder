# Style and conventions
- Rust 2024 edition; use `rustfmt`/`clippy` defaults. Modules are flat under `src`; shared types live in `board.rs`, `game.rs`, `reachability_problem.rs`, solver in `yices2_kissat_reachability_solver.rs`.
- Board representation: 6x6 grid indexed [row][col]; bitboard mapping uses bit `y*6+x` (A1=bit0, F6=bit35). Placement masks and board states can be serialized as 12-digit octal strings (each digit packs three cells; see docstrings in `Board::from_octal_strings`/`PlacementMask::from_octal_string`).
- Moves: `CellCoord` holds `column`/`row` (0-based); game records serialize as `A-F` + `1-6` pairs. Black moves first; if current player has no move, turn passes to opponent.
- Board API: `moves_available` checks legality via `can_place_disk` and flip detection; `place_disk` returns `Option<Board>` (None for illegal move) and flips captured lines; `to_bitboards`/`to_string_block` provide conversions for solver/UI.
- Solver encoding: bitvector SMT (36-bit) with Yices2; constraints ensure legality, passes only when no legal moves, and final bitboards match target. Flips computed directionally with edge masks to avoid wrap-around.
