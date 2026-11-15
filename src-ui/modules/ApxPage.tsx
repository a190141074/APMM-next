// APX 页面展示渠道、独立目录和文件校验，安装与导出仍走原有事务。
import { useState } from "react";
import { Download, RefreshCw } from "lucide-react";
import {
  useActions,
  useBackend,
  useTasks,
  useWorkspace,
} from "../hooks/useApp";
import { IPC } from "../core/ipc/commands";
import { bytes } from "../core/domain/resources";
import { apxFileState, apxInstallState } from "../core/domain/apx";
import { activeTask } from "../core/domain/tasks";
import { Badge, Modal } from "../shared/Ui";
import { ApxDirectories } from "./ApxDirectories";
import { SegmentedTabs } from "../shared/SegmentedTabs";
const channels = [
  ["CN_STEAM", "Steam 国服"],
  ["CN_TAPTAP", "TapTap"],
  ["CN_BILIBILI", "哔哩哔哩"],
  ["CN_WEGAME", "WeGame"],
  ["INT_STEAM", "Steam 国际服"],
  ["CN_ANDROID", "Android"],
];
export function ApxPage() {
  const workspace = useWorkspace();
  const { api, act, busy, notice } = useActions();
  const [platform, setPlatform] = useState(
    workspace.data?.settings.pluginPlatform || "CN_STEAM",
  );
  const q = useBackend(IPC.plugin_status, { platform });
  const tasks = useTasks();
  const data = q.data;
  const [confirm, setConfirm] = useState<
    "install" | "restore" | "export" | null
  >(null);
  const restricted = platform === "INT_STEAM" || platform === "CN_ANDROID";
  const running = (tasks.data || []).filter(
    (t) => t.kind.startsWith("apx-") && activeTask(t),
  );
  const waiting = busy || running.length > 0;
  const run = (kind: string, args: Record<string, unknown> = {}) =>
    act(
      () =>
        api.task(
          `apx-${kind}`,
          `${kind === "export" ? "导出" : kind === "cache" ? "下载缓存" : kind === "install" ? "安装" : "还原"} APX`,
          { platform, ...args },
        ),
      "任务已开始，完成前可在此查看进度",
    );
  const exportCache = async (useCached: boolean) => {
    const destination = await api.saveFile(`APX-${platform}.zip`);
    if (destination) {
      await run("export", { destination, useCached });
      setConfirm(null);
    }
  };
  return (
    <div className="apx-page">
      <div className="page-heading">
        <div>
          <div className="eyebrow">APX PLUGIN</div>
          <h1>APX 客户端补丁</h1>
          <p>选择渠道，检查适配版本后安装。安装前自动备份。</p>
        </div>
        <div className="action-row">
          <button
            className="button"
            disabled={waiting || platform === "INT_STEAM"}
            onClick={() =>
              void act(async () => {
                await api.command(IPC.plugin_check_update, { platform });
                await q.refetch();
              })
            }
          >
            <RefreshCw size={16} />
            检查更新
          </button>
          <button
            className="button primary"
            disabled={restricted || waiting || !data?.remoteVersion}
            onClick={() => setConfirm("install")}
          >
            <Download size={16} />
            安装补丁
          </button>
          <button
            className="button"
            disabled={restricted || waiting}
            onClick={() => setConfirm("restore")}
          >
            还原补丁
          </button>
          <button
            className="button"
            disabled={waiting || platform === "INT_STEAM"}
            onClick={() => setConfirm("export")}
          >
            导出补丁
          </button>
        </div>
      </div>
      <SegmentedTabs
        label="选择客户端"
        value={platform}
        disabled={waiting}
        options={channels.map(([value, label]) => ({
          value,
          label,
        }))}
        onChange={(key) => {
          setPlatform(key);
          void act(() => api.settings({ pluginPlatform: key }), "渠道已切换");
        }}
      />
      <div className="apx-status" role="status">
        {q.error
          ? String(q.error)
          : data?.error ||
            data?.directoryError ||
            data?.warning ||
            (platform === "CN_ANDROID"
              ? "Android 仅支持导出补丁，请在设备上安装。"
              : platform === "INT_STEAM"
                ? "国际服不支持 APX 补丁。"
                : data?.versionMatch === false
                  ? "本地游戏版本与补丁适配版本不一致，请核对后安装。"
                  : q.isPending
                    ? "正在检查本地文件…"
                    : "补丁安装状态来自文件大小与 SHA-256 校验。")}
      </div>
      <div className="stats-grid apx-stats">
        {[
          [
            "本地游戏版本",
            data?.localGameVersion || "未检测",
            data?.localVersionSource || "选择下方游戏目录",
          ],
          [
            "远端适配版本",
            data?.remoteGameVersion || "待检查",
            `补丁版本 ${data?.remoteVersion || "未知"}`,
          ],
          [
            "本地补丁状态",
            data ? apxInstallState(data) : "待检查",
            `本地版本 ${data?.localPluginVersion || "未安装"}`,
          ],
          [
            "已安装 / 总文件数",
            `${data?.installedFiles || 0} / ${data?.totalFiles || 0}`,
            bytes(data?.totalBytes || 0),
          ],
        ].map(([title, value, detail]) => (
          <div className="stat" key={title}>
            <small>{title}</small>
            <strong title={value}>{value}</strong>
            <small>{detail}</small>
          </div>
        ))}
      </div>
      {running.map((t) => (
        <div className="settings-card" key={t.id}>
          <div className="section-heading">
            <strong>{t.title}</strong>
            <button
              className="button"
              disabled={!t.canCancel}
              onClick={() => void act(() => api.cancel(t.id), "正在取消任务")}
            >
              取消
            </button>
          </div>
          <p>{t.message}</p>
          <progress
            max={t.total || undefined}
            value={t.total ? t.done : undefined}
          />
          <small>
            {t.total ? `${bytes(t.done)} / ${bytes(t.total)}` : "准备中"}
            {t.speed ? ` · ${bytes(t.speed)}/s` : ""}
          </small>
        </div>
      ))}
      <ApxDirectories
        key={platform}
        platform={platform}
        platformLabel={
          channels.find(([key]) => key === platform)?.[1] || platform
        }
        data={data}
        waiting={waiting}
        onRefresh={() => q.refetch()}
      />
      <details className="settings-card">
        <summary>文件清单与校验 · {data?.files?.length || 0} 项</summary>
        {(data?.files || []).map((f: any) => (
          <div className="apx-file" key={f.relativePath}>
            <div>
              <strong>{f.description || f.relativePath}</strong>
              <code>{f.target || f.relativePath}</code>
              <small>
                {bytes(f.actualSize || 0)} / {bytes(f.expectedSize)}
                {f.validationError && ` · ${f.validationError}`}
              </small>
            </div>
            <Badge tone={f.installed ? "success" : ""}>{apxFileState(f)}</Badge>
            <button
              className="button"
              disabled={!f.present}
              onClick={() =>
                void act(() =>
                  api.command(IPC.plugin_reveal, { path: f.target }),
                )
              }
            >
              定位
            </button>
          </div>
        ))}
      </details>
      {confirm && (
        <Modal
          open
          title={confirm === "export" ? "导出 APX 补丁" : "确认修改补丁"}
          onClose={() => setConfirm(null)}
        >
          <div className="wizard-body">
            {confirm === "export" ? (
              <>
                <p>
                  导出仅生成补丁包。可使用已校验的旧缓存，或下载最新版本后导出。
                </p>
                <button
                  className="button"
                  disabled={!data?.cacheComplete || waiting}
                  onClick={() => void exportCache(true)}
                >
                  导出当前缓存 {data?.cacheVersion}
                </button>
                <button
                  className="button primary"
                  disabled={waiting}
                  onClick={() => void exportCache(false)}
                >
                  下载最新缓存后导出
                </button>
              </>
            ) : (
              <>
                <p>
                  本地游戏：{data?.localGameVersion || "未知"}；适配版本：
                  {data?.remoteGameVersion || "未知"}；补丁版本：
                  {data?.remoteVersion || "未知"}。
                </p>
                {data?.versionMatch === false && (
                  <p className="danger">版本不匹配，补丁可能无法正常运行。</p>
                )}
                <p>
                  将{confirm === "install" ? "安装" : "还原"}
                  所选渠道的文件。落盘前备份，失败时恢复。
                </p>
                <button
                  className="button primary"
                  disabled={waiting}
                  onClick={() =>
                    void act(async () => {
                      await api.task(
                        `apx-${confirm}`,
                        `${confirm === "install" ? "安装" : "还原"} APX`,
                        { platform, force: data?.versionMatch === false },
                      );
                      setConfirm(null);
                    })
                  }
                >
                  确认{confirm === "install" ? "安装" : "还原"}
                </button>
              </>
            )}
          </div>
        </Modal>
      )}
      {notice && <div className="toast">{notice}</div>}
    </div>
  );
}
