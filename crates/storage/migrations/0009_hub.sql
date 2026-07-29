-- Hub-service (courier-hub) tables, prefixed `hub_` to keep them apart from
-- the desktop app's tables. The hub opens its own database file
-- (courier-hub.db) with the same migrator, so both schemas coexist in one
-- migration history; each database simply leaves the other side's tables empty.
--
-- hub_pending_links / hub_subscribers: the device-linking handshake and the
-- resulting per-install credential (see hub.rs for the flow). Both the link
-- token and the subscriber secret are stored only as SHA-256 digests; the
-- plaintext lives solely on the client that was handed it.
--
-- hub_tracked_accounts / hub_hero_names: state uploaded by the desktop app
-- via /sync. When a subscriber turns offline mode on, the hub's own tracker
-- polls `hub_tracked_accounts` and renders pushes with the synced names.

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
