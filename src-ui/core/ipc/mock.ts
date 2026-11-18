import { mockManagement } from "./mockManagement";
import { IPC } from "./commands";
import { version as appVersion } from "../../../package.json";
import type { WorkspaceSnapshot } from "./generated/WorkspaceSnapshot";
import type { ResourceRecord } from "./generated/ResourceRecord";
const rows: ResourceRecord[] = [];
let loaded = false;
export const demo: WorkspaceSnapshot = {
  revision: 0,
  version: appVersion,
  dataDir: "本地预览（应用操作仅影响本页）",
  gameDir: "Astral Party",
  resources: rows,
  settings: {
    authorMode: localStorage.getItem("apmm.demo-author-mode") === "true",
    appearance: JSON.parse(
      localStorage.getItem("apmm.demo-appearance") || "null",
    ),
    outimgPath: "",
    searchMode: "smart",
    updateSkippedVersion: null,
    updateLastCheckedAt: null,
    pluginPlatform: "CN_STEAM",
    presets: [],
    previewDownloadPrompt:
      localStorage.getItem("apmm.demo-preview-download-prompt") !== "false",
    updateAutoCheck: true,
    updateIntervalMinutes: 60,
  },
  migrated: true,
};
export async function mockCall<T>(
  command: string,
  args: Record<string, any> = {},
): Promise<T> {
  if (command === IPC.workspace_snapshot) {
    if (!loaded) {
      const response = await fetch("/__workspace");
      if (!response.ok) throw Error("本地预览目录读取失败");
      const snapshot = await response.json();
      demo.resources = snapshot.resources;
      demo.gameDir = snapshot.gameDir;
      loaded = true;
    }
    return structuredClone(demo) as T;
  }
  const management = mockManagement(command, args, demo);
  if (management.handled) return management.value as T;
  if (command === IPC.tasks_list) return [] as T;
  if (command === IPC.resources_apply) {
    for (const x of args.items) {
      const r = demo.resources.find((r) => r.canonical === x.canonical);
      if (r) r.activeHash = x.hash;
    }
    return undefined as T;
  }
  if (command === IPC.settings_set) {
    if ("authorMode" in args.patch)
      localStorage.setItem(
        "apmm.demo-author-mode",
        String(args.patch.authorMode),
      );
    Object.assign(demo.settings, args.patch);
    demo.revision += 1;
    if ("previewDownloadPrompt" in args.patch)
      localStorage.setItem(
        "apmm.demo-preview-download-prompt",
        String(args.patch.previewDownloadPrompt),
      );
    if (args.patch.appearance)
      localStorage.setItem(
        "apmm.demo-appearance",
        JSON.stringify(args.patch.appearance),
      );
    return demo.settings as T;
  }
  if (command === IPC.workshop_list)
    return {
      available: false,
      message: "浏览器预览使用演示数据，桌面应用连接真实工坊。",
      works: [],
    } as T;
  if (command === IPC.auxiliary_status)
    return {
      installed: false,
      version: null,
      availableVersion: "0.2.0",
      bytes: 0,
      message: "按需安装，主程序不包含工具运行时。",
    } as T;
  if (command === IPC.plugin_status)
    return {
      platform: args.platform || "CN_STEAM",
      platformName: "Steam 国服",
      files: [],
      installed: false,
      updateAvailable: false,
    } as T;
  if (command === IPC.preview_status)
    return {
      installed: true,
      message: `${demo.resources.filter((r) => r.originalUrl).length} 张本地原图可用`,
    } as T;
  if (command === IPC.update_check)
    return {
      available: false,
      currentVersion: appVersion,
      latestVersion: appVersion,
      message: "浏览器预览不检查软件更新",
    } as T;
  if (command === IPC.update_startup_status) return null as T;
  if (command === IPC.update_download_status) return null as T;
  throw Error("此操作需要在桌面应用中执行。");
}
