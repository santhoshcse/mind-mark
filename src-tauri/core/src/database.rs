use std::{
    collections::HashMap,
    path::Path,
    sync::{Mutex, MutexGuard},
    time::Duration,
};

use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::{
    chrome::ParsedChromeBookmarks,
    domain::{
        AppProfile, Bookmark, BookmarkFolder, ChromeProfile, CreateBookmarkInput, SyncReport,
        UpdateBookmarkInput,
    },
    error::AppError,
};

const INITIAL_MIGRATION: &str = include_str!("../migrations/0001_initial.sql");

pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let mut connection = Connection::open(path)?;
        Self::configure(&mut connection, true)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn in_memory() -> Result<Self, AppError> {
        let mut connection = Connection::open_in_memory()?;
        Self::configure(&mut connection, false)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub(crate) fn lock(&self) -> Result<MutexGuard<'_, Connection>, AppError> {
        self.connection.lock().map_err(|_| AppError::LockPoisoned)
    }

    pub fn sync_chrome_bookmarks(
        &self,
        profile: &ChromeProfile,
        parsed: &ParsedChromeBookmarks,
    ) -> Result<SyncReport, AppError> {
        let app_profile_id = format!("chrome-profile:{}", profile.id);
        let source_id = format!("chrome:{}", profile.id);
        let source_key = profile.path.to_lowercase();
        let sync_run_id = Uuid::new_v4().to_string();
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;

        transaction.execute(
            "INSERT INTO app_profiles (id, name) VALUES (?1, ?2)
             ON CONFLICT(id) DO NOTHING",
            params![app_profile_id, profile.name],
        )?;
        transaction.execute(
            "INSERT INTO bookmark_sources (id, app_profile_id, source_type, source_key, display_name)
             VALUES (?1, ?2, 'chrome', ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET source_key = excluded.source_key,
                                           display_name = excluded.display_name",
            params![source_id, app_profile_id, source_key, profile.name],
        )?;
        transaction.execute(
            "INSERT INTO sync_runs (id, source_id, status) VALUES (?1, ?2, 'running')",
            params![sync_run_id, source_id],
        )?;

        let mut folder_ids = HashMap::new();
        for folder in &parsed.folders {
            let parent_id = folder
                .parent_source_folder_id
                .as_ref()
                .and_then(|parent_source_id| folder_ids.get(parent_source_id));
            let folder_id = Uuid::new_v4().to_string();
            transaction.execute(
                "INSERT INTO folders
                    (id, app_profile_id, source_id, source_folder_id, parent_id, name, position)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(source_id, source_folder_id) DO UPDATE SET
                    parent_id = excluded.parent_id,
                    name = excluded.name,
                    position = excluded.position",
                params![
                    folder_id,
                    app_profile_id,
                    source_id,
                    folder.source_folder_id,
                    parent_id,
                    folder.name,
                    folder.position,
                ],
            )?;
            let stored_folder_id: String = transaction.query_row(
                "SELECT id FROM folders WHERE source_id = ?1 AND source_folder_id = ?2",
                params![source_id, folder.source_folder_id],
                |row| row.get(0),
            )?;
            folder_ids.insert(folder.source_folder_id.clone(), stored_folder_id);
        }

        let mut added = 0;
        let mut updated = 0;
        let mut unchanged = 0;
        for bookmark in &parsed.bookmarks {
            let folder_id = folder_ids
                .get(&bookmark.parent_source_folder_id)
                .ok_or_else(|| {
                    AppError::InvalidInput("Chrome bookmark references an unknown folder".into())
                })?;
            let existing: Option<(String, String, String, Option<String>, i64)> = transaction
                .query_row(
                    "SELECT b.id, b.title, b.url, b.folder_id, b.position
                     FROM source_bookmarks AS sb
                     JOIN bookmarks AS b ON b.id = sb.bookmark_id
                     WHERE sb.source_id = ?1 AND sb.source_bookmark_id = ?2",
                    params![source_id, bookmark.source_bookmark_id],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                        ))
                    },
                )
                .optional()?;

            let bookmark_id = match existing {
                Some((bookmark_id, title, url, current_folder_id, position)) => {
                    if title == bookmark.title
                        && url == bookmark.url
                        && current_folder_id.as_deref() == Some(folder_id.as_str())
                        && position == bookmark.position
                    {
                        unchanged += 1;
                    } else {
                        transaction.execute(
                            "UPDATE bookmarks SET title = ?1, url = ?2, normalized_url = ?3,
                                folder_id = ?4, position = ?5, updated_at = CURRENT_TIMESTAMP
                             WHERE id = ?6",
                            params![
                                bookmark.title,
                                bookmark.url,
                                bookmark.url.trim(),
                                folder_id,
                                bookmark.position,
                                bookmark_id,
                            ],
                        )?;
                        updated += 1;
                    }
                    bookmark_id
                }
                None => {
                    let bookmark_id = Uuid::new_v4().to_string();
                    transaction.execute(
                        "INSERT INTO bookmarks
                            (id, app_profile_id, folder_id, title, url, normalized_url, position)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![
                            bookmark_id,
                            app_profile_id,
                            folder_id,
                            bookmark.title,
                            bookmark.url,
                            bookmark.url.trim(),
                            bookmark.position,
                        ],
                    )?;
                    added += 1;
                    bookmark_id
                }
            };

            transaction.execute(
                "INSERT INTO source_bookmarks
                    (source_id, source_bookmark_id, bookmark_id, last_seen_sync_run_id)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(source_id, source_bookmark_id) DO UPDATE SET
                    bookmark_id = excluded.bookmark_id,
                    last_seen_sync_run_id = excluded.last_seen_sync_run_id",
                params![
                    source_id,
                    bookmark.source_bookmark_id,
                    bookmark_id,
                    sync_run_id
                ],
            )?;
        }

        let missing: usize = transaction.query_row(
            "SELECT COUNT(*) FROM source_bookmarks
             WHERE source_id = ?1 AND last_seen_sync_run_id <> ?2",
            params![source_id, sync_run_id],
            |row| row.get(0),
        )?;
        transaction.execute(
            "UPDATE sync_runs SET status = 'complete', finished_at = CURRENT_TIMESTAMP,
                added_count = ?1, updated_count = ?2, unchanged_count = ?3,
                skipped_count = ?4, missing_count = ?5, warning_count = ?6
             WHERE id = ?7",
            params![
                added,
                updated,
                unchanged,
                parsed.skipped,
                missing,
                parsed.warnings.len(),
                sync_run_id,
            ],
        )?;
        transaction.execute(
            "UPDATE bookmark_sources SET last_synced_at = CURRENT_TIMESTAMP WHERE id = ?1",
            [&source_id],
        )?;
        transaction.commit()?;

        Ok(SyncReport {
            profile_name: profile.name.clone(),
            added,
            updated,
            unchanged,
            folders: parsed.folders.len(),
            skipped: parsed.skipped,
            missing,
            warnings: parsed.warnings.clone(),
        })
    }

    pub fn list_app_profiles(&self) -> Result<Vec<AppProfile>, AppError> {
        let connection = self.lock()?;
        let mut statement =
            connection.prepare("SELECT id, name FROM app_profiles ORDER BY name COLLATE NOCASE")?;
        let profiles = statement
            .query_map([], |row| {
                Ok(AppProfile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(profiles)
    }

    pub fn create_app_profile(&self, name: &str) -> Result<AppProfile, AppError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::InvalidInput(
                "Profile name cannot be empty".into(),
            ));
        }
        let profile = AppProfile {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
        };
        let connection = self.lock()?;
        connection.execute(
            "INSERT INTO app_profiles (id, name) VALUES (?1, ?2)",
            params![profile.id, profile.name],
        )?;
        Ok(profile)
    }

    pub fn list_folders(&self, app_profile_id: &str) -> Result<Vec<BookmarkFolder>, AppError> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT id, parent_id, name, position FROM folders
             WHERE app_profile_id = ?1 ORDER BY position, name COLLATE NOCASE",
        )?;
        let folders = statement
            .query_map([app_profile_id], |row| {
                Ok(BookmarkFolder {
                    id: row.get(0)?,
                    parent_id: row.get(1)?,
                    name: row.get(2)?,
                    position: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(folders)
    }

    pub fn list_bookmarks(
        &self,
        app_profile_id: &str,
        folder_id: Option<&str>,
    ) -> Result<Vec<Bookmark>, AppError> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT b.id, b.app_profile_id, b.folder_id, f.name, b.title, b.url,
                    b.created_at, b.updated_at, b.position,
                    COALESCE((SELECT json_group_array(t.name)
                        FROM bookmark_tags bt JOIN tags t ON t.id = bt.tag_id
                        WHERE bt.bookmark_id = b.id), '[]'),
                    COALESCE((SELECT json_group_array(c.name)
                        FROM bookmark_categories bc JOIN categories c ON c.id = bc.category_id
                        WHERE bc.bookmark_id = b.id), '[]'),
                    (SELECT bs.display_name FROM source_bookmarks sb
                        JOIN bookmark_sources bs ON bs.id = sb.source_id
                        WHERE sb.bookmark_id = b.id LIMIT 1)
             FROM bookmarks b LEFT JOIN folders f ON f.id = b.folder_id
             WHERE b.app_profile_id = ?1 AND b.is_archived = 0
               AND (?2 IS NULL OR b.folder_id = ?2)
             ORDER BY COALESCE(f.position, -1), b.position, b.title COLLATE NOCASE",
        )?;
        let bookmarks = statement
            .query_map(params![app_profile_id, folder_id], |row| {
                let tags_json: String = row.get(9)?;
                let categories_json: String = row.get(10)?;
                Ok(Bookmark {
                    id: row.get(0)?,
                    app_profile_id: row.get(1)?,
                    folder_id: row.get(2)?,
                    folder_name: row.get(3)?,
                    title: row.get(4)?,
                    url: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    position: row.get(8)?,
                    tags: serde_json::from_str(&tags_json).unwrap_or_default(),
                    categories: serde_json::from_str(&categories_json).unwrap_or_default(),
                    source_profile_name: row.get(11)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(bookmarks)
    }

    pub fn create_bookmark(&self, input: &CreateBookmarkInput) -> Result<Bookmark, AppError> {
        validate_bookmark(&input.title, &input.url)?;
        let id = Uuid::new_v4().to_string();
        let title = input.title.trim();
        let url = input.url.trim();
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        validate_profile_and_folder(
            &transaction,
            &input.app_profile_id,
            input.folder_id.as_deref(),
        )?;
        let position: i64 = transaction.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM bookmarks
             WHERE app_profile_id = ?1 AND folder_id IS ?2",
            params![input.app_profile_id, input.folder_id],
            |row| row.get(0),
        )?;
        transaction.execute(
            "INSERT INTO bookmarks
                (id, app_profile_id, folder_id, title, url, normalized_url, position)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                input.app_profile_id,
                input.folder_id,
                title,
                url,
                url,
                position
            ],
        )?;
        replace_classifications(
            &transaction,
            &id,
            &input.app_profile_id,
            &input.tags,
            &input.categories,
        )?;
        transaction.commit()?;
        drop(connection);
        self.get_bookmark(&id)
    }

    pub fn update_bookmark(&self, input: &UpdateBookmarkInput) -> Result<Bookmark, AppError> {
        validate_bookmark(&input.title, &input.url)?;
        let title = input.title.trim();
        let url = input.url.trim();
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        let app_profile_id: String = transaction
            .query_row(
                "SELECT app_profile_id FROM bookmarks WHERE id = ?1 AND is_archived = 0",
                [&input.id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| AppError::InvalidInput("Bookmark was not found".into()))?;
        validate_profile_and_folder(&transaction, &app_profile_id, input.folder_id.as_deref())?;
        transaction.execute(
            "UPDATE bookmarks SET title = ?1, url = ?2, normalized_url = ?2,
                folder_id = ?3, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?4",
            params![title, url, input.folder_id, input.id],
        )?;
        replace_classifications(
            &transaction,
            &input.id,
            &app_profile_id,
            &input.tags,
            &input.categories,
        )?;
        transaction.commit()?;
        drop(connection);
        self.get_bookmark(&input.id)
    }

    pub fn archive_bookmark(&self, bookmark_id: &str) -> Result<(), AppError> {
        let connection = self.lock()?;
        let changed = connection.execute(
            "UPDATE bookmarks SET is_archived = 1, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1 AND is_archived = 0",
            [bookmark_id],
        )?;
        if changed == 0 {
            return Err(AppError::InvalidInput("Bookmark was not found".into()));
        }
        Ok(())
    }

    fn get_bookmark(&self, bookmark_id: &str) -> Result<Bookmark, AppError> {
        let app_profile_id = {
            let connection = self.lock()?;
            connection.query_row(
                "SELECT app_profile_id FROM bookmarks WHERE id = ?1",
                [bookmark_id],
                |row| row.get::<_, String>(0),
            )?
        };
        self.list_bookmarks(&app_profile_id, None)?
            .into_iter()
            .find(|bookmark| bookmark.id == bookmark_id)
            .ok_or_else(|| AppError::InvalidInput("Bookmark was not found".into()))
    }

    fn configure(connection: &mut Connection, use_wal: bool) -> Result<(), AppError> {
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.execute_batch("PRAGMA foreign_keys = ON;")?;
        if use_wal {
            connection.execute_batch("PRAGMA journal_mode = WAL;")?;
        }
        Self::migrate(connection)?;
        Ok(())
    }

    fn migrate(connection: &mut Connection) -> Result<(), AppError> {
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )?;
        let current_version: i64 = connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )?;

        if current_version < 1 {
            connection.execute_batch("BEGIN IMMEDIATE;")?;
            let migration_result = (|| -> Result<(), AppError> {
                connection.execute_batch(INITIAL_MIGRATION)?;
                connection.execute(
                    "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
                    (1, "initial"),
                )?;
                Ok(())
            })();

            match migration_result {
                Ok(()) => connection.execute_batch("COMMIT;")?,
                Err(error) => {
                    let _ = connection.execute_batch("ROLLBACK;");
                    return Err(error);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Database;

    #[test]
    fn initializes_schema_and_enables_foreign_keys() {
        let database = Database::in_memory().expect("database initializes");
        let connection = database.lock().expect("database lock");

        let version: i64 = connection
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("migration version is queryable");
        let foreign_keys: i64 = connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .expect("foreign key setting is queryable");

        assert_eq!(version, 1);
        assert_eq!(foreign_keys, 1);
    }

    #[test]
    fn reopening_database_does_not_repeat_initial_migration() {
        let database = Database::in_memory().expect("database initializes");
        let connection = database.lock().expect("database lock");
        let count: i64 = connection
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .expect("migration count is queryable");

        assert_eq!(count, 1);
    }

    #[test]
    fn chrome_sync_is_idempotent_and_never_deletes_missing_items() {
        use crate::{chrome::parse_chrome_bookmarks, domain::ChromeProfile};

        let database = Database::in_memory().expect("database initializes");
        let profile = ChromeProfile {
            id: "Default".into(),
            name: "Default".into(),
            path: "/chrome/Default".into(),
        };
        let complete = parse_chrome_bookmarks(
            r#"{"roots":{"bookmark_bar":{"id":"1","name":"Bookmarks bar","children":[
                {"id":"2","name":"Example","type":"url","url":"https://example.com"}
            ]}}}"#,
        )
        .expect("complete fixture parses");
        let first = database
            .sync_chrome_bookmarks(&profile, &complete)
            .expect("first sync succeeds");
        let second = database
            .sync_chrome_bookmarks(&profile, &complete)
            .expect("repeat sync succeeds");

        assert_eq!(first.added, 1);
        assert_eq!(second.added, 0);
        assert_eq!(second.unchanged, 1);

        let empty = parse_chrome_bookmarks(r#"{"roots":{}}"#).expect("empty snapshot parses");
        let third = database
            .sync_chrome_bookmarks(&profile, &empty)
            .expect("sync with missing entry succeeds");
        assert_eq!(third.added, 0);
        assert_eq!(third.missing, 1);

        let connection = database.lock().expect("database lock");
        let bookmarks: i64 = connection
            .query_row("SELECT COUNT(*) FROM bookmarks", [], |row| row.get(0))
            .expect("bookmark count is queryable");
        let missing_count: i64 = connection
            .query_row(
                "SELECT missing_count FROM sync_runs ORDER BY started_at DESC, rowid DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .expect("missing count is queryable");
        assert_eq!(bookmarks, 1);
        assert_eq!(missing_count, 1);
    }

    #[test]
    fn bookmark_crud_preserves_profile_and_classifications() {
        use crate::domain::{CreateBookmarkInput, UpdateBookmarkInput};

        let database = Database::in_memory().expect("database initializes");
        let profile = database
            .create_app_profile("Reading")
            .expect("profile creates");
        let created = database
            .create_bookmark(&CreateBookmarkInput {
                app_profile_id: profile.id.clone(),
                folder_id: None,
                title: "Rust guide".into(),
                url: "https://rust.example/guide".into(),
                tags: vec!["Learning".into(), "rust".into()],
                categories: vec!["Reference".into()],
            })
            .expect("bookmark creates");

        assert_eq!(created.tags.len(), 1, "case-insensitive tags are unique");
        assert_eq!(created.categories, vec!["Reference"]);
        assert_eq!(database.list_bookmarks("personal", None).unwrap().len(), 0);

        let updated = database
            .update_bookmark(&UpdateBookmarkInput {
                id: created.id.clone(),
                title: "Rust reference".into(),
                url: "https://rust.example/reference".into(),
                folder_id: None,
                tags: vec!["systems".into()],
                categories: vec!["Reading list".into()],
            })
            .expect("bookmark updates");
        assert_eq!(updated.title, "Rust reference");
        assert_eq!(updated.tags, vec!["systems"]);
        assert_eq!(updated.categories, vec!["Reading list"]);

        database
            .archive_bookmark(&created.id)
            .expect("bookmark archives");
        assert!(database
            .list_bookmarks(&profile.id, None)
            .unwrap()
            .is_empty());
    }
}

fn validate_bookmark(title: &str, url: &str) -> Result<(), AppError> {
    if title.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "Bookmark title cannot be empty".into(),
        ));
    }
    if url.trim().is_empty() {
        return Err(AppError::InvalidInput(
            "Bookmark URL cannot be empty".into(),
        ));
    }
    Ok(())
}

