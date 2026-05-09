import { useEffect, useState } from "react";
import { api, type SyncLog } from "../lib/api";

interface Props {
  taskId: number | null;
  open: boolean;
  onToggle: () => void;
}

function formatBytes(b: number): string {
  if (b < 1024) return `${b} B`;
  if (b < 1048576) return `${(b / 1024).toFixed(1)} KB`;
  return `${(b / 1048576).toFixed(1)} MB`;
}

function formatTs(ts: number): string {
  return new Date(ts * 1000).toLocaleString();
}

export function SyncLogPanel({ taskId, open, onToggle }: Props) {
  const [logs, setLogs] = useState<SyncLog[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!open || taskId === null) return;
    setLoading(true);
    api
      .getSyncLogs(taskId, 50)
      .then(setLogs)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [open, taskId]);

  return (
    <div className="border-t border-gray-800 flex-shrink-0">
      {/* Toggle bar */}
      <button
        className="w-full flex items-center justify-between px-4 py-2 text-xs font-medium text-gray-400 hover:bg-gray-800/50 transition-colors"
        onClick={onToggle}
      >
        <span>同步日志{taskId ? "" : "（全部任务）"}</span>
        <span>{open ? "▼" : "▶"}</span>
      </button>

      {open && (
        <div className="h-48 overflow-y-auto">
          {taskId === null ? (
            <div className="flex items-center justify-center h-full text-gray-600 text-sm">
              请在左侧选择任务以查看日志
            </div>
          ) : loading ? (
            <div className="flex items-center justify-center h-full text-gray-500 text-sm">
              加载中…
            </div>
          ) : logs.length === 0 ? (
            <div className="flex items-center justify-center h-full text-gray-600 text-sm">
              暂无同步记录
            </div>
          ) : (
            <table className="w-full text-xs">
              <thead className="sticky top-0 bg-gray-900 border-b border-gray-800">
                <tr className="text-gray-500">
                  <th className="text-left px-4 py-1.5 font-medium">时间</th>
                  <th className="text-left px-2 py-1.5 font-medium">状态</th>
                  <th className="text-right px-2 py-1.5 font-medium">复制</th>
                  <th className="text-right px-2 py-1.5 font-medium">删除</th>
                  <th className="text-right px-4 py-1.5 font-medium">数据量</th>
                </tr>
              </thead>
              <tbody>
                {logs.map((log) => (
                  <tr
                    key={log.id}
                    className="border-b border-gray-800/50 hover:bg-gray-800/30"
                  >
                    <td className="px-4 py-1.5 text-gray-400 whitespace-nowrap">
                      {formatTs(log.startedAt)}
                    </td>
                    <td className="px-2 py-1.5">
                      <span
                        className={
                          log.status === "success"
                            ? "text-green-400"
                            : log.status === "error"
                            ? "text-red-400"
                            : "text-yellow-400"
                        }
                      >
                        {log.status === "success"
                          ? "成功"
                          : log.status === "error"
                          ? "失败"
                          : log.status}
                      </span>
                    </td>
                    <td className="px-2 py-1.5 text-right text-gray-300">
                      {log.filesCopied}
                    </td>
                    <td className="px-2 py-1.5 text-right text-gray-300">
                      {log.filesDeleted}
                    </td>
                    <td className="px-4 py-1.5 text-right text-gray-400">
                      {formatBytes(log.bytesTransferred)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}
    </div>
  );
}
