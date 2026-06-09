-- Static reference data (heroes, items) plus the user's friend list.
--
-- Localized names live in their own tables keyed by (entity, locale) so adding a
-- new UI locale needs no schema change: the sync layer just writes more rows.

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

-- Last successful sync per data set ('heroes', 'items', 'friends'), unix seconds.
CREATE TABLE sync_meta (
    key       TEXT PRIMARY KEY,
    synced_at INTEGER NOT NULL
);
