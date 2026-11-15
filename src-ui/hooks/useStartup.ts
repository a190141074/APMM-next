import { IPC } from "../core/ipc/commands";
// 工作区成功读取后确认更新启动；自动检查按旧版间隔运行，页面切换不重建计时器。
import { useEffect, useRef } from "react";
import { appService } from "../core/services/app";
import type { WorkspaceSnapshot } from "../core/ipc/generated/WorkspaceSnapshot";
export function useStartup(
  snapshot: WorkspaceSnapshot | undefined,
  notify: (v: string) => void,
) {
  const latest = useRef(snapshot);
  latest.current = snapshot;
  useEffect(() => {
    document.documentElement.dataset.reduceMotion =
      localStorage.getItem("apmm.reduce-motion") || "false";
  }, []);
  const ready = !!snapshot;
  const startupResult = useRef<Promise<any>>();
  useEffect(() => {
    if (!ready || !appService.native) return;
    let disposed = false;
    let lastAttempt = 0;
    startupResult.current ??= appService.command(IPC.update_startup_status);
    void startupResult.current
      .then((result) => {
        if (!disposed && result?.message) notify(result.message);
      })
      .catch((e) => {
        if (!disposed) notify(`更新恢复记录：${String(e)}`);
      });
    const check = async () => {
      const s = latest.current?.settings;
      const now = Date.now() / 1000;
      if (
        !s?.updateAutoCheck ||
        now - Math.max(s.updateLastCheckedAt || 0, lastAttempt) <
          (s.updateIntervalHours || 24) * 3600
      )
        return;
      lastAttempt = now;
      try {
        const result = await appService.command(IPC.update_check);
        if (
          !disposed &&
          result.available &&
          result.latestVersion !== s.updateSkippedVersion
        )
          notify(
            `发现 APMM ${result.latestVersion}，可在设置中查看更新说明并下载。`,
          );
      } catch {
        /* 自动检查失败不打断用户操作，手动检查仍显示具体错误。 */
      }
    };
    void check();
    const timer = setInterval(() => void check(), 15 * 60 * 1000);
    return () => {
      disposed = true;
      clearInterval(timer);
    };
  }, [ready, notify]);
}
