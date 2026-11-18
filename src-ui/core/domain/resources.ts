import type { ResourceRecord } from "../ipc/generated/ResourceRecord";
export type Route =
  | "overview"
  | "presets"
  | "heroes"
  | "cards"
  | "monsters"
  | "chips"
  | "lands"
  | "cursors"
  | "temporary"
  | "apx"
  | "workshop"
  | "author"
  | "auxiliary"
  | "debug"
  | "settings";
export const labels: Record<Route, string> = {
  overview: "总览",
  presets: "预设",
  heroes: "角色",
  cards: "卡面",
  monsters: "怪物",
  chips: "筹码",
  lands: "地块",
  cursors: "鼠标",
  temporary: "临时",
  apx: "APX 插件",
  workshop: "创意工坊",
  author: "投稿",
  auxiliary: "辅助插件",
  debug: "调试",
  settings: "设置",
};
export type Filter = "all" | "applied" | "has-mod" | "no-mod";
export type SearchMode = "smart" | "id";
export function searchText(r: ResourceRecord, mode: SearchMode) {
  const identity = `${r.key} ${r.entityId} ${r.canonical} ${r.fullName} ${r.files.map((f) => `${f.sourceName} ${f.hash}`).join(" ")}`;
  return (
    mode === "id"
      ? identity
      : `${identity} ${r.entityName} ${(r.aliases || []).join(" ")} ${r.resourceLabel} ${r.sectionLabel} ${r.variantLabel}`
  ).toLowerCase();
}
export function matchesSearch(
  r: ResourceRecord,
  query: string,
  mode: SearchMode = "smart",
) {
  return searchScore(r, query, mode) !== null;
}
function directSearch(r: ResourceRecord, query: string, mode: SearchMode) {
  const text = searchText(r, mode);
  return query
    .trim()
    .toLowerCase()
    .split(/\s+/)
    .filter(Boolean)
    .every((w) => text.includes(w));
}
// 编辑距离只用于名称和昵称，稳定身份只参与直接匹配及同分排序。
export function editDistance(a: string, b: string) {
  const left = [...a],
    right = [...b];
  let previous = right.map((_, i) => i + 1);
  previous.unshift(0);
  for (let i = 0; i < left.length; i++) {
    const next = [i + 1];
    for (let j = 0; j < right.length; j++)
      next.push(
        Math.min(
          next[j] + 1,
          previous[j + 1] + 1,
          previous[j] + (left[i] === right[j] ? 0 : 1),
        ),
      );
    previous = next;
  }
  return previous[right.length];
}
export function searchScore(
  r: ResourceRecord,
  query: string,
  mode: SearchMode = "smart",
): number | null {
  if (directSearch(r, query, mode)) return 0;
  const words = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (
    mode === "id" ||
    words.some(
      (w) =>
        [...w].length < 2 ||
        /^\d+$/.test(w) ||
        /^[0-9a-f]{8,}$/i.test(w) ||
        /[_:/\\]/.test(w),
    )
  )
    return null;
  const names = [
    r.entityName,
    ...(r.aliases || []),
    ...r.files.map((f) => f.sourceName),
  ]
    .filter((s) => s && s !== r.entityId && !s.startsWith(r.canonical))
    .map((s) => s.toLowerCase());
  let score = 0;
  for (const word of words) {
    if (searchText(r, mode).includes(word)) continue;
    const chars = [...word],
      threshold = chars.length > 5 ? 2 : 1;
    let best = threshold + 1;
    for (const name of names) {
      const tokens = name.split(/[\s_.\-:/\\]+/).filter(Boolean);
      for (const token of tokens) {
        if (/^\d+$/.test(token)) continue;
        if (/[\u3400-\u9fff]/.test(token)) {
          const cs = [...token];
          for (
            let size = Math.max(2, chars.length - threshold);
            size <= chars.length + threshold;
            size++
          ) {
            for (let i = 0; i + size <= cs.length; i++)
              best = Math.min(
                best,
                editDistance(word, cs.slice(i, i + size).join("")),
              );
          }
        } else if (Math.abs(token.length - chars.length) <= threshold)
          best = Math.min(best, editDistance(word, token));
      }
    }
    if (best > threshold) return null;
    score += best;
  }
  return score || 1;
}
export function rankSearch(
  rows: ResourceRecord[],
  query: string,
  mode: SearchMode = "smart",
) {
  if (!query.trim()) return rows.map((row) => ({ row, score: 0 }));
  return rows
    .map((row) => ({ row, score: searchScore(row, query, mode) }))
    .filter(
      (x): x is { row: ResourceRecord; score: number } => x.score !== null,
    )
    .sort(
      (a, b) =>
        a.score - b.score ||
        (a.row.key < b.row.key ? -1 : a.row.key > b.row.key ? 1 : 0),
    );
}
export function slotOfResource(r: ResourceRecord) {
  return {
    category: r.category,
    heroId: r.entityId,
    sectionKey: r.section,
    canonical: r.canonical,
  };
}
export function resourceGroupKey(
  r: ResourceRecord,
  route: string = r.category,
) {
  return ["heroes", "monsters", "chips"].includes(route)
    ? r.entityId
    : ["lands", "cursors"].includes(route)
      ? r.groupKey
      : r.key;
}
export function variantGroups(rows: ResourceRecord[], byPart = false) {
  const groups = new Map<
    string,
    { key: string; label: string; order: number; rows: ResourceRecord[] }
  >();
  for (const r of rows) {
    const key = byPart ? r.section : r.variantKey;
    const group = groups.get(key) || {
      key,
      label: byPart ? r.sectionLabel : r.variantLabel,
      order: byPart ? 0 : r.variantOrder,
      rows: [],
    };
    group.rows.push(r);
    groups.set(key, group);
  }
  return [...groups.values()]
    .sort((a, b) => a.order - b.order)
    .map((g) => ({
      ...g,
      rows: g.rows.sort(
        (a, b) =>
          a.variantOrder - b.variantOrder ||
          a.section.localeCompare(b.section) ||
          a.canonical.localeCompare(b.canonical, undefined, { numeric: true }),
      ),
    }));
}
export function storageCategory(r: ResourceRecord) {
  return r.category === "monsters" ? "monsters" : "heroes";
}
export function isResourceApplied(r: ResourceRecord) {
  return !!r.activeHash && !r.disabled;
}
export function matchesResourceFilter(r: ResourceRecord, filter: Filter) {
  return (
    filter === "all" ||
    (filter === "applied" && isResourceApplied(r)) ||
    (filter === "has-mod" && r.files.length > 0) ||
    (filter === "no-mod" && r.files.length === 0)
  );
}
export function filterResources(
  rows: ResourceRecord[],
  route: Route,
  search: string,
  filter: Filter,
  section: string,
  mode: SearchMode = "smart",
) {
  const filtered = rows.filter(
    (r) =>
      r.category === route &&
      (!section || r.section === section) &&
      matchesResourceFilter(r, filter),
  );
  return rankSearch(filtered, search, mode).map((x) => x.row);
}
export function groupResources(rows: ResourceRecord[], route: Route) {
  const groups = new Map<string, ResourceRecord[]>();
  for (const r of rows) {
    const key = resourceGroupKey(r, route);
    groups.set(key, [...(groups.get(key) || []), r]);
  }
  return [...groups.entries()].map(([key, resources]) => ({ key, resources }));
}
export type ResourceGroup = ReturnType<typeof groupResources>[number];
export function resourceGroupSummary({ resources }: ResourceGroup) {
  return {
    cover:
      resources.find(
        (r) => r.section === "card" || r.section === "monster_card",
      ) || resources[0],
    mods: resources.reduce((total, r) => total + r.files.length, 0),
    applied: resources.filter(isResourceApplied).length,
  };
}
// 角色先按完整实体判断状态；搜索和部位只决定是否展示，不裁剪封面与统计。
export function filterResourceGroups(
  rows: ResourceRecord[],
  route: Route,
  search: string,
  filter: Filter,
  section: string,
  mode: SearchMode = "smart",
) {
  const fullGroups = groupResources(
    rows.filter((r) => r.category === route),
    route,
  );
  const visibleRows = filterResources(
    rows,
    route,
    search,
    route === "heroes" ? "all" : filter,
    section,
    mode,
  );
  const order = new Map<string, number>();
  for (const row of visibleRows) {
    const key = resourceGroupKey(row, route);
    if (!order.has(key)) order.set(key, order.size);
  }
  return fullGroups
    .filter((group) => {
      if (!order.has(group.key)) return false;
      if (route !== "heroes" || filter === "all") return true;
      const summary = resourceGroupSummary(group);
      return filter === "applied"
        ? summary.applied > 0
        : filter === "has-mod"
          ? summary.mods > 0
          : summary.mods === 0;
    })
    .sort((a, b) => order.get(a.key)! - order.get(b.key)!);
}
export function bytes(n: number) {
  if (n < 1024) return `${n} B`;
  if (n < 1024 ** 2) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 ** 2).toFixed(1)} MB`;
}
export function errorText(e: unknown) {
  return e instanceof Error
    ? e.message
    : typeof e === "string"
      ? e
      : JSON.stringify(e);
}
