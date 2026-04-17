// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

//! Mistake classification for chess move evaluations.
//!
//! Classifies moves as inaccuracies, mistakes, or blunders based on the
//! centipawn evaluation drop from the user's perspective. Handles mate
//! scores, already-losing positions, and configurable thresholds.

/// Classification severity of a chess mistake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MistakeClassification {
    /// 50–99 cp drop (default). Minor imprecision.
    Inaccuracy,
    /// 100–199 cp drop (default). Significant error.
    Mistake,
    /// 200+ cp drop (default). Serious error, puzzle candidate.
    Blunder,
}

/// Configurable centipawn thresholds for mistake classification.
///
/// All values are positive centipawn drops from the user's perspective.
/// A drop of exactly `inaccuracy_cp` counts as an inaccuracy, a drop of
/// exactly `mistake_cp` counts as a mistake, etc.
#[derive(Debug, Clone)]
pub struct MistakeThresholds {
    /// Minimum cp drop to classify as an inaccuracy.
    pub inaccuracy_cp: i32,
    /// Minimum cp drop to classify as a mistake.
    pub mistake_cp: i32,
    /// Minimum cp drop to classify as a blunder.
    pub blunder_cp: i32,
    /// When the user's position is already this bad (from their perspective),
    /// require a larger drop (`losing_extra_cp`) to classify as a blunder.
    /// This avoids flagging moves in already-hopeless positions.
    pub already_losing_cp: i32,
    /// Extra centipawns of drop required when already in a losing position
    /// (beyond `already_losing_cp`).
    pub losing_extra_cp: i32,
}

impl Default for MistakeThresholds {
    /// Default thresholds tuned for ~1600 rated play (see PROJECT_CONTEXT.md §6).
    fn default() -> Self {
        Self {
            inaccuracy_cp: 50,
            mistake_cp: 100,
            blunder_cp: 200,
            already_losing_cp: 500,
            losing_extra_cp: 100,
        }
    }
}

/// Sentinel value representing mate scores as centipawns for comparison purposes.
const MATE_CP: i32 = 10_000;

/// Converts eval fields (cp / mate) into a single centipawn-equivalent value
/// from the user's perspective.
///
/// - `eval_cp`: centipawn eval from White's perspective (positive = White better).
/// - `eval_mate`: mate-in-N from White's perspective (positive = White mates).
/// - `user_is_white`: whether the user is playing White.
///
/// Returns the eval from the **user's** perspective (positive = user is better).
fn to_user_cp(eval_cp: Option<i32>, eval_mate: Option<i32>, user_is_white: bool) -> i32 {
    let white_cp = if let Some(mate) = eval_mate {
        // Positive mate = White mates → good for White.
        // Use MATE_CP * signum for large sentinel value.
        if mate > 0 {
            MATE_CP
        } else if mate < 0 {
            -MATE_CP
        } else {
            // mate == 0 is checkmate already delivered; treat as max advantage for
            // the side that just mated. In practice eval_mate == 0 shouldn't appear
            // in stored evals, but handle defensively.
            0
        }
    } else {
        eval_cp.unwrap_or(0)
    };

    if user_is_white {
        white_cp
    } else {
        -white_cp
    }
}

/// Determines whether a transition between two evaluations is a "mate-to-mate"
/// transition where both sides are mating and only the distance changed.
///
/// Example: user has mate-in-5 and plays a move that gives mate-in-7 —
/// both are winning mates, so this should NOT be flagged.
fn is_mate_to_mate_same_sign(
    before_mate: Option<i32>,
    after_mate: Option<i32>,
    user_is_white: bool,
) -> bool {
    match (before_mate, after_mate) {
        (Some(m1), Some(m2)) => {
            // Convert to user's perspective
            let um1 = if user_is_white { m1 } else { -m1 };
            let um2 = if user_is_white { m2 } else { -m2 };
            // Both positive = both user-mating; both negative = both opponent-mating
            (um1 > 0 && um2 > 0) || (um1 < 0 && um2 < 0)
        }
        _ => false,
    }
}

