CREATE TABLE app_profiles (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO app_profiles (id, name) VALUES ('personal', 'Personal');

CREATE TABLE bookmark_sources (
    id TEXT PRIMARY KEY,
    app_profile_id TEXT NOT NULL REFERENCES app_profiles(id),
    source_type TEXT NOT NULL,
    source_key TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    last_synced_at TEXT
);

CREATE TABLE sync_runs (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL REFERENCES bookmark_sources(id),
    status TEXT NOT NULL,
    started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at TEXT,
    added_count INTEGER NOT NULL DEFAULT 0,
    updated_count INTEGER NOT NULL DEFAULT 0,
    unchanged_count INTEGER NOT NULL DEFAULT 0,
    skipped_count INTEGER NOT NULL DEFAULT 0,
    missing_count INTEGER NOT NULL DEFAULT 0,
    warning_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE folders (
    id TEXT PRIMARY KEY,
    app_profile_id TEXT NOT NULL REFERENCES app_profiles(id),
    source_id TEXT REFERENCES bookmark_sources(id),
    source_folder_id TEXT,
    parent_id TEXT REFERENCES folders(id),
    name TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    UNIQUE (source_id, source_folder_id)
);

CREATE INDEX folders_parent_position_idx ON folders(parent_id, position);
CREATE INDEX folders_profile_idx ON folders(app_profile_id);

CREATE TABLE bookmarks (
    id TEXT PRIMARY KEY,
    app_profile_id TEXT NOT NULL REFERENCES app_profiles(id),
    folder_id TEXT REFERENCES folders(id),
    title TEXT NOT NULL,
    url TEXT NOT NULL,
    normalized_url TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    is_archived INTEGER NOT NULL DEFAULT 0 CHECK (is_archived IN (0, 1))
);

CREATE INDEX bookmarks_profile_updated_idx ON bookmarks(app_profile_id, is_archived, updated_at);
CREATE INDEX bookmarks_folder_idx ON bookmarks(folder_id);
CREATE INDEX bookmarks_normalized_url_idx ON bookmarks(normalized_url);

CREATE TABLE source_bookmarks (
    source_id TEXT NOT NULL REFERENCES bookmark_sources(id),
    source_bookmark_id TEXT NOT NULL,
    bookmark_id TEXT NOT NULL REFERENCES bookmarks(id),
    last_seen_sync_run_id TEXT REFERENCES sync_runs(id),
    PRIMARY KEY (source_id, source_bookmark_id)
);

CREATE INDEX source_bookmarks_bookmark_idx ON source_bookmarks(bookmark_id);

CREATE TABLE categories (
    id TEXT PRIMARY KEY,
    app_profile_id TEXT NOT NULL REFERENCES app_profiles(id),
    name TEXT NOT NULL,
    normalized_name TEXT NOT NULL,
    UNIQUE (app_profile_id, normalized_name)
);

CREATE TABLE bookmark_categories (
    bookmark_id TEXT NOT NULL REFERENCES bookmarks(id) ON DELETE CASCADE,
    category_id TEXT NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    PRIMARY KEY (bookmark_id, category_id)
);

CREATE TABLE tags (
    id TEXT PRIMARY KEY,
    app_profile_id TEXT NOT NULL REFERENCES app_profiles(id),
    name TEXT NOT NULL,
    normalized_name TEXT NOT NULL,
    UNIQUE (app_profile_id, normalized_name)
);

CREATE TABLE bookmark_tags (
    bookmark_id TEXT NOT NULL REFERENCES bookmarks(id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (bookmark_id, tag_id)
);

CREATE TABLE import_runs (
    id TEXT PRIMARY KEY,
    app_profile_id TEXT NOT NULL REFERENCES app_profiles(id),
    format TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at TEXT,
    imported_count INTEGER NOT NULL DEFAULT 0,
    skipped_count INTEGER NOT NULL DEFAULT 0,
    warning_count INTEGER NOT NULL DEFAULT 0
);

CREATE VIRTUAL TABLE bookmarks_fts USING fts5(bookmark_id UNINDEXED, title, url);

CREATE TRIGGER bookmarks_fts_insert AFTER INSERT ON bookmarks
WHEN new.is_archived = 0
BEGIN
    INSERT INTO bookmarks_fts (bookmark_id, title, url)
    VALUES (new.id, new.title, new.url);
END;

CREATE TRIGGER bookmarks_fts_update AFTER UPDATE ON bookmarks
BEGIN
    DELETE FROM bookmarks_fts WHERE bookmark_id = old.id;
    INSERT INTO bookmarks_fts (bookmark_id, title, url)
    SELECT new.id, new.title, new.url WHERE new.is_archived = 0;
END;

CREATE TRIGGER bookmarks_fts_delete AFTER DELETE ON bookmarks
BEGIN
    DELETE FROM bookmarks_fts WHERE bookmark_id = old.id;
END;