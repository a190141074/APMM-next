// 下载收据与安装确认沿用更新服务，安装类型决定升级与恢复策略。
import { useState } from "react";
import { useActions, useBackend } from "../hooks/useApp";
import { IPC } from "../core/ipc/commands";
import { Modal } from "../shared/Ui";
import { bytes } from "../core/domain/resources";
import type { WorkspaceSnapshot } from "../core/ipc/generated/WorkspaceSnapshot";
export function UpdateSettings({ snapshot }: { snapshot: WorkspaceSnapshot }) {
  const { api, act, busy, notice } = useActions();
  const [update, setUpdate] = useState<any>(null),
    [confirm, setConfirm] = useState(false);
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
          检查间隔<small>1–720 小时</small>
        </span>
        <input
          aria-label="检查间隔（小时）"
          type="number"
          min="1"
          max="720"
          defaultValue={settings.updateIntervalHours || 24}
          onBlur={(e) =>
            void act(() =>
              api.settings({
                updateIntervalHours: Math.max(
                  1,
                  Math.min(720, Number(e.target.value) || 24),
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
          disabled={busy}
          onClick={() =>
            void act(async () => setUpdate(await api.command(IPC.update_check)))
          }
        >
          检查软件更新
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
      {update?.available && (
        <div className="action-row">
          <button
            className="button primary"
            disabled={busy}
            onClick={() =>
              void act(() =>
                api.task("update-download", "下载软件更新", {
                  version: update.version || update.latestVersion,
                }),
              )
            }
          >
            下载更新
          </button>
          <button
            className="text-button"
            disabled={busy}
            onClick={() =>
              void act(
                () =>
                  api.settings({ updateSkippedVersion: update.latestVersion }),
                "已跳过该版本的自动提醒",
              )
            }
          >
            跳过此版本
          </button>
        </div>
      )}
      {receipt.data && (
        <div className="update-notes">
          <p>
            版本 {receipt.data.version} 已下载并校验，
            {bytes(receipt.data.bytes)}。
          </p>
          <button
            className="button primary"
            disabled={busy}
            onClick={() => setConfirm(true)}
          >
            安装并重启
          </button>
        </div>
      )}
      {confirm && (
        <Modal open title="安装更新" onClose={() => setConfirm(false)}>
          <div className="wizard-body">
            <p>
              安装 {receipt.data?.version} 将等待写入任务结束并重启程序。
              {receipt.data?.kind === "msi"
                ? "Windows Installer 管理安装及安装失败恢复。"
                : "更新前备份程序文件，启动失败时恢复旧版本。"}
            </p>
            <button
              className="button primary"
              disabled={busy}
              onClick={() =>
                void act(async () => {
                  await api.command(IPC.update_apply, {
                    version: receipt.data.version,
                    assetPath: receipt.data.path,
                  });
                  setConfirm(false);
                })
              }
            >
              确认安装并重启
            </button>
          </div>
        </Modal>
      )}
      {notice && <p role="status">{notice}</p>}
    </section>
  );
}