/// Classifies a move based on the evaluation drop.
///
/// # Arguments
///
/// - `eval_before_cp` / `eval_before_mate`: evaluation of the position **before**
///   the user's move, from White's perspective.
/// - `eval_after_cp` / `eval_after_mate`: evaluation of the position **after**
///   the user's move, from White's perspective.
/// - `user_is_white`: whether the user is playing White.
/// - `thresholds`: configurable classification thresholds.
///
/// # Returns
///
/// `Some(classification)` if the move is classified as a mistake, `None` if the
/// move is acceptable.
///
/// # Mate score handling
///
/// Per PROJECT_CONTEXT.md §6:
/// - Mate-to-mate transitions where both are winning (or both losing) are **not**
///   flagged — going from "mate in 3" to "mate in 5" is not a meaningful mistake.
/// - A transition from winning/drawn to "opponent mates" (or from "user mates"
///   to a non-mate losing position) **is** flagged as a blunder.
pub fn classify_mistake(
    eval_before_cp: Option<i32>,
    eval_before_mate: Option<i32>,
    eval_after_cp: Option<i32>,
    eval_after_mate: Option<i32>,
    user_is_white: bool,
    thresholds: &MistakeThresholds,
) -> Option<MistakeClassification> {
    // Rule: mate-to-mate where both are on the same side → not a meaningful mistake.
    if is_mate_to_mate_same_sign(eval_before_mate, eval_after_mate, user_is_white) {
        return None;
    }

    let user_before = to_user_cp(eval_before_cp, eval_before_mate, user_is_white);
    let user_after = to_user_cp(eval_after_cp, eval_after_mate, user_is_white);

    // Drop from the user's perspective (positive = got worse).
    let drop = user_before - user_after;

    if drop < thresholds.inaccuracy_cp {
        return None;
    }

    // Already-losing cap: if the position was already bad for the user, require
    // a bigger drop to call it a blunder.
    let effective_blunder_threshold = if user_before < -thresholds.already_losing_cp {
        thresholds.blunder_cp + thresholds.losing_extra_cp
    } else {
        thresholds.blunder_cp
    };

    if drop >= effective_blunder_threshold {
        Some(MistakeClassification::Blunder)
    } else if drop >= thresholds.mistake_cp {
        Some(MistakeClassification::Mistake)
    } else {
        Some(MistakeClassification::Inaccuracy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defaults() -> MistakeThresholds {
        MistakeThresholds::default()
    }

    // -------------------------------------------------------------------
    // Basic centipawn classification
    // -------------------------------------------------------------------

    #[test]
    fn no_drop_is_not_mistake() {
        // Eval stays at +30cp
        assert_eq!(
            classify_mistake(Some(30), None, Some(30), None, true, &defaults()),
            None
        );
    }

    #[test]
    fn small_drop_under_inaccuracy() {
        // 30cp drop: below inaccuracy threshold
        assert_eq!(
            classify_mistake(Some(50), None, Some(20), None, true, &defaults()),
            None
        );
    }

    #[test]
    fn inaccuracy_at_threshold() {
        // Exactly 50cp drop → inaccuracy
        assert_eq!(
            classify_mistake(Some(100), None, Some(50), None, true, &defaults()),
            Some(MistakeClassification::Inaccuracy)
        );
    }

    #[test]
    fn inaccuracy_just_above_threshold() {
        // 70cp drop → inaccuracy
        assert_eq!(
            classify_mistake(Some(100), None, Some(30), None, true, &defaults()),
            Some(MistakeClassification::Inaccuracy)
        );
    }

    #[test]
    fn mistake_at_threshold() {
        // Exactly 100cp drop → mistake
        assert_eq!(
            classify_mistake(Some(200), None, Some(100), None, true, &defaults()),
            Some(MistakeClassification::Mistake)
        );
    }

    #[test]
    fn mistake_150cp() {
        // 150cp drop → mistake
        assert_eq!(
            classify_mistake(Some(200), None, Some(50), None, true, &defaults()),
            Some(MistakeClassification::Mistake)
        );
    }

    #[test]
    fn blunder_at_threshold() {
        // Exactly 200cp drop → blunder
        assert_eq!(
            classify_mistake(Some(250), None, Some(50), None, true, &defaults()),
            Some(MistakeClassification::Blunder)
        );
    }

    #[test]
    fn blunder_large_drop() {
        // 500cp drop → blunder
        assert_eq!(
            classify_mistake(Some(300), None, Some(-200), None, true, &defaults()),
            Some(MistakeClassification::Blunder)
        );
    }

    // -------------------------------------------------------------------
    // Black's perspective
    // -------------------------------------------------------------------

    #[test]
    fn blunder_as_black() {
        // White's perspective: before = -200 (Black is better), after = 100 (White is better)
        // Black's perspective: before = +200, after = -100 → drop = 300 → blunder
        assert_eq!(
            classify_mistake(Some(-200), None, Some(100), None, false, &defaults()),
            Some(MistakeClassification::Blunder)
        );
    }

    #[test]
    fn no_mistake_as_black_when_improving() {
        // White's perspective: before = 200, after = -50
        // Black's perspective: before = -200, after = 50 → drop = -250 (improved!) → None
        assert_eq!(
            classify_mistake(Some(200), None, Some(-50), None, false, &defaults()),
            None
        );
    }

    // -------------------------------------------------------------------
    // Mate score handling
    // -------------------------------------------------------------------

    #[test]
    fn mate_to_mate_same_side_not_mistake() {
        // User (White) has mate-in-3 and plays a move giving mate-in-5.
        // Both are winning mates → not flagged.
        assert_eq!(
            classify_mistake(None, Some(3), None, Some(5), true, &defaults()),
            None
        );
    }

    #[test]
    fn mate_to_mate_opposite_side_is_blunder() {
        // User (White) had mate-in-3 but now opponent has mate-in-2.
        // before_mate = +3 (White mates), after_mate = -2 (Black mates)
        // Not same-sign → not filtered. Drop = 10000 - (-10000) = 20000 → blunder.
        assert_eq!(
            classify_mistake(None, Some(3), None, Some(-2), true, &defaults()),
            Some(MistakeClassification::Blunder)
        );
    }

    #[test]
    fn user_loses_mate_to_equal() {
        // User (White) had mate-in-3, now position is 0cp.
        // user_before = 10000, user_after = 0 → drop = 10000 → blunder
        assert_eq!(
            classify_mistake(None, Some(3), Some(0), None, true, &defaults()),
            Some(MistakeClassification::Blunder)
        );
    }

    #[test]
    fn equal_to_getting_mated_is_blunder() {
        // User (White) was at 0cp, now opponent has mate-in-2.
        // user_before = 0, user_after = -10000 → drop = 10000 → blunder
        assert_eq!(
            classify_mistake(Some(0), None, None, Some(-2), true, &defaults()),
            Some(MistakeClassification::Blunder)
        );
    }

    #[test]
    fn getting_mated_to_still_getting_mated_not_mistake() {
        // User (Black) is being mated: before_mate = 5 (White mates in 5),
        // after_mate = 3 (White mates in 3). From Black's perspective both are losing mates.
        // user_before_mate = -5, user_after_mate = -3 → same sign (both negative) → not flagged.
        assert_eq!(
            classify_mistake(None, Some(5), None, Some(3), false, &defaults()),
            None
        );
    }

    // -------------------------------------------------------------------
    // Already-losing-position cap
    // -------------------------------------------------------------------

    #[test]
    fn already_losing_requires_bigger_drop_for_blunder() {
        // User (White) at -600cp (already losing badly).
        // Plays a move making it -800cp. Drop = 200cp.
        // Normally 200cp = blunder, but already_losing_cp = 500 and losing_extra_cp = 100,
        // so effective blunder threshold = 300. Drop of 200 < 300 → mistake, not blunder.
        assert_eq!(
            classify_mistake(Some(-600), None, Some(-800), None, true, &defaults()),
            Some(MistakeClassification::Mistake)
        );
    }

    #[test]
    fn already_losing_big_drop_still_blunder() {
        // User (White) at -600cp. Plays move making it -1000cp. Drop = 400cp.
        // Effective blunder threshold = 300. Drop 400 >= 300 → blunder.
        assert_eq!(
            classify_mistake(Some(-600), None, Some(-1000), None, true, &defaults()),
            Some(MistakeClassification::Blunder)
        );
    }

    #[test]
    fn not_losing_normal_threshold_applies() {
        // User (White) at -400cp (bad, but not past the -500 threshold).
        // Plays move making it -600cp. Drop = 200cp → blunder (normal threshold).
        assert_eq!(
            classify_mistake(Some(-400), None, Some(-600), None, true, &defaults()),
            Some(MistakeClassification::Blunder)
        );
    }

    #[test]
    fn already_losing_inaccuracy_still_detected() {
        // User (White) at -600cp. Drop of 70cp → inaccuracy
        // (losing cap only affects blunder threshold, not inaccuracy/mistake).
        assert_eq!(
            classify_mistake(Some(-600), None, Some(-670), None, true, &defaults()),
            Some(MistakeClassification::Inaccuracy)
        );
    }

    // -------------------------------------------------------------------
    // Edge cases
    // -------------------------------------------------------------------

    #[test]
    fn improving_position_is_not_mistake() {
        // Eval improves: before = -100, after = 50 → drop = -150 → None
        assert_eq!(
            classify_mistake(Some(-100), None, Some(50), None, true, &defaults()),
            None
        );
    }

    #[test]
    fn both_none_treated_as_zero() {
        // Both eval_cp and eval_mate are None → treated as 0cp → no drop
        assert_eq!(
            classify_mistake(None, None, None, None, true, &defaults()),
            None
        );
    }

    #[test]
    fn custom_thresholds() {
        let thresholds = MistakeThresholds {
            inaccuracy_cp: 30,
            mistake_cp: 80,
            blunder_cp: 150,
            already_losing_cp: 300,
            losing_extra_cp: 50,
        };
        // 40cp drop with inaccuracy_cp=30 → inaccuracy
        assert_eq!(
            classify_mistake(Some(100), None, Some(60), None, true, &thresholds),
            Some(MistakeClassification::Inaccuracy)
        );
        // 100cp drop with mistake_cp=80, blunder_cp=150 → mistake
        assert_eq!(
            classify_mistake(Some(200), None, Some(100), None, true, &thresholds),
            Some(MistakeClassification::Mistake)
        );
        // 160cp drop with blunder_cp=150 → blunder
        assert_eq!(
            classify_mistake(Some(200), None, Some(40), None, true, &thresholds),
            Some(MistakeClassification::Blunder)
        );
    }

    #[test]
    fn threshold_ordering_is_correct() {
        assert!(MistakeClassification::Inaccuracy < MistakeClassification::Mistake);
        assert!(MistakeClassification::Mistake < MistakeClassification::Blunder);
    }
}
