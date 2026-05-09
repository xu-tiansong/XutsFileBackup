use tauri::State;

use crate::db::DbState;
use crate::error::AppResult;
use crate::search;
use crate::sync::engine::SyncResult;
use crate::tags;
use crate::task_manager;

// ── Sync ─────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn sync_task(task_id: i64, state: State<'_, DbState>) -> AppResult<SyncResult> {
    crate::sync::engine::run(task_id, &state)
}

#[tauri::command]
pub async fn start_task_watcher(task_id: i64, app: tauri::AppHandle) -> AppResult<()> {
    crate::watcher::start(task_id, app).await
}

#[tauri::command]
pub fn stop_task_watcher(task_id: i64, app: tauri::AppHandle) -> AppResult<()> {
    crate::watcher::stop(task_id, &app)
}

// ── Tasks ─────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_tasks(state: State<'_, DbState>) -> AppResult<Vec<task_manager::Task>> {
    task_manager::list_tasks(&state)
}

#[tauri::command]
pub fn get_task(task_id: i64, state: State<'_, DbState>) -> AppResult<task_manager::Task> {
    task_manager::get_task(&state, task_id)
}

#[tauri::command]
pub async fn create_task(
    app: tauri::AppHandle,
    payload: task_manager::CreateTaskPayload,
) -> AppResult<task_manager::Task> {
    task_manager::create_task(app, payload).await
}

#[tauri::command]
pub async fn update_task(
    task_id: i64,
    app: tauri::AppHandle,
    payload: task_manager::UpdateTaskPayload,
) -> AppResult<task_manager::Task> {
    task_manager::update_task(app, task_id, payload).await
}

#[tauri::command]
pub async fn delete_task(task_id: i64, app: tauri::AppHandle) -> AppResult<()> {
    task_manager::delete_task(app, task_id).await
}

#[tauri::command]
pub async fn set_task_enabled(
    task_id: i64,
    enabled: bool,
    app: tauri::AppHandle,
) -> AppResult<task_manager::Task> {
    task_manager::set_task_enabled(app, task_id, enabled).await
}

// ── Tags ──────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn create_tag(
    state: State<'_, DbState>,
    name: String,
    color: String,
    parent_id: Option<i64>,
    description: Option<String>,
) -> AppResult<tags::Tag> {
    tags::create_tag(&state, name, color, parent_id, description)
}

#[tauri::command]
pub fn update_tag(
    state: State<'_, DbState>,
    tag_id: i64,
    name: String,
    color: String,
    parent_id: Option<i64>,
    description: Option<String>,
) -> AppResult<tags::Tag> {
    tags::update_tag(&state, tag_id, name, color, parent_id, description)
}

#[tauri::command]
pub fn delete_tag(state: State<'_, DbState>, tag_id: i64, with_descendants: bool) -> AppResult<()> {
    tags::delete_tag(&state, tag_id, with_descendants)
}

#[tauri::command]
pub fn list_tags(state: State<'_, DbState>) -> AppResult<Vec<tags::Tag>> {
    tags::list_tags(&state)
}

#[tauri::command]
pub fn get_tag_tree(state: State<'_, DbState>) -> AppResult<Vec<tags::TagNode>> {
    tags::get_tag_tree(&state)
}

// ── File-Tag associations ─────────────────────────────────────────────────────

#[tauri::command]
pub fn add_file_tag(
    state: State<'_, DbState>,
    file_id: i64,
    tag_id: i64,
    note: Option<String>,
) -> AppResult<()> {
    tags::add_file_tag(&state, file_id, tag_id, note)
}

#[tauri::command]
pub fn remove_file_tag(
    state: State<'_, DbState>,
    file_id: i64,
    tag_id: i64,
) -> AppResult<()> {
    tags::remove_file_tag(&state, file_id, tag_id)
}

#[tauri::command]
pub fn get_file_tags(
    state: State<'_, DbState>,
    file_id: i64,
) -> AppResult<Vec<tags::FileTagRecord>> {
    tags::get_file_tags(&state, file_id)
}

// ── Search ────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn search_files(
    state: State<'_, DbState>,
    query: search::SearchQuery,
) -> AppResult<Vec<search::FileRecord>> {
    search::search_files(&state, query)
}

#[tauri::command]
pub fn get_task_files(
    state: State<'_, DbState>,
    task_id: i64,
    limit: Option<i64>,
    offset: Option<i64>,
) -> AppResult<Vec<search::FileRecord>> {
    search::get_task_files(&state, task_id, limit, offset)
}

#[tauri::command]
pub fn get_sync_logs(
    state: State<'_, DbState>,
    task_id: i64,
    limit: Option<i64>,
) -> AppResult<Vec<search::SyncLog>> {
    search::get_sync_logs(&state, task_id, limit)
}
