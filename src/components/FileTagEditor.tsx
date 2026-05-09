import { useEffect, useState } from "react";
import { api, type FileRecord, type FileTagRecord, type Tag, type TagSummary } from "../lib/api";

interface Props {
  file: FileRecord;
  allTags: Tag[];
  onClose: () => void;
  onTagsChanged?: (tags: TagSummary[]) => void;
}

export function FileTagEditor({ file, allTags, onClose, onTagsChanged }: Props) {
  const [fileTags, setFileTags] = useState<FileTagRecord[]>([]);
  const [loading, setLoading] = useState(true);
  const [adding, setAdding] = useState<number | null>(null); // tagId being added
  const [error, setError] = useState("");

  const filename = file.relativePath.split("/").pop() ?? file.relativePath;

  useEffect(() => {
    api
      .getFileTags(file.id)
      .then(setFileTags)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [file.id]);

  const assignedIds = new Set(fileTags.map((ft) => ft.tagId));
  const availableTags = allTags.filter((t) => !assignedIds.has(t.id));

  const toSummary = (ft: FileTagRecord) => ({ id: ft.tagId, name: ft.tagName, color: ft.tagColor });

  const handleAdd = async (tagId: number) => {
    setAdding(tagId);
    setError("");
    try {
      await api.addFileTag(file.id, tagId);
      const updated = await api.getFileTags(file.id);
      setFileTags(updated);
      onTagsChanged?.(updated.map(toSummary));
    } catch (e) {
      setError(String(e));
    } finally {
      setAdding(null);
    }
  };

  const handleRemove = async (tagId: number) => {
    setError("");
    try {
      await api.removeFileTag(file.id, tagId);
      const next = fileTags.filter((ft) => ft.tagId !== tagId);
      setFileTags(next);
      onTagsChanged?.(next.map(toSummary));
    } catch (e) {
      setError(String(e));
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60">
      <div className="bg-gray-900 border border-gray-700 rounded-lg shadow-2xl w-[420px] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-gray-800">
          <div className="min-w-0">
            <div className="flex items-center gap-2">
              <span>{file.kind === "dir" ? "📁" : "📄"}</span>
              <span className="text-sm font-semibold text-gray-100 truncate">{filename}</span>
            </div>
            {file.relativePath !== filename && (
              <p className="text-xs text-gray-600 font-mono mt-0.5 truncate">
                {file.relativePath}
              </p>
            )}
          </div>
          <button onClick={onClose} className="btn-ghost text-lg leading-none ml-3 flex-shrink-0">
            ×
          </button>
        </div>

        {/* Body */}
        <div className="px-5 py-4 space-y-4">
          {/* Current tags */}
          <div>
            <p className="text-xs font-medium text-gray-400 mb-2">已有标签</p>
            {loading ? (
              <p className="text-xs text-gray-600">加载中…</p>
            ) : fileTags.length === 0 ? (
              <p className="text-xs text-gray-600">暂无标签</p>
            ) : (
              <div className="flex flex-wrap gap-1.5">
                {fileTags.map((ft) => (
                  <span
                    key={ft.tagId}
                    className="flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium"
                    style={{
                      backgroundColor: ft.tagColor + "33",
                      color: ft.tagColor,
                      border: `1px solid ${ft.tagColor}55`,
                    }}
                  >
                    <span
                      className="w-1.5 h-1.5 rounded-full flex-shrink-0"
                      style={{ backgroundColor: ft.tagColor }}
                    />
                    {ft.tagName}
                    <button
                      onClick={() => handleRemove(ft.tagId)}
                      className="ml-0.5 opacity-60 hover:opacity-100 leading-none"
                      title="移除标签"
                    >
                      ×
                    </button>
                  </span>
                ))}
              </div>
            )}
          </div>

          {/* Add tag */}
          {availableTags.length > 0 && (
            <div>
              <p className="text-xs font-medium text-gray-400 mb-2">添加标签</p>
              <div className="flex flex-wrap gap-1.5">
                {availableTags.map((tag) => (
                  <button
                    key={tag.id}
                    onClick={() => handleAdd(tag.id)}
                    disabled={adding === tag.id}
                    className="flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium border transition-opacity hover:opacity-80 disabled:opacity-40"
                    style={{
                      backgroundColor: tag.color + "22",
                      color: tag.color,
                      borderColor: tag.color + "44",
                    }}
                  >
                    <span
                      className="w-1.5 h-1.5 rounded-full flex-shrink-0"
                      style={{ backgroundColor: tag.color }}
                    />
                    {adding === tag.id ? "…" : `+ ${tag.name}`}
                  </button>
                ))}
              </div>
            </div>
          )}

          {allTags.length === 0 && (
            <p className="text-xs text-gray-600">
              暂无可用标签，请先在侧边栏创建标签。
            </p>
          )}

          {error && <p className="text-red-400 text-xs">{error}</p>}
        </div>

        {/* Footer */}
        <div className="px-5 py-3 border-t border-gray-800 flex justify-end">
          <button onClick={onClose} className="btn-secondary">
            关闭
          </button>
        </div>
      </div>
    </div>
  );
}
