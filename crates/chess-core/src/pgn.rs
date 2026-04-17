// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

//! PGN parsing and evaluation extraction.
//!
//! Parses a PGN string into a sequence of [`ParsedMove`] structs, replaying
//! the game with `shakmaty` for position tracking. Extracts embedded `%eval`
//! comments (Lichess format) when present.

use crate::ChessCoreError;
use shakmaty::fen::Fen;
use shakmaty::san::San;
use shakmaty::{CastlingMode, Chess, Color, EnPassantMode, Position};

/// Evaluation extracted from a PGN `%eval` comment.
///
/// Values are from White's perspective (positive = White is better).
#[derive(Debug, Clone, PartialEq)]
pub struct PgnEval {
    /// Centipawn evaluation, or `None` if this is a mate score.
    pub eval_cp: Option<i32>,
    /// Mate-in-N (positive = White mates, negative = Black mates), or `None` for cp scores.
    pub eval_mate: Option<i32>,
}

/// A single move parsed from a PGN, with position context and optional eval.
#[derive(Debug, Clone)]
pub struct ParsedMove {
    /// Half-move index (0 = White's first move, 1 = Black's first move, etc.).
    pub ply: u32,
    /// FEN of the position before this move was played.
    pub fen_before: String,
    /// FEN of the position after this move was played.
    pub fen_after: String,
    /// The move in UCI notation (e.g., "e2e4").
    pub move_uci: String,
    /// Whether this move was made by the specified user.
    pub is_user_move: bool,
    /// Evaluation from a PGN `[%eval ...]` comment, if present.
    /// This is the eval of the position *after* this move, from White's perspective.
    pub lichess_eval: Option<PgnEval>,
}

/// Parses a PGN string into a sequence of moves with position tracking.
///
/// `user_color` should be `"white"` or `"black"` and is used to tag which
/// moves belong to the user. Evaluations from `[%eval ...]` comments are
/// extracted when present.
pub fn parse_pgn(pgn: &str, user_color: &str) -> Result<Vec<ParsedMove>, ChessCoreError> {
    let color = match user_color {
        "white" => Color::White,
        "black" => Color::Black,
        _ => {
            return Err(ChessCoreError::PgnParse(format!(
                "invalid user_color: {user_color}"
            )))
        }
    };

    let movetext = extract_movetext(pgn);
    let tokens = tokenize_movetext(&movetext);

    let mut position = Chess::default();
    let mut ply = 0u32;
    let mut moves = Vec::new();

    for token in &tokens {
        match token {
            Token::Move(san_str) => {
                let fen_before =
                    Fen::from_position(position.clone(), EnPassantMode::Legal).to_string();

                let san: San = san_str.parse().map_err(|_| ChessCoreError::IllegalMove {
                    ply,
                    san: san_str.clone(),
                })?;

                let m = san
                    .to_move(&position)
                    .map_err(|_| ChessCoreError::IllegalMove {
                        ply,
                        san: san_str.clone(),
                    })?;

                let uci = m.to_uci(CastlingMode::Standard).to_string();

                position.play_unchecked(&m);

                let fen_after =
                    Fen::from_position(position.clone(), EnPassantMode::Legal).to_string();

                let is_user = (ply % 2 == 0 && color == Color::White)
                    || (ply % 2 == 1 && color == Color::Black);

                moves.push(ParsedMove {
                    ply,
                    fen_before,
                    fen_after,
                    move_uci: uci,
                    is_user_move: is_user,
                    lichess_eval: None,
                });

                ply += 1;
            }
            Token::Comment(text) => {
                if let Some(eval) = parse_eval_comment(text) {
                    if let Some(last_move) = moves.last_mut() {
                        last_move.lichess_eval = Some(eval);
                    }
                }
            }
            Token::Result(_) => break,
        }
    }

    Ok(moves)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum Token {
    Move(String),
    Comment(String),
    Result(()),
}

/// Strips PGN headers, returning only the movetext portion.
fn extract_movetext(pgn: &str) -> String {
    pgn.lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !(trimmed.starts_with('[') && trimmed.ends_with(']'))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Tokenizes PGN movetext into moves, comments, and game results.
fn tokenize_movetext(movetext: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = movetext.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            '{' => {
                chars.next();
                let mut comment = String::new();
                let mut depth = 1u32;
                while let Some(c) = chars.next() {
                    if c == '{' {
                        depth += 1;
                        comment.push(c);
                    } else if c == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        comment.push(c);
                    } else {
                        comment.push(c);
                    }
                }
                tokens.push(Token::Comment(comment));
            }
            ' ' | '\n' | '\r' | '\t' => {
                chars.next();
            }
            _ => {
                let mut word = String::new();
                while let Some(&c) = chars.peek() {
                    if c == ' ' || c == '\n' || c == '\r' || c == '\t' || c == '{' {
                        break;
                    }
                    word.push(c);
                    chars.next();
                }
                if word == "1-0" || word == "0-1" || word == "1/2-1/2" || word == "*" {
                    tokens.push(Token::Result(()));
                } else if !is_move_number(&word) {
                    tokens.push(Token::Move(word));
                }
            }
        }
    }
    tokens
}

