import { IPC } from "../core/ipc/commands";
import { useEffect, useState, useCallback } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { appService } from "../core/services/app";
import type { TaskSnapshot } from "../core/ipc/generated/TaskSnapshot";
import { errorText } from "../core/domain/resources";
import { mergeTasks, activeTask } from "../core/domain/tasks";
export function useWorkspace() {
  return useQuery({
    queryKey: ["workspace"],
    queryFn: appService.snapshot,
    staleTime: 15000,
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
    void appService
      .listenWorkspace(() => {
        void client.invalidateQueries({ queryKey: ["workspace"] });
        void client.invalidateQueries({ queryKey: ["preset-detail"] });
      })
      .then((f) => {
        if (disposed) f();
        else offWorkspace = f;
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
export function useDrop(handler: (paths: string[]) => void) {
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
