-- Slice 3: Move evaluations table for game analysis pipeline
CREATE TABLE IF NOT EXISTS move_evaluations (
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    ply INTEGER NOT NULL,
    eval_cp INTEGER,            -- centipawn eval from White's perspective
    eval_mate INTEGER,          -- mate-in-N from White's perspective (+ = White mates)
    source TEXT NOT NULL,       -- 'lichess' | 'stockfish'
    PRIMARY KEY (game_id, ply)
);
