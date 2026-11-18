import type { ListedWork } from "../ipc/generated/ListedWork";
export type WorkStatus = "new" | "installed" | "update" | "modified";
export function workStatus(item: ListedWork): WorkStatus {
  if (item.locallyModified) return "modified";
  if (!item.installedVersion) return "new";
  return item.updateAvailable || item.installedVersion !== item.work.version
    ? "update"
    : "installed";
}
export const workStatusLabel: Record<WorkStatus, string> = {
  new: "未获取",
  installed: "已加入本地库",
  update: "有新版本",
  modified: "本地已修改",
};
export function filterWorks(
  items: ListedWork[],
  search: string,
  category: string,
  state: string,
  sort: string,
) {
  const terms = search.trim().toLocaleLowerCase().split(/\s+/).filter(Boolean);
  return items
    .filter((item) => {
      const work = item.work;
      const text = [
        work.title,
        work.description,
        work.author,
        work.artist,
        ...work.tags.map((t) => t.name),
      ]
        .join(" ")
        .toLocaleLowerCase();
      return (
        terms.every((term) => text.includes(term)) &&
        (!category || work.category === category) &&
        (!state || workStatus(item) === state)
      );
    })
    .sort((a, b) =>
      sort === "name"
        ? a.work.title.localeCompare(b.work.title, "zh")
        : Number(b.work.updatedAt) - Number(a.work.updatedAt),
    );
}
