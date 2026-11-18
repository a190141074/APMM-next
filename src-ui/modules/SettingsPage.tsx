import { AppearanceSettingsPanel } from "./AppearanceSettingsPanel";
import { ThemesPanel } from "./ThemesPanel";
import { UpdateSettings } from "./UpdateSettings";
const categories = [
  ["directories", "目录与资源"],
  ["appearance", "外观"],
  ["colors", "配色"],
  ["workshop", "工坊"],
  ["updates", "更新"],
  ["maintenance", "安全与维护"],
  ["about", "关于"],
];
import { MaintenancePanel } from "./MaintenancePanel";
import { IPC } from "../core/ipc/commands";
import { useState, lazy, Suspense } from "react";
import { FolderOpen, Download, Archive, ShieldCheck, Copy } from "lucide-react";
import type { WorkspaceSnapshot } from "../core/ipc/generated/WorkspaceSnapshot";
import { useActions } from "../hooks/useApp";
import { Modal, Badge, Loading } from "../shared/Ui";
const AboutPage = lazy(() => import("./AboutPage"));
import { bytes } from "../core/domain/resources";
import { useMigrationStatus } from "./LegacyMigrationNotice";
export function SettingsPage({
  snapshot,
  onPresets,
  initialCategory,
}: {
  snapshot: WorkspaceSnapshot;
  onPresets?: () => void;
  initialCategory?: string;
}) {
  const { api, act, busy, notice } = useActions();
  const [migration, setMigration] = useState<any>(null);
  const migrationStatus = useMigrationStatus(snapshot.revision);
  const [category, setCategory] = useState(() =>
    categories.some(([key]) => key === initialCategory)
      ? initialCategory!
      : "directories",
  );

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
          {category === "about" && (
            <Suspense fallback={<Loading />}>
              <AboutPage version={snapshot.version} embedded />
            </Suspense>
          )}
          {category === "workshop" && (
            <section className="settings-card">
              <h2>作者模式</h2>
              <label className="settings-row">
                <span>
                  作者模式
                  <small>
                    开启后在导航中显示投稿入口。本地编辑和导出投稿包，通过约定渠道交给维护者审核。
                  </small>
                </span>
                <input
                  type="checkbox"
                  checked={settings.authorMode || false}
                  disabled={busy}
                  onChange={(event) =>
                    void act(
                      () => api.settings({ authorMode: event.target.checked }),
                      "作者模式已保存",
                    )
                  }
                />
              </label>
            </section>
          )}
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
              <label className="settings-row">
                <span>
                  缺少预览图时提醒
                  <small>关闭后可在此重新开启；预览图也能通过上方按钮手动补下载。</small>
                </span>
                <input
                  type="checkbox"
                  checked={settings.previewDownloadPrompt !== false}
                  disabled={busy}
                  onChange={(event) =>
                    void act(
                      () =>
                        api.settings({
                          previewDownloadPrompt: event.target.checked,
                        }),
                      "预览图提醒设置已保存",
                    )
                  }
                />
              </label>
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
                空库接续旧版资源、设置和预设；已有资源时补齐全部缺失候选，保留当前应用项和设置。
              </p>
              <p className="resource-code">当前 data：{snapshot.dataDir}</p>
              {migrationStatus.data?.mode === "inherited" && (
                <p>
                  已就地接续当前 data，包含 {migrationStatus.data.mods} 个候选、
                  {migrationStatus.data.multipleSlots}{" "}
                  个多候选部位，无需把此目录复制到自身。
                </p>
              )}
              {migrationStatus.data?.sources.map((source) => (
                <div className="settings-row" key={source.source}>
                  <span>
                    发现旧库{source.version ? " v" + source.version : ""}
                    <small>
                      {source.source} · 共 {source.mods} 个候选，待补{" "}
                      {source.newMods} 个
                    </small>
                  </span>
                  <button
                    className="button"
                    disabled={busy || source.newMods === 0}
                    onClick={() =>
                      void act(async () => {
                        setMigration(
                          await api.command(IPC.migration_inspect, {
                            source: source.source,
                          }),
                        );
                      })
                    }
                  >
                    {source.newMods > 0 ? "检查并补齐" : "候选已齐全"}
                  </button>
                </div>
              ))}
              {migrationStatus.isSuccess &&
                migrationStatus.data.mods === 0 &&
                !migrationStatus.data.sources.some(
                  (source) => source.newMods > 0,
                ) && (
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
                  手动选择旧 data 目录
                </button>
              )}
              <p className="muted">
                {snapshot.migrated
                  ? "已记录迁移；仍会检查旧库是否有缺失备选，不以标记代替完整性核对。"
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
      {migration?.mode === "inherited" ? (
        <Modal
          open
          onClose={() => setMigration(null)}
          title="当前 data 已就地接续"
        >
          <div className="wizard-body">
            <p>{migration.message}</p>
            <p className="resource-code">{migration.destination}</p>
            <button className="button" onClick={() => setMigration(null)}>
              知道了
            </button>
          </div>
        </Modal>
      ) : (
        migration && (
          <Modal
            open
            onClose={() => setMigration(null)}
            title={
              migration.mode === "merge"
                ? "确认补齐旧库候选"
                : "确认复制旧资源库"
            }
          >
            <div className="wizard-body">
              <p>
                旧库包含 {migration.mods} 个候选（含未应用项）。
                {migration.mode === "merge" ? (
                  <>
                    本次补齐 {migration.newMods} 个缺失候选，同部位多个 Mod
                    均保留。
                  </>
                ) : (
                  <>
                    将复制 {migration.files} 个文件，占用约{" "}
                    {bytes(migration.bytes)}。
                  </>
                )}
              </p>
              <p className="resource-code">来源：{migration.source}</p>
              <p className="resource-code">目标：{migration.destination}</p>
              <p>
                {migration.mode === "merge"
                  ? "保留新版当前应用项、设置和预设；只补齐候选。"
                  : "复制和校验完成后才启用新版数据。"}
                源文件保持不变，不向游戏应用 Mod。
              </p>
              <button
                className="button primary"
                disabled={busy}
                onClick={() =>
                  void act(async () => {
                    await api.task(
                      "migration",
                      migration.mode === "merge"
                        ? "补齐旧库候选"
                        : "复制旧资源库",
                      {
                        source: migration.source,
                      },
                    );
                    setMigration(null);
                  })
                }
              >
                {migration.mode === "merge" ? "补齐缺失候选" : "复制并迁移"}
              </button>
            </div>
          </Modal>
        )
      )}
      {notice && <div className="toast">{notice}</div>}
    </>
  );
}
