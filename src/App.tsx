import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, type FileRecord, type SearchQuery, type SyncResult, type Tag, type TagNode, type TagSummary, type Task } from "./lib/api";
import { Sidebar } from "./components/Sidebar";
import { SearchBar } from "./components/SearchBar";
import { FileTable } from "./components/FileTable";
import { Toasts, type ToastItem } from "./components/Toasts";
import { TaskDialog } from "./components/TaskDialog";
import { TagDialog } from "./components/TagDialog";
import { SyncLogPanel } from "./components/SyncLogPanel";
import { FileTagEditor } from "./components/FileTagEditor";

let toastCounter = 0;

function flattenTags(nodes: TagNode[]): Tag[] {
  const result: Tag[] = [];
  function walk(node: TagNode) {
    result.push(node);
    node.children.forEach(walk);
  }
  nodes.forEach(walk);
  return result;
}

export default function App() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [tagTree, setTagTree] = useState<TagNode[]>([]);
  const [files, setFiles] = useState<FileRecord[]>([]);

  const [selectedTaskId, setSelectedTaskId] = useState<number | null>(null);
  const [searchText, setSearchText] = useState("");
  const [activeTagIds, setActiveTagIds] = useState<number[]>([]);
  const [tagLogic, setTagLogic] = useState<"and" | "or">("or");

  const [loading, setLoading] = useState(false);
  const [syncing, setSyncing] = useState<number | null>(null);
  const [currentPath, setCurrentPath] = useState("");

  const [toasts, setToasts] = useState<ToastItem[]>([]);

  // Dialog state
  const [taskDialogOpen, setTaskDialogOpen] = useState(false);
  const [editingTask, setEditingTask] = useState<Task | null>(null);
  const [tagDialogOpen, setTagDialogOpen] = useState(false);
  const [editingTag, setEditingTag] = useState<TagNode | null>(null);
  const [logPanelOpen, setLogPanelOpen] = useState(false);
  const [taggingFile, setTaggingFile] = useState<FileRecord | null>(null);

  const allTags = useMemo(() => flattenTags(tagTree), [tagTree]);

  const addToast = useCallback((message: string, type: ToastItem["type"] = "info") => {
    setToasts((prev) => [...prev, { id: ++toastCounter, message, type }]);
  }, []);

  const dismissToast = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  // ── Bootstrap ────────────────────────────────────────────────────────────

  useEffect(() => {
    api.listTasks().then(setTasks).catch(console.error);
    api.getTagTree().then(setTagTree).catch(console.error);
  }, []);

  // ── Search (debounced) ───────────────────────────────────────────────────

  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const runSearch = useCallback(
    (taskId: number | null, text: string, tagIds: number[], logic: "and" | "or") => {
      if (debounceRef.current) clearTimeout(debounceRef.current);
      debounceRef.current = setTimeout(async () => {
        setLoading(true);
        try {
          const query: SearchQuery = {
            taskId: taskId ?? undefined,
            text: text.trim() || undefined,
            tagIds: tagIds.length ? tagIds : undefined,
            tagLogic: tagIds.length > 1 ? logic : undefined,
            limit: 500,
          };
          const result = await api.searchFiles(query);
          setFiles(result);
        } catch (e) {
          console.error(e);
        } finally {
          setLoading(false);
        }
      }, 250);
    },
    []
  );

  // ── Stable refs so event callbacks always see current state ──────────────

  const tasksRef = useRef(tasks);
  useEffect(() => { tasksRef.current = tasks; }, [tasks]);

  const searchStateRef = useRef({ selectedTaskId, searchText, activeTagIds, tagLogic });
  useEffect(() => {
    searchStateRef.current = { selectedTaskId, searchText, activeTagIds, tagLogic };
  });

  // ── Tauri event listeners (set up once, refs read current state inside) ───

  useEffect(() => {
    const unlistenComplete = listen<SyncResult>("sync-complete", (e) => {
      const { taskId, filesCopied, filesDeleted } = e.payload;
      const task = tasksRef.current.find((t) => t.id === taskId);
      const label = task?.name ?? `任务 ${taskId}`;
      addToast(`${label} 同步完成：复制 ${filesCopied}，删除 ${filesDeleted}`, "success");
      const { selectedTaskId: tid, searchText: text, activeTagIds: tags, tagLogic: logic } =
        searchStateRef.current;
      runSearch(tid, text, tags, logic);
    });

    const unlistenError = listen<{ taskId: number; error: string }>("sync-error", (e) => {
      const { taskId, error } = e.payload;
      const task = tasksRef.current.find((t) => t.id === taskId);
      const label = task?.name ?? `任务 ${taskId}`;
      addToast(`${label} 同步失败：${error}`, "error");
    });

    return () => {
      unlistenComplete.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, [addToast, runSearch]); // both stable (useCallback with [])

  useEffect(() => {
    runSearch(selectedTaskId, searchText, activeTagIds, tagLogic);
  }, [selectedTaskId, searchText, activeTagIds, tagLogic, runSearch]);

  // ── Handlers ─────────────────────────────────────────────────────────────

  const handleSelectTask = (id: number | null) => {
    setSelectedTaskId(id);
    setCurrentPath("");
  };

  const handleToggleTag = (id: number) => {
    setActiveTagIds((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]
    );
  };

  const handleSync = async (taskId: number) => {
    setSyncing(taskId);
    try {
      const result = await api.syncTask(taskId);
      addToast(
        `同步完成：复制 ${result.filesCopied} 个，删除 ${result.filesDeleted} 个`,
        "success"
      );
      runSearch(selectedTaskId, searchText, activeTagIds, tagLogic);
    } catch (e: unknown) {
      addToast(`同步失败：${e}`, "error");
    } finally {
      setSyncing(null);
    }
  };

  const handleSaveTask = (task: Task) => {
    setTasks((prev) => {
      const idx = prev.findIndex((t) => t.id === task.id);
      return idx >= 0 ? prev.map((t) => (t.id === task.id ? task : t)) : [...prev, task];
    });
    setTaskDialogOpen(false);
    setEditingTask(null);
    addToast(`任务"${task.name}"已保存`, "success");
  };

  const handleDeleteTask = (taskId: number) => {
    setTasks((prev) => prev.filter((t) => t.id !== taskId));
    if (selectedTaskId === taskId) setSelectedTaskId(null);
    setTaskDialogOpen(false);
    setEditingTask(null);
    addToast("任务已删除", "info");
  };

  const handleSaveTag = (_savedTag: Tag) => {
    setTagDialogOpen(false);
    setEditingTag(null);
    addToast("标签已保存", "success");
    api.getTagTree().then(setTagTree).catch(console.error);
  };

  const handleDeleteTag = (deletedTagId: number, descendantIds: number[]) => {
    setTagDialogOpen(false);
    setEditingTag(null);
    const removed = new Set([deletedTagId, ...descendantIds]);
    setActiveTagIds((prev) => prev.filter((id) => !removed.has(id)));
    addToast("标签已删除", "info");
    api.getTagTree().then(setTagTree).catch(console.error);
  };

  // ── Render ────────────────────────────────────────────────────────────────

  return (
    <div className="flex h-screen bg-gray-950 text-gray-100 overflow-hidden">
      <Sidebar
        tasks={tasks}
        tagTree={tagTree}
        selectedTaskId={selectedTaskId}
        activeTagIds={activeTagIds}
        syncing={syncing}
        onSelectTask={handleSelectTask}
        onToggleTag={handleToggleTag}
        onSync={handleSync}
        onNewTask={() => {
          setEditingTask(null);
          setTaskDialogOpen(true);
        }}
        onEditTask={(task) => {
          setEditingTask(task);
          setTaskDialogOpen(true);
        }}
        onNewTag={() => {
          setEditingTag(null);
          setTagDialogOpen(true);
        }}
        onEditTag={(node) => {
          setEditingTag(node);
          setTagDialogOpen(true);
        }}
      />

      <main className="flex-1 flex flex-col overflow-hidden min-w-0">
        <SearchBar
          text={searchText}
          activeTagIds={activeTagIds}
          tagLogic={tagLogic}
          allTags={allTags}
          onTextChange={setSearchText}
          onRemoveTag={handleToggleTag}
          onTagLogicChange={setTagLogic}
        />

        <FileTable
          files={files}
          loading={loading}
          isFiltered={!!searchText.trim() || activeTagIds.length > 0}
          currentPath={currentPath}
          onNavigate={setCurrentPath}
          onTagFile={setTaggingFile}
        />

        <SyncLogPanel
          taskId={selectedTaskId}
          open={logPanelOpen}
          onToggle={() => setLogPanelOpen((v) => !v)}
        />
      </main>

      {taskDialogOpen && (
        <TaskDialog
          task={editingTask}
          onSave={handleSaveTask}
          onDelete={handleDeleteTask}
          onClose={() => {
            setTaskDialogOpen(false);
            setEditingTask(null);
          }}
        />
      )}

      {tagDialogOpen && (
        <TagDialog
          tag={editingTag}
          allTags={allTags}
          onSave={handleSaveTag}
          onDelete={handleDeleteTag}
          onClose={() => {
            setTagDialogOpen(false);
            setEditingTag(null);
          }}
        />
      )}

      {taggingFile && (
        <FileTagEditor
          file={taggingFile}
          allTags={allTags}
          onClose={() => setTaggingFile(null)}
          onTagsChanged={(tags: TagSummary[]) =>
            setFiles((prev) =>
              prev.map((f) => (f.id === taggingFile.id ? { ...f, tags } : f))
            )
          }
        />
      )}

      <Toasts items={toasts} onDismiss={dismissToast} />
    </div>
  );
}
