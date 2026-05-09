import { useEffect, useMemo } from "react";
import type { FileRecord } from "../lib/api";

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`;
}

function formatMtime(mtime: number): string {
  return new Date(mtime * 1000).toLocaleDateString("zh-CN", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  });
}

interface FileTableProps {
  files: FileRecord[];
  loading: boolean;
  isFiltered?: boolean;
  currentPath: string;
  onNavigate: (path: string) => void;
  onTagFile: (file: FileRecord) => void;
}

export function FileTable({
  files,
  loading,
  isFiltered = false,
  currentPath,
  onNavigate,
  onTagFile,
}: FileTableProps) {
  const setCurrentPath = onNavigate;

  // Reset to root if current dir no longer exists in the file list
  useEffect(() => {
    if (currentPath === "") return;
    const exists = files.some((f) => f.kind === "dir" && f.relativePath === currentPath);
    if (!exists) setCurrentPath("");
  }, [files, currentPath, setCurrentPath]);

  // Direct children of currentPath (tree mode)
  const treeItems = useMemo(() => {
    const items = files.filter((f) => {
      if (currentPath === "") {
        return !f.relativePath.includes("/");
      }
      if (!f.relativePath.startsWith(currentPath + "/")) return false;
      const rest = f.relativePath.slice(currentPath.length + 1);
      return !rest.includes("/");
    });
    return [...items].sort((a, b) => {
      if (a.kind !== b.kind) return a.kind === "dir" ? -1 : 1;
      return a.relativePath.localeCompare(b.relativePath);
    });
  }, [files, currentPath]);

  // Flat results (filtered mode) — dirs first, then files
  const flatItems = useMemo(
    () =>
      [...files].sort((a, b) => {
        if (a.kind !== b.kind) return a.kind === "dir" ? -1 : 1;
        return a.relativePath.localeCompare(b.relativePath);
      }),
    [files]
  );

  const displayItems = isFiltered ? flatItems : treeItems;

  const breadcrumbs = currentPath ? currentPath.split("/") : [];

  const displayName = (f: FileRecord) =>
    currentPath ? f.relativePath.slice(currentPath.length + 1) : f.relativePath;

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center text-gray-600 text-sm">
        加载中…
      </div>
    );
  }

  if (files.length === 0) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center text-gray-600 gap-2">
        <span className="text-3xl">📂</span>
        <span className="text-sm">暂无文件，请先同步任务</span>
      </div>
    );
  }

  const fileCount = displayItems.filter((f) => f.kind === "file").length;
  const dirCount = displayItems.filter((f) => f.kind === "dir").length;

  return (
    <div className="flex-1 flex flex-col overflow-hidden">
      {/* Breadcrumb (tree mode only) */}
      {!isFiltered && (
        <div className="flex items-center gap-1 px-4 py-1.5 text-xs text-gray-500 border-b border-gray-800 flex-shrink-0 overflow-x-auto whitespace-nowrap">
          <button
            onClick={() => setCurrentPath("")}
            className={`hover:text-gray-200 transition-colors ${currentPath === "" ? "text-gray-300 font-medium" : ""}`}
          >
            根目录
          </button>
          {breadcrumbs.map((segment, i) => {
            const path = breadcrumbs.slice(0, i + 1).join("/");
            const isLast = i === breadcrumbs.length - 1;
            return (
              <span key={path} className="flex items-center gap-1">
                <span className="text-gray-700">/</span>
                <button
                  onClick={() => setCurrentPath(path)}
                  className={`hover:text-gray-200 transition-colors ${isLast ? "text-gray-300 font-medium" : ""}`}
                >
                  {segment}
                </button>
              </span>
            );
          })}
        </div>
      )}

      {/* Table */}
      <div className="flex-1 overflow-auto">
        {displayItems.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-32 text-gray-600 gap-1">
            <span className="text-2xl">📁</span>
            <span className="text-xs">此目录为空</span>
          </div>
        ) : (
          <table className="w-full text-sm border-collapse">
            <thead className="sticky top-0 bg-gray-950 z-10">
              <tr className="border-b border-gray-800">
                <th className="px-4 py-2.5 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">
                  名称
                </th>
                {isFiltered && (
                  <th className="px-4 py-2.5 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider">
                    路径
                  </th>
                )}
                <th className="px-4 py-2.5 text-right text-xs font-semibold text-gray-500 uppercase tracking-wider w-24">
                  大小
                </th>
                <th className="px-4 py-2.5 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider w-28">
                  修改时间
                </th>
                <th className="w-8" />
              </tr>
            </thead>
            <tbody>
              {displayItems.map((file) =>
                file.kind === "dir" ? (
                  <tr
                    key={file.id}
                    className="border-b border-gray-800/30 hover:bg-gray-800/50 transition-colors cursor-pointer group"
                    onClick={() => !isFiltered && setCurrentPath(file.relativePath)}
                  >
                    <td className="px-4 py-2 text-indigo-300 font-medium truncate max-w-xs">
                      <span className="mr-2">📁</span>
                      {isFiltered ? file.relativePath : displayName(file)}
                      {file.tags.length > 0 && (
                        <span className="inline-flex items-center gap-0.5 ml-1.5 align-middle">
                          {file.tags.slice(0, 4).map((t) => (
                            <span
                              key={t.id}
                              className="w-2 h-2 rounded-full flex-shrink-0"
                              style={{ backgroundColor: t.color }}
                              title={t.name}
                            />
                          ))}
                          {file.tags.length > 4 && (
                            <span className="text-gray-500 text-xs leading-none">+{file.tags.length - 4}</span>
                          )}
                        </span>
                      )}
                    </td>
                    {isFiltered && <td className="px-4 py-2 text-gray-600 text-xs">—</td>}
                    <td className="px-4 py-2 text-gray-600 text-right">—</td>
                    <td className="px-4 py-2 text-gray-600 tabular-nums">
                      {formatMtime(file.mtime)}
                    </td>
                    <td className="px-2 py-2 w-8">
                      <button
                        className="opacity-0 group-hover:opacity-60 hover:!opacity-100 text-gray-400 text-xs px-1.5 py-0.5 rounded hover:bg-gray-700 transition-all"
                        title="管理标签"
                        onClick={(e) => { e.stopPropagation(); onTagFile(file); }}
                      >
                        🏷
                      </button>
                    </td>
                  </tr>
                ) : (
                  <tr
                    key={file.id}
                    className="border-b border-gray-800/50 hover:bg-gray-900 transition-colors cursor-default group"
                  >
                    <td className="px-4 py-2 text-gray-200 font-medium truncate max-w-xs">
                      <span className="mr-2 text-gray-600">📄</span>
                      {isFiltered ? file.relativePath : displayName(file)}
                      {file.tags.length > 0 && (
                        <span className="inline-flex items-center gap-0.5 ml-1.5 align-middle">
                          {file.tags.slice(0, 4).map((t) => (
                            <span
                              key={t.id}
                              className="w-2 h-2 rounded-full flex-shrink-0"
                              style={{ backgroundColor: t.color }}
                              title={t.name}
                            />
                          ))}
                          {file.tags.length > 4 && (
                            <span className="text-gray-500 text-xs leading-none">+{file.tags.length - 4}</span>
                          )}
                        </span>
                      )}
                    </td>
                    {isFiltered && (
                      <td className="px-4 py-2 text-gray-500 font-mono text-xs truncate max-w-sm">
                        {(() => {
                          const parts = file.relativePath.split("/");
                          return parts.length > 1 ? parts.slice(0, -1).join("/") + "/" : "";
                        })()}
                      </td>
                    )}
                    <td className="px-4 py-2 text-gray-400 text-right tabular-nums">
                      {formatSize(file.size)}
                    </td>
                    <td className="px-4 py-2 text-gray-500 tabular-nums">
                      {formatMtime(file.mtime)}
                    </td>
                    <td className="px-2 py-2 w-8">
                      <button
                        className="opacity-0 group-hover:opacity-60 hover:!opacity-100 text-gray-400 text-xs px-1.5 py-0.5 rounded hover:bg-gray-700 transition-all"
                        title="管理标签"
                        onClick={() => onTagFile(file)}
                      >
                        🏷
                      </button>
                    </td>
                  </tr>
                )
              )}
            </tbody>
          </table>
        )}
      </div>

      {/* Footer */}
      <div className="px-4 py-1.5 text-xs text-gray-600 border-t border-gray-800 flex-shrink-0">
        {dirCount > 0 && <span>{dirCount} 个目录　</span>}
        {fileCount > 0 && <span>{fileCount} 个文件</span>}
        {dirCount === 0 && fileCount === 0 && <span>空</span>}
      </div>
    </div>
  );
}