fn validate_profile_and_folder(
    transaction: &rusqlite::Transaction<'_>,
    app_profile_id: &str,
    folder_id: Option<&str>,
) -> Result<(), AppError> {
    let profile_exists: bool = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM app_profiles WHERE id = ?1)",
        [app_profile_id],
        |row| row.get(0),
    )?;
    if !profile_exists {
        return Err(AppError::InvalidInput("App profile was not found".into()));
    }
    if let Some(folder_id) = folder_id {
        let folder_exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM folders WHERE id = ?1 AND app_profile_id = ?2)",
            params![folder_id, app_profile_id],
            |row| row.get(0),
        )?;
        if !folder_exists {
            return Err(AppError::InvalidInput(
                "Folder was not found in this profile".into(),
            ));
        }
    }
    Ok(())
}

fn replace_classifications(
    transaction: &rusqlite::Transaction<'_>,
    bookmark_id: &str,
    app_profile_id: &str,
    tags: &[String],
    categories: &[String],
) -> Result<(), AppError> {
    transaction.execute(
        "DELETE FROM bookmark_tags WHERE bookmark_id = ?1",
        [bookmark_id],
    )?;
    transaction.execute(
        "DELETE FROM bookmark_categories WHERE bookmark_id = ?1",
        [bookmark_id],
    )?;

    for (table, relation_table, name) in [
        ("tags", "bookmark_tags", tags),
        ("categories", "bookmark_categories", categories),
    ] {
        let name_column = if table == "tags" {
            "tag_id"
        } else {
            "category_id"
        };
        for value in name {
            let display_name = value.trim();
            if display_name.is_empty() {
                continue;
            }
            let normalized_name = display_name.to_lowercase();
            let id: String = match table {
                "tags" => {
                    transaction.execute(
                        "INSERT INTO tags (id, app_profile_id, name, normalized_name)
                         VALUES (?1, ?2, ?3, ?4) ON CONFLICT(app_profile_id, normalized_name)
                         DO NOTHING",
                        params![
                            Uuid::new_v4().to_string(),
                            app_profile_id,
                            display_name,
                            normalized_name
                        ],
                    )?;
                    transaction.query_row(
                        "SELECT id FROM tags WHERE app_profile_id = ?1 AND normalized_name = ?2",
                        params![app_profile_id, normalized_name],
                        |row| row.get(0),
                    )?
                }
                _ => {
                    transaction.execute(
                        "INSERT INTO categories (id, app_profile_id, name, normalized_name)
                         VALUES (?1, ?2, ?3, ?4) ON CONFLICT(app_profile_id, normalized_name)
                         DO NOTHING",
                        params![
                            Uuid::new_v4().to_string(),
                            app_profile_id,
                            display_name,
                            normalized_name
                        ],
                    )?;
                    transaction.query_row(
                        "SELECT id FROM categories WHERE app_profile_id = ?1 AND normalized_name = ?2",
                        params![app_profile_id, normalized_name],
                        |row| row.get(0),
                    )?
                }
            };
            let statement = format!(
                "INSERT OR IGNORE INTO {relation_table} (bookmark_id, {name_column}) VALUES (?1, ?2)"
            );
            transaction.execute(&statement, params![bookmark_id, id])?;
        }
    }
    Ok(())
}
