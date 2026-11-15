// 用修订号合并事件与轮询快照，避免较慢的请求覆盖已经显示的新状态。
import type { TaskSnapshot } from "../ipc/generated/TaskSnapshot";
export const activeTask = (t: TaskSnapshot) =>
  ["queued", "running"].includes(t.status);
export function mergeTasks(
  previous: TaskSnapshot[] = [],
  incoming: TaskSnapshot[] = [],
) {
  const tasks = new Map(previous.map((t) => [t.id, t]));
  for (const t of incoming) {
    const old = tasks.get(t.id);
    if (!old || old.revision < t.revision) tasks.set(t.id, t);
  }
  return [...tasks.values()].sort((a, b) => a.id.localeCompare(b.id));
}
