-- User-managed watchlist: any player the user chooses to track, independent of
-- the Steam friend graph (pro players, strangers, or the user themselves).
-- Structurally mirrors `friends` / `friend_recent_games` but is populated by
-- direct add/remove rather than a wholesale Steam sync.

CREATE TABLE follows (
    steam_id         TEXT PRIMARY KEY,
    added_at         INTEGER NOT NULL,
    persona_name     TEXT NOT NULL,
    avatar           TEXT NOT NULL,
    profile_url      TEXT NOT NULL,
    persona_state    INTEGER NOT NULL,
    last_log_off     INTEGER,
    game_extra_info  TEXT,
    updated_at       INTEGER NOT NULL
);

CREATE TABLE follow_recent_games (
    steam_id         TEXT NOT NULL,
    app_id           INTEGER NOT NULL,
    name             TEXT NOT NULL,
    playtime_2weeks  INTEGER NOT NULL,
    playtime_forever INTEGER NOT NULL,
    img_icon_url     TEXT NOT NULL,
    PRIMARY KEY (steam_id, app_id)
);
