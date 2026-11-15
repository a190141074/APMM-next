import { CreatePresetDialog } from "./CreatePresetDialog";
import { ResourceSlotSelect } from "../shared/ResourceSlotSelect";
// 预设页管理固定范围、锁定和问题修复；任何应用必须先查看实际变更清单。
import { useEffect, useMemo, useState } from "react";
import type { WorkspaceSnapshot } from "../core/ipc/generated/WorkspaceSnapshot";
import type { PresetItem } from "../core/ipc/generated/PresetItem";
import type { ResourceRecord } from "../core/ipc/generated/ResourceRecord";
import { useActions } from "../hooks/useApp";
import { usePresetDetail, usePresetService } from "../hooks/useManagement";
import { AssetImage, Modal } from "../shared/Ui";
import { ThemedSelect } from "../shared/ThemedSelect";
import { slotOfResource } from "../core/domain/resources";

export function PresetsPage({ snapshot }: { snapshot: WorkspaceSnapshot }) {
  const { act, busy, notice } = useActions();
  const service = usePresetService();
  const [id, setId] = useState("");
  const detail = usePresetDetail(id);
  const [create, setCreate] = useState(false);
  const [draft, setDraft] = useState<PresetItem[] | null>(null);
  const [editingName, setEditingName] = useState("");
  const [confirm, setConfirm] = useState<"apply" | "delete" | null>(null);
  const [all, setAll] = useState(false);
  const [limit, setLimit] = useState(30);
  const rows = useMemo(
    () => snapshot.resources.filter((r) => r.category !== "temporary"),
    [snapshot.resources],
  );
  const preset = detail.data?.preset;
  useEffect(() => {
    setDraft(null);
    setConfirm(null);
    setAll(false);
    setLimit(30);
  }, [id]);
  const run = async (action: () => Promise<unknown>) => {
    await act(action);
    await detail.refetch();
  };
  function editItem(
    canonical: string,
    hash: string | null,
    row?: ResourceRecord,
  ) {
    const items = draft || preset?.items || [];
    if (hash === null) setDraft(items.filter((i) => i.canonical !== canonical));
    else {
      const old = items.find((i) => i.canonical === canonical);
      if (!old && !row) return;
      const changed: PresetItem = old
        ? { ...old, hash }
        : {
            category: slotOfResource(row!).category,
            heroId: row!.entityId,
            sectionKey: row!.section,
            canonical,
            hash,
            displayName: row!.canonical,
          };
      changed.displayName =
        row?.files.find((f) => f.hash === hash)?.sourceName ||
        changed.displayName;
      setDraft([...items.filter((i) => i.canonical !== canonical), changed]);
    }
  }
  const changes = (detail.data?.changes || []).filter(
    (c) => all || c.changed || c.issues.length,
  );
  return (
    <>
      <div className="page-heading">
        <div>
          <div className="eyebrow">PRESETS</div>
          <h1>预设</h1>
          <p>保存搭配和范围，应用前检查每个变更。</p>
        </div>
        <button
          className="button primary"
          onClick={() => {
            setCreate(true);
          }}
        >
          创建预设
        </button>
      </div>
      <div className="presets-layout">
        <div className="preset-list">
          {snapshot.settings.presets.map((p) => (
            <button
              className={`quick-card ${id === p.id ? "selected" : ""}`}
              key={p.id}
              onClick={() => setId(p.id)}
            >
              <strong>{p.name}</strong>
            </button>
          ))}
          {!snapshot.settings.presets.length && (
            <p>还没有预设。保存当前搭配开始使用。</p>
          )}
        </div>
        <section className="settings-card preset-detail">
          {!id ? (
            <p>选择一个预设查看内容。</p>
          ) : detail.isPending ? (
            <p>正在校验图片与槽位…</p>
          ) : detail.error ? (
            <p role="alert">{String(detail.error)}</p>
          ) : (
            preset && (
              <>
                <h2>
                  {preset.name} {preset.locked ? "· 已锁定" : ""}
                </h2>
                <p>
                  {preset.scope === "global"
                    ? "全局搭配"
                    : `固定局部范围 · ${preset.items.length} 个槽位`}
                </p>
                <div className="action-row">
                  <button
                    className="button primary"
                    disabled={busy || !detail.data?.canApply || !!draft}
                    onClick={() => setConfirm("apply")}
                  >
                    查看应用清单
                  </button>
                  <button
                    className="button"
                    disabled={busy}
                    onClick={() =>
                      void run(async () => {
                        const p = await service.copy(id);
                        setId(p.id);
                      })
                    }
                  >
                    复制
                  </button>
                  <button
                    className="button"
                    disabled={busy}
                    onClick={() =>
                      void run(() => service.lock(id, !preset.locked))
                    }
                  >
                    {preset.locked ? "解锁" : "锁定"}
                  </button>
                  <button
                    className="button"
                    disabled={busy || preset.locked}
                    onClick={() => {
                      setDraft(preset.items);
                      setAll(true);
                      setEditingName(preset.name);
                    }}
                  >
                    编辑 / 修复
                  </button>
                  <button
                    className="text-button danger"
                    disabled={busy || preset.locked}
                    onClick={() => setConfirm("delete")}
                  >
                    删除
                  </button>
                </div>
                {!!detail.data?.issues.length && (
                  <div className="inline-notice" role="alert">
                    {detail.data.issues.length}{" "}
                    个问题阻止整批应用。请编辑并修复。
                    <ul>
                      {detail.data.issues.map((e, i) => (
                        <li key={i}>{e}</li>
                      ))}
                    </ul>
                  </div>
                )}
                {draft && (
                  <div className="field">
                    <label>
                      预设名称
                      <input
                        value={editingName}
                        onChange={(e) => setEditingName(e.target.value)}
                      />
                    </label>
                    <div className="action-row">
                      <button
                        className="button primary"
                        disabled={busy || preset.locked || !editingName.trim()}
                        onClick={() =>
                          void run(async () => {
                            await service.update(id, editingName, draft);
                            setDraft(null);
                          })
                        }
                      >
                        保存编辑
                      </button>
                      <button className="button" onClick={() => setDraft(null)}>
                        取消编辑
                      </button>
                    </div>
                  </div>
                )}
                {draft && preset.scope === "local" && (
                  <div>
                    <h3>添加部位</h3>
                    <ResourceSlotSelect
                      resources={rows}
                      value=""
                      onChange={(row) => {
                        if (row)
                          editItem(
                            row.canonical,
                            row.disabled ? "" : row.activeHash || "",
                            row,
                          );
                      }}
                    />
                  </div>
                )}
                <label className="check">
                  <input
                    type="checkbox"
                    checked={all}
                    onChange={(e) => {
                      setAll(e.target.checked);
                      setLimit(30);
                    }}
                  />
                  显示全部保存范围（{detail.data?.changes.length} 个槽位）
                </label>
                <p>
                  {detail.data?.changes.filter((c) => c.changed).length}{" "}
                  个实际变更
                </p>
                <div className="preset-items">
                  {changes.slice(0, limit).map((c) => {
                    const row = rows.find(
                      (r) => r.canonical === c.item.canonical,
                    );
                    const item =
                      draft?.find((i) => i.canonical === c.item.canonical) ||
                      c.item;
                    const removed =
                      draft &&
                      !draft.some((i) => i.canonical === c.item.canonical);
                    return (
                      <div
                        className={`preset-item ${removed ? "removed" : ""}`}
                        key={c.item.canonical}
                      >
                        <strong>
                          {row
                            ? `${row.entityName} · ${row.variantLabel} · ${row.resourceLabel}`
                            : c.item.displayName || c.item.canonical}
                        </strong>
                        <code>{c.item.canonical}</code>
                        <div className="preset-images">
                          <div>
                            <AssetImage
                              thumbnail
                              revision={c.item.hash}
                              path={
                                c.item.hash ? c.savedPath : row?.originalUrl
                              }
                              label="保存的图片"
                            />
                            <small>
                              保存：{c.item.hash ? c.item.displayName : "原版"}
                            </small>
                          </div>
                          <div>
                            <AssetImage
                              thumbnail
                              revision={c.currentHash}
                              path={
                                c.currentHash ? c.currentPath : row?.originalUrl
                              }
                              label="当前图片"
                            />
                            <small>
                              当前：{c.currentHash ? "Mod" : "原版"}
                            </small>
                          </div>
                        </div>
                        {!!c.issues.length && (
                          <p className="danger">{c.issues.join("；")}</p>
                        )}
                        {draft && (
                          <div className="action-row">
                            <ThemedSelect
                              label={`修复 ${c.item.canonical}`}
                              value={removed ? "__removed" : item.hash}
                              onValueChange={(v) =>
                                editItem(c.item.canonical, v, row)
                              }
                              options={[
                                { value: "", label: "原版" },
                                ...(removed
                                  ? [{ value: "__removed", label: "已移除" }]
                                  : []),
                                ...(item.hash &&
                                !row?.files.some((f) => f.hash === item.hash)
                                  ? [
                                      {
                                        value: item.hash,
                                        label: `缺失：${item.displayName}`,
                                      },
                                    ]
                                  : []),
                                ...(row?.files || []).map((f) => ({
                                  value: f.hash,
                                  label: f.sourceName,
                                })),
                              ]}
                            />
                            <button
                              className="text-button danger"
                              onClick={() => editItem(c.item.canonical, null)}
                            >
                              移除项
                              {preset.scope === "local" ? "并缩小范围" : ""}
                            </button>
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
                {changes.length > limit && (
                  <button
                    className="button"
                    onClick={() => setLimit((n) => n + 30)}
                  >
                    显示更多
                  </button>
                )}
              </>
            )
          )}
        </section>
      </div>
      {notice && <p role="status">{notice}</p>}
      {create && (
        <CreatePresetDialog
          resources={rows}
          onClose={() => setCreate(false)}
          onSaved={(id) => {
            setId(id);
            setCreate(false);
          }}
        />
      )}
      {confirm && preset && (
        <Modal
          open
          wide
          onClose={() => !busy && setConfirm(null)}
          title={confirm === "delete" ? "删除预设" : "应用变更清单"}
        >
          <div className="wizard-body">
            {confirm === "delete" ? (
              <p>删除“{preset.name}”？</p>
            ) : (
              <>
                <p>
                  {preset.scope === "local"
                    ? "只修改固定范围，范围外保持现状。"
                    : "恢复全局搭配。"}
                  提交时重新校验全部文件。
                </p>
                {detail.data?.changes
                  .filter((c) => c.changed)
                  .map((c) => (
                    <div className="setting-row" key={c.item.canonical}>
                      <code>{c.item.canonical}</code>
                      <span>
                        {c.currentHash ? "当前 Mod" : "原版"} →{" "}
                        {c.item.hash ? c.item.displayName : "原版"}
                      </span>
                    </div>
                  ))}
              </>
            )}
            <button
              className={`button ${confirm === "delete" ? "danger" : "primary"}`}
              disabled={busy}
              onClick={() =>
                void run(async () => {
                  if (confirm === "delete") {
                    await service.remove(id);
                    setId("");
                  } else await service.apply(id);
                  setConfirm(null);
                })
              }
            >
              {confirm === "delete" ? "确认删除" : "确认整批应用"}
            </button>
            {notice && <p role="status">{notice}</p>}
          </div>
        </Modal>
      )}
    </>
  );
}
