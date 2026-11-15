// 仅浏览器开发预览使用的内存夹具；正式桌面始终由 Rust IPC 维护状态。
import type { WorkspaceSnapshot } from "./generated/WorkspaceSnapshot";
import type { GlobalPreset } from "./generated/GlobalPreset";
import type { PresetPreview } from "./generated/PresetPreview";
import { IPC } from "./commands";
import { slotOfResource } from "../domain/resources";
const presets = new Map<string, GlobalPreset>();
export function mockManagement(
  command: string,
  args: Record<string, any>,
  workspace: WorkspaceSnapshot,
): { handled: boolean; value?: unknown } {
  const current = () => {
    const p = presets.get(args.id);
    if (!p) throw Error("预设不存在");
    return p;
  };
  const editable = () => {
    const p = current();
    if (p.locked) throw Error("预设已锁定");
    return p;
  };
  const summaries = () => {
    workspace.settings.presets = [...presets.values()].map(({ id, name }) => ({
      id,
      name,
    }));
  };
  let value: unknown;
  if (command === IPC.preset_create || command === IPC.preset_save) {
    const rows = workspace.resources.filter(
      (r) =>
        r.category !== "temporary" &&
        (args.scope !== "local" ||
          args.selected.some((s: any) => s.canonical === r.canonical)),
    );
    const p: GlobalPreset = {
      id: crypto.randomUUID(),
      name: args.name,
      scope: args.scope || "global",
      locked: false,
      items: rows.map((r) => ({
        ...slotOfResource(r),
        hash: r.disabled ? "" : r.activeHash || "",
        displayName:
          r.files.find((f) => f.hash === r.activeHash)?.sourceName ||
          r.canonical,
      })),
    };
    presets.set(p.id, p);
    summaries();
    value = p;
  } else if (command === IPC.preset_detail) {
    const p = current();
    const rows = workspace.resources.filter(
      (r) =>
        r.category !== "temporary" &&
        (p.scope === "global" ||
          p.items.some((i) => i.canonical === r.canonical)),
    );
    const changes = rows.map((r) => {
      const item = p.items.find((i) => i.canonical === r.canonical) || {
        ...slotOfResource(r),
        hash: "",
        displayName: r.canonical,
      };
      const f = r.files.find((f) => f.hash === item.hash);
      const issues = item.hash && !f ? ["保存的候选缺失"] : [];
      return {
        item,
        savedPath: f?.path || null,
        currentPath: r.files.find((f) => f.hash === r.activeHash)?.path || null,
        currentHash: r.disabled ? null : r.activeHash,
        changed: (r.disabled ? "" : r.activeHash || "") !== item.hash,
        issues,
      };
    });
    const issues = changes.flatMap((c) => c.issues);
    value = {
      preset: p,
      changes,
      issues,
      canApply: !issues.length,
    } satisfies PresetPreview;
  } else if (command === IPC.preset_update) {
    const p = editable();
    p.name = args.name;
    p.items = args.items;
    summaries();
    value = p;
  } else if (command === IPC.preset_lock) current().locked = args.locked;
  else if (command === IPC.preset_copy) {
    const p = {
      ...structuredClone(current()),
      id: crypto.randomUUID(),
      name: current().name + " 副本",
      locked: false,
    };
    presets.set(p.id, p);
    summaries();
    value = p;
  } else if (command === IPC.preset_delete) {
    editable();
    presets.delete(args.id);
    summaries();
  } else if (command === IPC.preset_apply) {
    const p = current();
    for (const row of workspace.resources) {
      const item = p.items.find((i) => i.canonical === row.canonical);
      if (p.scope === "global" || item) {
        row.activeHash = item?.hash || null;
        row.disabled = false;
      }
    }
  } else if (
    command === IPC.candidate_info_set ||
    command === IPC.rename_file ||
    command === IPC.candidate_classify
  ) {
    const r = workspace.resources.find(
      (r) => r.canonical === args.args.canonical,
    );
    const f = r?.files.find((f) => f.hash === args.args.hash);
    if (!r || !f) throw Error("候选不存在");
    if (command === IPC.candidate_info_set) f.info = args.info;
    else if (command === IPC.rename_file) {
      f.sourceName = args.args.name;
      value = f.sourceName;
    } else if (!args.keepCurrent) {
      const to = workspace.resources.find(
        (r) => r.canonical === args.target.canonical,
      );
      if (!to) throw Error("槽位不存在");
      r.files = r.files.filter((x) => x !== f);
      to.files.push(f);
    }
  } else return { handled: false };
  return { handled: true, value: structuredClone(value) };
}
