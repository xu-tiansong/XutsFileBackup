use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{Emitter, Manager};
use tokio::task::AbortHandle;

use crate::db::DbState;
use crate::error::{AppError, AppResult};
use crate::sync::engine;

/// Stores the AbortHandle for each active background task (watcher or scheduler).
pub struct TaskHandles(pub Mutex<HashMap<i64, AbortHandle>>);

/// Start the appropriate background service for a task based on its trigger_type.
/// Reads task config from the DB, then delegates to `spawn_watcher` or `crate::scheduler::spawn`.
pub async fn start(task_id: i64, app: tauri::AppHandle) -> AppResult<()> {
    let (source_path, trigger_type, cron_expr) = {
        let db = app.state::<DbState>();
        let conn = db.0.lock().map_err(|_| AppError::Other("DB lock".into()))?;
        conn.query_row(
            "SELECT source_path, trigger_type, cron_expr
               FROM tasks WHERE id = ?1 AND enabled = 1",
            rusqlite::params![task_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .map_err(|_| AppError::Other(format!("Task {} not found", task_id)))?
    };

    let abort = match trigger_type.as_str() {
        "realtime" => spawn_watcher(app.clone(), task_id, PathBuf::from(source_path))?,
        "schedule" => {
            let expr = cron_expr
                .ok_or_else(|| AppError::Other("Missing cron_expr for scheduled task".into()))?;
            crate::scheduler::spawn(app.clone(), task_id, expr)?
        }
        _ => return Ok(()), // "manual" — nothing to start
    };

    let handles = app.state::<TaskHandles>();
    let mut map = handles.0.lock().map_err(|_| AppError::Other("handles lock".into()))?;
    if let Some(old) = map.insert(task_id, abort) {
        old.abort(); // replace any previously running handle
    }
    Ok(())
}

/// Stop and remove the background service for a task.
pub fn stop(task_id: i64, app: &tauri::AppHandle) -> AppResult<()> {
    let handles = app.state::<TaskHandles>();
    let mut map = handles.0.lock().map_err(|_| AppError::Other("handles lock".into()))?;
    if let Some(handle) = map.remove(&task_id) {
        handle.abort();
    }
    Ok(())
}

/// Spawn a file-system watcher for `source` that debounces events and triggers sync.
fn spawn_watcher(
    app: tauri::AppHandle,
    task_id: i64,
    source: PathBuf,
) -> AppResult<AbortHandle> {
    use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
    use tokio::sync::mpsc;
    use tokio::time::{Duration, Instant};

    // Bridge: notify's sync callback → async mpsc channel
    let (tx, mut rx) = mpsc::channel::<()>(200);

    let mut watcher = RecommendedWatcher::new(
        move |result: notify::Result<notify::Event>| {
            if let Ok(event) = result {
                match event.kind {
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                        let _ = tx.blocking_send(());
                    }
                    _ => {}
                }
            }
        },
        Config::default(),
    )
    .map_err(|e| AppError::Other(format!("watcher init: {}", e)))?;

    watcher
        .watch(&source, RecursiveMode::Recursive)
        .map_err(|e| AppError::Other(format!("watcher watch '{}': {}", source.display(), e)))?;

    let handle = tokio::task::spawn(async move {
        let _watcher = watcher; // keep alive for the duration of this task

        loop {
            // Block until the first change event arrives
            if rx.recv().await.is_none() {
                break; // sender dropped — watcher was stopped
            }

            // Debounce: absorb all further events for 500 ms
            let deadline = Instant::now() + Duration::from_millis(500);
            loop {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    break;
                }
                match tokio::time::timeout(remaining, rx.recv()).await {
                    Ok(None) => return, // channel closed
                    Ok(Some(())) => {}  // more events — keep draining
                    Err(_) => break,    // debounce window expired
                }
            }

            // Trigger sync and emit result to frontend
            let db = app.state::<DbState>();
            match engine::run(task_id, &db) {
                Ok(result) => {
                    let _ = app.emit("sync-complete", &result);
                }
                Err(e) => {
                    #[derive(serde::Serialize, Clone)]
                    struct SyncError { task_id: i64, error: String }
                    let _ = app.emit("sync-error", SyncError {
                        task_id,
                        error: e.to_string(),
                    });
                }
            }
        }
    });

    Ok(handle.abort_handle())
}
