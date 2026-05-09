use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

use crate::db::DbState;
use crate::error::{AppError, AppResult};
use crate::sync::{
    diff::{compute_diff, DbEntry, DbMap},
    executor::{copy_file, delete_file, hash_file},
    scanner::scan_directory,
    SyncAction,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub task_id: i64,
    pub files_scanned: usize,
    pub files_copied: u64,
    pub files_deleted: u64,
    pub bytes_transferred: u64,
    pub errors: Vec<String>,
}

/// Core sync logic — shared by the Tauri command, file watcher, and cron scheduler.
pub fn run(task_id: i64, db: &DbState) -> AppResult<SyncResult> {
    // ── Phase A: snapshot task config + DB index (brief lock) ──────────────
    let (source, target, excludes, db_map) = {
        let conn = db.0.lock().map_err(|_| AppError::Other("DB lock poisoned".into()))?;

        let (src, tgt, filter_json): (String, String, String) = conn
            .query_row(
                "SELECT source_path, target_path, filter_rules
                   FROM tasks WHERE id = ?1 AND enabled = 1",
                params![task_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|_| AppError::Other(format!("Task {} not found or disabled", task_id)))?;

        let filters: serde_json::Value =
            serde_json::from_str(&filter_json).unwrap_or_default();
        let excludes: Vec<String> = filters["exclude_globs"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let mut db_map = DbMap::new();
        let mut stmt = conn.prepare(
            "SELECT id, relative_path, size, mtime, content_hash
               FROM file_records WHERE task_id = ?1 AND status = 'active' AND kind = 'file'",
        )?;
        let rows = stmt.query_map(params![task_id], |row| {
            Ok((
                row.get::<_, String>(1)?,
                DbEntry {
                    id: row.get(0)?,
                    size: row.get::<_, i64>(2)? as u64,
                    mtime_secs: row.get(3)?,
                    content_hash: row.get(4)?,
                },
            ))
        })?;
        for row in rows {
            let (path, entry) = row?;
            db_map.insert(path, entry);
        }

        (PathBuf::from(src), PathBuf::from(tgt), excludes, db_map)
    };

    std::fs::create_dir_all(&target)?;

    let now_secs: i64 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    // ── Phase B: mirror directories + record them in DB ─────────────────────
    // File scanner skips directories; we create them in the target and track
    // them in file_records (kind='dir') so they appear in the UI.
    // source_dir_set is kept for stale-dir cleanup after file actions.
    let source_dir_set: std::collections::HashSet<String> = {
        let mut dirs: Vec<(String, i64)> = Vec::new();
        for entry in walkdir::WalkDir::new(&source).follow_links(false).into_iter().flatten() {
            if entry.file_type().is_dir() {
                if let Ok(rel) = entry.path().strip_prefix(&source) {
                    if rel == std::path::Path::new("") {
                        continue;
                    }
                    let rel_str = rel.to_string_lossy().replace('\\', "/");
                    let mtime = entry
                        .metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    let _ = std::fs::create_dir_all(target.join(rel));
                    dirs.push((rel_str, mtime));
                }
            }
        }
        {
            let conn = db.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
            conn.execute(
                "DELETE FROM file_records WHERE task_id = ?1 AND kind = 'dir'",
                params![task_id],
            )?;
            for (rel, mtime) in &dirs {
                conn.execute(
                    "INSERT OR REPLACE INTO file_records
                       (task_id, relative_path, size, mtime, content_hash, status, last_synced_at, kind)
                     VALUES (?1, ?2, 0, ?3, '', 'active', ?4, 'dir')",
                    params![task_id, rel, mtime, now_secs],
                )?;
            }
        }
        dirs.into_iter().map(|(p, _)| p).collect()
    };

    let scan = scan_directory(&source, &excludes)?;
    let files_scanned = scan.len();
    let actions = compute_diff(&scan, &db_map);

    // ── Phase D: execute actions, brief DB lock per file ────────────────────
    let mut copied = 0u64;
    let mut deleted = 0u64;
    let mut transferred = 0u64;
    let mut errors: Vec<String> = Vec::new();

    for action in actions {
        match action {
            SyncAction::Add { relative_path, size, mtime_secs } => {
                match copy_file(&source, &target, &relative_path) {
                    Ok(bytes) => {
                        let hash = hash_file(&source, &relative_path).unwrap_or_default();
                        let conn =
                            db.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
                        let filename =
                            relative_path.rsplit('/').next().unwrap_or(&relative_path).to_string();
                        conn.execute(
                            "INSERT INTO file_records
                               (task_id, relative_path, size, mtime, content_hash, status, last_synced_at)
                             VALUES (?1,?2,?3,?4,?5,'active',?6)",
                            params![task_id, relative_path, size as i64, mtime_secs, hash, now_secs],
                        )?;
                        let fid = conn.last_insert_rowid();
                        conn.execute(
                            "INSERT INTO file_search_index
                               (rowid, file_id, filename, relative_path, tag_names)
                             VALUES (?1,?1,?2,?3,'')",
                            params![fid, filename, relative_path],
                        )?;
                        copied += 1;
                        transferred += bytes;
                    }
                    Err(e) => errors.push(e),
                }
            }

            SyncAction::Update { file_id, relative_path, size, mtime_secs } => {
                let new_hash = hash_file(&source, &relative_path).unwrap_or_default();
                let stored: String = {
                    let conn =
                        db.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
                    conn.query_row(
                        "SELECT content_hash FROM file_records WHERE id = ?1",
                        params![file_id],
                        |r| r.get(0),
                    )
                    .unwrap_or_default()
                };

                if new_hash == stored {
                    // metadata-only change, no copy needed
                    let conn =
                        db.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
                    conn.execute(
                        "UPDATE file_records
                            SET size = ?1, mtime = ?2, last_synced_at = ?3
                          WHERE id = ?4",
                        params![size as i64, mtime_secs, now_secs, file_id],
                    )?;
                } else {
                    match copy_file(&source, &target, &relative_path) {
                        Ok(bytes) => {
                            let conn =
                                db.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
                            conn.execute(
                                "UPDATE file_records
                                    SET size = ?1, mtime = ?2, content_hash = ?3, last_synced_at = ?4
                                  WHERE id = ?5",
                                params![size as i64, mtime_secs, new_hash, now_secs, file_id],
                            )?;
                            copied += 1;
                            transferred += bytes;
                        }
                        Err(e) => errors.push(e),
                    }
                }
            }

            SyncAction::Delete { file_id, relative_path } => {
                if let Err(e) = delete_file(&target, &relative_path) {
                    errors.push(e);
                }
                let conn = db.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
                conn.execute(
                    "DELETE FROM file_search_index WHERE rowid = ?1",
                    params![file_id],
                )?;
                conn.execute(
                    "DELETE FROM file_records WHERE id = ?1",
                    params![file_id],
                )?;
                deleted += 1;
            }
        }
    }

    // ── Phase E: remove stale target directories ────────────────────────────
    // Directories that exist in the target but no longer in the source are
    // removed. Sort deepest-first so children are deleted before parents.
    {
        let mut stale: Vec<std::path::PathBuf> = Vec::new();
        for entry in walkdir::WalkDir::new(&target).follow_links(false).into_iter().flatten() {
            if entry.file_type().is_dir() {
                if let Ok(rel) = entry.path().strip_prefix(&target) {
                    if rel == std::path::Path::new("") {
                        continue;
                    }
                    let rel_str = rel.to_string_lossy().replace('\\', "/");
                    if !source_dir_set.contains(&rel_str) {
                        stale.push(entry.path().to_path_buf());
                    }
                }
            }
        }
        // Deepest paths first so we don't try to remove a parent before its children
        stale.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
        for dir in stale {
            if dir.exists() {
                let _ = std::fs::remove_dir_all(&dir);
            }
        }
    }

    // ── Phase G: write sync log ──────────────────────────────────────────────
    {
        let conn = db.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
        let status = if errors.is_empty() { "success" } else { "partial_error" };
        let errs_json = serde_json::to_string(&errors).unwrap_or_default();
        conn.execute(
            "INSERT INTO sync_logs
               (task_id, started_at, finished_at,
                files_scanned, files_copied, files_deleted,
                bytes_transferred, errors, status)
             VALUES (?1,?2,?2,?3,?4,?5,?6,?7,?8)",
            params![
                task_id, now_secs,
                files_scanned as i64, copied as i64, deleted as i64,
                transferred as i64, errs_json, status,
            ],
        )?;
    }

    Ok(SyncResult {
        task_id,
        files_scanned,
        files_copied: copied,
        files_deleted: deleted,
        bytes_transferred: transferred,
        errors,
    })
}
