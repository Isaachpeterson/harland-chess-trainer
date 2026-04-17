// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

//! Property-based tests for mistake classification.
//! Verifies that classification is monotonic in eval drop and consistent.

use chess_core::{classify_mistake, MistakeThresholds};
use proptest::prelude::*;

fn default_thresholds() -> MistakeThresholds {
    MistakeThresholds::default()
}

proptest! {
    /// Classification severity is monotonic: if the eval drop increases (gets worse),
    /// the classification can only stay the same or get more severe, never less severe.
    #[test]
    fn classification_monotonic_in_drop(
        eval_before in -3000i32..3000,
        drop_small in 0i32..2000,
        drop_extra in 1i32..2000,
    ) {
        let thresholds = default_thresholds();

        let eval_after_small = eval_before - drop_small;
        let eval_after_large = eval_before - drop_small - drop_extra;

        let class_small = classify_mistake(
            Some(eval_before), None,
            Some(eval_after_small), None,
            true, &thresholds,
        );
        let class_large = classify_mistake(
            Some(eval_before), None,
            Some(eval_after_large), None,
            true, &thresholds,
        );

        // If the smaller drop produced a classification, the larger drop
        // must produce the same or more severe classification.
        match (class_small, class_large) {
            (Some(s), Some(l)) => prop_assert!(l >= s,
                "Larger drop ({}) produced less severe classification ({:?}) than smaller drop ({}) ({:?})",
                drop_small + drop_extra, l, drop_small, s),
            (Some(_), None) => prop_assert!(false,
                "Larger drop ({}) produced None but smaller drop ({}) produced {:?}",
                drop_small + drop_extra, drop_small, class_small),
            _ => {} // None → Some or None → None are both fine
        }
    }

    /// Improving moves (negative drop) are never classified as mistakes.
    #[test]
    fn improving_moves_never_classified(
        eval_before in -5000i32..5000,
        improvement in 1i32..5000,
    ) {
        let thresholds = default_thresholds();
        let eval_after = eval_before + improvement;

        let result = classify_mistake(
            Some(eval_before), None,
            Some(eval_after), None,
            true, &thresholds,
        );

        prop_assert_eq!(result, None,
            "Improving move (before={}, after={}) should not be classified",
            eval_before, eval_after);
    }

    /// Classification works symmetrically for Black — a drop from Black's
    /// perspective should produce the same classification as the equivalent
    /// drop from White's perspective.
    #[test]
    fn symmetric_for_black(
        user_eval_before in -3000i32..3000,
        drop in 0i32..3000,
    ) {
        let thresholds = default_thresholds();

        // White: before = user_eval_before, after = user_eval_before - drop
        let white_class = classify_mistake(
            Some(user_eval_before), None,
            Some(user_eval_before - drop), None,
            true, &thresholds,
        );

        // Black: evals from White's perspective are negated.
        // Black's "user_eval_before" = -eval_before_white → eval_before_white = -user_eval_before
        // Black's "user_eval_after" = user_eval_before - drop → eval_after_white = -(user_eval_before - drop)
        let black_class = classify_mistake(
            Some(-user_eval_before), None,
            Some(-(user_eval_before - drop)), None,
            false, &thresholds,
        );

        prop_assert_eq!(white_class, black_class,
            "White classification ({:?}) != Black classification ({:?}) for user_before={}, drop={}",
            white_class, black_class, user_eval_before, drop);
    }

    /// No drop means no classification.
    #[test]
    fn zero_drop_never_classified(
        eval in -5000i32..5000,
        user_is_white in proptest::bool::ANY,
    ) {
        let thresholds = default_thresholds();
        let white_eval = if user_is_white { eval } else { -eval };

        let result = classify_mistake(
            Some(white_eval), None,
            Some(white_eval), None,
            user_is_white, &thresholds,
        );

        prop_assert_eq!(result, None, "Zero drop should never be classified");
    }
}
