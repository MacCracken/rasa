use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use rasa_core::error::RasaError;

/// A recently-opened file entry in the catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub path: PathBuf,
    pub name: String,
    pub last_opened: DateTime<Utc>,
    pub file_size: u64,
    pub width: u32,
    pub height: u32,
}

/// SQLite-backed recent files / project catalog.
pub struct Catalog {
    conn: Connection,
}

impl Catalog {
    /// Open or create a catalog database at the given path.
    pub fn open(db_path: &Path) -> Result<Self, RasaError> {
        let conn = Connection::open(db_path)
            .map_err(|e| RasaError::Other(format!("catalog open failed: {e}")))?;
        let catalog = Self { conn };
        catalog.init_schema()?;
        Ok(catalog)
    }

    /// Create an in-memory catalog (for testing).
    pub fn in_memory() -> Result<Self, RasaError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| RasaError::Other(format!("catalog open failed: {e}")))?;
        let catalog = Self { conn };
        catalog.init_schema()?;
        Ok(catalog)
    }

    fn init_schema(&self) -> Result<(), RasaError> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS recent_files (
                    path TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    last_opened TEXT NOT NULL,
                    file_size INTEGER NOT NULL DEFAULT 0,
                    width INTEGER NOT NULL DEFAULT 0,
                    height INTEGER NOT NULL DEFAULT 0
                );
                CREATE INDEX IF NOT EXISTS idx_recent_last_opened
                    ON recent_files(last_opened DESC);",
            )
            .map_err(|e| RasaError::Other(format!("catalog schema init failed: {e}")))?;
        Ok(())
    }

    /// Record that a file was opened (insert or update).
    pub fn record_open(&self, entry: &CatalogEntry) -> Result<(), RasaError> {
        self.conn
            .execute(
                "INSERT INTO recent_files (path, name, last_opened, file_size, width, height)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(path) DO UPDATE SET
                    name = excluded.name,
                    last_opened = excluded.last_opened,
                    file_size = excluded.file_size,
                    width = excluded.width,
                    height = excluded.height",
                params![
                    entry.path.to_string_lossy().as_ref(),
                    entry.name,
                    entry.last_opened.to_rfc3339(),
                    entry.file_size as i64,
                    entry.width,
                    entry.height,
                ],
            )
            .map_err(|e| RasaError::Other(format!("catalog insert failed: {e}")))?;
        Ok(())
    }

    /// Get the most recently opened files, up to `limit`.
    pub fn recent(&self, limit: usize) -> Result<Vec<CatalogEntry>, RasaError> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT path, name, last_opened, file_size, width, height
                 FROM recent_files
                 ORDER BY last_opened DESC
                 LIMIT ?1",
            )
            .map_err(|e| RasaError::Other(format!("catalog query failed: {e}")))?;

        let entries = stmt
            .query_map(params![limit as i64], |row| {
                let path_str: String = row.get(0)?;
                let name: String = row.get(1)?;
                let last_opened_str: String = row.get(2)?;
                let file_size: i64 = row.get(3)?;
                let width: u32 = row.get(4)?;
                let height: u32 = row.get(5)?;

                let last_opened = DateTime::parse_from_rfc3339(&last_opened_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());

                Ok(CatalogEntry {
                    path: PathBuf::from(path_str),
                    name,
                    last_opened,
                    file_size: file_size as u64,
                    width,
                    height,
                })
            })
            .map_err(|e| RasaError::Other(format!("catalog query failed: {e}")))?;

        let mut result = Vec::new();
        for entry in entries {
            result.push(
                entry.map_err(|e| RasaError::Other(format!("catalog row read failed: {e}")))?,
            );
        }
        Ok(result)
    }

    /// Remove a file from the catalog.
    pub fn remove(&self, path: &Path) -> Result<bool, RasaError> {
        let count = self
            .conn
            .execute(
                "DELETE FROM recent_files WHERE path = ?1",
                params![path.to_string_lossy().as_ref()],
            )
            .map_err(|e| RasaError::Other(format!("catalog delete failed: {e}")))?;
        Ok(count > 0)
    }

    /// Clear all entries from the catalog.
    pub fn clear(&self) -> Result<(), RasaError> {
        self.conn
            .execute("DELETE FROM recent_files", [])
            .map_err(|e| RasaError::Other(format!("catalog clear failed: {e}")))?;
        Ok(())
    }

    /// Get the total number of entries in the catalog.
    pub fn count(&self) -> Result<usize, RasaError> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM recent_files", [], |row| row.get(0))
            .map_err(|e| RasaError::Other(format!("catalog count failed: {e}")))?;
        Ok(count as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(name: &str, path: &str) -> CatalogEntry {
        CatalogEntry {
            path: PathBuf::from(path),
            name: name.into(),
            last_opened: Utc::now(),
            file_size: 1024,
            width: 1920,
            height: 1080,
        }
    }

    #[test]
    fn create_in_memory() {
        let catalog = Catalog::in_memory().unwrap();
        assert_eq!(catalog.count().unwrap(), 0);
    }

    #[test]
    fn record_and_retrieve() {
        let catalog = Catalog::in_memory().unwrap();
        let entry = make_entry("photo.png", "/home/user/photo.png");
        catalog.record_open(&entry).unwrap();

        let recent = catalog.recent(10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].name, "photo.png");
        assert_eq!(recent[0].path, PathBuf::from("/home/user/photo.png"));
        assert_eq!(recent[0].width, 1920);
        assert_eq!(recent[0].height, 1080);
    }

    #[test]
    fn upsert_updates_existing() {
        let catalog = Catalog::in_memory().unwrap();
        let entry1 = CatalogEntry {
            path: PathBuf::from("/test.png"),
            name: "old name".into(),
            last_opened: Utc::now(),
            file_size: 100,
            width: 100,
            height: 100,
        };
        catalog.record_open(&entry1).unwrap();

        let entry2 = CatalogEntry {
            path: PathBuf::from("/test.png"),
            name: "new name".into(),
            last_opened: Utc::now(),
            file_size: 200,
            width: 200,
            height: 200,
        };
        catalog.record_open(&entry2).unwrap();

        assert_eq!(catalog.count().unwrap(), 1);
        let recent = catalog.recent(10).unwrap();
        assert_eq!(recent[0].name, "new name");
        assert_eq!(recent[0].width, 200);
    }

    #[test]
    fn recent_returns_newest_first() {
        let catalog = Catalog::in_memory().unwrap();

        let old = CatalogEntry {
            path: PathBuf::from("/old.png"),
            name: "old".into(),
            last_opened: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            file_size: 0,
            width: 0,
            height: 0,
        };
        let new = CatalogEntry {
            path: PathBuf::from("/new.png"),
            name: "new".into(),
            last_opened: DateTime::parse_from_rfc3339("2026-03-13T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            file_size: 0,
            width: 0,
            height: 0,
        };

        catalog.record_open(&old).unwrap();
        catalog.record_open(&new).unwrap();

        let recent = catalog.recent(10).unwrap();
        assert_eq!(recent[0].name, "new");
        assert_eq!(recent[1].name, "old");
    }

    #[test]
    fn recent_respects_limit() {
        let catalog = Catalog::in_memory().unwrap();
        for i in 0..10 {
            catalog
                .record_open(&make_entry(&format!("file{i}"), &format!("/file{i}.png")))
                .unwrap();
        }
        let recent = catalog.recent(3).unwrap();
        assert_eq!(recent.len(), 3);
    }

    #[test]
    fn remove_entry() {
        let catalog = Catalog::in_memory().unwrap();
        catalog
            .record_open(&make_entry("test", "/test.png"))
            .unwrap();
        assert_eq!(catalog.count().unwrap(), 1);

        let removed = catalog.remove(Path::new("/test.png")).unwrap();
        assert!(removed);
        assert_eq!(catalog.count().unwrap(), 0);
    }

    #[test]
    fn remove_nonexistent_returns_false() {
        let catalog = Catalog::in_memory().unwrap();
        let removed = catalog.remove(Path::new("/nope.png")).unwrap();
        assert!(!removed);
    }

    #[test]
    fn clear_removes_all() {
        let catalog = Catalog::in_memory().unwrap();
        for i in 0..5 {
            catalog
                .record_open(&make_entry(&format!("f{i}"), &format!("/f{i}.png")))
                .unwrap();
        }
        assert_eq!(catalog.count().unwrap(), 5);
        catalog.clear().unwrap();
        assert_eq!(catalog.count().unwrap(), 0);
    }

    #[test]
    fn file_backed_catalog() {
        let dir = std::env::temp_dir().join("rasa_test_catalog");
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("test_catalog.db");

        // Write
        {
            let catalog = Catalog::open(&db_path).unwrap();
            catalog
                .record_open(&make_entry("photo", "/photo.rasa"))
                .unwrap();
        }

        // Re-open and verify persistence
        {
            let catalog = Catalog::open(&db_path).unwrap();
            let recent = catalog.recent(10).unwrap();
            assert_eq!(recent.len(), 1);
            assert_eq!(recent[0].name, "photo");
        }

        std::fs::remove_file(&db_path).ok();
    }

    #[test]
    fn preserves_file_metadata() {
        let catalog = Catalog::in_memory().unwrap();
        let entry = CatalogEntry {
            path: PathBuf::from("/project.rasa"),
            name: "My Project".into(),
            last_opened: Utc::now(),
            file_size: 4_500_000,
            width: 3840,
            height: 2160,
        };
        catalog.record_open(&entry).unwrap();

        let recent = catalog.recent(1).unwrap();
        assert_eq!(recent[0].file_size, 4_500_000);
        assert_eq!(recent[0].width, 3840);
        assert_eq!(recent[0].height, 2160);
    }
}
