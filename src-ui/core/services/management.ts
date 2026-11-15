// 管理契约的类型转换集中于服务，界面查询只持有 Rust 状态的展示快照。
import { call } from "./app";
import { IPC } from "../ipc/commands";
import type { GlobalPreset } from "../ipc/generated/GlobalPreset";
import type { PresetItem } from "../ipc/generated/PresetItem";
import type { ResourceSlot } from "../ipc/generated/ResourceSlot";
import type { PresetPreview } from "../ipc/generated/PresetPreview";
export const managementService = {
  detail: (id: string) => call<PresetPreview>(IPC.preset_detail, { id }),
  create: (name: string, scope: string, selected: ResourceSlot[]) =>
    call<GlobalPreset>(IPC.preset_create, { name, scope, selected }),
  update: (id: string, name: string, items: PresetItem[]) =>
    call<GlobalPreset>(IPC.preset_update, { id, name, items }),
  copy: (id: string) => call<GlobalPreset>(IPC.preset_copy, { id }),
  lock: (id: string, locked: boolean) =>
    call<void>(IPC.preset_lock, { id, locked }),
  remove: (id: string) => call<void>(IPC.preset_delete, { id }),
  apply: (id: string) => call<void>(IPC.preset_apply, { id }),
};
