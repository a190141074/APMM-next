// 总览只读已有补丁状态，不主动下载、安装或发起远端版本检查。
import { appService } from "./app";
import { IPC } from "../ipc/commands";
export type ApxOverview = {
  platformLabel: string;
  localPluginVersion: string | null;
  remoteVersion: string | null;
  installedFiles: number;
  totalFiles: number;
  directoryError: string | null;
  error: string | null;
  files: {
    present: boolean;
    installed: boolean;
    validationError?: string | null;
  }[];
};
export const overviewService = {
  apx: (platform: string) =>
    appService.command<ApxOverview>(IPC.plugin_status, { platform }),
};
