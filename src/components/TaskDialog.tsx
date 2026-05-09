import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, type Task, type CreateTaskPayload, type UpdateTaskPayload } from "../lib/api";

interface Props {
  task: Task | null; // null = create new
  onSave: (task: Task) => void;
  onDelete?: (taskId: number) => void;
  onClose: () => void;
}

function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div>
      <label className="block text-xs font-medium text-gray-400 mb-1">{label}</label>
      {hint && <p className="text-xs text-gray-600 mb-1">{hint}</p>}
      {children}
    </div>
  );
}

export function TaskDialog({ task, onSave, onDelete, onClose }: Props) {
  const [name, setName] = useState(task?.name ?? "");
  const [sourcePath, setSourcePath] = useState(task?.sourcePath ?? "");
  const [targetPath, setTargetPath] = useState(task?.targetPath ?? "");
  const [triggerType, setTriggerType] = useState<"manual" | "realtime" | "schedule">(
    task?.triggerType ?? "manual"
  );
  const [cronExpr, setCronExpr] = useState(task?.cronExpr ?? "0 0 2 * * *");
  const [excludeGlobs, setExcludeGlobs] = useState<string[]>(() => {
    try {
      return JSON.parse(task?.filterRules ?? "{}").exclude_globs ?? [];
    } catch {
      return [];
    }
  });
  const [enabled, setEnabled] = useState(task?.enabled ?? true);
  const [saving, setSaving] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [error, setError] = useState("");

  const browse = async (setter: (v: string) => void) => {
    const selected = await open({ directory: true, multiple: false, title: "选择目录" });
    if (typeof selected === "string") setter(selected);
  };

  const handleSave = async () => {
    if (!name.trim() || !sourcePath.trim() || !targetPath.trim()) {
      setError("请填写任务名、源目录和目标目录");
      return;
    }
    setSaving(true);
    setError("");
    const filterRules = JSON.stringify({
      exclude_globs: excludeGlobs.filter(Boolean),
    });
    try {
      let result: Task;
      if (task) {
        const payload: UpdateTaskPayload = {
          name,
          sourcePath,
          targetPath,
          triggerType,
          cronExpr: triggerType === "schedule" ? cronExpr : null,
          filterRules,
          enabled,
        };
        result = await api.updateTask(task.id, payload);
      } else {
        const payload: CreateTaskPayload = {
          name,
          sourcePath,
          targetPath,
          triggerType,
          cronExpr: triggerType === "schedule" ? cronExpr : undefined,
          filterRules,
        };
        result = await api.createTask(payload);
      }
      onSave(result);
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    if (!task) return;
    try {
      await api.deleteTask(task.id);
      onDelete?.(task.id);
    } catch (e: unknown) {
      setError(String(e));
    }
  };

  const updateGlob = (i: number, v: string) => {
    const next = [...excludeGlobs];
    next[i] = v;
    setExcludeGlobs(next);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-gray-900 border border-gray-700 rounded-lg shadow-2xl w-[500px] max-h-[90vh] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-gray-800 flex-shrink-0">
          <h2 className="text-sm font-semibold text-gray-100">
            {task ? "编辑任务" : "新建任务"}
          </h2>
          <button onClick={onClose} className="btn-ghost text-lg leading-none">
            ×
          </button>
        </div>

        {/* Body */}
        <div className="px-5 py-4 space-y-4 overflow-y-auto flex-1">
          <Field label="任务名称">
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="input-base"
              placeholder="我的备份任务"
              autoFocus
            />
          </Field>

          <Field label="源目录">
            <div className="flex gap-2">
              <input
                value={sourcePath}
                onChange={(e) => setSourcePath(e.target.value)}
                className="input-base flex-1"
                placeholder="C:\Users\..."
              />
              <button onClick={() => browse(setSourcePath)} className="btn-secondary flex-shrink-0">
                浏览
              </button>
            </div>
          </Field>

          <Field label="目标目录">
            <div className="flex gap-2">
              <input
                value={targetPath}
                onChange={(e) => setTargetPath(e.target.value)}
                className="input-base flex-1"
                placeholder="D:\Backup\..."
              />
              <button onClick={() => browse(setTargetPath)} className="btn-secondary flex-shrink-0">
                浏览
              </button>
            </div>
          </Field>

          <Field label="触发方式">
            <select
              value={triggerType}
              onChange={(e) => setTriggerType(e.target.value as "manual" | "realtime" | "schedule")}
              className="input-base"
            >
              <option value="manual">手动</option>
              <option value="realtime">实时监控（文件变化时自动同步）</option>
              <option value="schedule">定时（Cron 表达式）</option>
            </select>
          </Field>

          {triggerType === "schedule" && (
            <Field
              label="Cron 表达式"
              hint="格式：秒 分 时 日 月 周  ·  例：0 0 2 * * *（每天凌晨 2 点）"
            >
              <input
                value={cronExpr}
                onChange={(e) => setCronExpr(e.target.value)}
                className="input-base font-mono"
                placeholder="0 0 2 * * *"
              />
            </Field>
          )}

          <Field label="排除规则（Glob）">
            <div className="space-y-1.5">
              {excludeGlobs.map((g, i) => (
                <div key={i} className="flex gap-2">
                  <input
                    value={g}
                    onChange={(e) => updateGlob(i, e.target.value)}
                    className="input-base flex-1 font-mono text-xs"
                    placeholder="node_modules/  或  *.tmp"
                  />
                  <button
                    onClick={() => setExcludeGlobs(excludeGlobs.filter((_, j) => j !== i))}
                    className="btn-ghost"
                  >
                    ×
                  </button>
                </div>
              ))}
              <button
                onClick={() => setExcludeGlobs([...excludeGlobs, ""])}
                className="text-xs text-indigo-400 hover:text-indigo-300"
              >
                + 添加排除项
              </button>
            </div>
          </Field>

          {task && (
            <Field label="状态">
              <label className="flex items-center gap-2.5 cursor-pointer select-none">
                <div
                  className={`relative w-9 h-5 rounded-full transition-colors cursor-pointer ${
                    enabled ? "bg-indigo-600" : "bg-gray-700"
                  }`}
                  onClick={() => setEnabled(!enabled)}
                >
                  <span
                    className={`absolute top-0.5 w-4 h-4 bg-white rounded-full shadow transition-transform ${
                      enabled ? "translate-x-4" : "translate-x-0.5"
                    }`}
                  />
                </div>
                <span className="text-sm text-gray-300">{enabled ? "已启用" : "已禁用"}</span>
              </label>
            </Field>
          )}

          {error && <p className="text-red-400 text-xs">{error}</p>}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between px-5 py-4 border-t border-gray-800 flex-shrink-0">
          <div>
            {task &&
              (confirmDelete ? (
                <div className="flex items-center gap-2">
                  <span className="text-xs text-red-400">确认删除？</span>
                  <button onClick={handleDelete} className="btn-danger text-xs py-1 px-3">
                    删除
                  </button>
                  <button
                    onClick={() => setConfirmDelete(false)}
                    className="btn-secondary text-xs py-1 px-3"
                  >
                    取消
                  </button>
                </div>
              ) : (
                <button
                  onClick={() => setConfirmDelete(true)}
                  className="text-xs text-red-500 hover:text-red-400"
                >
                  删除任务
                </button>
              ))}
          </div>
          <div className="flex gap-2">
            <button onClick={onClose} className="btn-secondary">
              取消
            </button>
            <button onClick={handleSave} disabled={saving} className="btn-primary">
              {saving ? "保存中…" : "保存"}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
