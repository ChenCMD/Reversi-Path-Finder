use crate::{
    board::{Board, PlacementMask, PlayerColor},
    game::UncheckedGameProgression,
};

pub struct ReachabilityProblem {
    pub target_board: Board,
    pub black_placement_mask: PlacementMask,
    pub white_placement_mask: PlacementMask,
    pub initial_board: Board,
}

impl ReachabilityProblem {
    pub fn new(
        target_board: Board,
        black_placement_mask: PlacementMask,
        white_placement_mask: PlacementMask,
        initial_board: Board,
    ) -> Self {
        ReachabilityProblem {
            target_board,
            black_placement_mask,
            white_placement_mask,
            initial_board,
        }
    }

    pub fn admits_as_solution(&self, progression: &UncheckedGameProgression) -> bool {
        if let Some(final_board) = progression.play_through(
            &self.black_placement_mask,
            &self.white_placement_mask,
            &self.initial_board,
        ) {
            self.target_board == final_board
                && progression
                    .to_moves(
                        &self.black_placement_mask,
                        &self.white_placement_mask,
                        &self.initial_board,
                    )
                    .iter()
                    .all(|mv| match mv.player {
                        PlayerColor::Black => self.black_placement_mask.can_place_at_cell(mv.cell),
                        PlayerColor::White => self.white_placement_mask.can_place_at_cell(mv.cell),
                    })
        } else {
            false
        }
    }
}

pub enum ReachabilitySolverResult<ExtraTraceDataOnSAT = (), ExtraTraceDataOnUNSAT = ()> {
    Reachable(UncheckedGameProgression, ExtraTraceDataOnSAT),
    Unreachable(ExtraTraceDataOnUNSAT),
    Unknown,
}

pub trait ReachabilitySolver {
    type ExtraTraceDataOnSAT;
    type ExtraTraceDataOnUNSAT;

    /// Attempts to solve the given reachability problem.
    fn solve(
        &mut self,
        problem: &ReachabilityProblem,
    ) -> ReachabilitySolverResult<Self::ExtraTraceDataOnSAT, Self::ExtraTraceDataOnUNSAT>;
}
