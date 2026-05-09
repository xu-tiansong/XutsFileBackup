import { invoke } from "@tauri-apps/api/core";

// ── Types (mirror Rust structs with camelCase) ─────────────────────────────

export interface Task {
  id: number;
  name: string;
  sourcePath: string;
  targetPath: string;
  triggerType: "manual" | "realtime" | "schedule";
  cronExpr: string | null;
  filterRules: string;
  enabled: boolean;
  createdAt: string;
}

export interface CreateTaskPayload {
  name: string;
  sourcePath: string;
  targetPath: string;
  triggerType: string;
  cronExpr?: string;
  filterRules?: string;
}

export interface UpdateTaskPayload {
  name: string;
  sourcePath: string;
  targetPath: string;
  triggerType: string;
  cronExpr?: string | null;
  filterRules: string;
  enabled: boolean;
}

export interface TagSummary {
  id: number;
  name: string;
  color: string;
}

export interface Tag {
  id: number;
  name: string;
  color: string;
  parentId: number | null;
  description: string | null;
  createdAt: string;
}

export interface TagNode extends Tag {
  children: TagNode[];
}

export interface FileTagRecord {
  fileId: number;
  tagId: number;
  taggedAt: number;
  note: string | null;
  tagName: string;
  tagColor: string;
}

export interface FileRecord {
  id: number;
  taskId: number;
  relativePath: string;
  size: number;
  mtime: number;
  contentHash: string;
  status: string;
  lastSyncedAt: number;
  kind: "file" | "dir";
  tags: TagSummary[];
}

export interface SearchQuery {
  text?: string;
  tagIds?: number[];
  tagLogic?: "and" | "or";
  taskId?: number;
  limit?: number;
  offset?: number;
}

export interface SyncResult {
  taskId: number;
  filesScanned: number;
  filesCopied: number;
  filesDeleted: number;
  bytesTransferred: number;
  errors: string[];
}

export interface SyncLog {
  id: number;
  taskId: number;
  startedAt: number;
  finishedAt: number | null;
  filesScanned: number;
  filesCopied: number;
  filesDeleted: number;
  bytesTransferred: number;
  errors: string;
  status: string;
}

// ── API wrappers ──────────────────────────────────────────────────────────

export const api = {
  // Tasks
  listTasks: () => invoke<Task[]>("list_tasks"),
  getTask: (taskId: number) => invoke<Task>("get_task", { taskId }),
  createTask: (payload: CreateTaskPayload) =>
    invoke<Task>("create_task", { payload }),
  updateTask: (taskId: number, payload: UpdateTaskPayload) =>
    invoke<Task>("update_task", { taskId, payload }),
  deleteTask: (taskId: number) => invoke<void>("delete_task", { taskId }),
  setTaskEnabled: (taskId: number, enabled: boolean) =>
    invoke<Task>("set_task_enabled", { taskId, enabled }),
  syncTask: (taskId: number) => invoke<SyncResult>("sync_task", { taskId }),
  startWatcher: (taskId: number) =>
    invoke<void>("start_task_watcher", { taskId }),
  stopWatcher: (taskId: number) =>
    invoke<void>("stop_task_watcher", { taskId }),

  // Tags
  getTagTree: () => invoke<TagNode[]>("get_tag_tree"),
  listTags: () => invoke<Tag[]>("list_tags"),
  createTag: (payload: { name: string; color: string; parentId?: number | null }) =>
    invoke<Tag>("create_tag", {
      name: payload.name,
      color: payload.color,
      parentId: payload.parentId ?? null,
      description: null,
    }),
  updateTag: (tagId: number, payload: { name: string; color: string; parentId?: number | null }) =>
    invoke<Tag>("update_tag", {
      tagId,
      name: payload.name,
      color: payload.color,
      parentId: payload.parentId ?? null,
      description: null,
    }),
  deleteTag: (tagId: number) => invoke<void>("delete_tag", { tagId }),

  // File-tag associations
  addFileTag: (fileId: number, tagId: number, note?: string) =>
    invoke<void>("add_file_tag", { fileId, tagId, note }),
  removeFileTag: (fileId: number, tagId: number) =>
    invoke<void>("remove_file_tag", { fileId, tagId }),
  getFileTags: (fileId: number) =>
    invoke<FileTagRecord[]>("get_file_tags", { fileId }),

  // Search & browse
  searchFiles: (query: SearchQuery) =>
    invoke<FileRecord[]>("search_files", { query }),
  getTaskFiles: (taskId: number, limit?: number, offset?: number) =>
    invoke<FileRecord[]>("get_task_files", { taskId, limit, offset }),
  getSyncLogs: (taskId: number, limit?: number) =>
    invoke<SyncLog[]>("get_sync_logs", { taskId, limit }),
};
