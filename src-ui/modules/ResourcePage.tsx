import { ResourceDetail } from "./ResourceDetail";
import { useState, useMemo, useRef, useEffect } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  Check,
  SlidersHorizontal,
  LayoutGrid,
  Maximize2,
  Image,
  Undo2,
  Download,
  FolderOpen,
  ChevronRight,
  CheckSquare,
  Trash2,
  Power,
} from "lucide-react";
import type { ResourceRecord } from "../core/ipc/generated/ResourceRecord";
import {
  filterResources,
  groupResources,
  labels,
  type Route,
  type Filter,
  type SearchMode,
  resourceGroupKey,
  storageCategory,
} from "../core/domain/resources";
import { useActions } from "../hooks/useApp";
import { AssetImage, Empty, Modal, Badge } from "../shared/Ui";
import { ThemedSelect } from "../shared/ThemedSelect";
const memory = new Map<
  string,
  {
    filter: Filter;
    section: string;
    size: string;
    scroll: number;
    selected: string[];
  }
>();
export function ResourcePage({
  route,
  search,
  resources,
  searchMode = "smart",
  focusKey,
  onFocusClear,
}: {
  route: Route;
  search: string;
  resources: ResourceRecord[];
  searchMode?: SearchMode;
  focusKey?: string;
  onFocusClear?: () => void;
}) {
  const initial = memory.get(route);
  const [filter, setFilter] = useState<Filter>(initial?.filter || "all");
  const [section, setSection] = useState(initial?.section || "");
  const [size, setSize] = useState(initial?.size || "medium");
  const [selected, setSelected] = useState<Set<string>>(
    new Set(initial?.selected),
  );
  const [detail, setDetail] = useState<string | null>(null);
  const { api, act, busy, notice, setNotice } = useActions();
  const scroll = useRef<HTMLDivElement>(null);
  const scrollPosition = useRef(initial?.scroll || 0);
  const [width, setWidth] = useState(900);
  const filtered = useMemo(
    () =>
      filterResources(resources, route, search, filter, section, searchMode),
    [resources, route, search, filter, section, searchMode],
  );
  const groups = useMemo(
    () => groupResources(filtered, route),
    [filtered, route],
  );
  const columns = Math.max(
    2,
    Math.floor(width / (size === "large" ? 270 : size === "small" ? 165 : 215)),
  );
  const height = size === "large" ? 320 : size === "small" ? 230 : 280;
  const virtual = useVirtualizer({
    count: Math.ceil(groups.length / columns),
    getScrollElement: () => scroll.current,
    estimateSize: () => height,
    overscan: 2,
  });
  useEffect(() => {
    // 切换预览尺寸时清除旧行高，避免相邻卡片重叠或留下多余空隙。
    virtual.measure();
  }, [height, virtual]);
  useEffect(() => {
    const el = scroll.current;
    if (!el) return;
    const observer = new ResizeObserver(() => setWidth(el.clientWidth));
    observer.observe(el);
    el.scrollTop = initial?.scroll || 0;
    return () => observer.disconnect();
  }, []);
  useEffect(
    () => () => {
      memory.set(route, {
        filter,
        section,
        size,
        scroll: scrollPosition.current,
        selected: [...selected],
      });
    },
    [route, filter, section, size, selected],
  );
  const sections = [
    ...new Map(
      resources
        .filter((r) => r.category === route)
        .map((r) => [r.section, r.sectionLabel]),
    ).entries(),
  ];
  const detailGroup = groupResources(
    resources.filter((r) => r.category === route),
    route,
  ).find((g) => g.key === detail);
  useEffect(() => {
    const row = resources.find((r) => r.key === focusKey);
    if (row) setDetail(resourceGroupKey(row));
  }, [focusKey]);
  const toggle = (key: string) =>
    setSelected((s) => {
      const next = new Set(s);
      next.has(key) ? next.delete(key) : next.add(key);
      return next;
    });
  const selectedRows = groups
    .filter((g) => selected.has(g.key))
    .flatMap((g) => g.resources);
  return (
    <div className={`resource-page ${detailGroup ? "showing-detail" : ""}`}>
      <div className="resource-toolbar">
        <div className="resource-heading">
          <h1>
            {labels[route]}
            <span className="heading-count">{groups.length}</span>
          </h1>
        </div>
        <div className="filterbar">
          <div className="segments">
            {[
              ["all", "全部"],
              ["applied", "已应用"],
              ["has-mod", "有 Mod"],
              ["no-mod", "无 Mod"],
            ].map(([value, label]) => (
              <button
                key={value}
                className={filter === value ? "active" : ""}
                onClick={() => setFilter(value as Filter)}
              >
                {label}
              </button>
            ))}
          </div>
          <ThemedSelect
            value={section}
            onValueChange={setSection}
            label="资源部位"
            options={[
              { value: "", label: "全部部位" },
              ...sections.map(([value, label]) => ({ value, label })),
            ]}
          />
          <div className="spacer" />
          <ThemedSelect
            value={size}
            onValueChange={setSize}
            label="预览尺寸"
            options={[
              { value: "small", label: "小图" },
              { value: "medium", label: "中图" },
              { value: "large", label: "大图" },
            ]}
          />
          <button
            className="icon-button"
            title="全选当前结果"
            onClick={() =>
              setSelected(
                selected.size === groups.length
                  ? new Set()
                  : new Set(groups.map((g) => g.key)),
              )
            }
          >
            <CheckSquare size={18} />
          </button>
          <button
            className="button subtle"
            onClick={() => void act(() => api.task("scan", "扫描游戏 Mod"))}
          >
            <FolderOpen size={16} />
            扫描本地
          </button>
        </div>
      </div>
      {selected.size > 0 && (
        <div className="selectionbar">
          <strong>已选择 {selected.size} 项</strong>
          <button
            className="button"
            disabled={selectedRows.some((r) => r.category === "temporary")}
            onClick={() =>
              void act(() =>
                api.apply(
                  selectedRows.map((r) => ({
                    entityId: r.entityId,
                    section: r.section,
                    canonical: r.canonical,
                    hash: null,
                  })),
                ),
              )
            }
          >
            <Undo2 size={14} />
            还原所选
          </button>
          <button
            className="button"
            onClick={() =>
              void act(async () => {
                const destination = await api.saveFile("APMM-selected.zip");
                if (destination)
                  return api.task("quick-export", "批量导出", {
                    destination,
                    mode: "zip",
                    items: selectedRows.flatMap((r) =>
                      r.files.map((f) => ({
                        category: storageCategory(r),
                        entityId: r.entityId,
                        sectionKey: r.section,
                        canonical: r.canonical,
                        hash: f.hash,
                      })),
                    ),
                  });
              })
            }
          >
            <Download size={14} />
            导出所选 Mod
          </button>
          <button
            className="text-button"
            onClick={() => setSelected(new Set())}
          >
            取消选择
          </button>
        </div>
      )}
      <div
        className="resource-scroll"
        ref={scroll}
        onScroll={(e) => {
          scrollPosition.current = e.currentTarget.scrollTop;
        }}
      >
        {!groups.length ? (
          <Empty
            title="还没有找到资源"
            detail="调整筛选条件，导入 Mod，或安装辅助插件提取游戏原图。"
          />
        ) : (
          <div style={{ height: virtual.getTotalSize(), position: "relative" }}>
            {virtual.getVirtualItems().map((v) => (
              <div
                key={v.key}
                className="resource-row"
                style={{
                  height,
                  transform: `translateY(${v.start}px)`,
                  gridTemplateColumns: `repeat(${columns},minmax(0,1fr))`,
                }}
              >
                {groups
                  .slice(v.index * columns, (v.index + 1) * columns)
                  .map((g) => {
                    const r =
                      g.resources.find(
                        (x) =>
                          x.section === "card" || x.section === "monster_card",
                      ) || g.resources[0];
                    const mods = g.resources.reduce(
                      (n, x) => n + x.files.length,
                      0,
                    );
                    const applied = g.resources.some((x) => x.activeHash);
                    return (
                      <article
                        key={g.key}
                        className={`resource-card ${selected.has(g.key) ? "selected" : ""}`}
                      >
                        <button
                          className={`select-mark ${selected.has(g.key) ? "checked" : ""}`}
                          aria-label={`选择 ${r.entityName}`}
                          onClick={() => toggle(g.key)}
                        >
                          {selected.has(g.key) && <Check size={13} />}
                        </button>
                        <button
                          className="resource-open"
                          onClick={() => setDetail(g.key)}
                        >
                          <AssetImage
                            thumbnail
                            path={r.originalUrl || r.files[0]?.path}
                            revision={
                              !r.originalUrl ? r.files[0]?.hash : undefined
                            }
                            label={r.entityName}
                            status={r.originalStatus}
                          />
                          <div className="resource-caption">
                            <div>
                              <strong>{r.entityName}</strong>
                              <span>
                                {route === "heroes" || route === "monsters"
                                  ? `#${r.entityId} · ${g.resources.length} 个部位`
                                  : r.canonical}
                              </span>
                            </div>
                            <ChevronRight size={15} />
                          </div>
                        </button>
                        <div className="resource-footer">
                          <span>{mods ? `${mods} 个 Mod` : "无 Mod"}</span>
                          {applied ? (
                            <Badge tone="success">已应用</Badge>
                          ) : (
                            <span className="muted">
                              {route === "lands" || route === "cursors"
                                ? `${g.resources.length}/2`
                                : "原版"}
                            </span>
                          )}
                        </div>
                      </article>
                    );
                  })}
              </div>
            ))}
          </div>
        )}
      </div>
      {detailGroup && (
        <ResourceDetail
          resources={detailGroup.resources}
          focusKey={focusKey}
          onClose={() => {
            setDetail(null);
            onFocusClear?.();
          }}
        />
      )}
      {notice && (
        <button className="toast" onClick={() => setNotice("")}>
          {notice}
        </button>
      )}
    </div>
  );
}
