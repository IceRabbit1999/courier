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
