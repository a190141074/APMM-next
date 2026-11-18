import { IPC } from "../core/ipc/commands";
// 工作区成功读取后确认更新启动；分钟计时器按剩余时间调度，卸载时取消。
import { useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { appService } from "../core/services/app";
import type { WorkspaceSnapshot } from "../core/ipc/generated/WorkspaceSnapshot";
import { updateCheckDelay } from "../core/domain/updateSchedule";
export function useStartup(
  snapshot: WorkspaceSnapshot | undefined,
  notify: (v: string) => void,
) {
  const client = useQueryClient();
  const [availableVersion, setAvailableVersion] = useState<string | null>(null);
  const latest = useRef(snapshot);
  latest.current = snapshot;
  useEffect(() => {
    document.documentElement.dataset.reduceMotion =
      localStorage.getItem("apmm.reduce-motion") || "false";
  }, []);
  const ready = !!snapshot;
  const startupResult = useRef<Promise<any>>();
  const lastAttempt = useRef(0);
  useEffect(() => {
    if (!ready || !appService.native) return;
    let disposed = false;
    startupResult.current ??= appService.command(IPC.update_startup_status);
    void startupResult.current
      .then((result) => {
        if (!disposed && result?.message) notify(result.message);
      })
      .catch((e) => {
        if (!disposed) notify(`更新恢复记录：${String(e)}`);
      });
    return () => {
      disposed = true;
    };
  }, [ready, notify]);
  useEffect(() => {
    if (!ready || !appService.native) return;
    let disposed = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const check = async () => {
      const now = Date.now() / 1000;
      const delay = updateCheckDelay(
        latest.current?.settings,
        lastAttempt.current,
        now,
      );
      if (disposed || delay === null) return;
      if (delay > 0) {
        timer = setTimeout(() => void check(), delay);
        return;
      }
      lastAttempt.current = now;
      try {
        const result = await appService.command(IPC.update_check);
        if (!disposed) client.setQueryData([IPC.update_check, {}], result);
        if (!disposed)
          setAvailableVersion(
            result.available &&
              result.latestVersion !==
                latest.current?.settings.updateSkippedVersion
              ? result.latestVersion
              : null,
          );
      } catch {
        /* 自动检查失败不打断用户操作，手动检查仍显示具体错误。 */
      }
      if (!disposed) {
        const remaining = updateCheckDelay(
          latest.current?.settings,
          lastAttempt.current,
          Date.now() / 1000,
        );
        if (remaining !== null)
          timer = setTimeout(() => void check(), remaining);
      }
    };
    void check();
    return () => {
      disposed = true;
      clearTimeout(timer);
    };
  }, [
    ready,
    client,
    snapshot?.settings.updateAutoCheck,
    snapshot?.settings.updateIntervalMinutes,
  ]);
  const settings = snapshot?.settings;
  return {
    availableVersion:
      settings?.updateAutoCheck !== false &&
      settings?.updateSkippedVersion !== availableVersion
        ? availableVersion
        : null,
    dismissUpdate: () => setAvailableVersion(null),
  };
}
