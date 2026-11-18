import { IPC, EVENTS } from "../ipc/commands";
import * as transport from "../ipc/transport";
import { mockCall } from "../ipc/mock";
import type { WorkspaceSnapshot } from "../ipc/generated/WorkspaceSnapshot";
import type { ResourceAction } from "../ipc/generated/ResourceAction";
import type { TaskSnapshot } from "../ipc/generated/TaskSnapshot";
import type { DirectorySyncStatus } from "../ipc/generated/DirectorySyncStatus";
import type { WorkspaceDelta } from "../ipc/generated/WorkspaceDelta";
import type { ImageDropMatch } from "../ipc/generated/ImageDropMatch";
import type { DropTarget, DropResult } from "../domain/dropImport";
import { orderedQueue } from "../domain/applyQueue";
const enqueueApply = orderedQueue();
let latestWorkspaceRevision = 0;
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
  snapshot: async () => {
    let result = await call<WorkspaceSnapshot>(IPC.workspace_snapshot);
    if (result.revision < latestWorkspaceRevision)
      result = await call<WorkspaceSnapshot>(IPC.workspace_snapshot);
    return result;
  },
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
  listenWorkspace: async (handler: (status: DirectorySyncStatus) => void) =>
    transport.native
      ? transport.listen<DirectorySyncStatus>(
          EVENTS.workspaceChanged,
          (event) => handler(event.payload),
        )
      : () => {},
  listenWorkspaceDelta: async (handler: (delta: WorkspaceDelta) => void) =>
    transport.native
      ? transport.listen<WorkspaceDelta>(EVENTS.workspaceUpdated, (event) => {
          latestWorkspaceRevision = Math.max(
            latestWorkspaceRevision,
            event.payload.revision,
          );
          handler(event.payload);
        })
      : () => {},
  onDrop: transport.fileDrop,
  matchDrop: (paths: string[]) =>
    call<ImageDropMatch[]>(IPC.images_match_drop, { paths }),
  importDrop: async (
    paths: string[],
    targets: Record<string, DropTarget>,
    signal?: AbortSignal,
    expectedHashes?: Record<string, string>,
  ) => {
    if (!transport.native)
      throw new Error(
        "请在桌面版拖入本地 PNG／APMod；浏览器预览只展示拖入反馈。",
      );
    const id = await appService.task("images-import", "拖入 PNG", {
      paths,
      targets,
      dropMode: true,
      expectedHashes,
    });
    const task = await waitTask(id, signal);
    if (task.status !== "completed") throw new Error(task.message);
    if (!Array.isArray(task.result)) throw new Error("图片导入结果无效");
    return task.result as unknown as DropResult[];
  },
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
