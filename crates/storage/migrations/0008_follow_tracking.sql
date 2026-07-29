-- Background match tracking for a subset of follows. `tracked` enrolls a follow
-- in the polling loop; `last_tracked_match_id` is the watermark of the newest
-- match already seen, so only genuinely new matches trigger a push.

ALTER TABLE follows ADD COLUMN tracked INTEGER NOT NULL DEFAULT 0;
ALTER TABLE follows ADD COLUMN last_tracked_match_id INTEGER;