/// Returns `true` if the string looks like a PGN move number (e.g., "1.", "15...", "2.").
fn is_move_number(s: &str) -> bool {
    let trimmed = s.trim_end_matches('.');
    !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_digit())
}

/// Parses a `[%eval ...]` annotation from a PGN comment.
///
/// Handles both centipawn evals (`[%eval 0.35]`, `[%eval -1.5]`) and mate
/// scores (`[%eval #3]`, `[%eval #-2]`).
fn parse_eval_comment(comment: &str) -> Option<PgnEval> {
    let eval_start = comment.find("[%eval ")?;
    let after_prefix = &comment[eval_start + 7..];
    let bracket_end = after_prefix.find(']')?;
    let eval_str = after_prefix[..bracket_end].trim();

    if let Some(mate_str) = eval_str.strip_prefix('#') {
        let mate_in: i32 = mate_str.parse().ok()?;
        Some(PgnEval {
            eval_cp: None,
            eval_mate: Some(mate_in),
        })
    } else {
        let cp_float: f64 = eval_str.parse().ok()?;
        let cp = (cp_float * 100.0).round() as i32;
        Some(PgnEval {
            eval_cp: Some(cp),
            eval_mate: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // parse_eval_comment
    // -----------------------------------------------------------------------

    #[test]
    fn eval_positive_cp() {
        let eval = parse_eval_comment("[%eval 0.35]").unwrap();
        assert_eq!(eval.eval_cp, Some(35));
        assert_eq!(eval.eval_mate, None);
    }

    #[test]
    fn eval_negative_cp() {
        let eval = parse_eval_comment("[%eval -1.5]").unwrap();
        assert_eq!(eval.eval_cp, Some(-150));
        assert_eq!(eval.eval_mate, None);
    }

    #[test]
    fn eval_mate_positive() {
        let eval = parse_eval_comment("[%eval #3]").unwrap();
        assert_eq!(eval.eval_cp, None);
        assert_eq!(eval.eval_mate, Some(3));
    }

    #[test]
    fn eval_mate_negative() {
        let eval = parse_eval_comment("[%eval #-2]").unwrap();
        assert_eq!(eval.eval_cp, None);
        assert_eq!(eval.eval_mate, Some(-2));
    }

    #[test]
    fn eval_with_clock_comment() {
        // Lichess PGN comments often contain both eval and clock
        let eval = parse_eval_comment(" [%eval 0.3] [%clk 0:10:00] ").unwrap();
        assert_eq!(eval.eval_cp, Some(30));
    }

    #[test]
    fn eval_zero() {
        let eval = parse_eval_comment("[%eval 0.0]").unwrap();
        assert_eq!(eval.eval_cp, Some(0));
    }

    #[test]
    fn eval_no_eval_returns_none() {
        assert!(parse_eval_comment("[%clk 0:10:00]").is_none());
    }

    #[test]
    fn eval_empty_returns_none() {
        assert!(parse_eval_comment("").is_none());
    }

    // -----------------------------------------------------------------------
    // tokenize_movetext
    // -----------------------------------------------------------------------

    #[test]
    fn tokenize_simple_movetext() {
        let tokens = tokenize_movetext("1. e4 e5 2. Nf3 Nc6 *");
        let move_count = tokens
            .iter()
            .filter(|t| matches!(t, Token::Move(_)))
            .count();
        assert_eq!(move_count, 4); // e4, e5, Nf3, Nc6
    }

    #[test]
    fn tokenize_with_comments() {
        let tokens = tokenize_movetext("1. e4 { [%eval 0.3] } 1... e5 { [%eval 0.23] } *");
        let moves: Vec<_> = tokens
            .iter()
            .filter_map(|t| match t {
                Token::Move(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(moves, vec!["e4", "e5"]);

        let comments: Vec<_> = tokens
            .iter()
            .filter_map(|t| match t {
                Token::Comment(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(comments.len(), 2);
        assert!(comments[0].contains("%eval 0.3"));
    }

    #[test]
    fn tokenize_result_stops() {
        let tokens = tokenize_movetext("1. e4 e5 1-0");
        let results = tokens
            .iter()
            .filter(|t| matches!(t, Token::Result(())))
            .count();
        assert_eq!(results, 1);
    }

    // -----------------------------------------------------------------------
    // extract_movetext
    // -----------------------------------------------------------------------

    #[test]
    fn extract_movetext_strips_headers() {
        let pgn = r#"[Event "Rated Rapid game"]
[Site "https://lichess.org/xxxx"]

1. e4 e5 2. Nf3 *"#;
        let mt = extract_movetext(pgn);
        assert!(mt.starts_with("1."));
        assert!(!mt.contains("[Event"));
    }

    // -----------------------------------------------------------------------
    // parse_pgn
    // -----------------------------------------------------------------------

    #[test]
    fn parse_simple_pgn_white() {
        let pgn = "1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 *";
        let moves = parse_pgn(pgn, "white").unwrap();
        assert_eq!(moves.len(), 6);

        assert_eq!(moves[0].ply, 0);
        assert_eq!(moves[0].move_uci, "e2e4");
        assert!(moves[0].is_user_move);

        assert_eq!(moves[1].ply, 1);
        assert_eq!(moves[1].move_uci, "e7e5");
        assert!(!moves[1].is_user_move);

        assert_eq!(moves[2].move_uci, "g1f3");
        assert!(moves[2].is_user_move);
    }

    #[test]
    fn parse_simple_pgn_black() {
        let pgn = "1. e4 e5 *";
        let moves = parse_pgn(pgn, "black").unwrap();
        assert!(!moves[0].is_user_move); // e4 is White
        assert!(moves[1].is_user_move); // e5 is Black
    }

    #[test]
    fn parse_pgn_with_evals() {
        let pgn = "1. e4 { [%eval 0.3] } 1... e5 { [%eval 0.23] } 2. Nf3 { [%eval 0.5] } *";
        let moves = parse_pgn(pgn, "white").unwrap();
        assert_eq!(moves.len(), 3);

        let eval0 = moves[0].lichess_eval.as_ref().unwrap();
        assert_eq!(eval0.eval_cp, Some(30));

        let eval1 = moves[1].lichess_eval.as_ref().unwrap();
        assert_eq!(eval1.eval_cp, Some(23));

        let eval2 = moves[2].lichess_eval.as_ref().unwrap();
        assert_eq!(eval2.eval_cp, Some(50));
    }

    #[test]
    fn parse_pgn_with_headers() {
        let pgn = r#"[Event "Rated Rapid game"]
[Site "https://lichess.org/xxxx"]
[White "Player1"]
[Black "Player2"]
[Result "1-0"]

1. e4 e5 2. Nf3 Nc6 1-0"#;
        let moves = parse_pgn(pgn, "white").unwrap();
        assert_eq!(moves.len(), 4);
        assert_eq!(moves[0].move_uci, "e2e4");
    }

    #[test]
    fn parse_pgn_fen_tracking() {
        let pgn = "1. e4 e5 *";
        let moves = parse_pgn(pgn, "white").unwrap();

        // Starting FEN
        assert!(moves[0]
            .fen_before
            .contains("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR"));
        // After 1. e4
        assert!(moves[0]
            .fen_after
            .contains("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR"));
        // Before 1... e5 = after 1. e4
        assert_eq!(moves[1].fen_before, moves[0].fen_after);
    }

    #[test]
    fn parse_pgn_invalid_color() {
        let result = parse_pgn("1. e4 *", "purple");
        assert!(result.is_err());
    }

    #[test]
    fn parse_pgn_castling() {
        // Italian Game with kingside castling
        let pgn = "1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 4. O-O Nf6 *";
        let moves = parse_pgn(pgn, "white").unwrap();
        assert_eq!(moves.len(), 8);
        // O-O in UCI is e1g1
        assert_eq!(moves[6].move_uci, "e1g1");
    }

    #[test]
    fn parse_pgn_promotion() {
        // Contrived position via PGN is hard; test that the parser doesn't choke on
        // a game that reaches promotion. We'll just test that SAN with = is recognized
        // by verifying the tokenizer handles it.
        let tokens = tokenize_movetext("1. a8=Q *");
        let moves: Vec<_> = tokens
            .iter()
            .filter_map(|t| match t {
                Token::Move(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(moves, vec!["a8=Q"]);
    }

    #[test]
    fn parse_pgn_mate_eval() {
        let pgn = "1. e4 { [%eval #3] } *";
        let moves = parse_pgn(pgn, "white").unwrap();
        let eval = moves[0].lichess_eval.as_ref().unwrap();
        assert_eq!(eval.eval_mate, Some(3));
        assert_eq!(eval.eval_cp, None);
    }

    #[test]
    fn parse_pgn_empty_moves() {
        let moves = parse_pgn("*", "white").unwrap();
        assert!(moves.is_empty());
    }
}
