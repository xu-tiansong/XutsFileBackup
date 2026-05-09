use cron::Schedule;
use std::str::FromStr;
use tauri::{Emitter, Manager};
use tokio::task::AbortHandle;

use crate::db::DbState;
use crate::error::{AppError, AppResult};
use crate::sync::engine;

/// Spawn a cron-scheduled sync task.
///
/// `cron_expr` uses Quartz format: `<sec> <min> <hour> <day> <month> <weekday>`
/// Example: `"0 30 2 * * *"` = every day at 02:30:00
pub fn spawn(app: tauri::AppHandle, task_id: i64, cron_expr: String) -> AppResult<AbortHandle> {
    // Validate the expression before spawning
    Schedule::from_str(&cron_expr)
        .map_err(|e| AppError::Other(format!("invalid cron '{}': {}", cron_expr, e)))?;

    let handle = tokio::task::spawn(async move {
        loop {
            // Re-parse each iteration to get the next upcoming time.
            // This avoids Send/Sync issues with the iterator and is negligible overhead
            // since we're sleeping for at least seconds between runs.
            let next = Schedule::from_str(&cron_expr)
                .ok()
                .and_then(|s| s.upcoming(chrono::Utc).next());

            match next {
                None => break, // expression has no more scheduled times
                Some(next_time) => {
                    let now = chrono::Utc::now();
                    let delay = (next_time - now)
                        .to_std()
                        .unwrap_or(tokio::time::Duration::from_secs(60));
                    tokio::time::sleep(delay).await;
                }
            }

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
