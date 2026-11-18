// 维护入口只读取诊断状态；还原预设必须经过用户确认，并复用后端事务。
import { useState } from "react";
import { IPC } from "../core/ipc/commands";
import { useActions, usePreviewStatus, useTasks } from "../hooks/useApp";
import type { WorkspaceSnapshot } from "../core/ipc/generated/WorkspaceSnapshot";
import { Modal, Badge } from "../shared/Ui";
export function MaintenancePanel({
  snapshot,
}: {
  snapshot: WorkspaceSnapshot;
}) {
  const { api, act, busy, notice } = useActions();
  const preview = usePreviewStatus();
  const tasks = useTasks();
  const [nonPng, setNonPng] = useState<any>(null);
  const [restore, setRestore] = useState(false);
  const failures = (tasks.data || []).flatMap((t) => {
    if (!["images-import", "package-commit"].includes(t.kind)) return [];
    if (t.status === "failed")
      return [{ id: t.id, file: t.title, reason: t.message }];
    if (Array.isArray(t.result))
      return t.result
        .filter((r) => !r.ok)
        .map((r, i) => ({ id: t.id + i, file: r.file, reason: r.reason }));
    return [];
  });
  const local = snapshot.resources.filter((r) =>
    ["local", "external"].includes(r.originalStatus),
  ).length;
  const missing = snapshot.resources.filter((r) => !r.originalUrl).length;
  return (
    <section className="settings-card">
      <h2>资源维护与导入记录</h2>
      <p>
        本地原图 {local} · 缺少预览 {missing} ·{" "}
        {preview.data?.message || preview.data?.error || "正在读取预览缓存"}
      </p>
      <div className="action-row">
        <button
          className="button"
          onClick={() =>
            void act(async () =>
              setNonPng(await api.command(IPC.non_png_mods_check)),
            )
          }
        >
          检查非 PNG Mod
        </button>
        <button
          className="button"
          onClick={() => void act(() => api.command(IPC.logs_reveal))}
        >
          打开诊断日志
        </button>
        <button
          className="button"
          onClick={() => void act(() => api.command(IPC.open_modimg))}
        >
          打开游戏 Mod 目录
        </button>
        <button
          className="button"
          onClick={() =>
            void act(async () => {
              await api.command(IPC.preview_check_update);
              await preview.refetch();
            })
          }
        >
          检查预览缓存
        </button>
        <button
          className="button"
          onClick={() => void act(() => api.task("preview", "补齐远端预览"))}
        >
          补齐远端预览
        </button>
        <button
          className="button"
          onClick={() =>
            void act(() =>
              api.task("aux-run", "补齐本地原图", {
                command: "sync",
                category: "all",
              }),
            )
          }
        >
          提取本地原图
        </button>
        <button className="button" onClick={() => setRestore(true)}>
          一键还原全部应用
        </button>
      </div>
      <details className="update-notes">
        <summary>导入失败记录 · {failures.length}</summary>
        {failures.length ? (
          failures.map((f) => (
            <div key={f.id}>
              <strong>{f.file}</strong>
              <p className="danger">{f.reason}</p>
            </div>
          ))
        ) : (
          <p>
            最近任务中没有导入失败记录。识别时拒绝的文件会在导入向导中显示原因。
          </p>
        )}
      </details>
      {nonPng && (
        <details open className="update-notes">
          <summary>非 PNG 检查结果</summary>
          {["active", "stored", "orphaned"].map((key) => (
            <div key={key}>
              <Badge>
                {key === "active"
                  ? "正在应用"
                  : key === "stored"
                    ? "资源库"
                    : "未关联文件"}{" "}
                · {nonPng[key]?.length || 0}
              </Badge>
              {(nonPng[key] || []).map((f: any) => (
                <div className="directory-row" key={f.path}>
                  <div>
                    <strong>{f.sourceName}</strong>
                    <code>{f.path}</code>
                  </div>
                  <button
                    className="button"
                    onClick={() =>
                      void act(() =>
                        api.command(IPC.reveal_non_png_mod, { path: f.path }),
                      )
                    }
                  >
                    定位
                  </button>
                </div>
              ))}
            </div>
          ))}
        </details>
      )}
      {restore && (
        <Modal open title="确认还原全部应用" onClose={() => setRestore(false)}>
          <div className="wizard-body">
            <p>
              将清除当前应用的
              Mod，恢复全部部位原版。资源库中的候选和已保存预设会保留。
            </p>
            <button
              className="button primary"
              disabled={busy}
              onClick={() =>
                void act(async () => {
                  await api.command(IPC.preset_restore_all);
                  setRestore(false);
                })
              }
            >
              确认还原
            </button>
          </div>
        </Modal>
      )}
      {notice && <p role="status">{notice}</p>}
    </section>
  );
}
