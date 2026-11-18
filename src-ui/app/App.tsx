import { useState, useEffect } from "react";
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
  Info,
  FlaskConical,
} from "lucide-react";
import {
  useWorkspace,
  useTasks,
  useActions,
  useNativeWindow,
} from "../hooks/useApp";
import { labels, type Route, type SearchMode } from "../core/domain/resources";
import { Loading, Badge } from "../shared/Ui";
import { ResourceSearch } from "../modules/ResourceSearch";
import { DirectorySyncIndicator } from "../modules/DirectorySyncIndicator";
import { version as appVersion } from "../../package.json";
import { OverviewPage } from "../modules/OverviewPage";
import { ApxPage } from "../modules/ApxPage";
import { ResourcePage } from "../modules/ResourcePage";
import { PresetsPage } from "../modules/PresetsPage";
import { SettingsPage } from "../modules/SettingsPage";
import { PluginsPage } from "../modules/PluginsPage";
import { WorkshopPage } from "../modules/WorkshopPage";
import { AuthorPage } from "../modules/AuthorPage";
import { PackageDialog } from "../modules/PackageDialog";
import { TaskPopover } from "../modules/TaskPopover";
import { useStartup } from "../hooks/useStartup";
import { activeTask } from "../core/domain/tasks";
import { WelcomeDialog } from "../modules/WelcomeDialog";
import { useAppearance } from "../hooks/useAppearance";
import { DebugPage } from "../modules/DebugPage";
import { usePageNavigation } from "../hooks/usePageNavigation";
import { usePageReveal } from "../hooks/usePageReveal";
import { useResourceDrop } from "../hooks/useResourceDrop";
import { DropImportDialog } from "../modules/DropImportDialog";
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
      ["author", Upload],
      ["auxiliary", Puzzle],
    ],
  },
  {
    title: "应用",
    items: [["settings", Settings]],
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
  usePageReveal(pageRef, route);
  const [collapsed, setCollapsed] = useState(false);
  const authorMode = workspace.data?.settings.authorMode || false;
  useEffect(() => {
    if (!authorMode && target === "author") setRoute("workshop");
  }, [authorMode, target]);
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
  const [unknownDrop, setUnknownDrop] = useState<{
    paths: string[];
    packages: string[];
  } | null>(null);
  const openDroppedPackages = (paths: string[]) => {
    setImportKind("packages");
    setDrop(paths);
    setPkg("import");
  };
  const { feedback: dropFeedback, importing: dropBusy } = useResourceDrop({
    resources: workspace.data?.resources || [],
    route,
    blocked: !!pkg || !!unknownDrop,
    onUnknown: (paths, packages) => setUnknownDrop({ paths, packages }),
    onPackages: openDroppedPackages,
    notice: setNotice,
  });
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
    <div
      className={`app ${collapsed ? "collapsed" : ""} ${dropFeedback ? "resource-drop-active" : ""}`}
    >
      {dropFeedback && (
        <div className="resource-drop-hint" role="status">
          {dropFeedback.message}
        </div>
      )}
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
              {g.items
                .filter(([key]) => key !== "author" || authorMode)
                .map(([key, Icon]) => (
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
          {!collapsed && (
            <small>v{workspace.data?.version || appVersion}</small>
          )}
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
          <DirectorySyncIndicator />
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
                disabled={dropBusy}
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
                disabled={dropBusy}
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
                disabled={dropBusy}
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
        <main className="main-content" ref={pageRef} data-route={route}>
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
                  <OverviewPage
                    snapshot={workspace.data}
                    running={running}
                    native={api.native}
                    navigate={navigate}
                  />
                ) : route === "presets" ? (
                  <PresetsPage snapshot={workspace.data} />
                ) : route === "settings" ? (
                  <SettingsPage
                    snapshot={workspace.data}
                    onPresets={() => navigate("presets")}
                  />
                ) : route === "debug" ? (
                  <DebugPage />
                ) : route === "apx" ? (
                  <ApxPage />
                ) : route === "workshop" ? (
                  <WorkshopPage
                    onImport={() => {
                      if (dropBusy) return;
                      setImportKind("packages");
                      setPkg("import");
                    }}
                  />
                ) : route === "author" ? (
                  <AuthorPage />
                ) : route === "auxiliary" ? (
                  <PluginsPage route={route} />
                ) : (
                  <ResourcePage
                    key={route}
                    route={route}
                    search={search}
                    onSearchClear={() => setSearch("")}
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
      {unknownDrop && (
        <DropImportDialog
          paths={unknownDrop.paths}
          hasPackages={unknownDrop.packages.length > 0}
          onClose={() => {
            const packages = unknownDrop.packages;
            setUnknownDrop(null);
            if (packages.length) openDroppedPackages(packages);
          }}
        />
      )}
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
