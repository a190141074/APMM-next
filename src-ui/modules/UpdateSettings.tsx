// 一次明确点击完成下载、校验和安装交接，实际安装仍使用既有更新服务。
import { useState } from "react";
import { useActions, useBackend } from "../hooks/useApp";
import { IPC } from "../core/ipc/commands";
import { installSoftwareUpdate } from "../core/services/softwareUpdate";
import { bytes } from "../core/domain/resources";
import type { WorkspaceSnapshot } from "../core/ipc/generated/WorkspaceSnapshot";
export function UpdateSettings({ snapshot }: { snapshot: WorkspaceSnapshot }) {
  const { api, act, busy, notice } = useActions();
  const [phase, setPhase] = useState<"downloading" | "installing" | null>(null);
  const updateQuery = useBackend(IPC.update_check);
  const update = updateQuery.data;
  const receipt = useBackend(IPC.update_download_status),
    settings = snapshot.settings;
  return (
    <section className="settings-card">
      <h2>软件更新</h2>
      <label className="settings-row">
        <span>
          自动检查更新<small>发现新版本时在工作区提醒。</small>
        </span>
        <input
          type="checkbox"
          checked={settings.updateAutoCheck !== false}
          onChange={(e) =>
            void act(() => api.settings({ updateAutoCheck: e.target.checked }))
          }
        />
      </label>
      <label className="settings-row">
        <span>
          检查间隔<small>1–43200 分钟</small>
        </span>
        <input
          aria-label="检查间隔（分钟）"
          type="number"
          min="1"
          max="43200"
          defaultValue={settings.updateIntervalMinutes || 60}
          onBlur={(e) =>
            void act(() =>
              api.settings({
                updateIntervalMinutes: Math.max(
                  1,
                  Math.min(43200, Math.round(Number(e.target.value)) || 60),
                ),
              }),
            )
          }
        />
      </label>
      <div className="settings-row">
        <span>
          当前版本<small>v{snapshot.version}</small>
        </span>
        <button
          className="button"
          disabled={busy || updateQuery.isFetching}
          onClick={() =>
            void act(async () => {
              const result = await updateQuery.refetch();
              if (result.error) throw result.error;
            }, "更新检查完成")
          }
        >
          {updateQuery.isFetching ? "正在检查更新…" : "检查软件更新"}
        </button>
      </div>
      {update && (
        <div className="update-notes" role="status">
          <strong>
            {update.available
              ? `可更新到 ${update.latestVersion}`
              : "当前已是最新版本"}
          </strong>
          <p>{update.notes || update.message}</p>
        </div>
      )}
      {updateQuery.error && <p role="alert">{String(updateQuery.error)}</p>}
      {(update?.available || receipt.data) && (
        <div className="action-row">
          <button
            className="button primary"
            disabled={busy || updateQuery.isFetching}
            onClick={() =>
              void act(async () => {
                try {
                  await installSoftwareUpdate(
                    update?.available
                      ? update.latestVersion
                      : receipt.data.version,
                    setPhase,
                  );
                } finally {
                  setPhase(null);
                  void receipt.refetch();
                }
              }, "更新安装已交接")
            }
          >
            {phase === "downloading"
              ? "正在下载并校验…"
              : phase === "installing"
                ? "正在准备安装…"
                : "安装更新并重启"}
          </button>
          {update?.available && (
            <button
              className="text-button"
              disabled={busy}
              onClick={() =>
                void act(
                  () =>
                    api.settings({
                      updateSkippedVersion: update.latestVersion,
                    }),
                  "已跳过该版本的自动提醒",
                )
              }
            >
              跳过此版本
            </button>
          )}
        </div>
      )}
      {(update?.available || receipt.data) && (
        <p className="muted">
          点击后自动下载、校验并安装，完成后重启程序。下载进度可在任务列表查看。
        </p>
      )}
      {receipt.data && (
        <div className="update-notes">
          <p>
            版本 {receipt.data.version} 已下载并校验，
            {bytes(receipt.data.bytes)}。
          </p>
        </div>
      )}
      {notice && <p role="status">{notice}</p>}
    </section>
  );
}
