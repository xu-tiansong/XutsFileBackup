use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::db::DbState;
use crate::error::{AppError, AppResult};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: i64,
    pub name: String,
    pub source_path: String,
    pub target_path: String,
    pub trigger_type: String,
    pub cron_expr: Option<String>,
    pub filter_rules: String,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTaskPayload {
    pub name: String,
    pub source_path: String,
    pub target_path: String,
    pub trigger_type: String,
    pub cron_expr: Option<String>,
    pub filter_rules: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTaskPayload {
    pub name: String,
    pub source_path: String,
    pub target_path: String,
    pub trigger_type: String,
    pub cron_expr: Option<String>,
    pub filter_rules: String,
    pub enabled: bool,
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

pub fn list_tasks(state: &DbState) -> AppResult<Vec<Task>> {
    let conn = state.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
    let mut stmt = conn.prepare(
        "SELECT id, name, source_path, target_path, trigger_type, cron_expr, \
                filter_rules, enabled, created_at \
           FROM tasks ORDER BY created_at DESC",
    )?;
    let tasks = stmt
        .query_map([], |row| {
            Ok(Task {
                id: row.get(0)?,
                name: row.get(1)?,
                source_path: row.get(2)?,
                target_path: row.get(3)?,
                trigger_type: row.get(4)?,
                cron_expr: row.get(5)?,
                filter_rules: row.get(6)?,
                enabled: row.get::<_, i64>(7)? != 0,
                created_at: row.get(8)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(tasks)
}

pub fn get_task(state: &DbState, id: i64) -> AppResult<Task> {
    let conn = state.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
    conn.query_row(
        "SELECT id, name, source_path, target_path, trigger_type, cron_expr, \
                filter_rules, enabled, created_at \
           FROM tasks WHERE id = ?1",
        params![id],
        |row| {
            Ok(Task {
                id: row.get(0)?,
                name: row.get(1)?,
                source_path: row.get(2)?,
                target_path: row.get(3)?,
                trigger_type: row.get(4)?,
                cron_expr: row.get(5)?,
                filter_rules: row.get(6)?,
                enabled: row.get::<_, i64>(7)? != 0,
                created_at: row.get(8)?,
            })
        },
    )
    .map_err(|_| AppError::Other(format!("Task {} not found", id)))
}

pub async fn create_task(app: tauri::AppHandle, payload: CreateTaskPayload) -> AppResult<Task> {
    let state = app.state::<DbState>();
    let filter_rules = payload.filter_rules.unwrap_or_else(|| "{}".into());
    let created_at = now();
    let task_id = {
        let conn = state.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
        conn.execute(
            "INSERT INTO tasks (name, source_path, target_path, trigger_type, cron_expr, \
                                filter_rules, enabled, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7)",
            params![
                payload.name,
                payload.source_path,
                payload.target_path,
                payload.trigger_type,
                payload.cron_expr,
                filter_rules,
                created_at
            ],
        )?;
        conn.last_insert_rowid()
    };
    if payload.trigger_type != "manual" {
        let _ = crate::watcher::start(task_id, app.clone()).await;
    }
    get_task(&state, task_id)
}

pub async fn update_task(
    app: tauri::AppHandle,
    id: i64,
    payload: UpdateTaskPayload,
) -> AppResult<Task> {
    // Stop any running watcher before modifying config
    let _ = crate::watcher::stop(id, &app);

    let state = app.state::<DbState>();
    {
        let conn = state.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
        let rows = conn.execute(
            "UPDATE tasks \
                SET name = ?1, source_path = ?2, target_path = ?3, \
                    trigger_type = ?4, cron_expr = ?5, filter_rules = ?6, enabled = ?7 \
              WHERE id = ?8",
            params![
                payload.name,
                payload.source_path,
                payload.target_path,
                payload.trigger_type,
                payload.cron_expr,
                payload.filter_rules,
                payload.enabled as i64,
                id
            ],
        )?;
        if rows == 0 {
            return Err(AppError::Other(format!("Task {} not found", id)));
        }
    }
    if payload.enabled && payload.trigger_type != "manual" {
        let _ = crate::watcher::start(id, app.clone()).await;
    }
    get_task(&state, id)
}

pub async fn delete_task(app: tauri::AppHandle, id: i64) -> AppResult<()> {
    let _ = crate::watcher::stop(id, &app);
    let state = app.state::<DbState>();
    let conn = state.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
    conn.execute("DELETE FROM tasks WHERE id = ?1", params![id])?;
    Ok(())
}

pub async fn set_task_enabled(
    app: tauri::AppHandle,
    id: i64,
    enabled: bool,
) -> AppResult<Task> {
    let state = app.state::<DbState>();
    {
        let conn = state.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
        conn.execute(
            "UPDATE tasks SET enabled = ?1 WHERE id = ?2",
            params![enabled as i64, id],
        )?;
    }
    if enabled {
        let _ = crate::watcher::start(id, app.clone()).await;
    } else {
        let _ = crate::watcher::stop(id, &app);
    }
    get_task(&state, id)
}
