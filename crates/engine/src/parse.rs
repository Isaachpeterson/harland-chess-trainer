// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

//! Parsing helpers for UCI protocol messages from the engine.

use crate::MultiPvLine;

/// Parses a UCI `info` line into a `MultiPvLine`, if the line contains
/// sufficient data (depth, score, pv).
///
/// Example info line:
/// ```text
/// info depth 20 seldepth 30 multipv 1 score cp 35 nodes 1234567 nps 654321 time 1886 pv e2e4 e7e5
/// ```
pub fn parse_info_line(line: &str) -> Option<MultiPvLine> {
    let tokens: Vec<&str> = line.split_whitespace().collect();

    let depth = extract_u32(&tokens, "depth")?;
    let pv_index = extract_u32(&tokens, "multipv").unwrap_or(1);
    let (score_cp, mate_in) = extract_score(&tokens)?;
    let pv = extract_pv(&tokens);

    Some(MultiPvLine {
        pv_index,
        score_cp,
        mate_in,
        depth,
        pv,
    })
}

/// From a collection of info lines, extracts the final (deepest) result for each
/// PV index up to `multipv`. Returns them sorted by PV index.
pub fn extract_final_pvs(info_lines: &[String], multipv: u32) -> Vec<MultiPvLine> {
    let mut best: std::collections::HashMap<u32, MultiPvLine> = std::collections::HashMap::new();

    for line in info_lines {
        if let Some(parsed) = parse_info_line(line) {
            if parsed.pv_index <= multipv {
                let entry = best.entry(parsed.pv_index).or_insert_with(|| parsed.clone());
                // Keep the deepest line for each PV index
                if parsed.depth >= entry.depth {
                    *entry = parsed;
                }
            }
        }
    }

    let mut result: Vec<MultiPvLine> = best.into_values().collect();
    result.sort_by_key(|line| line.pv_index);
    result
}

/// Extracts a `u32` value following the given key in the token list.
fn extract_u32(tokens: &[&str], key: &str) -> Option<u32> {
    tokens
        .iter()
        .position(|&t| t == key)
        .and_then(|i| tokens.get(i + 1))
        .and_then(|v| v.parse().ok())
}

/// Extracts the score from a UCI info line.
/// Returns `(Some(cp), None)` for centipawn scores, `(None, Some(mate))` for mate scores.
fn extract_score(tokens: &[&str]) -> Option<(Option<i32>, Option<i32>)> {
    let score_idx = tokens.iter().position(|&t| t == "score")?;
    let score_type = tokens.get(score_idx + 1)?;
    let score_val: i32 = tokens.get(score_idx + 2)?.parse().ok()?;

    match *score_type {
        "cp" => Some((Some(score_val), None)),
        "mate" => Some((None, Some(score_val))),
        _ => None,
    }
}

