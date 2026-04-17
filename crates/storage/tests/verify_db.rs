// Quick verification test — reads the real database to confirm game count.
// Run with: cargo test -p storage --test verify_db -- --ignored

use std::path::PathBuf;

#[tokio::test]
#[ignore]
async fn verify_synced_games() {
    let db_path = PathBuf::from(std::env::var("APPDATA").expect("APPDATA not set"))
        .join("com.harland.chesstrainer")
        .join("harland.db");

    assert!(
        db_path.exists(),
        "Database not found at {}",
        db_path.display()
    );

    let storage = storage::Storage::new(&db_path)
        .await
        .expect("failed to open db");
    let count = storage.game_count().await.expect("failed to count games");
    println!("Games in database: {count}");
    assert_eq!(count, 50, "Expected 50 games, found {count}");

    // Spot-check one game has sensible data
    // We don't know a specific ID, so just query any
    let row = sqlx::query("SELECT id, user_color, user_result, pgn FROM games LIMIT 1")
        .fetch_one(storage.pool_ref())
        .await
        .expect("failed to fetch a game");

    use sqlx::Row;
    let id: String = row.get("id");
    let pgn: String = row.get("pgn");
    let color: String = row.get("user_color");
    let result: String = row.get("user_result");

    println!(
        "Sample game: id={id}, color={color}, result={result}, pgn_len={}",
        pgn.len()
    );
    assert!(!id.is_empty());
    assert!(!pgn.is_empty());
    assert!(color == "white" || color == "black");
    assert!(result == "win" || result == "loss" || result == "draw");
}
