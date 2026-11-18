import { ResourceDetail } from "./ResourceDetail";
import { ResourceList } from "./ResourceList";
import { ResourceGroupCard } from "./ResourceGroupCard";
import {
  resourceSizes,
  resourceSizeStyle,
  readResourceSize,
  type ResourceSize,
} from "../core/domain/resourcePresentation";
import { useResourceLayout } from "../hooks/useResourceLayout";
import { useState, useMemo, useRef, useEffect } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  LayoutGrid,
  List,
  Undo2,
  Download,
  FolderOpen,
  CheckSquare,
} from "lucide-react";
import type { ResourceRecord } from "../core/ipc/generated/ResourceRecord";
import {
  filterResources,
  filterResourceGroups,
  groupResources,
  labels,
  type Route,
  type Filter,
  type SearchMode,
  resourceGroupKey,
  storageCategory,
} from "../core/domain/resources";
import { useActions } from "../hooks/useApp";
import { Empty } from "../shared/Ui";
import { ResourceSizeControls } from "./ResourceSizeControls";
import { ThemedSelect } from "../shared/ThemedSelect";
const memory = new Map<
  string,
  {
    filter: Filter;
    section: string;
    size: ResourceSize;
    scroll: number;
    selected: string[];
  }
>();
export function ResourcePage({
  route,
  search,
  resources,
  searchMode = "smart",
  onSearchClear,
  focusKey,
  onFocusClear,
  resetToken,
}: {
  route: Route;
  search: string;
  resources: ResourceRecord[];
  searchMode?: SearchMode;
  onSearchClear?: () => void;
  focusKey?: string;
  onFocusClear?: () => void;
  resetToken?: number;
}) {
  const initial = memory.get(route);
  const [filter, setFilter] = useState<Filter>(initial?.filter || "all");
  const [section, setSection] = useState(initial?.section || "");
  const [unified, setUnified] = useState(
    () => localStorage.getItem("apmm.resource-size-unified") !== "false",
  );
  const [size, setSize] = useState<ResourceSize>(() =>
    readResourceSize(localStorage, route),
  );
  const [selected, setSelected] = useState<Set<string>>(
    new Set(initial?.selected),
  );
  const [detail, setDetail] = useState<string | null>(null);
  const { api, act, notice, setNotice } = useActions();
  const scroll = useRef<HTMLDivElement>(null);
  const {
    layout,
    change: changeLayout,
    rememberScroll,
  } = useResourceLayout(scroll, route, initial?.scroll || 0);
  const scrollPosition = useRef(initial?.scroll || 0);
  const [width, setWidth] = useState(900);
  const filtered = useMemo(
    () =>
      filterResources(resources, route, search, filter, section, searchMode),
    [resources, route, search, filter, section, searchMode],
  );
  const groups = useMemo(
    () =>
      filterResourceGroups(
        resources,
        route,
        search,
        filter,
        section,
        searchMode,
      ),
    [resources, route, search, filter, section, searchMode],
  );
  const compact = layout === "compact";
  const columns = Math.max(
    1,
    Math.floor(
      width /
        (compact
          ? resourceSizes[size].heroCardWidth
          : resourceSizes[size].cardWidth),
    ),
  );
  const height = compact
    ? resourceSizes[size].heroRowHeight
    : resourceSizes[size].cardHeight;
  const virtual = useVirtualizer({
    count: layout !== "list" ? Math.ceil(groups.length / columns) : 0,
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
  // 侧栏重复点击当前资源分类时，仅刷新列表层，不保留上一次打开的详情。
  const previousReset = useRef(resetToken);
  useEffect(() => {
    if (resetToken === previousReset.current) return;
    previousReset.current = resetToken;
    setDetail(null);
    scroll.current?.scrollTo({ top: 0 });
    onFocusClear?.();
  }, [resetToken]);
  const toggle = (key: string) =>
    setSelected((s) => {
      const next = new Set(s);
      next.has(key) ? next.delete(key) : next.add(key);
      return next;
    });
  const changeSize = (next: ResourceSize) => {
    setSize(next);
    localStorage.setItem(
      unified ? "apmm.resource-size-global" : "apmm.resource-size." + route,
      next,
    );
  };
  const changeUnified = (next: boolean) => {
    setUnified(next);
    localStorage.setItem("apmm.resource-size-unified", String(next));
    if (next) localStorage.setItem("apmm.resource-size-global", size);
    else setSize(readResourceSize(localStorage, route));
  };
  const selectedRows = groups
    .filter((g) => selected.has(g.key))
    .flatMap((g) => g.resources);
  return (
    <div className={`resource-page ${detailGroup ? "showing-detail" : ""}`}>
      <div className="resource-toolbar">
        <div className="resource-heading">
          <h1>
            {labels[route]}
            <span className="heading-count">
              {layout === "list" ? filtered.length : groups.length}
            </span>
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
          <div
            className="resource-view-mode"
            role="group"
            aria-label="资源布局"
          >
            <button
              title={route === "heroes" ? "紧凑模式" : "列表模式"}
              aria-label={route === "heroes" ? "紧凑模式" : "列表模式"}
              aria-pressed={
                layout === (route === "heroes" ? "compact" : "list")
              }
              onClick={() =>
                changeLayout(route === "heroes" ? "compact" : "list")
              }
            >
              <List size={16} />
            </button>
            <button
              title="卡片模式"
              aria-label="卡片模式"
              aria-pressed={layout === "grid"}
              onClick={() => changeLayout("grid")}
            >
              <LayoutGrid size={16} />
            </button>
          </div>
          <ResourceSizeControls
            size={size}
            unified={unified}
            onSizeChange={changeSize}
            onUnifiedChange={changeUnified}
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
        data-layout={layout}
        style={resourceSizeStyle(size)}
        ref={scroll}
        onScroll={(e) => {
          scrollPosition.current = e.currentTarget.scrollTop;
          rememberScroll(e.currentTarget.scrollTop);
        }}
      >
        {!groups.length ? (
          <Empty
            title="还没有找到资源"
            detail="调整筛选条件，导入 Mod，或安装辅助插件提取游戏原图。"
          />
        ) : layout === "list" ? (
          <ResourceList
            rows={filtered}
            scrollRef={scroll}
            size={size}
            width={width}
            selected={selected}
            onSelect={toggle}
            onMore={setDetail}
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
                  .map((g) => (
                    <ResourceGroupCard
                      key={g.key}
                      group={g}
                      route={route}
                      compact={compact}
                      selected={selected.has(g.key)}
                      onSelect={() => toggle(g.key)}
                      onOpen={() => setDetail(g.key)}
                    />
                  ))}
              </div>
            ))}
          </div>
        )}
      </div>
      {detailGroup && (
        <ResourceDetail
          resources={detailGroup.resources}
          filter={filter}
          section={section}
          search={search}
          searchMode={searchMode}
          size={size}
          unified={unified}
          onSizeChange={changeSize}
          onUnifiedChange={changeUnified}
          onClearFilters={() => {
            setFilter("all");
            setSection("");
            onSearchClear?.();
          }}
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
