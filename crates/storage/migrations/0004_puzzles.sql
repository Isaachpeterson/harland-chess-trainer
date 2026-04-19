-- Slice 5: Puzzles table for generated training puzzles
CREATE TABLE IF NOT EXISTS puzzles (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mistake_id INTEGER NOT NULL REFERENCES mistakes(id) ON DELETE CASCADE,
    fen TEXT NOT NULL,
    solution_moves TEXT NOT NULL,           -- JSON array of UCI moves
    themes TEXT,                            -- JSON array, populated later
    created_at INTEGER NOT NULL
);
