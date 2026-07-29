-- Cumulative mirror of the schema produced by all migrations in
-- `migrations/` (currently through 0008). This file is not itself applied —
-- it exists purely as a single reference for the current shape of the
-- database. Update it alongside every new migration.

CREATE TABLE heroes (
    id            INTEGER PRIMARY KEY,
    internal_name TEXT NOT NULL
);

CREATE TABLE hero_names (
    hero_id      INTEGER NOT NULL REFERENCES heroes(id) ON DELETE CASCADE,
    locale       TEXT NOT NULL,
    display_name TEXT NOT NULL,
    PRIMARY KEY (hero_id, locale)
);

CREATE TABLE items (
    id         INTEGER PRIMARY KEY,
    short_name TEXT NOT NULL
);

CREATE TABLE item_names (
    item_id      INTEGER NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    locale       TEXT NOT NULL,
    display_name TEXT NOT NULL,
    PRIMARY KEY (item_id, locale)
);

CREATE TABLE friends (
    steam_id         TEXT PRIMARY KEY,
    friend_since     INTEGER NOT NULL,
    persona_name     TEXT NOT NULL,
    avatar           TEXT NOT NULL,
    profile_url      TEXT NOT NULL,
    persona_state    INTEGER NOT NULL,
    last_log_off     INTEGER,
    game_extra_info  TEXT,
    updated_at       INTEGER NOT NULL
);

-- Last successful sync per data set ('heroes', 'items', 'friends', 'matches'), unix seconds.
CREATE TABLE sync_meta (
    key       TEXT PRIMARY KEY,
    synced_at INTEGER NOT NULL
);

-- Recently played games per friend (Steam GetRecentlyPlayedGames). Rewritten
-- wholesale alongside the friend list on each status refresh, so no foreign key
-- is declared; `replace_friends` clears this table explicitly.
CREATE TABLE friend_recent_games (
    steam_id         TEXT NOT NULL,
    app_id           INTEGER NOT NULL,
    name             TEXT NOT NULL,
    playtime_2weeks  INTEGER NOT NULL,
    playtime_forever INTEGER NOT NULL,
    img_icon_url     TEXT NOT NULL,
    PRIMARY KEY (steam_id, app_id)
);

-- Friends' fetched Dota 2 matches. Summaries come from OpenDota
-- /players/{account_id}/recentMatches and are keyed by the friend (steam_id)
-- whose history they belong to. Full detail (both teams + final items) is
-- fetched on demand into match_details / match_players, shared across friends.
CREATE TABLE match_summaries (
    steam_id      TEXT NOT NULL,
    match_id      INTEGER NOT NULL,
    hero_id       INTEGER NOT NULL,
    player_slot   INTEGER NOT NULL,
    radiant_win   INTEGER NOT NULL,
    start_time    INTEGER NOT NULL,
    duration      INTEGER NOT NULL,
    game_mode     INTEGER NOT NULL,
    lobby_type    INTEGER NOT NULL,
    kills         INTEGER NOT NULL,
    deaths        INTEGER NOT NULL,
    assists       INTEGER NOT NULL,
    gold_per_min  INTEGER NOT NULL,
    xp_per_min    INTEGER NOT NULL,
    last_hits     INTEGER NOT NULL,
    hero_damage   INTEGER NOT NULL,
    tower_damage  INTEGER NOT NULL,
    hero_healing  INTEGER NOT NULL,
    PRIMARY KEY (steam_id, match_id)
);

CREATE TABLE match_details (
    match_id         INTEGER PRIMARY KEY,
    radiant_win      INTEGER NOT NULL,
    duration         INTEGER NOT NULL,
    start_time       INTEGER NOT NULL,
    game_mode        INTEGER NOT NULL,
    lobby_type       INTEGER NOT NULL,
    radiant_score    INTEGER NOT NULL,
    dire_score       INTEGER NOT NULL,
    first_blood_time INTEGER NOT NULL
);