/// Extracts the PV moves from a UCI info line (everything after the `pv` token).
fn extract_pv(tokens: &[&str]) -> Vec<String> {
    if let Some(pv_idx) = tokens.iter().position(|&t| t == "pv") {
        tokens[pv_idx + 1..]
            .iter()
            .map(|&s| s.to_owned())
            .collect()
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_info_line_with_cp_score() {
        let line =
            "info depth 20 seldepth 30 multipv 1 score cp 35 nodes 1234567 nps 654321 time 1886 pv e2e4 e7e5 g1f3";
        let result = parse_info_line(line).expect("should parse");
        assert_eq!(result.depth, 20);
        assert_eq!(result.pv_index, 1);
        assert_eq!(result.score_cp, Some(35));
        assert_eq!(result.mate_in, None);
        assert_eq!(result.pv, vec!["e2e4", "e7e5", "g1f3"]);
    }

    #[test]
    fn parse_info_line_with_mate_score() {
        let line = "info depth 15 seldepth 15 multipv 1 score mate 3 nodes 500000 pv d1h5 f7f6 h5f7";
        let result = parse_info_line(line).expect("should parse");
        assert_eq!(result.score_cp, None);
        assert_eq!(result.mate_in, Some(3));
        assert_eq!(result.pv, vec!["d1h5", "f7f6", "h5f7"]);
    }

    #[test]
    fn parse_info_line_with_negative_mate() {
        let line = "info depth 18 seldepth 20 multipv 1 score mate -2 nodes 800000 pv e1g1 d8h4";
        let result = parse_info_line(line).expect("should parse");
        assert_eq!(result.mate_in, Some(-2));
    }

    #[test]
    fn parse_info_line_with_negative_cp() {
        let line = "info depth 22 seldepth 32 multipv 1 score cp -150 nodes 2000000 pv d7d5 e4d5";
        let result = parse_info_line(line).expect("should parse");
        assert_eq!(result.score_cp, Some(-150));
    }

    #[test]
    fn parse_info_line_multipv_2() {
        let line = "info depth 20 seldepth 28 multipv 2 score cp 10 nodes 1500000 pv d2d4 d7d5";
        let result = parse_info_line(line).expect("should parse");
        assert_eq!(result.pv_index, 2);
        assert_eq!(result.score_cp, Some(10));
        assert_eq!(result.pv, vec!["d2d4", "d7d5"]);
    }

    #[test]
    fn parse_info_line_no_multipv_defaults_to_1() {
        let line = "info depth 10 score cp 50 pv e2e4";
        let result = parse_info_line(line).expect("should parse");
        assert_eq!(result.pv_index, 1);
    }

    #[test]
    fn parse_info_line_no_score_returns_none() {
        let line = "info depth 10 seldepth 10 nodes 500";
        assert!(parse_info_line(line).is_none());
    }

    #[test]
    fn parse_info_line_no_depth_returns_none() {
        let line = "info score cp 50 pv e2e4";
        assert!(parse_info_line(line).is_none());
    }

    #[test]
    fn parse_info_line_no_pv_returns_empty_pv() {
        let line = "info depth 10 score cp 50";
        let result = parse_info_line(line).expect("should parse");
        assert!(result.pv.is_empty());
    }

    #[test]
    fn extract_final_pvs_keeps_deepest_per_index() {
        let lines = vec![
            "info depth 10 multipv 1 score cp 30 pv e2e4".to_owned(),
            "info depth 15 multipv 1 score cp 35 pv e2e4 e7e5".to_owned(),
            "info depth 20 multipv 1 score cp 40 pv e2e4 e7e5 g1f3".to_owned(),
            "info depth 10 multipv 2 score cp 20 pv d2d4".to_owned(),
            "info depth 15 multipv 2 score cp 25 pv d2d4 d7d5".to_owned(),
        ];
        let result = extract_final_pvs(&lines, 2);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].pv_index, 1);
        assert_eq!(result[0].depth, 20);
        assert_eq!(result[0].score_cp, Some(40));
        assert_eq!(result[1].pv_index, 2);
        assert_eq!(result[1].depth, 15);
        assert_eq!(result[1].score_cp, Some(25));
    }

    #[test]
    fn extract_final_pvs_ignores_pv_index_above_multipv() {
        let lines = vec![
            "info depth 20 multipv 1 score cp 40 pv e2e4".to_owned(),
            "info depth 20 multipv 2 score cp 30 pv d2d4".to_owned(),
            "info depth 20 multipv 3 score cp 20 pv c2c4".to_owned(),
        ];
        let result = extract_final_pvs(&lines, 2);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|l| l.pv_index <= 2));
    }

    #[test]
    fn extract_final_pvs_empty_input() {
        let result = extract_final_pvs(&[], 1);
        assert!(result.is_empty());
    }

    #[test]
    fn extract_final_pvs_skips_unparseable() {
        let lines = vec![
            "info string some debug message".to_owned(),
            "info depth 20 multipv 1 score cp 40 pv e2e4".to_owned(),
        ];
        let result = extract_final_pvs(&lines, 1);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].score_cp, Some(40));
    }
}
