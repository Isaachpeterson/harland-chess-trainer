-- Slice 4: Mistakes table for blunder detection
CREATE TABLE IF NOT EXISTS mistakes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id TEXT NOT NULL REFERENCES games(id) ON DELETE CASCADE,
    ply INTEGER NOT NULL,
    fen_before TEXT NOT NULL,
    user_move TEXT NOT NULL,                -- UCI notation
    best_move TEXT NOT NULL,                -- UCI notation (from engine / lichess PV)
    eval_before_cp INTEGER,                -- centipawns, White's perspective
    eval_before_mate INTEGER,              -- mate-in-N, White's perspective
    eval_after_cp INTEGER,
    eval_after_mate INTEGER,
    classification TEXT NOT NULL            -- 'inaccuracy' | 'mistake' | 'blunder'
);
