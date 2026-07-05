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
    match_id      INTEGER NOT NULL,
    player_slot   INTEGER NOT NULL,
    account_id    INTEGER,
    hero_id       INTEGER NOT NULL,
    personaname   TEXT,
    kills         INTEGER NOT NULL,
    deaths        INTEGER NOT NULL,
    assists       INTEGER NOT NULL,
    last_hits     INTEGER NOT NULL,
    denies        INTEGER NOT NULL,
    gold_per_min  INTEGER NOT NULL,
    xp_per_min    INTEGER NOT NULL,
    level         INTEGER NOT NULL,
    net_worth     INTEGER NOT NULL,
    hero_damage   INTEGER NOT NULL,
    tower_damage  INTEGER NOT NULL,
    hero_healing  INTEGER NOT NULL,
    item_0        INTEGER NOT NULL,
    item_1        INTEGER NOT NULL,
    item_2        INTEGER NOT NULL,
    item_3        INTEGER NOT NULL,
    item_4        INTEGER NOT NULL,
    item_5        INTEGER NOT NULL,
    backpack_0    INTEGER NOT NULL,
    backpack_1    INTEGER NOT NULL,
    backpack_2    INTEGER NOT NULL,
    item_neutral  INTEGER NOT NULL,
    PRIMARY KEY (match_id, player_slot)
);
