// 拖入按完整目录规则、明确落点、未知归属依次分流，页面筛选不参与判断。
import type { ImageDropMatch } from "../ipc/generated/ImageDropMatch";
import type { ResourceRecord } from "../ipc/generated/ResourceRecord";
import type { ImportTarget } from "./packages";
import { uniquePackages } from "./packages";

export type DropEvent = {
  type: "enter" | "over" | "leave" | "drop";
  paths?: string[];
  position?: { x: number; y: number };
};
export type DropTarget = ImportTarget | { temporary: true };
export type DropResult = {
  file: string;
  ok: boolean;
  duplicate: boolean;
  applied: boolean;
  applyError?: string | null;
  reason?: string | null;
};
export function rowTarget(row: ResourceRecord): ImportTarget | undefined {
  return row.applicationSlot && row.section !== "temporary"
    ? {
        heroId: row.entityId,
        sectionKey: row.section,
        canonical: row.canonical,
      }
    : undefined;
}
export function planDrop(
  paths: string[],
  matches: ImageDropMatch[],
  landing?: ImportTarget,
) {
  const unique = [
    ...new Map(
      paths.map((p) => [p.replaceAll("\\", "/").toLowerCase(), p]),
    ).values(),
  ];
  const rules = new Map(matches.map((m) => [m.path, m.target]));
  const images = unique.filter((p) => /\.png$/i.test(p));
  const targets: Record<string, ImportTarget> = {};
  const known: string[] = [],
    unknown: string[] = [];
  for (const path of images) {
    const rule = rules.get(path);
    const target = rule
      ? {
          heroId: rule.heroId,
          sectionKey: rule.sectionKey,
          canonical: rule.canonical,
        }
      : landing;
    if (target) {
      known.push(path);
      targets[path] = target;
    } else unknown.push(path);
  }
  return {
    known,
    unknown,
    targets,
    packages: uniquePackages(unique),
    unsupported: unique.filter((p) => !/\.(png|apmod)$/i.test(p)),
  };
}
export function dropSummary(results: DropResult[]) {
  const count = {
    imported: results.filter((r) => r.ok && !r.duplicate).length,
    duplicate: results.filter((r) => r.ok && r.duplicate).length,
    failed: results.filter((r) => !r.ok).length,
    applied: results.filter((r) => r.applied).length,
    applyFailed: results.filter((r) => r.ok && r.applyError).length,
  };
  return `新入库 ${count.imported} · 重复 ${count.duplicate} · 导入失败 ${count.failed} · 已应用 ${count.applied} · 应用失败 ${count.applyFailed}`;
}