CREATE TABLE match_players (
    match_id           INTEGER NOT NULL,
    player_slot        INTEGER NOT NULL,
    account_id         INTEGER,
    hero_id            INTEGER NOT NULL,
    personaname        TEXT,
    kills              INTEGER NOT NULL,
    deaths             INTEGER NOT NULL,
    assists            INTEGER NOT NULL,
    last_hits          INTEGER NOT NULL,
    denies             INTEGER NOT NULL,
    gold_per_min       INTEGER NOT NULL,
    xp_per_min         INTEGER NOT NULL,
    level              INTEGER NOT NULL,
    net_worth          INTEGER NOT NULL,
    hero_damage        INTEGER NOT NULL,
    tower_damage       INTEGER NOT NULL,
    hero_healing       INTEGER NOT NULL,
    item_0             INTEGER NOT NULL,
    item_1             INTEGER NOT NULL,
    item_2             INTEGER NOT NULL,
    item_3             INTEGER NOT NULL,
    item_4             INTEGER NOT NULL,
    item_5             INTEGER NOT NULL,
    backpack_0         INTEGER NOT NULL,
    backpack_1         INTEGER NOT NULL,
    backpack_2         INTEGER NOT NULL,
    item_neutral       INTEGER NOT NULL,
    aghanims_scepter   INTEGER NOT NULL DEFAULT 0,
    aghanims_shard     INTEGER NOT NULL DEFAULT 0,
    item_neutral2      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (match_id, player_slot)
);

-- User-managed watchlist: any player the user chooses to track, independent of
-- the Steam friend graph (pro players, strangers, or the user themselves).
-- Structurally mirrors `friends` / `friend_recent_games` but is populated by
-- direct add/remove rather than a wholesale Steam sync.
CREATE TABLE follows (
    steam_id              TEXT PRIMARY KEY,
    added_at              INTEGER NOT NULL,
    persona_name          TEXT NOT NULL,
    avatar                TEXT NOT NULL,
    profile_url           TEXT NOT NULL,
    persona_state         INTEGER NOT NULL,
    last_log_off          INTEGER,
    game_extra_info       TEXT,
    updated_at            INTEGER NOT NULL,
    tracked               INTEGER NOT NULL DEFAULT 0,
    last_tracked_match_id INTEGER
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

-- Hub-service (courier-hub) tables, prefixed `hub_` to keep them apart from
-- the desktop app's tables. Live in the hub's own database file
-- (courier-hub.db), created by the shared migrator.
CREATE TABLE hub_pending_links (
    token_hash    TEXT PRIMARY KEY,
    secret_hash   TEXT NOT NULL,
    created_at    INTEGER NOT NULL,
    subscriber_id TEXT
);

CREATE TABLE hub_subscribers (
    subscriber_id TEXT PRIMARY KEY,
    secret_hash   TEXT NOT NULL UNIQUE,
    chat_id       INTEGER NOT NULL,
    created_at    INTEGER NOT NULL,
    offline_mode  INTEGER NOT NULL DEFAULT 0,
    locale        TEXT NOT NULL DEFAULT 'en'
);

CREATE TABLE hub_tracked_accounts (
    subscriber_id TEXT NOT NULL,
    steam_id      TEXT NOT NULL,
    persona_name  TEXT NOT NULL,
    last_match_id INTEGER,
    PRIMARY KEY (subscriber_id, steam_id)
);

CREATE TABLE hub_hero_names (
    locale  TEXT NOT NULL,
    hero_id INTEGER NOT NULL,
    name    TEXT NOT NULL,
    PRIMARY KEY (locale, hero_id)
);

CREATE TABLE hub_item_names (
    locale  TEXT NOT NULL,
    item_id INTEGER NOT NULL,
    name    TEXT NOT NULL,
    PRIMARY KEY (locale, item_id)
);
