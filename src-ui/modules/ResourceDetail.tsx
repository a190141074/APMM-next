// 详情页按皮肤或部位浏览；应用状态只读取后台快照。
import { useEffect, useState } from "react";
import {
  ArrowLeft,
  Plus,
  Maximize2,
  MoreHorizontal,
  Check,
} from "lucide-react";
import type { ResourceRecord } from "../core/ipc/generated/ResourceRecord";
import type { ModFile } from "../core/ipc/generated/ModFile";
import { variantGroups, labels, type Route } from "../core/domain/resources";
import { useActions } from "../hooks/useApp";
import { AssetImage, Modal, Badge } from "../shared/Ui";
import { PackageDialog } from "./PackageDialog";
import { PanZoomImage } from "../shared/PanZoomImage";
import { ResourceFileActions } from "./ResourceFileActions";
import { IPC } from "../core/ipc/commands";
export function ResourceDetail({
  resources,
  onClose,
  focusKey,
}: {
  resources: ResourceRecord[];
  onClose: () => void;
  focusKey?: string;
}) {
  const [byPart, setByPart] = useState(false);
  const [zoom, setZoom] = useState<{
    row: ResourceRecord;
    file?: ModFile;
  } | null>(null);
  const [manage, setManage] = useState<{
    row: ResourceRecord;
    file: ModFile;
  } | null>(null);
  const [importing, setImporting] = useState<ResourceRecord | null>(null);
  const [choices, setChoices] = useState<Record<string, string>>({});
  const [pending, setPending] = useState(0);
  const { api, act, notice } = useActions();
  const first = resources[0];
  const paired = ["lands", "cursors"].includes(first.category);
  const groups = variantGroups(resources, byPart);
  const zoomFile = zoom?.file
    ? resources
        .find((r) => r.key === zoom.row.key)
        ?.files.find(
          (f) => f.hash === zoom.file?.hash || f.path === zoom.file?.path,
        ) || zoom.file
    : undefined;
  useEffect(() => {
    if (focusKey)
      document
        .getElementById(`part-${focusKey}`)
        ?.scrollIntoView({ block: "center" });
  }, [focusKey, first.entityId]);
  const apply = async (rows: ResourceRecord[], hash?: string | null) => {
    setPending((n) => n + 1);
    await act(
      () =>
        api.apply(
          rows.map((r) => ({
            entityId: r.entityId,
            section: r.section,
            canonical: r.canonical,
            hash: hash === undefined ? choices[r.key] || r.activeHash : hash,
          })),
        ),
      "应用状态已刷新",
    );
    setPending((n) => n - 1);
  };
  return (
    <section className="detail-page" aria-label={`${first.entityName}资源详情`}>
      <div className="detail-top">
        <button className="button subtle" onClick={onClose}>
          <ArrowLeft size={17} />
          返回列表
        </button>
        <span className="breadcrumb">
          {labels[first.category as Route]} /{" "}
          <strong>{first.entityName}</strong>
        </span>
        <div className="spacer" />
        <div className="segments">
          <button
            className={!byPart ? "active" : ""}
            onClick={() => setByPart(false)}
          >
            按皮肤
          </button>
          <button
            className={byPart ? "active" : ""}
            onClick={() => setByPart(true)}
          >
            按部位
          </button>
        </div>
      </div>
      <div className="detail-scroll">
        <div className="page-heading">
          <div>
            <h1>
              {first.entityName}{" "}
              <span className="heading-count">#{first.entityId}</span>
            </h1>
            <p>
              {resources.length} 个资源 ·{" "}
              {resources.reduce((n, r) => n + r.files.length, 0)} 个
              Mod。点击候选即应用，点击原版还原。
            </p>
          </div>
          <Badge tone={pending ? "" : "success"}>
            {pending ? `${pending} 次切换处理中` : "与资源库同步"}
          </Badge>
        </div>
        {paired && (
          <div className="selectionbar">
            <strong>整组操作 · {resources.length}/2 个部位</strong>
            <button
              className="button primary"
              disabled={
                resources.length !== 2 ||
                resources.some((r) => !choices[r.key] && !r.activeHash)
              }
              onClick={() => void apply(resources)}
            >
              应用整组候选
            </button>
            <button
              className="button"
              onClick={() => void apply(resources, null)}
            >
              还原整组
            </button>
          </div>
        )}
        <nav className="skin-index" aria-label="皮肤定位">
          {groups.map((g) => (
            <button
              className="button subtle"
              key={g.key}
              onClick={() =>
                document
                  .getElementById(`skin-${g.key}`)
                  ?.scrollIntoView({ block: "start" })
              }
            >
              {g.label} <small>{g.rows.length}</small>
            </button>
          ))}
        </nav>
        {groups.map((g) => (
          <section className="skin-group" id={`skin-${g.key}`} key={g.key}>
            <div className="section-heading">
              <h2>{g.label}</h2>
              <span>{g.rows.length} 个部位</span>
            </div>
            {g.rows.map((r) => (
              <article
                id={`part-${r.key}`}
                className={`part-row ${r.key === focusKey ? "search-focus" : ""}`}
                key={r.key}
              >
                <div className="part-identity">
                  <h3>{r.resourceLabel || r.sectionLabel}</h3>
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
                  {r.files.map((f) => (
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
                  <button
                    className="candidate-add"
                    onClick={() => setImporting(r)}
                  >
                    <Plus size={25} />
                    <span>添加候选</span>
                    <small>导入 PNG</small>
                  </button>
                </div>
              </article>
            ))}
          </section>
        ))}
      </div>
      {notice && (
        <div className="detail-notice" role="status">
          {notice}
        </div>
      )}
      {importing && (
        <PackageDialog
          mode="import"
          importKind="images"
          target={{
            heroId: importing.entityId,
            sectionKey: importing.section,
            canonical: importing.canonical,
          }}
          onClose={() => setImporting(null)}
        />
      )}
      {manage && (
        <ResourceFileActions {...manage} onClose={() => setManage(null)} />
      )}
      {zoom && (
        <Modal
          open
          onClose={() => setZoom(null)}
          title={`${zoom.row.entityName} · ${zoom.row.variantLabel} · ${zoom.row.resourceLabel}`}
          wide
        >
          <div className="comparison">
            <div>
              <p>官方原图</p>
              <PanZoomImage
                path={zoom.row.originalUrl}
                status={zoom.row.originalStatus}
                label={zoom.row.canonical}
              />
            </div>
            {zoomFile && (
              <div>
                <p>{zoomFile.sourceName}</p>
                <PanZoomImage
                  path={zoomFile.path}
                  label={zoomFile.sourceName}
                  revision={zoomFile.hash}
                />
              </div>
            )}
          </div>
        </Modal>
      )}
    </section>
  );
}
