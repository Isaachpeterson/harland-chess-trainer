// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

//! Integration tests for lichess-client using wiremock.

use lichess_client::{LichessClient, LichessError, LichessGame};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const FIXTURE: &str = include_str!("fixtures/sample_games.ndjson");

#[tokio::test]
async fn fetch_parses_ndjson_fixture() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/games/user/TestUser"))
        .and(query_param("max", "50"))
        .and(query_param("evals", "true"))
        .and(query_param("clocks", "true"))
        .and(query_param("opening", "true"))
        .and(query_param("pgnInJson", "true"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(FIXTURE)
                .insert_header("content-type", "application/x-ndjson"),
        )
        .mount(&server)
        .await;

    let client = LichessClient::with_base_url(&server.uri()).unwrap();
    let games = client.fetch_user_games("TestUser", 50).await.unwrap();

    assert_eq!(games.len(), 3);

    // First game
    assert_eq!(games[0].id, "abcd1234");
    assert!(games[0].rated);
    assert_eq!(games[0].speed, "rapid");
    assert_eq!(games[0].winner.as_deref(), Some("white"));
    assert_eq!(games[0].user_color("TestUser"), Some("white".to_owned()));
    assert_eq!(games[0].user_result("TestUser"), Some("win".to_owned()));
    assert_eq!(
        games[0].pgn.as_deref(),
        Some("1. e4 e5 2. Nf3 Nc6 3. Bc4 Bc5 *")
    );

    // Second game — user is black, won
    assert_eq!(games[1].id, "efgh5678");
    assert_eq!(games[1].user_color("TestUser"), Some("black".to_owned()));
    assert_eq!(games[1].user_result("TestUser"), Some("win".to_owned()));

    // Third game — draw, unrated
    assert_eq!(games[2].id, "ijkl9012");
    assert!(!games[2].rated);
    assert_eq!(games[2].winner, None);
    assert_eq!(games[2].user_result("TestUser"), Some("draw".to_owned()));
}

#[tokio::test]
async fn fetch_returns_user_not_found_on_404() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/games/user/nonexistent"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let client = LichessClient::with_base_url(&server.uri()).unwrap();
    let result = client.fetch_user_games("nonexistent", 10).await;

    assert!(matches!(result, Err(LichessError::UserNotFound(_))));
}

#[tokio::test]
async fn fetch_retries_on_429_then_succeeds() {
    let server = MockServer::start().await;

    // First call → 429, second call → 200
    Mock::given(method("GET"))
        .and(path("/api/games/user/TestUser"))
        .respond_with(ResponseTemplate::new(429))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/api/games/user/TestUser"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(
                    r#"{"id":"retry01","rated":true,"variant":"standard","speed":"rapid","perf":"rapid","createdAt":1700000000000,"lastMoveAt":1700003600000,"status":"resign","players":{"white":{"user":{"name":"TestUser","id":"testuser"},"rating":1600},"black":{"user":{"name":"Bot","id":"bot"},"rating":1500}},"winner":"white","pgn":"1. e4 *"}"#,
                )
                .insert_header("content-type", "application/x-ndjson"),
        )
        .mount(&server)
        .await;

    let client = LichessClient::with_base_url(&server.uri()).unwrap();
    let games = client.fetch_user_games("TestUser", 10).await.unwrap();
    assert_eq!(games.len(), 1);
    assert_eq!(games[0].id, "retry01");
}

#[tokio::test]
async fn fetch_empty_response_returns_empty_vec() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/games/user/NewPlayer"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("")
                .insert_header("content-type", "application/x-ndjson"),
        )
        .mount(&server)
        .await;

    let client = LichessClient::with_base_url(&server.uri()).unwrap();
    let games = client.fetch_user_games("NewPlayer", 10).await.unwrap();
    assert!(games.is_empty());
}

#[tokio::test]
async fn user_color_is_case_insensitive() {
    let game: LichessGame = serde_json::from_str(
        r#"{"id":"case01","rated":true,"variant":"standard","speed":"rapid","perf":"rapid","createdAt":1700000000000,"lastMoveAt":1700003600000,"status":"resign","players":{"white":{"user":{"name":"TestUser","id":"testuser"},"rating":1600},"black":{"user":{"name":"Bot","id":"bot"},"rating":1500}},"winner":"white","pgn":"1. e4 *"}"#,
    )
    .unwrap();

    assert_eq!(game.user_color("testuser"), Some("white".to_owned()));
    assert_eq!(game.user_color("TESTUSER"), Some("white".to_owned()));
    assert_eq!(game.user_color("TestUser"), Some("white".to_owned()));
}

/// This test hits the real Lichess API. Run manually:
/// `cargo test -p lichess-client -- --ignored`
#[tokio::test]
#[ignore]
async fn real_lichess_fetch() {
    let client = LichessClient::new().unwrap();
    let games = client.fetch_user_games("Isaachpeterson", 5).await.unwrap();
    assert!(!games.is_empty());
    assert!(games.len() <= 5);
    for game in &games {
        assert!(!game.id.is_empty());
        assert!(game.pgn.is_some());
    }
}
