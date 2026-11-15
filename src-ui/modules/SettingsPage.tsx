import { AppearanceSettingsPanel } from "./AppearanceSettingsPanel";
import { ThemesPanel } from "./ThemesPanel";
import { UpdateSettings } from "./UpdateSettings";
const categories = [
  ["appearance", "外观"],
  ["colors", "配色"],
  ["directories", "目录与资源"],
  ["updates", "更新"],
  ["maintenance", "安全与维护"],
];
import { MaintenancePanel } from "./MaintenancePanel";
import { IPC } from "../core/ipc/commands";
import { useState } from "react";
import { FolderOpen, Download, Archive, ShieldCheck, Copy } from "lucide-react";
import type { WorkspaceSnapshot } from "../core/ipc/generated/WorkspaceSnapshot";
import { useActions } from "../hooks/useApp";
import { Modal, Badge } from "../shared/Ui";
import { bytes } from "../core/domain/resources";
export function SettingsPage({
  snapshot,
  onPresets,
}: {
  snapshot: WorkspaceSnapshot;
  onPresets?: () => void;
}) {
  const { api, act, busy, notice } = useActions();
  const [migration, setMigration] = useState<any>(null);
  const [category, setCategory] = useState("appearance");

  const [password, setPassword] = useState("");

  const settings = snapshot.settings || {};
  return (
    <>
      <div className="page-heading settings-heading">
        <div>
          <h1>设置</h1>
          <p>普通设置即时保存；配色编辑需手动保存。</p>
        </div>
        <Badge>v{snapshot.version}</Badge>
      </div>
      <div className="settings-layout">
        <nav
          className="settings-nav"
          role="tablist"
          aria-label="设置分类"
          aria-orientation="horizontal"
        >
          {categories.map(([key, label]) => (
            <button
              key={key}
              className={category === key ? "active" : ""}
              role="tab"
              id={`settings-tab-${key}`}
              aria-controls="settings-panel"
              aria-selected={category === key}
              tabIndex={category === key ? 0 : -1}
              onKeyDown={(event) => {
                const buttons = [
                  ...event.currentTarget.parentElement!.querySelectorAll<HTMLButtonElement>(
                    "button",
                  ),
                ];
                const index = buttons.indexOf(event.currentTarget);
                const next =
                  event.key === "ArrowRight"
                    ? (index + 1) % buttons.length
                    : event.key === "ArrowLeft"
                      ? (index + buttons.length - 1) % buttons.length
                      : event.key === "Home"
                        ? 0
                        : event.key === "End"
                          ? buttons.length - 1
                          : -1;
                if (next >= 0) {
                  event.preventDefault();
                  buttons[next].focus();
                }
              }}
              onClick={() => setCategory(key)}
            >
              {label}
            </button>
          ))}
        </nav>
        <div
          className="settings-content"
          id="settings-panel"
          role="tabpanel"
          aria-labelledby={`settings-tab-${category}`}
          key={category}
        >
          {category === "colors" && <ThemesPanel />}
          {category === "directories" && (
            <section className="settings-card">
              <h2>
                <FolderOpen size={18} />
                游戏与资源目录
              </h2>
              <label className="field">
                游戏目录
                <div className="input-action">
                  <input readOnly value={snapshot.gameDir || "尚未选择"} />
                  <button
                    className="button"
                    onClick={() =>
                      void act(async () => {
                        const dir = await api.chooseDirectory();
                        if (dir)
                          return api.command(IPC.resolve_manual, { dir });
                      })
                    }
                  >
                    选择
                  </button>
                </div>
              </label>
              <button
                className="button"
                onClick={() => void act(() => api.command(IPC.detect))}
              >
                自动检测游戏
              </button>
              <label className="field">
                外部原图目录
                <div className="input-action">
                  <input
                    readOnly
                    value={settings.outimgPath || "使用辅助插件及远端预览"}
                  />
                  <button
                    className="button"
                    onClick={() =>
                      void act(async () => {
                        const dir = await api.chooseDirectory();
                        if (dir) return api.settings({ outimgPath: dir });
                      })
                    }
                  >
                    选择
                  </button>
                </div>
              </label>
              <p className="muted">工作区数据：{snapshot.dataDir}</p>
              <div className="settings-row">
                <span>
                  远端预览图<small>下载资源预览，用于浏览与选择。</small>
                </span>
                <button
                  className="button"
                  disabled={busy}
                  onClick={() =>
                    void act(() => api.task("preview", "下载预览图", {}))
                  }
                >
                  <Download size={15} />
                  下载预览图
                </button>
              </div>
            </section>
          )}
          {category === "appearance" && <AppearanceSettingsPanel />}
          {category === "directories" && (
            <section className="settings-card">
              <h2>
                <Copy size={18} />
                接续旧资源库
              </h2>
              <p>
                复制旧版资源、设置和预设。旧数据保持原样，迁移不会自动应用 Mod。
              </p>
              <button
                className="button"
                disabled={busy}
                onClick={() =>
                  void act(async () => {
                    const source = await api.chooseDirectory();
                    if (source)
                      setMigration(
                        await api.command(IPC.migration_inspect, { source }),
                      );
                  })
                }
              >
                选择旧 data 目录
              </button>
              <p className="muted">
                {snapshot.migrated
                  ? "本工作区已完成一次迁移。"
                  : "支持旧版资源库及历史路径转换。"}
              </p>
            </section>
          )}
          {category === "updates" && <UpdateSettings snapshot={snapshot} />}
          {category === "directories" && (
            <section className="settings-card">
              <h2>
                <Archive size={18} />
                预设与自动同步
              </h2>
              <p>在独立预设页管理搭配、局部范围和问题修复。</p>
              <button className="button" onClick={onPresets}>
                打开预设页
              </button>
              <label className="field check">
                <input
                  type="checkbox"
                  checked={settings.monitorDirectories ?? true}
                  onChange={(e) =>
                    void act(() =>
                      api.settings({ monitorDirectories: e.target.checked }),
                    )
                  }
                />
                监听游戏 Mod 目录和资源库
              </label>
              <p>外部变化稳定后自动同步候选和实际应用状态。</p>
            </section>
          )}
          {category === "maintenance" && (
            <section className="settings-card">
              <h2>
                <ShieldCheck size={18} />
                包密码与诊断
              </h2>
              <label className="field">
                默认包密码
                <input
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  autoComplete="off"
                />
              </label>
              <div className="action-row">
                <button
                  className="button"
                  onClick={() =>
                    void act(() =>
                      api.command(IPC.credential_default_password_set, {
                        password,
                      }),
                    )
                  }
                >
                  保存到系统凭据
                </button>
                <button
                  className="button"
                  onClick={() =>
                    void act(() =>
                      api.command(IPC.credential_default_password_clear),
                    )
                  }
                >
                  清除
                </button>
                <button
                  className="button"
                  onClick={() =>
                    void act(async () => {
                      const destination = await api.saveFile(
                        "APMM-diagnostics.zip",
                      );
                      if (destination)
                        return api.command(IPC.logs_export, {
                          args: { destination },
                        });
                    })
                  }
                >
                  导出诊断
                </button>
              </div>
            </section>
          )}
          {category === "maintenance" && (
            <MaintenancePanel snapshot={snapshot} />
          )}
        </div>
      </div>
      {migration && (
        <Modal open onClose={() => setMigration(null)} title="确认复制旧资源库">
          <div className="wizard-body">
            <p>
              将复制 {migration.files} 个文件，占用约 {bytes(migration.bytes)}。
            </p>
            <p className="resource-code">来源：{migration.source}</p>
            <p className="resource-code">目标：{migration.destination}</p>
            <p>源文件保持不变。复制和校验完成后才启用新版数据。</p>
            <button
              className="button primary"
              disabled={busy}
              onClick={() =>
                void act(async () => {
                  await api.task("migration", "复制旧资源库", {
                    source: migration.source,
                  });
                  setMigration(null);
                })
              }
            >
              复制并迁移
            </button>
          </div>
        </Modal>
      )}
      {notice && <div className="toast">{notice}</div>}
    </>
  );
}
