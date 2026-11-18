// 增量只能接在匹配版本上；缺失基线时重新获取快照，迟到事件不能回退展示状态。
import type { WorkspaceSnapshot } from "../ipc/generated/WorkspaceSnapshot";
import type { WorkspaceDelta } from "../ipc/generated/WorkspaceDelta";

export function mergeWorkspaceDelta(
  current: WorkspaceSnapshot | undefined,
  delta: WorkspaceDelta,
): { snapshot: WorkspaceSnapshot | undefined; refresh: boolean } {
  if (!current) return { snapshot: current, refresh: true };
  if (delta.revision <= current.revision)
    return { snapshot: current, refresh: false };
  if (delta.fullRefresh || delta.baseRevision !== current.revision)
    return { snapshot: current, refresh: true };
  const updates = new Map(delta.resources.map((r) => [r.key, r]));
  const removed = new Set(delta.removedKeys);
  const resources = current.resources
    .filter((r) => !removed.has(r.key))
    .map((r) => {
      const next = updates.get(r.key) ?? r;
      updates.delete(r.key);
      return next;
    });
  resources.push(...updates.values());
  resources.sort((a, b) => (a.key < b.key ? -1 : a.key > b.key ? 1 : 0));
  return {
    snapshot: { ...current, revision: delta.revision, resources },
    refresh: false,
  };
}

export function acceptWorkspaceSnapshot(
  current: WorkspaceSnapshot | undefined,
  next: WorkspaceSnapshot,
): WorkspaceSnapshot {
  return current && next.revision < current.revision ? current : next;
}
