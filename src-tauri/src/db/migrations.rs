use rusqlite::{Connection, Result};

pub fn run(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL);
         INSERT OR IGNORE INTO schema_version (version) VALUES (0);",
    )?;

    let version: i64 =
        conn.query_row("SELECT version FROM schema_version", [], |row| row.get(0))?;

    if version < 1 {
        migrate_v1(conn)?;
        conn.execute("UPDATE schema_version SET version = 1", [])?;
    }

    if version < 2 {
        migrate_v2(conn)?;
        conn.execute("UPDATE schema_version SET version = 2", [])?;
    }

    if version < 3 {
        migrate_v3(conn)?;
        conn.execute("UPDATE schema_version SET version = 3", [])?;
    }

    if version < 4 {
        migrate_v4(conn)?;
        conn.execute("UPDATE schema_version SET version = 4", [])?;
    }

    Ok(())
}

/// content='' FTS5 tables silently discard UNINDEXED column values,
/// making WHERE file_id = ? and rowid-based lookups both broken.
/// Recreate the table as a regular FTS5 table that stores column data.
fn migrate_v2(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "DROP TABLE IF EXISTS file_search_index;
         CREATE VIRTUAL TABLE file_search_index USING fts5(
             file_id    UNINDEXED,
             filename,
             relative_path,
             tag_names
         );",
    )?;
    Ok(())
}

/// Add kind column to distinguish files from directories in file_records.
fn migrate_v4(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "ALTER TABLE file_records ADD COLUMN kind TEXT NOT NULL DEFAULT 'file';",
    )?;
    Ok(())
}

/// file_records.mtime and last_synced_at were declared TEXT but used as INTEGER.
/// SQLite stored them as text strings, breaking i64 reads. Recreate with INTEGER types.
/// sync_logs timestamps (started_at, finished_at) have the same issue — fix them too.
/// file_records is derived data (rebuilt by sync), so dropping is safe.
fn migrate_v3(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "DROP TABLE IF EXISTS file_search_index;
         DROP TABLE IF EXISTS file_tags;
         DROP TABLE IF EXISTS file_records;
         DROP TABLE IF EXISTS sync_logs;

         CREATE TABLE file_records (
             id             INTEGER PRIMARY KEY,
             task_id        INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
             relative_path  TEXT    NOT NULL,
             size           INTEGER NOT NULL,
             mtime          INTEGER NOT NULL,
             content_hash   TEXT    NOT NULL,
             status         TEXT    NOT NULL DEFAULT 'active',
             last_synced_at INTEGER NOT NULL,
             UNIQUE(task_id, relative_path)
         );

         CREATE TABLE sync_logs (
             id                INTEGER PRIMARY KEY,
             task_id           INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
             started_at        INTEGER NOT NULL,
             finished_at       INTEGER,
             files_scanned     INTEGER NOT NULL DEFAULT 0,
             files_copied      INTEGER NOT NULL DEFAULT 0,
             files_deleted     INTEGER NOT NULL DEFAULT 0,
             bytes_transferred INTEGER NOT NULL DEFAULT 0,
             errors            TEXT    NOT NULL DEFAULT '[]',
             status            TEXT    NOT NULL DEFAULT 'running'
         );

         CREATE TABLE file_tags (
             file_id    INTEGER NOT NULL REFERENCES file_records(id) ON DELETE CASCADE,
             tag_id     INTEGER NOT NULL REFERENCES tags(id)         ON DELETE CASCADE,
             tagged_at  INTEGER NOT NULL,
             note       TEXT,
             PRIMARY KEY (file_id, tag_id)
         );

         CREATE VIRTUAL TABLE file_search_index USING fts5(
             file_id    UNINDEXED,
             filename,
             relative_path,
             tag_names
         );

         CREATE INDEX idx_file_records_task   ON file_records(task_id);
         CREATE INDEX idx_file_records_status ON file_records(status);
         CREATE INDEX idx_file_tags_file      ON file_tags(file_id);
         CREATE INDEX idx_file_tags_tag       ON file_tags(tag_id);
         CREATE INDEX idx_sync_logs_task      ON sync_logs(task_id);",
    )?;
    Ok(())
}

fn migrate_v1(conn: &Connection) -> Result<()> {
    conn.execute_batch("
        CREATE TABLE tasks (
            id            INTEGER PRIMARY KEY,
            name          TEXT    NOT NULL,
            source_path   TEXT    NOT NULL,
            target_path   TEXT    NOT NULL,
            trigger_type  TEXT    NOT NULL DEFAULT 'manual',
            cron_expr     TEXT,
            filter_rules  TEXT    NOT NULL DEFAULT '{}',
            enabled       INTEGER NOT NULL DEFAULT 1,
            created_at    TEXT    NOT NULL
        );

        CREATE TABLE file_records (
            id             INTEGER PRIMARY KEY,
            task_id        INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            relative_path  TEXT    NOT NULL,
            size           INTEGER NOT NULL,
            mtime          TEXT    NOT NULL,
            content_hash   TEXT    NOT NULL,
            status         TEXT    NOT NULL DEFAULT 'active',
            last_synced_at TEXT    NOT NULL,
            UNIQUE(task_id, relative_path)
        );

        CREATE TABLE tags (
            id          INTEGER PRIMARY KEY,
            name        TEXT    NOT NULL UNIQUE,
            color       TEXT    NOT NULL DEFAULT '#6366f1',
            parent_id   INTEGER REFERENCES tags(id) ON DELETE SET NULL,
            description TEXT,
            created_at  TEXT    NOT NULL
        );

        CREATE TABLE file_tags (
            file_id    INTEGER NOT NULL REFERENCES file_records(id) ON DELETE CASCADE,
            tag_id     INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            tagged_at  TEXT    NOT NULL,
            note       TEXT,
            PRIMARY KEY (file_id, tag_id)
        );

        CREATE TABLE sync_logs (
            id                INTEGER PRIMARY KEY,
            task_id           INTEGER NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            started_at        TEXT    NOT NULL,
            finished_at       TEXT,
            files_scanned     INTEGER NOT NULL DEFAULT 0,
            files_copied      INTEGER NOT NULL DEFAULT 0,
            files_deleted     INTEGER NOT NULL DEFAULT 0,
            bytes_transferred INTEGER NOT NULL DEFAULT 0,
            errors            TEXT    NOT NULL DEFAULT '[]',
            status            TEXT    NOT NULL DEFAULT 'running'
        );

        CREATE VIRTUAL TABLE file_search_index USING fts5(
            file_id    UNINDEXED,
            filename,
            relative_path,
            tag_names,
            content=''
        );

        CREATE INDEX idx_file_records_task   ON file_records(task_id);
        CREATE INDEX idx_file_records_status ON file_records(status);
        CREATE INDEX idx_file_tags_file      ON file_tags(file_id);
        CREATE INDEX idx_file_tags_tag       ON file_tags(tag_id);
        CREATE INDEX idx_tags_parent         ON tags(parent_id);
        CREATE INDEX idx_sync_logs_task      ON sync_logs(task_id);
    ")?;
    Ok(())
}
