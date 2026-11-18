import { describe, expect, it } from "vitest";
import { acceptWorkspaceSnapshot, mergeWorkspaceDelta } from "./workspace";
import type { WorkspaceSnapshot } from "../ipc/generated/WorkspaceSnapshot";
import type { ResourceRecord } from "../ipc/generated/ResourceRecord";
import type { WorkspaceDelta } from "../ipc/generated/WorkspaceDelta";

const row = (key: string): ResourceRecord =>
  ({ key, files: [] }) as unknown as ResourceRecord;
const snapshot = (): WorkspaceSnapshot =>
  ({ revision: 3, resources: [row("a"), row("b")] }) as WorkspaceSnapshot;
const event = (patch: Partial<WorkspaceDelta> = {}): WorkspaceDelta => ({
  baseRevision: 3,
  revision: 4,
  resources: [row("c")],
  removedKeys: ["b"],
  fullRefresh: false,
  ...patch,
});
describe("工作区增量事件", () => {
  it("只替换变化行并保留未变化行身份", () => {
    const before = snapshot();
    const merged = mergeWorkspaceDelta(before, event());
    expect(merged.refresh).toBe(false);
    expect(merged.snapshot?.resources.map((r) => r.key)).toEqual(["a", "c"]);
    expect(merged.snapshot?.resources[0]).toBe(before.resources[0]);
    expect(before.resources.map((r) => r.key)).toEqual(["a", "b"]);
  });
  it("版本断档或配置变化必须重取完整快照", () => {
    expect(
      mergeWorkspaceDelta(snapshot(), event({ baseRevision: 2 })).refresh,
    ).toBe(true);
    expect(
      mergeWorkspaceDelta(snapshot(), event({ fullRefresh: true })).refresh,
    ).toBe(true);
    expect(mergeWorkspaceDelta(undefined, event()).refresh).toBe(true);
  });
  it("重复或迟到事件不能回退状态", () => {
    const current = snapshot();
    expect(mergeWorkspaceDelta(current, event({ revision: 3 })).snapshot).toBe(
      current,
    );
    expect(mergeWorkspaceDelta(current, event({ revision: 2 })).refresh).toBe(
      false,
    );
    expect(
      acceptWorkspaceSnapshot(current, {
        ...current,
        revision: 2,
        resources: [],
      }),
    ).toBe(current);
  });
});
