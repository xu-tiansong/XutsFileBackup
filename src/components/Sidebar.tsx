import { useState } from "react";
import type { Task, TagNode } from "../lib/api";

// ── Tag tree node (recursive) ──────────────────────────────────────────────

function TagTreeNode({
  node,
  activeTagIds,
  onToggle,
  onEdit,
  depth,
}: {
  node: TagNode;
  activeTagIds: number[];
  onToggle: (id: number) => void;
  onEdit: (node: TagNode) => void;
  depth: number;
}) {
  const [expanded, setExpanded] = useState(true);
  const [hovered, setHovered] = useState(false);
  const isActive = activeTagIds.includes(node.id);
  const hasChildren = node.children.length > 0;

  return (
    <div>
      <div
        className={`flex items-center gap-1.5 py-1 rounded cursor-pointer text-sm select-none group ${
          isActive
            ? "bg-indigo-600/30 text-indigo-300"
            : "text-gray-300 hover:bg-gray-800"
        }`}
        style={{ paddingLeft: `${8 + depth * 14}px`, paddingRight: "8px" }}
        onClick={() => onToggle(node.id)}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
      >
        <button
          className={`w-3 text-xs text-gray-500 flex-shrink-0 ${!hasChildren && "invisible"}`}
          onClick={(e) => {
            e.stopPropagation();
            setExpanded(!expanded);
          }}
        >
          {expanded ? "▼" : "▶"}
        </button>
        <span
          className="w-2 h-2 rounded-full flex-shrink-0"
          style={{ backgroundColor: node.color }}
        />
        <span className="truncate flex-1">{node.name}</span>
        {hovered && (
          <button
            className="w-5 h-5 flex items-center justify-center rounded hover:bg-white/20 text-xs opacity-60 hover:opacity-100 flex-shrink-0"
            title="编辑标签"
            onClick={(e) => {
              e.stopPropagation();
              onEdit(node);
            }}
          >
            ✎
          </button>
        )}
      </div>
      {expanded &&
        node.children.map((child) => (
          <TagTreeNode
            key={child.id}
            node={child}
            activeTagIds={activeTagIds}
            onToggle={onToggle}
            onEdit={onEdit}
            depth={depth + 1}
          />
        ))}
    </div>
  );
}

// ── Sidebar ────────────────────────────────────────────────────────────────

interface SidebarProps {
  tasks: Task[];
  tagTree: TagNode[];
  selectedTaskId: number | null;
  activeTagIds: number[];
  syncing: number | null;
  onSelectTask: (id: number | null) => void;
  onToggleTag: (id: number) => void;
  onSync: (id: number) => void;
  onNewTask: () => void;
  onEditTask: (task: Task) => void;
  onNewTag: () => void;
  onEditTag: (node: TagNode) => void;
}

const TRIGGER_LABEL: Record<string, string> = {
  manual: "手动",
  realtime: "实时",
  schedule: "定时",
};

export function Sidebar({
  tasks,
  tagTree,
  selectedTaskId,
  activeTagIds,
  syncing,
  onSelectTask,
  onToggleTag,
  onSync,
  onNewTask,
  onEditTask,
  onNewTag,
  onEditTag,
}: SidebarProps) {
  return (
    <aside className="w-60 bg-gray-900 border-r border-gray-800 flex flex-col overflow-hidden flex-shrink-0">
      {/* Tasks */}
      <div className="p-3 border-b border-gray-800">
        <div className="flex items-center justify-between mb-2 px-1">
          <span className="text-xs font-semibold text-gray-500 uppercase tracking-wider">任务</span>
          <button
            onClick={onNewTask}
            className="text-xs text-indigo-400 hover:text-indigo-300 leading-none"
            title="新建任务"
          >
            + 新建
          </button>
        </div>

        <div
          className={`flex items-center gap-2 px-2 py-1.5 rounded cursor-pointer text-sm ${
            selectedTaskId === null
              ? "bg-indigo-600 text-white"
              : "text-gray-300 hover:bg-gray-800"
          }`}
          onClick={() => onSelectTask(null)}
        >
          <span className="flex-1">全部文件</span>
        </div>

        {tasks.map((task) => (
          <TaskRow
            key={task.id}
            task={task}
            selected={selectedTaskId === task.id}
            syncing={syncing === task.id}
            onSelect={() => onSelectTask(task.id)}
            onSync={() => onSync(task.id)}
            onEdit={() => onEditTask(task)}
          />
        ))}

        {tasks.length === 0 && (
          <div className="text-gray-600 text-xs px-2 py-1">暂无任务</div>
        )}
      </div>

      {/* Tags */}
      <div className="flex-1 overflow-y-auto p-3">
        <div className="flex items-center justify-between mb-2 px-1">
          <span className="text-xs font-semibold text-gray-500 uppercase tracking-wider">标签</span>
          <button
            onClick={onNewTag}
            className="text-xs text-indigo-400 hover:text-indigo-300 leading-none"
            title="新建标签"
          >
            + 新建
          </button>
        </div>
        {tagTree.map((node) => (
          <TagTreeNode
            key={node.id}
            node={node}
            activeTagIds={activeTagIds}
            onToggle={onToggleTag}
            onEdit={onEditTag}
            depth={0}
          />
        ))}
        {tagTree.length === 0 && (
          <div className="text-gray-600 text-xs px-2">暂无标签</div>
        )}
      </div>
    </aside>
  );
}

function TaskRow({
  task,
  selected,
  syncing,
  onSelect,
  onSync,
  onEdit,
}: {
  task: Task;
  selected: boolean;
  syncing: boolean;
  onSelect: () => void;
  onSync: () => void;
  onEdit: () => void;
}) {
  const [hovered, setHovered] = useState(false);

  return (
    <div
      className={`flex items-center gap-2 px-2 py-1.5 rounded cursor-pointer text-sm ${
        selected ? "bg-indigo-600 text-white" : "text-gray-300 hover:bg-gray-800"
      }`}
      onClick={onSelect}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <span
        className={`w-1.5 h-1.5 rounded-full flex-shrink-0 ${
          task.enabled ? "bg-green-400" : "bg-gray-500"
        }`}
      />
      <span className="flex-1 truncate">{task.name}</span>
      {hovered ? (
        <button
          className="w-5 h-5 flex items-center justify-center rounded hover:bg-white/20 text-xs opacity-60 hover:opacity-100 flex-shrink-0"
          title="编辑任务"
          onClick={(e) => {
            e.stopPropagation();
            onEdit();
          }}
        >
          ✎
        </button>
      ) : (
        <span className="text-xs opacity-50">
          {TRIGGER_LABEL[task.triggerType] ?? task.triggerType}
        </span>
      )}
      <button
        className="w-5 h-5 flex items-center justify-center rounded hover:bg-white/20 text-xs opacity-60 hover:opacity-100 flex-shrink-0"
        title="手动同步"
        onClick={(e) => {
          e.stopPropagation();
          onSync();
        }}
        disabled={syncing}
      >
        {syncing ? "…" : "↻"}
      </button>
    </div>
  );
}
