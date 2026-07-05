-- Whether each player ended the match with Aghanim's Scepter / Shard. Added in a
-- separate migration so existing match_players rows keep their data.

ALTER TABLE match_players ADD COLUMN aghanims_scepter INTEGER NOT NULL DEFAULT 0;
ALTER TABLE match_players ADD COLUMN aghanims_shard INTEGER NOT NULL DEFAULT 0;
