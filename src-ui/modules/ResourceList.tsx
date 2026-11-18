// 旧版式列表按部位直接展开；虚拟化仅裁剪界面，候选身份和应用继续使用原始记录。
import { useMemo, useState, useLayoutEffect, type RefObject } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ChevronDown, ChevronRight } from "lucide-react";
import type { ResourceRecord } from "../core/ipc/generated/ResourceRecord";
import { resourceGroupKey } from "../core/domain/resources";
import {
  resourceSizes,
  type ResourceSize,
} from "../core/domain/resourcePresentation";
import { useResourceActions } from "../hooks/useResourceActions";
import { ResourceCandidateRow } from "./ResourceCandidateRow";
type Entry =
  | {
      key: string;
      kind: "heading";
      label: string;
      count: number;
      section: string;
    }
  | { key: string; kind: "resource"; row: ResourceRecord };
export function ResourceList({
  rows,
  scrollRef,
  size,
  width,
  selected,
  onSelect,
  onMore,
}: {
  rows: ResourceRecord[];
  scrollRef: RefObject<HTMLDivElement>;
  size: ResourceSize;
  width: number;
  selected: Set<string>;
  onSelect: (key: string) => void;
  onMore: (key: string) => void;
}) {
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const actions = useResourceActions(rows);
  const entries = useMemo(() => {
    const groups = new Map<string, ResourceRecord[]>();
    for (const row of rows) {
      const group = groups.get(row.section) || [];
      group.push(row);
      groups.set(row.section, group);
    }
    return [...groups].flatMap(([section, items]): Entry[] => [
      {
        kind: "heading",
        key: `section:${section}`,
        label: items[0].sectionLabel,
        count: items.length,
        section,
      },
      ...(collapsed.has(section)
        ? []
        : items.map((row) => ({
            kind: "resource" as const,
            key: row.key,
            row,
          }))),
    ]);
  }, [rows, collapsed]);
  const virtual = useVirtualizer({
    count: entries.length,
    getScrollElement: () => scrollRef.current,
    getItemKey: (index) => entries[index].key,
    estimateSize: (index) =>
      entries[index].kind === "heading" ? 46 : resourceSizes[size].thumb + 32,
    overscan: 3,
  });
  useLayoutEffect(() => {
    virtual.measure();
    // 等滚动容器挂载并完成换行后重测，避免旧估算高度遮住下一行按钮。
    const frame = requestAnimationFrame(() => {
      scrollRef.current
        ?.querySelectorAll<HTMLElement>(".resource-list-position")
        .forEach((element) => virtual.measureElement(element));
    });
    return () => cancelAnimationFrame(frame);
  }, [size, width, virtual, scrollRef]);
  return (
    <>
      <div
        className={`resource-list resource-list-${size}`}
        style={{ height: virtual.getTotalSize(), position: "relative" }}
      >
        {virtual.getVirtualItems().map((item) => {
          const entry = entries[item.index];
          return (
            <div
              key={item.key}
              data-index={item.index}
              ref={virtual.measureElement}
              className="resource-list-position"
              style={{ transform: `translateY(${item.start}px)` }}
            >
              {entry.kind === "heading" ? (
                <button
                  className="resource-list-heading"
                  aria-expanded={!collapsed.has(entry.section)}
                  onClick={() =>
                    setCollapsed((current) => {
                      const next = new Set(current);
                      next.has(entry.section)
                        ? next.delete(entry.section)
                        : next.add(entry.section);
                      return next;
                    })
                  }
                >
                  {collapsed.has(entry.section) ? (
                    <ChevronRight size={15} />
                  ) : (
                    <ChevronDown size={15} />
                  )}
                  <strong>{entry.label}</strong>
                  <span>{entry.count} 个资源</span>
                </button>
              ) : (
                <ResourceCandidateRow
                  row={entry.row}
                  actions={actions}
                  compact
                  selected={selected.has(resourceGroupKey(entry.row))}
                  onSelect={() => onSelect(resourceGroupKey(entry.row))}
                  onMore={() => onMore(resourceGroupKey(entry.row))}
                />
              )}
            </div>
          );
        })}
      </div>
      {actions.notice && (
        <button
          className="toast"
          role="status"
          onClick={() => actions.setNotice("")}
        >
          {actions.notice}
        </button>
      )}
      {actions.dialogs}
    </>
  );
}
