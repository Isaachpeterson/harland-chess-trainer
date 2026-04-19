-- Slice 6: Puzzle attempt tracking
CREATE TABLE IF NOT EXISTS puzzle_attempts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    puzzle_id INTEGER NOT NULL REFERENCES puzzles(id) ON DELETE CASCADE,
    attempted_at INTEGER NOT NULL,
    success INTEGER NOT NULL,               -- 0 or 1
    time_taken_ms INTEGER NOT NULL,
    move_played TEXT NOT NULL               -- what the user actually played (UCI)
);
