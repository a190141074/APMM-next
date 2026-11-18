import { IPC } from "../core/ipc/commands";
import { ThemedSelect } from "../shared/ThemedSelect";
import { useState } from "react";
import {
  Puzzle,
  Download,
  RefreshCw,
  FolderOpen,
  Play,
  Trash2,
  ShieldCheck,
  Box,
  ArrowUpRight,
} from "lucide-react";
import { useActions, useBackend, useWorkspace } from "../hooks/useApp";
import type { Route } from "../core/domain/resources";
import { labels, bytes } from "../core/domain/resources";
import { Empty, Badge, Modal, AssetImage } from "../shared/Ui";
export function PluginsPage({ route }: { route: Route }) {
  const { api, act, busy, notice } = useActions();
  const workspace = useWorkspace();
  const [platform, setPlatform] = useState(
    workspace.data?.settings.pluginPlatform || "CN_STEAM",
  );
  const [category, setCategory] = useState("all");
  const [release, setRelease] = useState<any>(null);
  const [confirmation, setConfirmation] = useState<(() => Promise<any>) | null>(
    null,
  );
  const q = useBackend(
    route === "apx"
      ? IPC.plugin_status
      : route === "auxiliary"
        ? IPC.auxiliary_status
        : IPC.workshop_list,
    route === "apx" ? { platform } : {},
  );
  const data = q.data;
  const run = (
    kind: string,
    title: string,
    args: Record<string, unknown> = {},
  ) => act(() => api.task(kind, title, args), "任务已加入后台队列");
  return (
    <>
      <div className="page-heading">
        <div>
          <div className="eyebrow">PLUGIN LIBRARY</div>
          <h1>{labels[route]}</h1>
          <p>
            {route === "auxiliary"
              ? "按需扩展工作区。安装你需要的工具。"
              : route === "apx"
                ? "检查客户端补丁，安装前自动备份。"
                : "发现社区作品，收藏喜欢的每一种风格。"}
          </p>
        </div>
        <button className="button subtle" onClick={() => void q.refetch()}>
          <RefreshCw size={16} />
          刷新
        </button>
      </div>
      {route === "auxiliary" ? (
        <>
          <div className="plugin-feature">
            <div className="plugin-logo">
              <Puzzle size={42} />
            </div>
            <div className="plugin-description">
              <div className="section-heading">
                <h2>APMM 资产工具</h2>
                <Badge tone={data?.installed ? "success" : ""}>
                  {data?.installed ? "已安装" : "可选插件"}
                </Badge>
              </div>
              <p>
                从本地游戏中提取原图、建立动态对应表，并诊断缺失资源。原尺寸、透明度和完整资源名都会保留。
              </p>
              <div className="plugin-tags">
                <Badge>角色与怪物</Badge>
                <Badge>卡面与筹码</Badge>
                <Badge>地块与鼠标</Badge>
                <Badge>无损 PNG</Badge>
              </div>
              <small>
                版本 {data?.version || data?.availableVersion || "待检查"} ·{" "}
                {data?.bytes ? bytes(data.bytes) : "大小由下载清单提供"}
              </small>
              {release && (
                <p>
                  可下载版本 {release.availableVersion} · {bytes(release.bytes)}{" "}
                  · 与当前主程序兼容
                </p>
              )}
              <button
                className="text-button"
                onClick={() =>
                  void act(
                    async () =>
                      setRelease(await api.command(IPC.auxiliary_check_update)),
                    "插件更新检查完成",
                  )
                }
              >
                检查更新
              </button>
            </div>
            <button
              className="button primary"
              disabled={busy}
              onClick={() =>
                void run(
                  "aux-install",
                  data?.installed ? "更新资产工具" : "安装资产工具",
                )
              }
            >
              <Download size={17} />
              {data?.installed ? "下载并更新" : "下载安装"}
            </button>
          </div>
          <div className="settings-grid">
            <section className="settings-card">
              <h2>提取游戏原图</h2>
              <p>优先使用已验证的本地原图；缺失时继续使用远端预览。</p>
              <label className="field">
                提取范围
                <ThemedSelect
                  label="提取范围"
                  value={category}
                  onValueChange={setCategory}
                  options={[
                    ["all", "全部资源"],
                    ["hero", "角色"],
                    ["cards", "全部卡面"],
                    ["monster", "怪物"],
                    ["relic", "筹码"],
                    ["lands", "角色地块"],
                    ["cursor", "鼠标样式"],
                  ].map(([value, label]) => ({ value, label }))}
                />
              </label>
              <div className="action-row">
                <button
                  className="button primary"
                  disabled={
                    !data?.installed || busy || !workspace.data?.gameDir
                  }
                  onClick={() =>
                    void run("aux-run", "提取游戏原图", {
                      command: "sync",
                      category,
                      gameDir: workspace.data?.gameDir,
                    })
                  }
                >
                  <Play size={16} />
                  开始提取
                </button>
                <button
                  className="button"
                  disabled={!data?.installed}
                  onClick={() =>
                    void run("aux-run", "检查游戏资源", {
                      command: "check",
                      gameDir: workspace.data?.gameDir,
                    })
                  }
                >
                  检查资源
                </button>
              </div>
              {!workspace.data?.gameDir && (
                <p className="muted">先在设置中连接游戏目录。</p>
              )}
            </section>
            <section className="settings-card">
              <h2>插件管理</h2>
              <p>
                {data?.message ||
                  "插件独立安装与更新，主程序不携带工具运行时。"}
              </p>
              <button
                className="button"
                onClick={() =>
                  void act(async () => {
                    const files = await api.chooseFiles("plugins");
                    const path = Array.isArray(files) ? files[0] : files;
                    if (path)
                      await api.task("aux-install", "安装离线资产工具", {
                        path,
                      });
                  })
                }
              >
                安装离线插件包
              </button>
              <button
                className="button danger"
                disabled={!data?.installed}
                onClick={() =>
                  setConfirmation(
                    () => () => api.command(IPC.auxiliary_uninstall),
                  )
                }
              >
                <Trash2 size={15} />
                卸载工具
              </button>
              <p className="muted">卸载保留已提取的原图和索引。</p>
            </section>
          </div>
        </>
      ) : route === "apx" ? (
        <>
          <div className="filterbar">
            <label>游戏渠道</label>
            <ThemedSelect
              label="游戏渠道"
              value={platform}
              onValueChange={(value) => {
                setPlatform(value);
                void act(() => api.settings({ pluginPlatform: value }));
              }}
              options={[
                ["CN_STEAM", "Steam 国服"],
                ["CN_TAPTAP", "TapTap"],
                ["CN_BILIBILI", "哔哩哔哩"],
                ["CN_WEGAME", "WeGame"],
                ["INT_STEAM", "Steam 国际服"],
                ["CN_ANDROID", "Android（仅导出）"],
              ].map(([value, label]) => ({ value, label }))}
            />
            <button
              className="button"
              onClick={() =>
                void act(async () => {
                  await api.command(IPC.plugin_check_update, { platform });
                  await q.refetch();
                })
              }
            >
              检查更新
            </button>
          </div>
          <section className="settings-card">
            <h2>
              <Box size={21} />
              APX 客户端补丁
            </h2>
            <p>
              {data?.error ||
                data?.directoryError ||
                data?.warning ||
                data?.message ||
                "检查当前客户端与远端补丁版本。"}
            </p>
            <div className="action-row">
              <Badge>本地：{data?.localGameVersion || "未检测"}</Badge>
              <Badge>
                远端：{data?.version || data?.remoteVersion || "待检查"}
              </Badge>
            </div>
            <div className="action-row">
              <button
                className="button primary"
                disabled={
                  platform === "INT_STEAM" || platform === "CN_ANDROID" || busy
                }
                onClick={() =>
                  setConfirmation(
                    () => () =>
                      api.task("apx-install", "安装 APX 补丁", {
                        platform,
                        force: data?.versionMatch === false,
                      }),
                  )
                }
              >
                安装补丁
              </button>
              <button
                className="button"
                disabled={
                  platform === "INT_STEAM" || platform === "CN_ANDROID" || busy
                }
                onClick={() =>
                  setConfirmation(
                    () => () =>
                      api.task("apx-restore", "还原 APX 补丁", { platform }),
                  )
                }
              >
                还原补丁
              </button>
              <button
                className="button"
                onClick={() =>
                  void act(async () => {
                    const destination = await api.saveFile(
                      `APX-${platform}.zip`,
                    );
                    if (destination)
                      return api.task("apx-export", "导出 APX 补丁", {
                        platform,
                        destination,
                        useCached: true,
                      });
                  })
                }
              >
                导出缓存
              </button>
              <button
                className="button"
                onClick={() =>
                  void run("apx-cache", "下载 APX 缓存", { platform })
                }
              >
                下载缓存
              </button>
            </div>
            {(data?.files || []).map((f: any, i: number) => (
              <div key={i} className="setting-row">
                <span>{f.description || f.relativePath || f.targetPath}</span>
                <Badge>{f.status || "待检查"}</Badge>
              </div>
            ))}
          </section>
        </>
      ) : null}
      {confirmation && (
        <Modal open title="确认操作" onClose={() => setConfirmation(null)}>
          <div className="wizard-body">
            <p>
              {route === "apx"
                ? `即将修改所选渠道的补丁文件。本地游戏版本 ${data?.localGameVersion || "未知"}，补丁要求游戏版本 ${data?.remoteGameVersion || "未知"}，补丁版本 ${data?.remoteVersion || "未知"}。${data?.versionMatch === false ? "版本不匹配，仍继续安装可能无法正常运行。" : ""}操作前会备份原文件。`
                : "将卸载所选工具或作品。辅助插件生成的原图会保留。"}
            </p>
            <button
              className="button primary"
              disabled={busy}
              onClick={() =>
                void act(async () => {
                  await confirmation();
                  setConfirmation(null);
                  await q.refetch();
                })
              }
            >
              确认
            </button>
          </div>
        </Modal>
      )}
      {notice && <div className="toast">{notice}</div>}
    </>
  );
}
