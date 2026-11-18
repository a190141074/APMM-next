// 原版、候选、比较和管理按钮共用同一布局，列表模式仅控制标题、选择和候选数量。
import {
  Plus,
  Check,
  FolderOpen,
  Maximize2,
  MoreHorizontal,
} from "lucide-react";
import type { ResourceRecord } from "../core/ipc/generated/ResourceRecord";
import type { useResourceActions } from "../hooks/useResourceActions";
import { AssetImage, Badge } from "../shared/Ui";
import { IPC } from "../core/ipc/commands";
import { resourceGroupKey } from "../core/domain/resources";
export function ResourceCandidateRow({
  row: r,
  actions,
  byPart = false,
  focusKey,
  compact = false,
  selected,
  onSelect,
  onMore,
}: {
  row: ResourceRecord;
  actions: ReturnType<typeof useResourceActions>;
  byPart?: boolean;
  focusKey?: string;
  compact?: boolean;
  selected?: boolean;
  onSelect?: () => void;
  onMore?: () => void;
}) {
  const { api, act, apply, setChoices, setZoom, setManage, setImporting } =
    actions;
  const { outimgPath } = r;
  return (
    <article
      id={`${compact ? "list-part" : "part"}-${r.key}`}
      data-resource-view-key={resourceGroupKey(r)}
      data-drop-resource-key={
        r.applicationSlot && r.section !== "temporary" ? r.key : undefined
      }
      className={`part-row ${compact ? "compact-part" : ""} ${selected ? "selected" : ""} ${r.key === focusKey ? "search-focus" : ""}`}
      key={r.key}
    >
      <div className="part-identity">
        {onSelect && (
          <button
            className={`list-select ${selected ? "checked" : ""}`}
            aria-label={`选择 ${r.entityName}`}
            aria-pressed={!!selected}
            onClick={onSelect}
          >
            {selected && <Check size={12} />}
          </button>
        )}
        <h3>{compact ? r.entityName : r.resourceLabel || r.sectionLabel}</h3>
        {byPart && <Badge>{r.variantLabel}</Badge>}
        <code>{r.fullName}</code>
        <small>
          {r.files.length} 个候选 {r.disabled ? "· 已禁用" : ""}
        </small>
        {!r.originalUrl && (
          <button
            className="text-button"
            onClick={() =>
              void act(() =>
                api.task("aux-run", "补齐本地原图", {
                  command: "sync",
                  category: "all",
                }),
              )
            }
          >
            补齐本地原图
          </button>
        )}
        {r.disabled && (
          <button
            className="text-button"
            onClick={() =>
              void act(() =>
                api.command(IPC.mod_set_disabled, {
                  args: {
                    heroId: r.entityId,
                    sectionKey: r.section,
                    canonical: r.canonical,
                  },
                  disabled: false,
                }),
              )
            }
          >
            启用资源
          </button>
        )}
      </div>
      <div className="candidate-strip">
        <div
          className={`candidate ${!r.activeHash || r.disabled ? "applied" : ""}`}
        >
          <button
            className="candidate-apply"
            aria-label={`还原 ${r.canonical}`}
            title={`原版 · ${!r.activeHash || r.disabled ? "当前应用" : "点击还原"}`}
            disabled={r.category === "temporary"}
            onClick={() => void apply([r], null)}
          >
            <AssetImage
              thumbnail
              path={r.originalUrl}
              label={`${r.resourceLabel}原版`}
              status={r.originalStatus}
            />
            <strong>原版</strong>
            <small>
              {!r.activeHash || r.disabled ? (
                <>
                  <Check size={12} />
                  当前应用
                </>
              ) : (
                "点击还原"
              )}
            </small>
          </button>
          <button
            className="candidate-tool"
            title={`放大原版 ${r.canonical}`}
            onClick={() => setZoom({ row: r })}
          >
            <Maximize2 size={14} />
          </button>
        </div>
        {r.files.slice(0, compact ? 3 : undefined).map((f) => (
          <div
            className={`candidate ${!r.disabled && r.activeHash === f.hash ? "applied" : ""}`}
            key={f.hash}
          >
            <button
              className="candidate-apply"
              title={`${f.sourceName}\n${f.hash}`}
              aria-label={`应用 ${f.sourceName} ${f.hash.slice(0, 8)}`}
              disabled={r.disabled || r.category === "temporary"}
              onClick={() => {
                setChoices((c) => ({ ...c, [r.key]: f.hash }));
                void apply([r], f.hash);
              }}
            >
              <AssetImage
                thumbnail
                path={f.path}
                revision={f.hash}
                label={f.sourceName}
              />
              <strong>{f.sourceName}</strong>
              <small>
                {!r.disabled && r.activeHash === f.hash ? (
                  <>
                    <Check size={12} />
                    当前应用
                  </>
                ) : (
                  `候选 · ${f.hash.slice(0, 8)}`
                )}
              </small>
            </button>
            <div className="candidate-tools">
              <button
                title={`比较 ${f.sourceName}`}
                onClick={() => setZoom({ row: r, file: f })}
              >
                <Maximize2 size={14} />
              </button>
              <button
                title={`管理 ${f.sourceName}`}
                onClick={() => setManage({ row: r, file: f })}
              >
                <MoreHorizontal size={16} />
              </button>
            </div>
          </div>
        ))}
        <button className="candidate-add" onClick={() => setImporting(r)}>
          <Plus size={25} />
          <span>添加候选</span>
          <small>导入 PNG</small>
        </button>
        {onMore && (
          <button
            className="candidate-more"
            onClick={onMore}
            aria-label={`更多 ${r.canonical}`}
          >
            <MoreHorizontal size={20} />
            <span>更多</span>
            {r.files.length > 3 && <small>共 {r.files.length} 个候选</small>}
          </button>
        )}
      </div>
      {outimgPath && (
        <button
          type="button"
          className="candidate-reveal"
          aria-label={"定位原图文件 " + r.canonical}
          title="在文件资源管理器中定位原图"
          onClick={() =>
            void act(() =>
              api.command(IPC.outimg_reveal, { path: outimgPath }),
            )
          }
        >
          <FolderOpen size={16} />
          <span>定位</span>
        </button>
      )}
    </article>
  );
}
