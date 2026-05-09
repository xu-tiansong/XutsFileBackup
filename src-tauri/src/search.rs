use rusqlite::{params, params_from_iter, types::Value};
use serde::{Deserialize, Serialize};

use crate::db::DbState;
use crate::error::{AppError, AppResult};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TagSummary {
    pub id: i64,
    pub name: String,
    pub color: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileRecord {
    pub id: i64,
    pub task_id: i64,
    pub relative_path: String,
    pub size: i64,
    pub mtime: i64,
    pub content_hash: String,
    pub status: String,
    pub last_synced_at: i64,
    pub kind: String,
    pub tags: Vec<TagSummary>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    pub text: Option<String>,
    pub tag_ids: Option<Vec<i64>>,
    /// "and" requires all specified tags present; "or" (default) requires any
    pub tag_logic: Option<String>,
    pub task_id: Option<i64>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncLog {
    pub id: i64,
    pub task_id: i64,
    pub started_at: i64,
    pub finished_at: Option<i64>,
    pub files_scanned: i64,
    pub files_copied: i64,
    pub files_deleted: i64,
    pub bytes_transferred: i64,
    pub errors: String,
    pub status: String,
}

/// Convert user text to a safe FTS5 prefix-match query.
/// Each word becomes word* (prefix search, implicit AND between words).
fn to_fts5(text: &str) -> String {
    let words: Vec<String> = text
        .split_whitespace()
        .filter_map(|w| {
            let clean: String = w
                .chars()
                .filter(|c| c.is_alphanumeric() || matches!(c, '_' | '-'))
                .collect();
            if clean.is_empty() { None } else { Some(format!("{}*", clean)) }
        })
        .collect();
    words.join(" ")
}

/// Inline subquery that aggregates tag id/name/color for a given file_id column reference.
/// Use `file_records.id` for unaliased queries, `fr.id` for CTE queries.
fn tags_subq(id_ref: &str) -> String {
    format!(
        "COALESCE((SELECT json_group_array(\
            json_object('id',t.id,'name',t.name,'color',t.color)) \
            FROM file_tags ft JOIN tags t ON ft.tag_id=t.id \
            WHERE ft.file_id={id_ref}),'[]')"
    )
}

fn map_file_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<FileRecord> {
    let tags_json: String = row.get(9).unwrap_or_else(|_| "[]".to_string());
    let tags: Vec<TagSummary> = serde_json::from_str(&tags_json).unwrap_or_default();
    Ok(FileRecord {
        id: row.get(0)?,
        task_id: row.get(1)?,
        relative_path: row.get(2)?,
        size: row.get(3)?,
        mtime: row.get(4)?,
        content_hash: row.get(5)?,
        status: row.get(6)?,
        last_synced_at: row.get(7)?,
        kind: row.get(8)?,
        tags,
    })
}

/// Full search: optional text (FTS5) + optional tag filter (hierarchical CTE) + task filter.
pub fn search_files(state: &DbState, q: SearchQuery) -> AppResult<Vec<FileRecord>> {
    let conn = state.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;

    let limit = q.limit.unwrap_or(100).min(500);
    let offset = q.offset.unwrap_or(0);
    let tag_ids = q.tag_ids.as_deref().unwrap_or(&[]);
    let fts_text = q.text.as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(to_fts5)
        .filter(|t| !t.is_empty());
    let use_tags = !tag_ids.is_empty();
    let tag_and = q.tag_logic.as_deref() == Some("and");

    let mut sql = String::new();
    let mut params: Vec<Value> = Vec::new();

    if use_tags {
        // Expand each root tag to include all its descendants, then filter files.
        // AND mode: file must have at least one tag from EACH root's subtree.
        // OR mode: file must have at least one tag from ANY root's subtree.
        let ph: String = tag_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
        for &tid in tag_ids {
            params.push(Value::Integer(tid));
        }
        let n = tag_ids.len();
        let having = if tag_and {
            format!(" HAVING COUNT(DISTINCT tt.root_id) = {}", n)
        } else {
            String::new()
        };
        let tsq = tags_subq("fr.id");
        sql.push_str(&format!(
            "WITH RECURSIVE tag_tree(id, root_id) AS (\
               SELECT id, id FROM tags WHERE id IN ({ph}) \
               UNION ALL \
               SELECT t.id, tt.root_id FROM tags t \
               JOIN tag_tree tt ON t.parent_id = tt.id \
             ), \
             tag_match(file_id) AS (\
               SELECT ft.file_id \
               FROM file_tags ft JOIN tag_tree tt ON ft.tag_id = tt.id \
               GROUP BY ft.file_id{having} \
             ) \
             SELECT fr.id, fr.task_id, fr.relative_path, fr.size, fr.mtime, \
                    fr.content_hash, fr.status, fr.last_synced_at, fr.kind, {tsq} \
             FROM file_records fr \
             JOIN tag_match tm ON fr.id = tm.file_id \
             WHERE fr.status = 'active'",
        ));
    } else {
        let tsq = tags_subq("file_records.id");
        sql.push_str(&format!(
            "SELECT id, task_id, relative_path, size, mtime, \
                    content_hash, status, last_synced_at, kind, {tsq} \
             FROM file_records \
             WHERE status = 'active'"
        ));
    }

    // FTS5 text filter
    if let Some(fts_q) = fts_text {
        params.push(Value::Text(fts_q));
        let prefix = if use_tags { "fr." } else { "" };
        sql.push_str(&format!(
            " AND {prefix}id IN \
              (SELECT rowid FROM file_search_index WHERE file_search_index MATCH ?)"
        ));
    }

    // Task filter
    if let Some(tid) = q.task_id {
        params.push(Value::Integer(tid));
        let prefix = if use_tags { "fr." } else { "" };
        sql.push_str(&format!(" AND {prefix}task_id = ?"));
    }

    // ORDER + pagination
    let col_prefix = if use_tags { "fr." } else { "" };
    sql.push_str(&format!(" ORDER BY {col_prefix}relative_path LIMIT ? OFFSET ?"));
    params.push(Value::Integer(limit));
    params.push(Value::Integer(offset));

    let mut stmt = conn.prepare(&sql)?;
    let records = stmt
        .query_map(params_from_iter(params.iter()), map_file_record)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(records)
}

/// List all active files for a task (no text/tag filter).
pub fn get_task_files(
    state: &DbState,
    task_id: i64,
    limit: Option<i64>,
    offset: Option<i64>,
) -> AppResult<Vec<FileRecord>> {
    let conn = state.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
    let limit = limit.unwrap_or(200).min(1000);
    let offset = offset.unwrap_or(0);
    let tsq = tags_subq("file_records.id");
    let mut stmt = conn.prepare(&format!(
        "SELECT id, task_id, relative_path, size, mtime, content_hash, status, last_synced_at, kind, {tsq} \
         FROM file_records \
         WHERE task_id = ?1 AND status = 'active' \
         ORDER BY relative_path \
         LIMIT ?2 OFFSET ?3"
    ))?;
    let records = stmt
        .query_map(params![task_id, limit, offset], map_file_record)?
        .filter_map(|r| r.ok())
        .collect();
    Ok(records)
}

/// Sync log history for a task, newest first.
pub fn get_sync_logs(
    state: &DbState,
    task_id: i64,
    limit: Option<i64>,
) -> AppResult<Vec<SyncLog>> {
    let conn = state.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
    let limit = limit.unwrap_or(50).min(200);
    let mut stmt = conn.prepare(
        "SELECT id, task_id, started_at, finished_at, files_scanned, files_copied, \
                files_deleted, bytes_transferred, errors, status \
         FROM sync_logs \
         WHERE task_id = ?1 \
         ORDER BY started_at DESC \
         LIMIT ?2",
    )?;
    let logs = stmt
        .query_map(params![task_id, limit], |row| {
            Ok(SyncLog {
                id: row.get(0)?,
                task_id: row.get(1)?,
                started_at: row.get(2)?,
                finished_at: row.get(3)?,
                files_scanned: row.get(4)?,
                files_copied: row.get(5)?,
                files_deleted: row.get(6)?,
                bytes_transferred: row.get(7)?,
                errors: row.get(8)?,
                status: row.get(9)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(logs)
}
