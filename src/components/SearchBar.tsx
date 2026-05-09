import type { Tag } from "../lib/api";

interface SearchBarProps {
  text: string;
  activeTagIds: number[];
  tagLogic: "and" | "or";
  allTags: Tag[];
  onTextChange: (v: string) => void;
  onRemoveTag: (id: number) => void;
  onTagLogicChange: (v: "and" | "or") => void;
}

export function SearchBar({
  text,
  activeTagIds,
  tagLogic,
  allTags,
  onTextChange,
  onRemoveTag,
  onTagLogicChange,
}: SearchBarProps) {
  const activeTags = allTags.filter((t) => activeTagIds.includes(t.id));

  return (
    <div className="border-b border-gray-800 bg-gray-900 px-4 py-3 flex-shrink-0">
      <div className="flex items-center gap-2">
        {/* Search input */}
        <div className="flex-1 relative">
          <span className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-500 text-sm">
            ⌕
          </span>
          <input
            type="text"
            value={text}
            onChange={(e) => onTextChange(e.target.value)}
            placeholder="搜索文件名或路径…"
            className="w-full bg-gray-800 border border-gray-700 rounded-md pl-8 pr-4 py-1.5 text-sm text-gray-100 placeholder-gray-600 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500"
          />
          {text && (
            <button
              className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-500 hover:text-gray-300"
              onClick={() => onTextChange("")}
            >
              ×
            </button>
          )}
        </div>

        {/* AND/OR toggle — only shown when 2+ tags selected */}
        {activeTagIds.length > 1 && (
          <div className="flex rounded overflow-hidden border border-gray-700 text-xs">
            <button
              className={`px-2 py-1.5 ${
                tagLogic === "or"
                  ? "bg-indigo-600 text-white"
                  : "bg-gray-800 text-gray-400 hover:bg-gray-700"
              }`}
              onClick={() => onTagLogicChange("or")}
            >
              OR
            </button>
            <button
              className={`px-2 py-1.5 ${
                tagLogic === "and"
                  ? "bg-indigo-600 text-white"
                  : "bg-gray-800 text-gray-400 hover:bg-gray-700"
              }`}
              onClick={() => onTagLogicChange("and")}
            >
              AND
            </button>
          </div>
        )}
      </div>

      {/* Active tag chips */}
      {activeTags.length > 0 && (
        <div className="flex flex-wrap gap-1.5 mt-2">
          {activeTags.map((tag) => (
            <span
              key={tag.id}
              className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium"
              style={{
                backgroundColor: tag.color + "33",
                color: tag.color,
                border: `1px solid ${tag.color}66`,
              }}
            >
              {tag.name}
              <button
                onClick={() => onRemoveTag(tag.id)}
                className="opacity-60 hover:opacity-100 leading-none"
              >
                ×
              </button>
            </span>
          ))}
        </div>
      )}
    </div>
  );
}
