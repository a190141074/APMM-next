import { IPC } from "../core/ipc/commands";
import { useEffect, useState, useCallback } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { appService } from "../core/services/app";
import type { TaskSnapshot } from "../core/ipc/generated/TaskSnapshot";
import { errorText } from "../core/domain/resources";
import { mergeTasks, activeTask } from "../core/domain/tasks";
import {
  mergeWorkspaceDelta,
  acceptWorkspaceSnapshot,
} from "../core/domain/workspace";
import type { WorkspaceSnapshot } from "../core/ipc/generated/WorkspaceSnapshot";
import type { DirectorySyncStatus } from "../core/ipc/generated/DirectorySyncStatus";
import type { DropEvent } from "../core/domain/dropImport";
export function useWorkspace() {
  return useQuery({
    queryKey: ["workspace"],
    queryFn: appService.snapshot,
    staleTime: 15000,
    structuralSharing: (old, next) =>
      acceptWorkspaceSnapshot(
        old as WorkspaceSnapshot | undefined,
        next as WorkspaceSnapshot,
      ),
  });
}
export function useDirectorySync() {
  return useQuery<DirectorySyncStatus>({
    queryKey: ["directory-sync"],
    queryFn: () => appService.command(IPC.directory_sync_status),
    enabled: appService.native,
    staleTime: Infinity,
  });
}
export function useTasks() {
  const client = useQueryClient();
  const q = useQuery({
    queryKey: ["tasks"],
    queryFn: appService.tasks,
    refetchInterval: 2000,
    structuralSharing: (old, next) =>
      mergeTasks(old as TaskSnapshot[] | undefined, next as TaskSnapshot[]),
  });
  useEffect(() => {
    let disposed = false;
    let off: (() => void) | undefined;
    let offWorkspace: (() => void) | undefined;
    let offDelta: (() => void) | undefined;
    void appService
      .listenWorkspace((status) => {
        client.setQueryData<DirectorySyncStatus>(["directory-sync"], (old) =>
          old && old.revision > status.revision ? old : status,
        );
      })
      .then((f) => {
        if (disposed) f();
        else offWorkspace = f;
      });
    void appService
      .listenWorkspaceDelta((delta) => {
        const current = client.getQueryData<WorkspaceSnapshot>(["workspace"]);
        const result = mergeWorkspaceDelta(current, delta);
        if (result.refresh)
          void client.invalidateQueries({ queryKey: ["workspace"] });
        else if (result.snapshot !== current)
          client.setQueryData(["workspace"], result.snapshot);
        if (delta.revision > (current?.revision ?? 0))
          void client.invalidateQueries({ queryKey: ["preset-detail"] });
      })
      .then((f) => {
        if (disposed) f();
        else {
          offDelta = f;
          // 订阅建立与首次查询之间可能有事件，补取一次确保基线完整。
          void client.invalidateQueries({ queryKey: ["workspace"] });
        }
      });
    void appService
      .listenTasks((t) => {
        client.setQueryData<TaskSnapshot[]>(["tasks"], (old = []) => {
          const prior = old.find((x) => x.id === t.id);
          if (prior && prior.revision >= t.revision) return old;
          return [...old.filter((x) => x.id !== t.id), t];
        });
        if (!activeTask(t)) {
          void client.invalidateQueries({ queryKey: ["workspace"] });
          for (const key of [
            IPC.auxiliary_status,
            IPC.plugin_status,
            IPC.workshop_list,
            IPC.update_download_status,
            IPC.migration_status,
          ])
            void client.invalidateQueries({ queryKey: [key] });
        }
      })
      .then((f) => {
        if (disposed) f();
        else off = f;
      });
    return () => {
      disposed = true;
      off?.();
      offWorkspace?.();
      offDelta?.();
    };
  }, [client]);
  return q;
}
export function useActions() {
  const client = useQueryClient();
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState("");
  const act = useCallback(
    async <T>(
      action: () => Promise<T>,
      message = "操作完成",
    ): Promise<T | undefined> => {
      setBusy(true);
      try {
        const result = await action();
        setNotice(message);
        await client.invalidateQueries({ queryKey: ["workspace"] });
        await client.invalidateQueries({ queryKey: ["tasks"] });
        return result;
      } catch (e) {
        setNotice(errorText(e));
        await client.invalidateQueries({ queryKey: ["workspace"] });
        return undefined;
      } finally {
        setBusy(false);
      }
    },
    [client],
  );
  return { act, busy, notice, setNotice, api: appService };
}
export function useDrop(handler: (event: DropEvent) => void) {
  useEffect(() => {
    let disposed = false;
    let off: (() => void) | undefined;
    void appService.onDrop(handler).then((f) => {
      if (disposed) f();
      else off = f;
    });
    return () => {
      disposed = true;
      off?.();
    };
  }, [handler]);
}
export const useImage = () => appService.image;
export function useNativeWindow() {
  return appService.window;
}
export function useBackend<T = any>(
  name: string,
  args: Record<string, unknown> = {},
) {
  return useQuery<T>({
    queryKey: [name, args],
    queryFn: () => appService.command(name, args),
    retry: false,
  });
}
type PreviewCacheStatus = {
  expectedFiles: number;
  missingFiles: number;
  bundles: Array<{ localVersion?: string | null }>;
  message?: string;
  error?: string;
};
export function usePreviewStatus(enabled = appService.native) {
  return useQuery<PreviewCacheStatus>({
    queryKey: [IPC.preview_status, {}],
    queryFn: () => appService.command(IPC.preview_status),
    enabled,
    staleTime: Infinity,
  });
}
