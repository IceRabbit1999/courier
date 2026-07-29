-- The hub's copy of the desktop app's localized item names, uploaded via /sync
-- alongside hub_hero_names (migration 0009). The offline-mode tracker fetches
-- full match detail from OpenDota, whose final inventory is item ids only; this
-- table is what lets the hub render the tracked player's items by name.

CREATE TABLE hub_item_names (
    locale  TEXT NOT NULL,
    item_id INTEGER NOT NULL,
    name    TEXT NOT NULL,
    PRIMARY KEY (locale, item_id)
);
