// Manual verification for Slice 6: Puzzle attempt tracking.
//
// Run with:
//   cargo test -p storage --test verify_attempts -- --ignored --nocapture

use std::path::PathBuf;

#[tokio::test]
#[ignore]
async fn verify_attempt_tracking() {
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

    // Step 1: Check puzzle count
    let puzzle_count = storage
        .puzzle_count()
        .await
        .expect("failed to count puzzles");
    println!("Puzzles in database: {puzzle_count}");
    assert!(
        puzzle_count > 0,
        "Need at least 1 puzzle for verification (run Slice 5 first)"
    );

    // Step 2: Get next puzzle repeatedly — confirm it returns puzzles
    let mut seen_ids = std::collections::HashSet::new();
    for i in 0..std::cmp::min(puzzle_count, 10) {
        let puzzle = storage
            .get_next_puzzle()
            .await
            .expect("failed to get next puzzle")
            .expect("expected a puzzle but got None");
        println!(
            "get_next_puzzle call {}: puzzle id={}, fen={}...",
            i + 1,
            puzzle.id,
            &puzzle.fen[..std::cmp::min(40, puzzle.fen.len())]
        );
        seen_ids.insert(puzzle.id);
    }
    println!("Unique puzzle IDs seen: {}", seen_ids.len());

    // Step 3: Submit attempts (mix of success/failure)
    let puzzle = storage
        .get_next_puzzle()
        .await
        .expect("failed to get puzzle")
        .expect("no puzzles");
    let pid = puzzle.id;

    println!("\nSubmitting attempts for puzzle id={pid}...");

    let a1 = storage
        .record_attempt(pid, false, 5000, "e7e5")
        .await
        .expect("failed to record attempt 1");
    println!("  Attempt 1 (failure): id={a1}");

    let a2 = storage
        .record_attempt(pid, true, 3000, "d7d5")
        .await
        .expect("failed to record attempt 2");
    println!("  Attempt 2 (success): id={a2}");

    let a3 = storage
        .record_attempt(pid, true, 2500, "d7d5")
        .await
        .expect("failed to record attempt 3");
    println!("  Attempt 3 (success): id={a3}");

    // Step 4: Verify attempts persisted
    let attempts = storage
        .get_attempts_for_puzzle(pid)
        .await
        .expect("failed to get attempts");
    println!("\nAttempts for puzzle {pid}: {}", attempts.len());
    for a in &attempts {
        println!(
            "  id={}, success={}, time={}ms, move={}",
            a.id, a.success, a.time_taken_ms, a.move_played
        );
    }
    // We submitted 3 (there may be more from prior runs)
    assert!(
        attempts.len() >= 3,
        "Expected at least 3 attempts, got {}",
        attempts.len()
    );

    // Step 5: Verify summary
    let summary = storage
        .get_attempts_summary()
        .await
        .expect("failed to get summary");
    println!("\nAttempts summary:");
    println!("  total_attempts: {}", summary.total_attempts);
    println!("  total_successes: {}", summary.total_successes);
    println!("  success_rate: {:.2}%", summary.success_rate * 100.0);
    println!("  puzzles_attempted: {}", summary.puzzles_attempted);
    println!(
        "  puzzles_attempted_today: {}",
        summary.puzzles_attempted_today
    );

    assert!(summary.total_attempts >= 3);
    assert!(summary.total_successes >= 2);
    assert!(summary.success_rate > 0.0);
    assert!(summary.puzzles_attempted >= 1);
    assert!(summary.puzzles_attempted_today >= 1);

    println!("\n✓ All Slice 6 manual verification steps passed.");

    // Cleanup: remove the test attempts we inserted so this test is re-runnable
    sqlx::query("DELETE FROM puzzle_attempts WHERE puzzle_id = ?1 AND move_played IN ('e7e5', 'd7d5') AND time_taken_ms IN (5000, 3000, 2500)")
        .bind(pid)
        .execute(storage.pool_ref())
        .await
        .expect("cleanup failed");
    println!("Cleaned up test attempts.");
}
