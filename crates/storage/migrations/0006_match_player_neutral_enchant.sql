-- The neutral item's active enchantment (OpenDota `item_neutral2`). Added in a
-- separate migration so existing match_players rows keep their data.

ALTER TABLE match_players ADD COLUMN item_neutral2 INTEGER NOT NULL DEFAULT 0;
