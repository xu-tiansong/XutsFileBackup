use chrono::Utc;
use rusqlite::{params, params_from_iter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::db::DbState;
use crate::error::{AppError, AppResult};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub parent_id: Option<i64>,
    pub description: Option<String>,
    pub created_at: String,
}

/// Tree node returned to the frontend — Tag fields plus recursive children.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagNode {
    pub id: i64,
    pub name: String,
    pub color: String,
    pub parent_id: Option<i64>,
    pub description: Option<String>,
    pub created_at: String,
    pub children: Vec<TagNode>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileTagRecord {
    pub file_id: i64,
    pub tag_id: i64,
    pub tagged_at: i64,
    pub note: Option<String>,
    pub tag_name: String,
    pub tag_color: String,
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn now_secs() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Rebuild the FTS5 entry for a file after its tags change.
/// Must be called while holding the DB lock (i.e. within the same connection).
fn rebuild_file_fts(conn: &rusqlite::Connection, file_id: i64) -> rusqlite::Result<()> {
    let relative_path: String = match conn.query_row(
        "SELECT relative_path FROM file_records WHERE id = ?1",
        params![file_id],
        |row| row.get(0),
    ) {
        Ok(p) => p,
        Err(_) => return Ok(()), // file not found — nothing to index
    };
    let filename = relative_path.rsplit('/').next().unwrap_or(&relative_path).to_string();
    let tag_names: String = {
        let mut stmt = conn.prepare(
            "SELECT t.name FROM file_tags ft JOIN tags t ON t.id = ft.tag_id WHERE ft.file_id = ?1",
        )?;
        let names: Vec<String> = stmt
            .query_map(params![file_id], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .collect();
        names.join(" ")
    };
    conn.execute("DELETE FROM file_search_index WHERE rowid = ?1", params![file_id])?;
    conn.execute(
        "INSERT INTO file_search_index(rowid, file_id, filename, relative_path, tag_names) \
         VALUES(?1, ?1, ?2, ?3, ?4)",
        params![file_id, filename, relative_path, tag_names],
    )?;
    Ok(())
}

pub fn create_tag(
    state: &DbState,
    name: String,
    color: String,
    parent_id: Option<i64>,
    description: Option<String>,
) -> AppResult<Tag> {
    let conn = state.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
    let created_at = now();
    conn.execute(
        "INSERT INTO tags (name, color, parent_id, description, created_at) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![name, color, parent_id, description, created_at],
    )?;
    let id = conn.last_insert_rowid();
    Ok(Tag { id, name, color, parent_id, description, created_at })
}

pub fn update_tag(
    state: &DbState,
    id: i64,
    name: String,
    color: String,
    parent_id: Option<i64>,
    description: Option<String>,
) -> AppResult<Tag> {
    let conn = state.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
    let rows = conn.execute(
        "UPDATE tags SET name = ?1, color = ?2, parent_id = ?3, description = ?4 \
         WHERE id = ?5",
        params![name, color, parent_id, description, id],
    )?;
    if rows == 0 {
        return Err(AppError::Other(format!("Tag {} not found", id)));
    }
    let created_at: String =
        conn.query_row("SELECT created_at FROM tags WHERE id = ?1", params![id], |r| r.get(0))?;
    Ok(Tag { id, name, color, parent_id, description, created_at })
}

pub fn delete_tag(state: &DbState, id: i64, with_descendants: bool) -> AppResult<()> {
    let conn = state.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;

    if with_descendants {
        // Collect entire subtree (root included) via recursive CTE.
        let ids: Vec<i64> = {
            let mut stmt = conn.prepare(
                "WITH RECURSIVE subtree(id) AS (
                   SELECT ?1
                   UNION ALL
                   SELECT t.id FROM tags t JOIN subtree s ON t.parent_id = s.id
                 )
                 SELECT id FROM subtree",
            )?;
            let v: Vec<i64> = stmt.query_map(params![id], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            v
        };

        // Collect affected files before tags (and their file_tags) are removed.
        let ph: String = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let affected: Vec<i64> = {
            let mut stmt = conn.prepare(&format!(
                "SELECT DISTINCT file_id FROM file_tags WHERE tag_id IN ({ph})"
            ))?;
            let v: Vec<i64> = stmt.query_map(params_from_iter(ids.iter()), |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            v
        };

        // Delete the whole subtree. file_tags CASCADE; parent_id SET NULL is harmless.
        conn.execute(
            &format!("DELETE FROM tags WHERE id IN ({ph})"),
            params_from_iter(ids.iter()),
        )?;

        for fid in affected {
            rebuild_file_fts(&conn, fid)?;
        }
    } else {
        // Collect files that had this tag before removing it.
        let affected: Vec<i64> = {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT file_id FROM file_tags WHERE tag_id = ?1",
            )?;
            let v: Vec<i64> = stmt.query_map(params![id], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            v
        };

        // Children's parent_id becomes NULL (ON DELETE SET NULL) — they become root tags.
        conn.execute("DELETE FROM tags WHERE id = ?1", params![id])?;

        for fid in affected {
            rebuild_file_fts(&conn, fid)?;
        }
    }

    Ok(())
}

pub fn list_tags(state: &DbState) -> AppResult<Vec<Tag>> {
    let conn = state.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
    let mut stmt = conn.prepare(
        "SELECT id, name, color, parent_id, description, created_at \
         FROM tags ORDER BY name",
    )?;
    let tags = stmt
        .query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                parent_id: row.get(3)?,
                description: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(tags)
}

/// Returns the full tag hierarchy as a forest (multiple root nodes possible).
pub fn get_tag_tree(state: &DbState) -> AppResult<Vec<TagNode>> {
    let flat = list_tags(state)?;
    Ok(build_tree(flat))
}

fn build_tree(flat: Vec<Tag>) -> Vec<TagNode> {
    let mut children_map: HashMap<i64, Vec<Tag>> = HashMap::new();
    let mut roots: Vec<Tag> = Vec::new();
    for tag in flat {
        match tag.parent_id {
            None => roots.push(tag),
            Some(pid) => children_map.entry(pid).or_default().push(tag),
        }
    }
    fn to_nodes(tags: Vec<Tag>, map: &mut HashMap<i64, Vec<Tag>>) -> Vec<TagNode> {
        tags.into_iter()
            .map(|t| {
                let ch = map.remove(&t.id).unwrap_or_default();
                TagNode {
                    id: t.id,
                    name: t.name,
                    color: t.color,
                    parent_id: t.parent_id,
                    description: t.description,
                    created_at: t.created_at,
                    children: to_nodes(ch, map),
                }
            })
            .collect()
    }
    to_nodes(roots, &mut children_map)
}

pub fn add_file_tag(
    state: &DbState,
    file_id: i64,
    tag_id: i64,
    note: Option<String>,
) -> AppResult<()> {
    let conn = state.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
    conn.execute(
        "INSERT OR IGNORE INTO file_tags (file_id, tag_id, tagged_at, note) \
         VALUES (?1, ?2, ?3, ?4)",
        params![file_id, tag_id, now_secs(), note],
    )?;
    rebuild_file_fts(&conn, file_id)?;
    Ok(())
}

pub fn remove_file_tag(state: &DbState, file_id: i64, tag_id: i64) -> AppResult<()> {
    let conn = state.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
    conn.execute(
        "DELETE FROM file_tags WHERE file_id = ?1 AND tag_id = ?2",
        params![file_id, tag_id],
    )?;
    rebuild_file_fts(&conn, file_id)?;
    Ok(())
}

pub fn get_file_tags(state: &DbState, file_id: i64) -> AppResult<Vec<FileTagRecord>> {
    let conn = state.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
    let mut stmt = conn.prepare(
        "SELECT ft.file_id, ft.tag_id, ft.tagged_at, ft.note, t.name, t.color \
           FROM file_tags ft \
           JOIN tags t ON t.id = ft.tag_id \
          WHERE ft.file_id = ?1 \
          ORDER BY t.name",
    )?;
    let records = stmt
        .query_map(params![file_id], |row| {
            Ok(FileTagRecord {
                file_id: row.get(0)?,
                tag_id: row.get(1)?,
                tagged_at: row.get(2)?,
                note: row.get(3)?,
                tag_name: row.get(4)?,
                tag_color: row.get(5)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(records)
}
