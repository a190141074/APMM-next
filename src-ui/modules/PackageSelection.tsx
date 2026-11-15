// 包中的文件按现有资源目录显示，展示字段不参与后端身份或哈希匹配。
import { useEffect, useRef, useState } from "react";
import type { ResourceRecord } from "../core/ipc/generated/ResourceRecord";
import { IPC } from "../core/ipc/commands";
import { useActions } from "../hooks/useApp";
import { AssetImage, Badge } from "../shared/Ui";
import { labels, type Route } from "../core/domain/resources";
import { ResourceSlotSelect } from "../shared/ResourceSlotSelect";
function QueueImage({
  queue,
  id,
  path,
}: {
  queue: string;
  id: string;
  path?: string;
}) {
  const { api } = useActions();
  const [source, setSource] = useState(path);
  const [error, setError] = useState("");
  const container = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (path || !queue || !container.current) return;
    let disposed = false;
    const observer = new IntersectionObserver((entries) => {
      if (!entries.some((e) => e.isIntersecting)) return;
      observer.disconnect();
      void api
        .command(IPC.package_import_queue_preview_image, {
          args: { queueId: queue, itemId: id },
        })
        .then((value) => {
          if (!disposed) setSource(value);
        })
        .catch((e) => {
          if (!disposed) setError(String(e));
        });
    });
    observer.observe(container.current);
    return () => {
      disposed = true;
      observer.disconnect();
    };
  }, [api, queue, id, path]);
  return (
    <div className="package-thumbnail" ref={container} title={error}>
      <AssetImage path={source} label="候选缩略图" />
    </div>
  );
}
export function PackageSelection({
  rows,
  selected,
  onSelect,
  resources,
  queue,
  onTarget,
}: {
  rows: any[];
  selected: Set<string>;
  onSelect: (s: Set<string>) => void;
  resources: ResourceRecord[];
  queue: string;
  onTarget?: (id: string, row: ResourceRecord | null) => void;
}) {
  const known = new Map(resources.map((r) => [r.key, r]));
  const groups = new Map<string, { label: string; items: any[] }>();
  for (const x of rows) {
    const m = x.meta || x;
    const r = known.get(`${m.sectionKey}/${m.canonical}`);
    const label = [
      r ? labels[r.category as Route] : m.categoryName || "其他资源",
      r?.entityName || m.entityName || m.entityId || "无法识别",
      r?.variantLabel || "公共资源",
    ].join(" / ");
    const group = groups.get(label) || { label, items: [] };
    group.items.push({ ...x, resource: r });
    groups.set(label, group);
  }
  function toggle(ids: string[], checked: boolean) {
    const next = new Set(selected);
    for (const id of ids) checked ? next.add(id) : next.delete(id);
    onSelect(next);
  }
  return (
    <div className="package-rows">
      {[...groups.values()].map((group) => {
        const valid = group.items
          .filter((x) => x.status !== "error")
          .map((x) => x.id);
        return (
          <details className="package-group" key={group.label} open>
            <summary>
              <input
                type="checkbox"
                aria-label={`选择 ${group.label}`}
                checked={
                  valid.length > 0 && valid.every((id) => selected.has(id))
                }
                disabled={!valid.length}
                onClick={(e) => e.stopPropagation()}
                onChange={(e) => toggle(valid, e.target.checked)}
              />
              <strong>{group.label}</strong>
              <small>{group.items.length} 个文件</small>
            </summary>
            {group.items.map((x) => {
              const m = x.meta || x;
              const r: ResourceRecord | undefined = x.resource;
              return (
                <div className="package-row" key={x.id}>
                  <input
                    type="checkbox"
                    disabled={x.status === "error"}
                    checked={selected.has(x.id)}
                    onChange={(e) => toggle([x.id], e.target.checked)}
                  />
                  <QueueImage
                    queue={x.path ? "" : queue}
                    id={x.id}
                    path={x.previewUrl || m.previewUrl}
                  />
                  <div>
                    <strong>
                      {r?.resourceLabel || m.sectionName || m.sectionKey} ·{" "}
                      {m.sourceName || m.canonical}
                    </strong>
                    <code>{m.canonical}</code>
                    {x.error && <small className="danger">{x.error}</small>}
                  </div>
                  <Badge>
                    {x.status === "duplicate"
                      ? "相同文件"
                      : x.status === "error"
                        ? "无法导入"
                        : x.conflict
                          ? "同部位新候选"
                          : "可处理"}
                  </Badge>
                  {x.path && x.status !== "error" && onTarget && (
                    <div className="package-classification">
                      <ResourceSlotSelect
                        resources={resources}
                        value={
                          m.sectionKey === "temporary"
                            ? ""
                            : `${m.sectionKey}/${m.canonical}`
                        }
                        onChange={(r) => onTarget(x.id, r)}
                        allowTemporary
                      />
                    </div>
                  )}
                </div>
              );
            })}
          </details>
        );
      })}
    </div>
  );
}
