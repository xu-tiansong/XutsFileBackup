import { useEffect } from "react";

export interface ToastItem {
  id: number;
  message: string;
  type: "success" | "error" | "info";
}

interface Props {
  items: ToastItem[];
  onDismiss: (id: number) => void;
}

export function Toasts({ items, onDismiss }: Props) {
  return (
    <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 max-w-sm pointer-events-none">
      {items.map((t) => (
        <Toast key={t.id} item={t} onDismiss={onDismiss} />
      ))}
    </div>
  );
}

function Toast({ item, onDismiss }: { item: ToastItem; onDismiss: (id: number) => void }) {
  useEffect(() => {
    const timer = setTimeout(() => onDismiss(item.id), item.type === "error" ? 6000 : 4000);
    return () => clearTimeout(timer);
  }, [item.id, item.type, onDismiss]);

  const styles =
    item.type === "success"
      ? "bg-green-900/95 border-green-700 text-green-100"
      : item.type === "error"
      ? "bg-red-900/95 border-red-700 text-red-100"
      : "bg-gray-800/95 border-gray-700 text-gray-100";

  return (
    <div
      className={`pointer-events-auto flex items-start gap-3 px-4 py-3 rounded-lg shadow-xl border text-sm ${styles}`}
    >
      <span className="flex-1">{item.message}</span>
      <button
        onClick={() => onDismiss(item.id)}
        className="opacity-60 hover:opacity-100 leading-none flex-shrink-0"
      >
        ×
      </button>
    </div>
  );
}
