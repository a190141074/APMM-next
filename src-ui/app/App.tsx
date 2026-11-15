import { useState, useEffect, useCallback, lazy, Suspense } from "react";
import {
  LayoutDashboard,
  Users,
  CreditCard,
  Ghost,
  Gem,
  Map,
  MousePointer2,
  FolderClock,
  Blocks,
  Store,
  Puzzle,
  ListTodo,
  Settings,
  Search,
  PanelLeftClose,
  PanelLeftOpen,
  Minus,
  Square,
  X,
  Sun,
  Moon,
  Download,
  Upload,
  Sparkles,
  ArrowRight,
  Info,
  FlaskConical,
} from "lucide-react";
import {
  useWorkspace,
  useTasks,
  useActions,
  useNativeWindow,
  useDrop,
} from "../hooks/useApp";
import { labels, type Route, type SearchMode } from "../core/domain/resources";
import { Loading, Badge } from "../shared/Ui";
import { ResourceSearch } from "../modules/ResourceSearch";
import { ApxPage } from "../modules/ApxPage";
import { ResourcePage } from "../modules/ResourcePage";
import { PresetsPage } from "../modules/PresetsPage";
import { SettingsPage } from "../modules/SettingsPage";
import { PluginsPage } from "../modules/PluginsPage";
import { PackageDialog } from "../modules/PackageDialog";
import { TaskPopover } from "../modules/TaskPopover";
import { useStartup } from "../hooks/useStartup";
import { activeTask } from "../core/domain/tasks";
import { WelcomeDialog } from "../modules/WelcomeDialog";
import { useAppearance } from "../hooks/useAppearance";
import { DebugPage } from "../modules/DebugPage";
import { usePageNavigation } from "../hooks/usePageNavigation";
const AboutPage = lazy(() => import("../modules/AboutPage"));
const groups = [
  {
    title: "工作区",
    items: [
      ["overview", LayoutDashboard],
      ["presets", ListTodo],
      ["debug", FlaskConical],
    ],
  },
  {
    title: "资源库",
    items: [
      ["heroes", Users],
      ["cards", CreditCard],
      ["monsters", Ghost],
      ["chips", Gem],
      ["lands", Map],
      ["cursors", MousePointer2],
      ["temporary", FolderClock],
    ],
  },
  {
    title: "插件库",
    items: [
      ["apx", Blocks],
      ["workshop", Store],
      ["auxiliary", Puzzle],
    ],
  },
  {
    title: "关于",
    items: [
      ["settings", Settings],
      ["about", Info],
    ],
  },
] as const;
export function App() {
  const workspace = useWorkspace();
  const tasks = useTasks();
  const { api, act, notice, setNotice } = useActions();
  const nativeWindow = useNativeWindow();
  const { route, target, setRoute, pageRef } = usePageNavigation(
    groups.flatMap((g) => g.items.map(([key]) => key)),
  );
  const [collapsed, setCollapsed] = useState(false);
  const [tasksOpen, setTasksOpen] = useState(false);
  const [searches, setSearches] = useState<Partial<Record<Route, string>>>({});
  const search = searches[route] || "";
  const setSearch = (value: string) =>
    setSearches((previous) => ({ ...previous, [route]: value }));
  useStartup(workspace.data, setNotice);
  const {
    mode: theme,
    setMode: setTheme,
    error: appearanceError,
  } = useAppearance();
  const [pkg, setPkg] = useState<"import" | "export" | null>(null);
  const [importKind, setImportKind] = useState<"images" | "packages" | "all">(
    "all",
  );
  const [searchMode, setSearchMode] = useState<SearchMode>("smart");
  const [focusKey, setFocusKey] = useState<string>();
  useEffect(() => {
    if (workspace.data?.settings.searchMode)
      setSearchMode(
        workspace.data.settings.searchMode === "id" ? "id" : "smart",
      );
  }, [workspace.data?.settings.searchMode]);
  const [drop, setDrop] = useState<string[]>([]);
  const [welcomed, setWelcomed] = useState(
    localStorage.getItem("apmm.welcomed") === "true",
  );
  const acceptDrop = useCallback((paths: string[]) => {
    setImportKind("all");
    setDrop(paths);
    setPkg("import");
  }, []);
  useDrop(acceptDrop);
  useEffect(() => {
    if (!notice) return;
    const t = setTimeout(() => setNotice(""), 6500);
    return () => clearTimeout(t);
  }, [notice, setNotice]);
  const running = tasks.data?.filter(activeTask).length || 0;
  const navigate = (next: Route) => {
    setFocusKey(undefined);
    setRoute(next);
  };
  return (
    <div className={`app ${collapsed ? "collapsed" : ""}`}>
      <aside className="sidebar">
        <div
          className="brand"
          onMouseDown={(e) => {
            if (e.buttons === 1) void nativeWindow("startDragging");
          }}
        >
          <div className="brand-icon">
            <img src="/logo.svg" alt="APMM" />
          </div>
          {!collapsed && (
            <div>
              <strong>APMM</strong>
              <small>Astral Party Mod Manager</small>
            </div>
          )}
        </div>
        <nav>
          {groups.map((g) => (
            <section key={g.title}>
              <div className="nav-label">{collapsed ? "·" : g.title}</div>
              {g.items.map(([key, Icon]) => (
                <button
                  key={key}
                  title={labels[key]}
                  className={`nav-item ${target === key ? "active" : ""}`}
                  aria-current={target === key ? "page" : undefined}
                  onClick={() => navigate(key)}
                >
                  <Icon size={18} />
                  {!collapsed && <span>{labels[key]}</span>}
                </button>
              ))}
            </section>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <button
            className="icon-button"
            aria-label="折叠导航"
            onClick={() => setCollapsed(!collapsed)}
          >
            {collapsed ? (
              <PanelLeftOpen size={18} />
            ) : (
              <PanelLeftClose size={18} />
            )}
          </button>
          {!collapsed && <small>v{workspace.data?.version || "0.4.1"}</small>}
        </div>
      </aside>
      <div className="workspace">
        <div className="titlebar">
          <div
            className="titlebar-drag"
            onMouseDown={() => void nativeWindow("startDragging")}
          >
            <span className="status-dot" />
            本地 Mod 工作区
          </div>
          <TaskPopover
            open={tasksOpen}
            onOpenChange={setTasksOpen}
            tasks={tasks.data || []}
            loading={tasks.isPending}
            error={tasks.error}
            onRefresh={() => void tasks.refetch()}
          />
          <button
            className="icon-button"
            title="切换主题"
            onClick={() =>
              setTheme(
                document.documentElement.dataset.theme === "dark"
                  ? "light"
                  : "dark",
              )
            }
          >
            <Sun size={16} />
          </button>
          <button
            className="window-button"
            aria-label="最小化"
            onClick={() => void nativeWindow("minimize")}
          >
            <Minus size={14} />
          </button>
          <button
            className="window-button"
            aria-label="最大化"
            onClick={() => void nativeWindow("toggleMaximize")}
          >
            <Square size={12} />
          </button>
          <button
            className="window-button close"
            aria-label="关闭"
            onClick={() => {
              if (running) {
                setTasksOpen(true);
                setNotice(
                  "仍有任务进行中，请在右上角任务进程中等待完成或取消任务。",
                );
                return;
              }
              void nativeWindow("close");
            }}
          >
            <X size={16} />
          </button>
        </div>
        <header className="workspace-header">
          <div className="breadcrumb">
            工作区 <span>/</span> <strong>{labels[route]}</strong>
          </div>
          <ResourceSearch
            value={search}
            onChange={setSearch}
            mode={searchMode}
            onMode={(m) => {
              setSearchMode(m);
              void act(() => api.settings({ searchMode: m }), "搜索模式已保存");
            }}
            rows={workspace.data?.resources || []}
            onSelect={(r) => {
              setRoute(r.category as Route);
              setFocusKey(r.key);
            }}
          />
          <details className="package-menu">
            <summary className="button primary">
              <Download size={16} />
              Mod 包 ▾
            </summary>
            <div>
              <button
                onClick={(e) => {
                  setImportKind("images");
                  setPkg("import");
                  e.currentTarget.closest("details")?.removeAttribute("open");
                }}
              >
                <strong>导入 PNG</strong>
                <small>添加图片到对应部位</small>
              </button>
              <button
                onClick={(e) => {
                  setImportKind("packages");
                  setPkg("import");
                  e.currentTarget.closest("details")?.removeAttribute("open");
                }}
              >
                <strong>导入加密包</strong>
                <small>.apmod · 支持历史包和分卷</small>
              </button>
              <button
                onClick={(e) => {
                  setPkg("export");
                  e.currentTarget.closest("details")?.removeAttribute("open");
                }}
              >
                <strong>导出加密包</strong>
                <small>选择内容并设置包密码</small>
              </button>
            </div>
          </details>
        </header>
        <main className="main-content" ref={pageRef}>
          {workspace.isPending ? (
            <Loading />
          ) : workspace.error ? (
            <div className="empty">
              <h2>无法读取工作区</h2>
              <p>{String(workspace.error)}</p>
              <button
                className="button"
                onClick={() => void workspace.refetch()}
              >
                重试
              </button>
            </div>
          ) : (
            workspace.data && (
              <>
                {route === "overview" ? (
                  <>
                    <div className="page-heading">
                      <div>
                        <div className="eyebrow">YOUR CREATIVE SPACE</div>
                        <h1>让每一份灵感，各就其位。</h1>
                        <p>浏览原图、管理替换，定制你的 Astral Party。</p>
                      </div>
                      <Badge tone="success">
                        {api.native ? "桌面工作区" : "交互预览"}
                      </Badge>
                    </div>
                    <div className="overview-hero">
                      <div>
                        <span className="eyebrow">ASTRAL PARTY</span>
                        <h2>先安装 APX，再应用 Mod。</h2>
                        <p>
                          Mod 需要 APX
                          支持。请选择游戏渠道，确认适配版本后安装。
                        </p>
                        <button
                          className="button primary"
                          onClick={() => navigate("apx")}
                        >
                          安装 APX 插件 <ArrowRight size={17} />
                        </button>
                      </div>
                      <div className="hero-orbit">
                        <Sparkles size={90} />
                        <span>✦</span>
                        <b>AP</b>
                      </div>
                    </div>
                    <div className="stats-grid">
                      {[
                        ["资源槽位", workspace.data.resources.length],
                        [
                          "已收藏 Mod",
                          workspace.data.resources.reduce(
                            (n, r) => n + r.files.length,
                            0,
                          ),
                        ],
                        [
                          "正在应用",
                          workspace.data.resources.filter((r) => r.activeHash)
                            .length,
                        ],
                        ["进行中任务", running],
                      ].map(([label, value]) => (
                        <div className="stat" key={label}>
                          <small>{label}</small>
                          <strong>{value}</strong>
                        </div>
                      ))}
                    </div>
                    <div className="section-heading">
                      <h2>从这里开始</h2>
                      <span>先准备插件，再管理 Mod</span>
                    </div>
                    <div className="quick-grid">
                      {[
                        ["apx", "1. 安装 APX", "Mod 所需的客户端补丁", Blocks],
                        [
                          "heroes",
                          "2. 管理 Mod",
                          "浏览、导入并应用 Mod",
                          Users,
                        ],
                        [
                          "auxiliary",
                          "3. 提取原图",
                          "按需安装资产工具，保留完整画质",
                          Puzzle,
                        ],
                      ].map(([key, title, description, Icon]: any) => (
                        <button
                          className="quick-card"
                          key={key}
                          onClick={() => navigate(key)}
                        >
                          <Icon size={24} />
                          <h3>{title}</h3>
                          <p>{description}</p>
                          <ArrowRight size={17} />
                        </button>
                      ))}
                    </div>
                  </>
                ) : route === "presets" ? (
                  <PresetsPage snapshot={workspace.data} />
                ) : route === "settings" ? (
                  <SettingsPage
                    snapshot={workspace.data}
                    onPresets={() => navigate("presets")}
                  />
                ) : route === "debug" ? (
                  <DebugPage />
                ) : route === "about" ? (
                  <Suspense fallback={<Loading />}>
                    <AboutPage version={workspace.data.version} />
                  </Suspense>
                ) : route === "apx" ? (
                  <ApxPage />
                ) : ["workshop", "auxiliary"].includes(route) ? (
                  <PluginsPage route={route} />
                ) : (
                  <ResourcePage
                    key={route}
                    route={route}
                    search={search}
                    searchMode={searchMode}
                    focusKey={focusKey}
                    onFocusClear={() => setFocusKey(undefined)}
                    resources={workspace.data.resources}
                  />
                )}
              </>
            )
          )}
        </main>
        <footer className="statusbar">
          <span>
            <i className="status-dot" />
            {workspace.data?.gameDir ? "游戏目录已连接" : "尚未连接游戏目录"}
          </span>
          <span>{workspace.data?.resources.length || 0} 个资源槽位</span>
          <button onClick={() => setTasksOpen(true)}>
            {running ? `${running} 个任务进行中` : "工作区已就绪"}{" "}
            <ListTodo size={13} />
          </button>
        </footer>
      </div>
      {pkg && (
        <PackageDialog
          mode={pkg}
          importKind={importKind}
          initialPaths={drop}
          onClose={() => {
            setPkg(null);
            setDrop([]);
          }}
        />
      )}
      {api.native &&
        workspace.data &&
        !workspace.data.migrated &&
        !welcomed && (
          <WelcomeDialog
            dataDir={workspace.data.dataDir}
            onClose={() => {
              localStorage.setItem("apmm.welcomed", "true");
              setWelcomed(true);
            }}
          />
        )}
      {(notice || appearanceError) && (
        <button className="toast" onClick={() => setNotice("")}>
          {notice || appearanceError}
          <X size={14} />
        </button>
      )}
    </div>
  );
}
