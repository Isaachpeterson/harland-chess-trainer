-- Slice 7: User settings (single-row table)
CREATE TABLE IF NOT EXISTS user_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),   -- enforces exactly one row
    lichess_username TEXT NOT NULL DEFAULT '',
    max_games INTEGER NOT NULL DEFAULT 50,
    use_stockfish INTEGER NOT NULL DEFAULT 0,   -- boolean (0 | 1)
    inaccuracy_threshold_cp INTEGER NOT NULL DEFAULT 50,
    mistake_threshold_cp INTEGER NOT NULL DEFAULT 100,
    blunder_threshold_cp INTEGER NOT NULL DEFAULT 200,
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Seed the single row so that get_settings() always returns a value.
INSERT OR IGNORE INTO user_settings (id) VALUES (1);
