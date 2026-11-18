// 版本卡片展示当前程序和所选渠道的真实补丁状态，点击只进入对应管理页。
import { useQuery } from "@tanstack/react-query";
import { Blocks, ChevronRight } from "lucide-react";
import { overviewService } from "../core/services/overview";
import { IPC } from "../core/ipc/commands";
import { apxInstallState } from "../core/domain/apx";
import type { Route } from "../core/domain/resources";
export function OverviewVersions({
  version,
  platform,
  navigate,
}: {
  version: string;
  platform: string;
  navigate: (route: Route) => void;
}) {
  const q = useQuery({
    queryKey: [IPC.plugin_status, { platform }],
    queryFn: () => overviewService.apx(platform),
    staleTime: 60000,
  });
  const status = q.data;
  const state = q.isPending
    ? "读取中"
    : q.isError
      ? "读取失败"
      : status?.directoryError
        ? "待配置"
        : status?.error
          ? "待检查"
          : status
            ? apxInstallState({
                ...status,
                localPluginVersion: status.localPluginVersion || undefined,
                remoteVersion: status.remoteVersion || undefined,
              })
            : "待检查";
  return (
    <div className="overview-version-grid">
      <button
        className="overview-version-card"
        aria-label={`APMM 当前版本 ${version}，打开软件设置`}
        onClick={() => navigate("settings")}
      >
        <img src="/logo.svg" alt="" />
        <div>
          <strong>APMM</strong>
          <small>v{version}</small>
        </div>
        <ChevronRight size={14} />
        <span className="version-state">当前版本</span>
      </button>
      <button
        className="overview-version-card"
        aria-label="APX 版本与安装状态"
        onClick={() => navigate("apx")}
      >
        <span className="version-icon">
          <Blocks size={22} />
        </span>
        <div>
          <strong>APX</strong>
          <small>
            {status?.localPluginVersion
              ? `v${status.localPluginVersion}`
              : state === "未安装"
                ? "未安装"
                : "版本待确认"}
          </small>
        </div>
        <ChevronRight size={14} />
        <span
          className={`version-state ${state === "待更新" ? "update" : ""}`}
          title={status?.platformLabel}
        >
          {state}
          {status?.platformLabel ? ` · ${status.platformLabel}` : ""}
        </span>
      </button>
    </div>
  );
}
