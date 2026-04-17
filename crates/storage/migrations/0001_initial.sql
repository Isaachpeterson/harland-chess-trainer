-- Slice 1: Initial schema — games table
CREATE TABLE IF NOT EXISTS games (
    id TEXT PRIMARY KEY,              -- Lichess game ID
    pgn TEXT NOT NULL,
    user_color TEXT NOT NULL,          -- 'white' | 'black'
    user_result TEXT NOT NULL,         -- 'win' | 'loss' | 'draw'
    time_control TEXT,
    rated INTEGER NOT NULL,            -- 0 or 1
    created_at INTEGER NOT NULL,       -- unix timestamp (seconds)
    analysis_source TEXT,              -- 'lichess' | 'stockfish' | NULL
    analysis_completed_at INTEGER
);
