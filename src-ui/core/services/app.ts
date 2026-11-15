import { IPC, EVENTS } from "../ipc/commands";
import * as transport from "../ipc/transport";
import { mockCall } from "../ipc/mock";
import type { WorkspaceSnapshot } from "../ipc/generated/WorkspaceSnapshot";
import type { ResourceAction } from "../ipc/generated/ResourceAction";
import type { TaskSnapshot } from "../ipc/generated/TaskSnapshot";
import { orderedQueue } from "../domain/applyQueue";
const enqueueApply = orderedQueue();
// 结果页等待真实终态；界面关闭时由调用方停止等待，后台任务仍由后端任务队列管理。
async function waitTask(
  id: string,
  signal?: AbortSignal,
): Promise<TaskSnapshot> {
  while (!signal?.aborted) {
    const task = (await call<TaskSnapshot[]>(IPC.tasks_list)).find(
      (t) => t.id === id,
    );
    if (task && !["queued", "running"].includes(task.status)) return task;
    await new Promise<void>((resolve) => setTimeout(resolve, 500));
  }
  throw new Error("已停止等待任务结果");
}
// 旧版命令的动态载荷在兼容边界集中转换；新契约使用生成类型。
export const call = <T = any>(
  name: string,
  args: Record<string, unknown> = {},
) =>
  transport.native ? transport.invoke<T>(name, args) : mockCall<T>(name, args);
export const appService = {
  snapshot: () => call<WorkspaceSnapshot>(IPC.workspace_snapshot),
  tasks: () => call<TaskSnapshot[]>(IPC.tasks_list),
  apply: (items: ResourceAction[]) =>
    enqueueApply(() => call(IPC.resources_apply, { items })),
  task: (kind: string, title: string, args: Record<string, unknown> = {}) =>
    call<string>(IPC.task_execute, { kind, title, args }),
  cancel: (id: string) => call(IPC.task_cancel, { id }),
  waitTask,
  settings: (patch: Record<string, unknown>) =>
    call(IPC.settings_set, { patch }),
  command: call,
  listenTasks: async (handler: (task: TaskSnapshot) => void) =>
    transport.native
      ? transport.listen<TaskSnapshot>(EVENTS.taskProgress, (e) =>
          handler(e.payload),
        )
      : () => {},
  listenWorkspace: async (handler: () => void) =>
    transport.native
      ? transport.listen(EVENTS.workspaceChanged, handler)
      : () => {},
  onDrop: transport.fileDrop,
  chooseDirectory: transport.pickDirectory,
  chooseFiles: transport.pickFiles,
  saveFile: transport.saveFile,
  image: transport.imageUrl,
  window: transport.windowAction,
  native: transport.native,
  openExternal: async (url: string) => {
    if (transport.native) await call(IPC.open_external, { url });
    else window.open(url, "_blank", "noopener,noreferrer");
  },
};
