// Harland Chess Trainer
// Copyright (C) 2026 Isaac Peterson
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3 as
// published by the Free Software Foundation.
//
// See the LICENSE file for the full license text.

//! Unit tests for Settings CRUD (Slice 7).

#[tokio::test]
async fn settings_default_values() {
    let storage = storage::Storage::new_in_memory()
        .await
        .expect("failed to create in-memory db");

    let settings = storage
        .get_settings()
        .await
        .expect("failed to get settings");

    assert_eq!(settings.lichess_username, "");
    assert_eq!(settings.max_games, 50);
    assert!(!settings.use_stockfish);
    assert_eq!(settings.inaccuracy_threshold_cp, 50);
    assert_eq!(settings.mistake_threshold_cp, 100);
    assert_eq!(settings.blunder_threshold_cp, 200);
}

#[tokio::test]
async fn settings_save_and_reload() {
    let storage = storage::Storage::new_in_memory()
        .await
        .expect("failed to create in-memory db");

    let updated = storage::UserSettings {
        lichess_username: "TestUser".to_owned(),
        max_games: 100,
        use_stockfish: true,
        inaccuracy_threshold_cp: 40,
        mistake_threshold_cp: 80,
        blunder_threshold_cp: 150,
    };

    storage
        .save_settings(&updated)
        .await
        .expect("failed to save settings");

    let loaded = storage
        .get_settings()
        .await
        .expect("failed to reload settings");

    assert_eq!(loaded.lichess_username, "TestUser");
    assert_eq!(loaded.max_games, 100);
    assert!(loaded.use_stockfish);
    assert_eq!(loaded.inaccuracy_threshold_cp, 40);
    assert_eq!(loaded.mistake_threshold_cp, 80);
    assert_eq!(loaded.blunder_threshold_cp, 150);
}

#[tokio::test]
async fn settings_overwrite_existing() {
    let storage = storage::Storage::new_in_memory()
        .await
        .expect("failed to create in-memory db");

    // Save first time
    storage
        .save_settings(&storage::UserSettings {
            lichess_username: "First".to_owned(),
            ..storage::UserSettings::default()
        })
        .await
        .expect("first save failed");

    // Overwrite with new values
    storage
        .save_settings(&storage::UserSettings {
            lichess_username: "Second".to_owned(),
            max_games: 30,
            ..storage::UserSettings::default()
        })
        .await
        .expect("second save failed");

    let loaded = storage
        .get_settings()
        .await
        .expect("failed to reload settings");

    assert_eq!(loaded.lichess_username, "Second");
    assert_eq!(loaded.max_games, 30);
}
