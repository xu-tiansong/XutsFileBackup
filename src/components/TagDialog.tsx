import { useState } from "react";
import { api, type Tag } from "../lib/api";

interface Props {
  tag: Tag | null; // null = create new
  allTags: Tag[];
  onSave: (tag: Tag) => void;
  onDelete?: (tagId: number, descendantIds: number[]) => void;
  onClose: () => void;
}

const PRESET_COLORS = [
  "#6366f1", "#8b5cf6", "#ec4899", "#ef4444",
  "#f97316", "#eab308", "#22c55e", "#14b8a6",
  "#3b82f6", "#64748b",
];

export function TagDialog({ tag, allTags, onSave, onDelete, onClose }: Props) {
  const [name, setName] = useState(tag?.name ?? "");
  const [color, setColor] = useState(tag?.color ?? "#6366f1");
  const [parentId, setParentId] = useState<number | null>(tag?.parentId ?? null);
  const [saving, setSaving] = useState(false);
  // "idle" | "ask" (has children) | "confirm" (no children)
  const [deleteStep, setDeleteStep] = useState<"idle" | "ask" | "confirm">("idle");
  const [error, setError] = useState("");

  // Collect self + all descendants (to exclude from parent options and for cascade delete info)
  const forbidden = new Set<number>();
  if (tag) {
    forbidden.add(tag.id);
    const collectDescendants = (id: number) => {
      allTags.forEach((t) => {
        if (t.parentId === id && !forbidden.has(t.id)) {
          forbidden.add(t.id);
          collectDescendants(t.id);
        }
      });
    };
    collectDescendants(tag.id);
  }
  const parentOptions = allTags.filter((t) => !forbidden.has(t.id));
  // descendants = forbidden minus self
  const descendantIds = tag ? [...forbidden].filter((id) => id !== tag.id) : [];
  const descendantCount = descendantIds.length;

  const handleSave = async () => {
    if (!name.trim()) {
      setError("请填写标签名称");
      return;
    }
    setSaving(true);
    setError("");
    try {
      let result: Tag;
      if (tag) {
        result = await api.updateTag(tag.id, { name, color, parentId });
      } else {
        result = await api.createTag({ name, color, parentId });
      }
      onSave(result);
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (cascade: boolean) => {
    if (!tag) return;
    try {
      await api.deleteTag(tag.id, cascade);
      onDelete?.(tag.id, cascade ? descendantIds : []);
    } catch (e: unknown) {
      setError(String(e));
      setDeleteStep("idle");
    }
  };

  const onClickDeleteBtn = () => {
    if (descendantCount > 0) {
      setDeleteStep("ask");
    } else {
      setDeleteStep("confirm");
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-gray-900 border border-gray-700 rounded-lg shadow-2xl w-[380px] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-gray-800">
          <h2 className="text-sm font-semibold text-gray-100">
            {tag ? "编辑标签" : "新建标签"}
          </h2>
          <button onClick={onClose} className="btn-ghost text-lg leading-none">
            ×
          </button>
        </div>

        {/* Body */}
        <div className="px-5 py-4 space-y-4">
          {/* Name */}
          <div>
            <label className="block text-xs font-medium text-gray-400 mb-1">标签名称</label>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="input-base"
              placeholder="工作 / 项目 / ..."
              autoFocus
            />
          </div>

          {/* Color */}
          <div>
            <label className="block text-xs font-medium text-gray-400 mb-2">颜色</label>
            <div className="flex items-center gap-2 flex-wrap">
              {PRESET_COLORS.map((c) => (
                <button
                  key={c}
                  className={`w-6 h-6 rounded-full transition-transform ${
                    color === c ? "scale-125 ring-2 ring-white/60" : "hover:scale-110"
                  }`}
                  style={{ backgroundColor: c }}
                  onClick={() => setColor(c)}
                />
              ))}
              <input
                type="color"
                value={color}
                onChange={(e) => setColor(e.target.value)}
                className="w-6 h-6 rounded cursor-pointer bg-transparent border-0 p-0"
                title="自定义颜色"
              />
            </div>
          </div>

          {/* Parent tag */}
          <div>
            <label className="block text-xs font-medium text-gray-400 mb-1">父标签（可选）</label>
            <select
              value={parentId ?? ""}
              onChange={(e) => setParentId(e.target.value ? Number(e.target.value) : null)}
              className="input-base"
            >
              <option value="">无（顶级标签）</option>
              {parentOptions.map((t) => (
                <option key={t.id} value={t.id}>
                  {t.name}
                </option>
              ))}
            </select>
          </div>

          {/* Preview */}
          <div className="flex items-center gap-2">
            <span
              className="w-3 h-3 rounded-full flex-shrink-0"
              style={{ backgroundColor: color }}
            />
            <span className="text-sm text-gray-300">{name || "标签预览"}</span>
          </div>

          {error && <p className="text-red-400 text-xs">{error}</p>}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-between px-5 py-4 border-t border-gray-800">
          <div>
            {tag && deleteStep === "idle" && (
              <button
                onClick={onClickDeleteBtn}
                className="text-xs text-red-500 hover:text-red-400"
              >
                删除标签
              </button>
            )}

            {tag && deleteStep === "confirm" && (
              <div className="flex items-center gap-2">
                <span className="text-xs text-red-400">确认删除？</span>
                <button
                  onClick={() => handleDelete(false)}
                  className="btn-danger text-xs py-1 px-3"
                >
                  删除
                </button>
                <button
                  onClick={() => setDeleteStep("idle")}
                  className="btn-secondary text-xs py-1 px-3"
                >
                  取消
                </button>
              </div>
            )}

            {tag && deleteStep === "ask" && (
              <div className="space-y-2">
                <p className="text-xs text-gray-400">
                  此标签含 {descendantCount} 个子标签，如何处理？
                </p>
                <div className="flex flex-wrap gap-2">
                  <button
                    onClick={() => handleDelete(true)}
                    className="btn-danger text-xs py-1 px-2"
                  >
                    同时删除所有子标签
                  </button>
                  <button
                    onClick={() => handleDelete(false)}
                    className="btn-secondary text-xs py-1 px-2"
                  >
                    仅删除此标签
                  </button>
                  <button
                    onClick={() => setDeleteStep("idle")}
                    className="btn-ghost text-xs py-1 px-2"
                  >
                    取消
                  </button>
                </div>
              </div>
            )}
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
