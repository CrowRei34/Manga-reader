pub const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS manga (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    source TEXT NOT NULL,
    url TEXT NOT NULL,
    title TEXT NOT NULL,
    cover_url TEXT,
    description TEXT,
    blob_json TEXT NOT NULL,
    added_at INTEGER NOT NULL,
    library INTEGER NOT NULL DEFAULT 0,
    UNIQUE(source, url)
);

CREATE TABLE IF NOT EXISTS chapter (
    manga_id INTEGER NOT NULL REFERENCES manga(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    name TEXT NOT NULL,
    number REAL NOT NULL DEFAULT 0,
    blob_json TEXT NOT NULL,
    read INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (manga_id, url)
);

CREATE TABLE IF NOT EXISTS category (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    color TEXT NOT NULL,
    auto_download INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS manga_category (
    manga_id INTEGER NOT NULL REFERENCES manga(id) ON DELETE CASCADE,
    category_id INTEGER NOT NULL REFERENCES category(id) ON DELETE CASCADE,
    PRIMARY KEY (manga_id, category_id)
);

CREATE TABLE IF NOT EXISTS history (
    manga_id INTEGER NOT NULL PRIMARY KEY REFERENCES manga(id) ON DELETE CASCADE,
    chapter_index INTEGER NOT NULL,
    page_index INTEGER NOT NULL DEFAULT 0,
    updated_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS download (
    manga_id INTEGER NOT NULL REFERENCES manga(id) ON DELETE CASCADE,
    chapter_url TEXT NOT NULL,
    state TEXT NOT NULL,
    total_pages INTEGER NOT NULL DEFAULT 0,
    done_pages INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (manga_id, chapter_url)
);

CREATE INDEX IF NOT EXISTS idx_history_updated ON history(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_chapter_manga ON chapter(manga_id);
"#;
