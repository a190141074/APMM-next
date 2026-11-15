// 搜索与目录共用别名，键盘选中的结果定位到完整详情中的对应部位。
import { useMemo, useState } from "react";
import { Search, X } from "lucide-react";
import { ThemedSelect } from "../shared/ThemedSelect";
import type { ResourceRecord } from "../core/ipc/generated/ResourceRecord";
import {
  rankSearch,
  labels,
  type SearchMode,
  type Route,
} from "../core/domain/resources";
export function ResourceSearch({
  value,
  onChange,
  mode,
  onMode,
  rows,
  onSelect,
}: {
  value: string;
  onChange: (s: string) => void;
  mode: SearchMode;
  onMode: (s: SearchMode) => void;
  rows: ResourceRecord[];
  onSelect: (r: ResourceRecord) => void;
}) {
  const [open, setOpen] = useState(false);
  const [index, setIndex] = useState(0);
  const results = useMemo(
    () => (value.trim() ? rankSearch(rows, value, mode) : []),
    [rows, value, mode],
  );
  const suggestions = results.slice(0, 12);
  const choose = (r: ResourceRecord) => {
    onSelect(r);
    setOpen(false);
  };
  return (
    <div
      className="search-container"
      onBlur={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget)) setOpen(false);
      }}
    >
      <div className="global-search">
        <Search size={17} />
        <ThemedSelect
          label="搜索模式"
          className="search-mode-select"
          value={mode}
          onValueChange={(value) => {
            onMode(value as SearchMode);
            setIndex(0);
            setOpen(false);
          }}
          options={[
            { value: "smart", label: "智能" },
            { value: "id", label: "ID" },
          ]}
        />
        <input
          role="combobox"
          aria-label="搜索资源"
          aria-expanded={open && !!value}
          aria-controls="search-results"
          aria-autocomplete="list"
          aria-activedescendant={
            suggestions[index] && open ? `search-result-${index}` : undefined
          }
          value={value}
          onFocus={() => setOpen(true)}
          placeholder={
            mode === "smart"
              ? "中文、英文、昵称、文件名"
              : "ID、完整资源键、哈希"
          }
          onChange={(e) => {
            onChange(e.target.value);
            setIndex(0);
            setOpen(true);
          }}
          onKeyDown={(e) => {
            if (e.key === "Escape") setOpen(false);
            if (e.key === "ArrowDown") {
              e.preventDefault();
              setOpen(true);
              setIndex((i) =>
                Math.max(0, Math.min(i + 1, suggestions.length - 1)),
              );
            }
            if (e.key === "ArrowUp") {
              e.preventDefault();
              setIndex((i) => Math.max(0, i - 1));
            }
            if (e.key === "Enter" && suggestions[index]) {
              e.preventDefault();
              choose(suggestions[index].row);
            }
          }}
        />
        {value && (
          <button
            className="icon-button"
            aria-label="清空搜索"
            onClick={() => onChange("")}
          >
            <X size={14} />
          </button>
        )}
      </div>
      {open && value.trim() && (
        <div className="search-results" id="search-results" role="listbox">
          <div className="search-count">
            {results.length} 个部位命中 · ↑↓ 选择，Enter 定位
          </div>
          {suggestions.map(({ row: r, score }, i) => (
            <button
              role="option"
              id={`search-result-${i}`}
              aria-selected={index === i}
              key={r.key}
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => choose(r)}
            >
              <strong>
                {r.entityName}
                <small>
                  {labels[r.category as Route]} · {r.variantLabel} ·{" "}
                  {r.resourceLabel}
                </small>
              </strong>
              <code>{r.canonical}</code>
              {score > 0 && <small>相似结果</small>}
            </button>
          ))}
          {!results.length && <p>没有匹配项，请尝试名称、编号或文件名。</p>}
        </div>
      )}
    </div>
  );
}
